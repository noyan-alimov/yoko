use steel::*;

use super::YokoProgramAccount;

/// Seeds = [POSITION, fund, authority]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct Position {
    pub authority: Pubkey,

    pub fund: Pubkey,

    pub deposited: u64,

    pub payouts_counter: u64,
}

account!(YokoProgramAccount, Position);
