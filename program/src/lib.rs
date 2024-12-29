mod claim_payout;
mod create_fund;
mod create_fund_token_account;
mod create_payout;
mod create_position;
mod deposit;
mod swap;

use claim_payout::*;
use create_fund::*;
use create_fund_token_account::*;
use create_payout::*;
use create_position::*;
use deposit::*;
use steel::*;
use swap::*;
use yoko_program_api::prelude::*;

pub fn process_instruction<'a>(
    program_id: &Pubkey,
    accounts: &'a [AccountInfo<'a>],
    data: &[u8],
) -> ProgramResult {
    let (ix, data) = parse_instruction(&yoko_program_api::ID, program_id, data)?;

    match ix {
        YokoProgramInstruction::CreateFund => process_create_fund(accounts, data)?,
        YokoProgramInstruction::CreatePosition => process_create_position(accounts)?,
        YokoProgramInstruction::Deposit => process_deposit(accounts, data)?,
        YokoProgramInstruction::CreatePayout => process_create_payout(accounts, data)?,
        YokoProgramInstruction::ClaimPayout => process_claim_payout(accounts)?,
        YokoProgramInstruction::Swap => process_swap(accounts, data)?,
        YokoProgramInstruction::CreateFundTokenAccount => {
            process_create_fund_token_account(accounts)?
        }
    }

    Ok(())
}

entrypoint!(process_instruction);
