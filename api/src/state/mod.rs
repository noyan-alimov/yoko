mod fund;
mod payout;
mod position;

pub use fund::*;
pub use payout::*;
pub use position::*;
use steel::*;

use crate::consts::*;

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, IntoPrimitive, TryFromPrimitive)]
pub enum YokoProgramAccount {
    Fund = 0,
    Position = 1,
    Payout = 2,
}

pub fn fund_pda(authority: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[FUND, authority.as_ref()], &crate::id())
}

pub fn position_pda(fund: &Pubkey, authority: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[POSITION, fund.as_ref(), authority.as_ref()], &crate::id())
}

pub fn fund_token_account_pda(fund: &Pubkey, mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[TOKEN_ACCOUNT, fund.as_ref(), mint.as_ref()], &crate::id())
}

pub fn payout_pda(fund: &Pubkey, counter: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[PAYOUT, fund.as_ref(), &counter.to_le_bytes()],
        &crate::id(),
    )
}

pub fn payout_token_account_pda(payout: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[PAYOUT, payout.as_ref()], &crate::id())
}
