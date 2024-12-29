use steel::*;
use yoko_program_api::prelude::*;

pub fn process_create_position(accounts: &[AccountInfo<'_>]) -> ProgramResult {
    let [position_info, fund_info, authority_info, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    authority_info.is_signer()?;

    position_info.is_writable()?.has_seeds(
        &[
            POSITION,
            fund_info.key.as_ref(),
            authority_info.key.as_ref(),
        ],
        &yoko_program_api::ID,
    )?;

    create_account::<Position>(
        position_info,
        system_program,
        authority_info,
        &yoko_program_api::ID,
        &[
            POSITION,
            fund_info.key.as_ref(),
            authority_info.key.as_ref(),
        ],
    )?;

    let fund = fund_info.as_account::<Fund>(&yoko_program_api::ID)?;
    let position = position_info.as_account_mut::<Position>(&yoko_program_api::ID)?;

    position.authority = *authority_info.key;
    position.fund = *fund_info.key;
    position.deposited = 0;
    position.payouts_counter = fund.payouts_counter;

    Ok(())
}
