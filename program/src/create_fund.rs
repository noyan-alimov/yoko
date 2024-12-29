use solana_program::{program::invoke, program_pack::Pack};
use spl_associated_token_account::tools::account::create_pda_account;
use spl_token::{instruction::initialize_account3, state::Account as SplTokenAccount};
use steel::*;
use sysvar::rent::Rent;
use yoko_program_api::prelude::*;

pub fn process_create_fund(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    let args = CreateFund::try_from_bytes(data)?;
    let authority_fee = u64::from_le_bytes(args.authority_fee);

    if authority_fee >= 100 {
        return Err(ProgramError::InvalidArgument);
    }

    let [fund_info, authority_info, main_mint_info, main_token_account_info, token_program, system_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    authority_info.is_signer()?;

    fund_info
        .is_writable()?
        .has_seeds(&[FUND, authority_info.key.as_ref()], &yoko_program_api::ID)?;

    let main_token_account = fund_token_account_pda(&fund_info.key, &main_mint_info.key);
    if main_token_account_info.key != &main_token_account.0 {
        return Err(ProgramError::InvalidSeeds);
    }

    create_account::<Fund>(
        fund_info,
        system_program,
        authority_info,
        &yoko_program_api::ID,
        &[FUND, authority_info.key.as_ref()],
    )?;

    let fund = fund_info.as_account_mut::<Fund>(&yoko_program_api::ID)?;

    fund.authority = *authority_info.key;
    fund.total_deposited = 0;
    fund.payouts_counter = 0;
    fund.authority_fee = authority_fee;
    fund.main_mint = *main_mint_info.key;
    fund.other_mints = ArraySet::new();

    let rent = Rent::get()?;
    create_pda_account(
        authority_info,
        &rent,
        SplTokenAccount::LEN,
        token_program.key,
        system_program,
        main_token_account_info,
        &[
            &TOKEN_ACCOUNT[..],
            fund_info.key.as_ref(),
            main_mint_info.key.as_ref(),
            &[main_token_account.1],
        ],
    )?;

    invoke(
        &initialize_account3(
            token_program.key,
            main_token_account_info.key,
            main_mint_info.key,
            fund_info.key,
        )?,
        &[
            main_token_account_info.clone(),
            main_mint_info.clone(),
            token_program.clone(),
        ],
    )?;

    Ok(())
}
