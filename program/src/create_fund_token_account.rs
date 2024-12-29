use solana_program::{program::invoke, program_pack::Pack};
use spl_associated_token_account::tools::account::create_pda_account;
use spl_token::{instruction::initialize_account3, state::Account as SplTokenAccount};
use steel::*;
use sysvar::rent::Rent;
use yoko_program_api::prelude::*;

pub fn process_create_fund_token_account(accounts: &[AccountInfo<'_>]) -> ProgramResult {
    let [fund_info, authority_info, fund_token_account_info, mint_info, token_program, system_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    authority_info.is_signer()?;

    let fund = fund_info
        .is_writable()?
        .has_seeds(&[FUND, authority_info.key.as_ref()], &yoko_program_api::ID)?
        .as_account_mut::<Fund>(&yoko_program_api::ID)?
        .assert_mut(|fund| fund.authority == *authority_info.key)?;

    let inserted = fund.other_mints.insert(*mint_info.key);
    if !inserted {
        return Err(YokoProgramError::ErrorInsertingOtherMint.into());
    }

    let fund_token_account = fund_token_account_pda(&fund_info.key, mint_info.key);
    if fund_token_account_info.key != &fund_token_account.0 {
        return Err(ProgramError::InvalidSeeds);
    }

    let rent = Rent::get()?;
    create_pda_account(
        authority_info,
        &rent,
        SplTokenAccount::LEN,
        token_program.key,
        system_program,
        fund_token_account_info,
        &[
            &TOKEN_ACCOUNT[..],
            fund_info.key.as_ref(),
            mint_info.key.as_ref(),
            &[fund_token_account.1],
        ],
    )?;

    invoke(
        &initialize_account3(
            token_program.key,
            fund_token_account_info.key,
            mint_info.key,
            fund_info.key,
        )?,
        &[
            fund_token_account_info.clone(),
            mint_info.clone(),
            token_program.clone(),
        ],
    )?;

    Ok(())
}
