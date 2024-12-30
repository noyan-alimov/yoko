use std::str::FromStr;

use solana_program::program::invoke;
use spl_associated_token_account::{
    solana_program::program_pack::Pack, tools::account::create_pda_account,
};
use spl_token::{instruction::initialize_account3, state::Account as SplTokenAccount};
use steel::*;
use sysvar::rent::Rent;
use yoko_program_api::prelude::*;

const PROTOCOL_FEE: u64 = 1; // 1%
const PROTOCOL_FEE_TOKEN_ACCOUNT_OWNER: &str = "H61JjSDPCwvAs1k2vaPAX6d917Pu4dPWykcexvXXzGph";

pub fn process_create_payout(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    let args = CreatePayout::try_from_bytes(data)?;
    let amount = u64::from_le_bytes(args.amount);

    let [fund_authority_info, fund_authority_token_account_info, fund_info, fund_main_token_account_info, payout_info, payout_main_token_account_info, main_mint_info, protocol_fee_token_account_info, token_program, system_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    let protocol_fee_token_account_data =
        SplTokenAccount::unpack(&protocol_fee_token_account_info.data.borrow())?;

    if protocol_fee_token_account_data.owner
        != Pubkey::from_str(PROTOCOL_FEE_TOKEN_ACCOUNT_OWNER).unwrap()
    {
        return Err(ProgramError::InvalidAccountData);
    }

    if protocol_fee_token_account_data.mint != *main_mint_info.key {
        return Err(ProgramError::InvalidAccountData);
    }

    fund_authority_info.is_signer()?.is_writable()?;

    let fund = fund_info
        .is_writable()?
        .has_seeds(
            &[FUND, fund_authority_info.key.as_ref()],
            &yoko_program_api::ID,
        )?
        .as_account_mut::<Fund>(&yoko_program_api::ID)?
        .assert_mut(|fund| fund.authority == *fund_authority_info.key)?
        .assert_mut(|fund| fund.main_mint == *main_mint_info.key)?;

    fund.payouts_counter = fund
        .payouts_counter
        .checked_add(1)
        .ok_or(ProgramError::InvalidArgument)?;

    payout_info.is_writable()?.has_seeds(
        &[
            PAYOUT,
            fund_info.key.as_ref(),
            &fund.payouts_counter.to_le_bytes(),
        ],
        &yoko_program_api::ID,
    )?;

    create_account::<Payout>(
        payout_info,
        system_program,
        fund_authority_info,
        &yoko_program_api::ID,
        &[
            PAYOUT,
            fund_info.key.as_ref(),
            &fund.payouts_counter.to_le_bytes(),
        ],
    )?;

    let payout = payout_info.as_account_mut::<Payout>(&yoko_program_api::ID)?;
    payout.total_deposited = fund.total_deposited;

    let payout_main_token_account = payout_token_account_pda(payout_info.key);
    if payout_main_token_account_info.key != &payout_main_token_account.0 {
        return Err(ProgramError::InvalidSeeds);
    }

    let rent = Rent::get()?;
    create_pda_account(
        fund_authority_info,
        &rent,
        SplTokenAccount::LEN,
        token_program.key,
        system_program,
        payout_main_token_account_info,
        &[
            &PAYOUT[..],
            payout_info.key.as_ref(),
            &[payout_main_token_account.1],
        ],
    )?;

    invoke(
        &initialize_account3(
            token_program.key,
            payout_main_token_account_info.key,
            main_mint_info.key,
            payout_info.key,
        )?,
        &[
            payout_main_token_account_info.clone(),
            main_mint_info.clone(),
            token_program.clone(),
        ],
    )?;

    let authority_amount = amount
        .checked_mul(fund.authority_fee)
        .ok_or(ProgramError::InvalidArgument)?
        .checked_div(100)
        .ok_or(ProgramError::InvalidArgument)?;

    let protocol_fee_amount = amount
        .checked_mul(PROTOCOL_FEE)
        .ok_or(ProgramError::InvalidArgument)?
        .checked_div(100)
        .ok_or(ProgramError::InvalidArgument)?;

    let rest_amount = amount
        .checked_sub(authority_amount)
        .ok_or(ProgramError::InvalidArgument)?
        .checked_sub(protocol_fee_amount)
        .ok_or(ProgramError::InvalidArgument)?;

    transfer_signed(
        fund_info,
        fund_main_token_account_info,
        fund_authority_token_account_info,
        token_program,
        authority_amount,
        &[FUND, fund_authority_info.key.as_ref()],
    )?;

    transfer_signed(
        fund_info,
        fund_main_token_account_info,
        protocol_fee_token_account_info,
        token_program,
        protocol_fee_amount,
        &[FUND, fund_authority_info.key.as_ref()],
    )?;

    transfer_signed(
        fund_info,
        fund_main_token_account_info,
        payout_main_token_account_info,
        token_program,
        rest_amount,
        &[FUND, fund_authority_info.key.as_ref()],
    )?;

    Ok(())
}
