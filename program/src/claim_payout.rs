use solana_program::{program::invoke_signed, program_pack::Pack};
use spl_token::{instruction::close_account, state::Account as SplTokenAccount};
use steel::*;
use yoko_program_api::prelude::*;

pub fn process_claim_payout(accounts: &[AccountInfo<'_>]) -> ProgramResult {
    let [position_info, position_authority_info, payout_info, payout_main_token_account_info, depositor_main_token_account_info, fund_info, token_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    position_authority_info.is_signer()?;

    let fund = fund_info.as_account::<Fund>(&yoko_program_api::ID)?;

    let position = position_info
        .as_account_mut::<Position>(&yoko_program_api::ID)?
        .assert_mut(|position| position.authority == *position_authority_info.key)?
        .assert_mut(|position| position.fund == *fund_info.key)?
        .assert_mut(|position| position.payouts_counter < fund.payouts_counter)?;

    let new_counter = position
        .payouts_counter
        .checked_add(1)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    position.payouts_counter = new_counter;

    let payout_info_pda = payout_pda(fund_info.key, new_counter);
    if payout_info_pda.0 != *payout_info.key {
        return Err(ProgramError::InvalidAccountData);
    }
    let payout = payout_info.as_account::<Payout>(&yoko_program_api::ID)?;

    let proportion = (position.deposited as u128)
        .checked_mul(u128::pow(10, 9))
        .ok_or(ProgramError::ArithmeticOverflow)?
        .checked_div(payout.total_deposited as u128)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    payout_main_token_account_info
        .has_seeds(&[PAYOUT, payout_info.key.as_ref()], &yoko_program_api::ID)?;

    let amount = (payout.amount_transferred_on_creation as u128)
        .checked_mul(proportion)
        .ok_or(ProgramError::ArithmeticOverflow)?
        .checked_div(u128::pow(10, 9))
        .ok_or(ProgramError::ArithmeticOverflow)?;

    let amount = u64::try_from(amount).map_err(|_| ProgramError::ArithmeticOverflow)?;

    transfer_signed(
        payout_info,
        payout_main_token_account_info,
        depositor_main_token_account_info,
        token_program,
        amount,
        &[PAYOUT, fund_info.key.as_ref(), &new_counter.to_le_bytes()],
    )?;

    let payout_main_token_account_data =
        SplTokenAccount::unpack(&payout_main_token_account_info.data.borrow())?;

    if payout_main_token_account_data.amount <= 0 {
        invoke_signed(
            &close_account(
                token_program.key,
                payout_main_token_account_info.key,
                position_authority_info.key,
                payout_info.key,
                &[payout_info.key],
            )?,
            &[
                payout_main_token_account_info.clone(),
                position_authority_info.clone(),
                payout_info.clone(),
                token_program.clone(),
            ],
            &[&[
                PAYOUT,
                fund_info.key.as_ref(),
                &new_counter.to_le_bytes(),
                &[payout_info_pda.1],
            ]],
        )?;

        steel::close_account(payout_info, position_authority_info)?;
    }

    Ok(())
}
