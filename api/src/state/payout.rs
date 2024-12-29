use steel::*;

use super::YokoProgramAccount;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct Payout {
    pub total_deposited: u64, // total deposited in the fund when the payout was created
}

account!(YokoProgramAccount, Payout);
