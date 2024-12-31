use steel::*;

use super::YokoProgramAccount;

/// Seeds = [FUND, authority]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct Fund {
    /// The authority of the fund. Can make swaps and create payouts for depositors.
    pub authority: Pubkey,

    /// May be different from the main token account balance. It's purpose is to track each depositor's proportion of the fund.
    pub total_deposited: u64,

    /// Number of payouts created by fund authority.
    /// Seeds for payout account = [PAYOUT, fund pubkey, payouts_counter]
    pub payouts_counter: u64,

    /// The fee that the authority takes from each payout.
    pub authority_fee: u64,

    /// The main mint that this fund holds. Usually WSOL or USDC.
    /// Seeds for token account = [TOKEN_ACCOUNT, fund pubkey, mint pubkey]
    pub main_mint: Pubkey,

    /// The other mints that this fund holds. Inserted and removed in swaps.
    /// Seeds for token account = [TOKEN_ACCOUNT, fund pubkey, mint pubkey]
    pub other_mints: ArraySet,
}

account!(YokoProgramAccount, Fund);

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct ArraySet {
    data: [Pubkey; 96],
    len: u64,
}

impl ArraySet {
    pub const fn new() -> Self {
        Self {
            data: [Pubkey::new_from_array([0; 32]); 96],
            len: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.len as usize
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn is_full(&self) -> bool {
        self.len as usize == 96
    }

    pub fn contains(&self, value: &Pubkey) -> bool {
        self.binary_search(value).is_ok()
    }

    pub fn insert(&mut self, value: Pubkey) -> bool {
        if self.is_full() {
            return false;
        }

        match self.binary_search(&value) {
            Ok(_) => false,
            Err(pos) => {
                for i in (pos..self.len()).rev() {
                    self.data[i + 1] = self.data[i];
                }
                self.data[pos] = value;
                self.len += 1;
                true
            }
        }
    }

    pub fn remove(&mut self, value: &Pubkey) -> bool {
        match self.binary_search(value) {
            Ok(pos) => {
                for i in pos..(self.len() - 1) {
                    self.data[i] = self.data[i + 1];
                }
                self.data[self.len() - 1] = Pubkey::default();
                self.len -= 1;
                true
            }
            Err(_) => false,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Pubkey> {
        self.data[..self.len()].iter()
    }

    pub fn binary_search(&self, value: &Pubkey) -> Result<usize, usize> {
        self.data[..self.len()].binary_search(value)
    }
}
