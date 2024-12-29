use solana_program::{
    account_info::next_account_info,
    program::{invoke, invoke_signed},
    program_pack::Pack,
};
use spl_token::{instruction::close_account, state::Account as SplTokenAccount};
use steel::*;
use yoko_program_api::prelude::*;

const JUPITER_PROGRAM_ID: &str = "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4";

pub fn process_swap<'a>(accounts: &'a [AccountInfo<'a>], data: &[u8]) -> ProgramResult {
    let (in_amount, jupiter_route_cpi_data) = parse_instruction_data(data)?;

    let accounts = SwapAccounts::new(accounts)?;
    let (fund, fund_pda_bump) = accounts.validate()?;

    transfer_from_fund_to_user_source_ata(&accounts, in_amount)?;
    let out_amount = execute_jupiter_swap(&accounts, jupiter_route_cpi_data, in_amount)?;
    transfer_from_user_destination_ata_to_fund(&accounts, out_amount)?;
    maybe_close_token_accounts(&accounts, fund, fund_pda_bump)?;

    Ok(())
}

fn parse_instruction_data(data: &[u8]) -> Result<(u64, &[u8]), ProgramError> {
    if data.len() < 16 {
        return Err(ProgramError::InvalidInstructionData);
    }

    let (in_amount, jupiter_route_cpi_data) = data.split_at(8);
    let in_amount = in_amount
        .get(..8)
        .and_then(|slice| slice.try_into().ok())
        .map(u64::from_le_bytes)
        .ok_or(ProgramError::InvalidInstructionData)?;

    Ok((in_amount, jupiter_route_cpi_data))
}

struct SwapAccounts<'a> {
    fund_authority: &'a AccountInfo<'a>,
    fund: &'a AccountInfo<'a>,
    fund_destination_token_account: &'a AccountInfo<'a>,
    fund_source_token_account: &'a AccountInfo<'a>,
    jupiter_program: &'a AccountInfo<'a>,
    jupiter_accounts: Vec<AccountInfo<'a>>,
}

impl<'a> SwapAccounts<'a> {
    fn new(accounts: &'a [AccountInfo<'a>]) -> Result<Self, ProgramError> {
        let account_info_iter = &mut accounts.iter();

        Ok(Self {
            fund_authority: next_account_info(account_info_iter)?,
            fund: next_account_info(account_info_iter)?,
            fund_destination_token_account: next_account_info(account_info_iter)?,
            fund_source_token_account: next_account_info(account_info_iter)?,
            jupiter_program: next_account_info(account_info_iter)?,
            jupiter_accounts: accounts.iter().cloned().skip(5).collect(),
        })
    }

    fn validate(&self) -> Result<(&mut Fund, u8), ProgramError> {
        self.fund_authority.is_signer()?;

        if *self.jupiter_program.key != JUPITER_PROGRAM_ID.parse::<Pubkey>().unwrap() {
            return Err(YokoProgramError::InvalidAccount.into());
        }

        let fund_pda = fund_pda(&self.fund_authority.key);
        if *self.fund.key != fund_pda.0 {
            return Err(YokoProgramError::InvalidAccount.into());
        }

        let fund = self
            .fund
            .as_account_mut::<Fund>(&yoko_program_api::ID)?
            .assert_mut(|fund| fund.authority == *self.fund_authority.key)?;

        let fund_destination_token_account_data =
            SplTokenAccount::unpack(&self.fund_destination_token_account.data.borrow())?;

        if fund_destination_token_account_data.owner != *self.fund.key {
            return Err(YokoProgramError::InvalidAccount.into());
        }

        Ok((fund, fund_pda.1))
    }

    fn token_program(&self) -> &AccountInfo<'a> {
        &self.jupiter_accounts[0]
    }

    fn user_source_ata(&self) -> &AccountInfo<'a> {
        &self.jupiter_accounts[2]
    }

    fn user_destination_ata(&self) -> &AccountInfo<'a> {
        &self.jupiter_accounts[3]
    }
}

fn transfer_from_fund_to_user_source_ata(accounts: &SwapAccounts, amount: u64) -> ProgramResult {
    transfer_signed(
        accounts.fund,
        accounts.fund_source_token_account,
        accounts.user_source_ata(),
        accounts.token_program(),
        amount,
        &[FUND, accounts.fund_authority.key.as_ref()],
    )?;

    Ok(())
}

fn execute_jupiter_swap(
    accounts: &SwapAccounts,
    jupiter_route_cpi_data: &[u8],
    amount: u64,
) -> Result<u64, ProgramError> {
    let user_source_ata_amount_before_swap =
        SplTokenAccount::unpack(&accounts.user_source_ata().data.borrow())?.amount;

    let user_destination_ata_amount_before_swap =
        SplTokenAccount::unpack(&accounts.user_destination_ata().data.borrow())?.amount;

    let jup_accounts_metas: Vec<AccountMeta> = accounts
        .jupiter_accounts
        .iter()
        .map(|acc| AccountMeta {
            pubkey: *acc.key,
            is_signer: acc.is_signer,
            is_writable: acc.is_writable,
        })
        .collect();

    invoke(
        &Instruction {
            program_id: *accounts.jupiter_program.key,
            accounts: jup_accounts_metas,
            data: jupiter_route_cpi_data.to_vec(),
        },
        accounts.jupiter_accounts.as_slice(),
    )?;

    let user_source_ata_amount_after_swap =
        SplTokenAccount::unpack(&accounts.user_source_ata().data.borrow())?.amount;

    let user_destination_ata_amount_after_swap =
        SplTokenAccount::unpack(&accounts.user_destination_ata().data.borrow())?.amount;

    let source_difference = user_source_ata_amount_before_swap
        .checked_sub(user_source_ata_amount_after_swap)
        .ok_or(YokoProgramError::InvalidAmount)?;

    if source_difference != amount {
        return Err(YokoProgramError::InvalidAmount.into());
    }

    let destination_difference = user_destination_ata_amount_after_swap
        .checked_sub(user_destination_ata_amount_before_swap)
        .ok_or(YokoProgramError::InvalidAmount)?;

    Ok(destination_difference)
}

fn transfer_from_user_destination_ata_to_fund(
    accounts: &SwapAccounts,
    amount: u64,
) -> ProgramResult {
    transfer(
        accounts.fund_authority,
        accounts.user_destination_ata(),
        accounts.fund_destination_token_account,
        accounts.token_program(),
        amount,
    )
}

fn maybe_close_token_accounts(
    accounts: &SwapAccounts,
    fund: &mut Fund,
    fund_pda_bump: u8,
) -> ProgramResult {
    let user_source_ata_amount =
        SplTokenAccount::unpack(&accounts.user_source_ata().data.borrow())?.amount;

    if user_source_ata_amount <= 0 {
        invoke(
            &close_account(
                accounts.token_program().key,
                accounts.user_source_ata().key,
                accounts.fund_authority.key,
                accounts.fund_authority.key,
                &[accounts.fund_authority.key],
            )?,
            &[
                accounts.user_source_ata().clone(),
                accounts.fund_authority.clone(),
                accounts.fund_authority.clone(),
                accounts.token_program().clone(),
            ],
        )?;
    }

    let user_destination_ata_amount =
        SplTokenAccount::unpack(&accounts.user_destination_ata().data.borrow())?.amount;

    if user_destination_ata_amount <= 0 {
        invoke(
            &close_account(
                accounts.token_program().key,
                accounts.user_destination_ata().key,
                accounts.fund_authority.key,
                accounts.fund_authority.key,
                &[accounts.fund_authority.key],
            )?,
            &[
                accounts.user_destination_ata().clone(),
                accounts.fund_authority.clone(),
                accounts.fund_authority.clone(),
                accounts.token_program().clone(),
            ],
        )?;
    }

    let fund_source_token_account_data =
        SplTokenAccount::unpack(&accounts.fund_source_token_account.data.borrow())?;

    if fund_source_token_account_data.mint != fund.main_mint
        && fund_source_token_account_data.amount <= 0
    {
        invoke_signed(
            &close_account(
                accounts.token_program().key,
                accounts.fund_source_token_account.key,
                accounts.fund_authority.key,
                accounts.fund.key,
                &[accounts.fund.key],
            )?,
            &[
                accounts.fund_source_token_account.clone(),
                accounts.fund_authority.clone(),
                accounts.fund.clone(),
                accounts.token_program().clone(),
            ],
            &[&[FUND, accounts.fund_authority.key.as_ref(), &[fund_pda_bump]]],
        )?;

        let removed = fund
            .other_mints
            .remove(&fund_source_token_account_data.mint);
        if !removed {
            return Err(YokoProgramError::ErrorRemovingOtherMint.into());
        }
    }

    Ok(())
}
