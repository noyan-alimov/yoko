use solana_program::program_pack::Pack;
use spl_token::state::Account as SplTokenAccount;
use steel::*;
use yoko_program_api::prelude::*;

pub fn process_deposit(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    let args = Deposit::try_from_bytes(data)?;
    let amount = u64::from_le_bytes(args.amount);

    let [position_info, fund_info, fund_main_token_account_info, depositor_authority_info, depositor_token_account_info, token_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    depositor_authority_info.is_signer()?;

    position_info.has_seeds(
        &[
            POSITION,
            fund_info.key.as_ref(),
            depositor_authority_info.key.as_ref(),
        ],
        &yoko_program_api::ID,
    )?;

    let position = position_info
        .as_account_mut::<Position>(&yoko_program_api::ID)?
        .assert_mut(|position| position.authority == *depositor_authority_info.key)?
        .assert_mut(|position| position.fund == *fund_info.key)?;

    position.deposited = position
        .deposited
        .checked_add(amount)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    let fund_main_token_account_data =
        SplTokenAccount::unpack(&fund_main_token_account_info.data.borrow())?;

    let fund = fund_info
        .as_account_mut::<Fund>(&yoko_program_api::ID)?
        .assert_mut(|fund| fund.main_mint == fund_main_token_account_data.mint)?
        .assert_mut(|fund| fund.payouts_counter == position.payouts_counter)?;

    fund.total_deposited = fund
        .total_deposited
        .checked_add(amount)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    fund_main_token_account_info.has_seeds(
        &[
            TOKEN_ACCOUNT,
            fund_info.key.as_ref(),
            fund.main_mint.as_ref(),
        ],
        &yoko_program_api::ID,
    )?;

    transfer(
        depositor_authority_info,
        depositor_token_account_info,
        fund_main_token_account_info,
        token_program,
        amount,
    )?;

    Ok(())
}
