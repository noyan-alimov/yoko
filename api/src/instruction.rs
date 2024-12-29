use steel::*;

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, TryFromPrimitive)]
pub enum YokoProgramInstruction {
    CreateFund = 0,
    CreatePosition = 1,
    Deposit = 2,
    CreatePayout = 3,
    ClaimPayout = 4,
    Swap = 5,
    CreateFundTokenAccount = 6,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct CreateFund {
    pub authority_fee: [u8; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct CreatePosition {}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Deposit {
    pub amount: [u8; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct CreatePayout {
    pub amount: [u8; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct ClaimPayout {}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Swap {}
// data unpacked in the processor:
// 1. in_amount
// 2. jupiter_route_cpi_data

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct CreateFundTokenAccount {}

instruction!(YokoProgramInstruction, CreateFund);
instruction!(YokoProgramInstruction, CreatePosition);
instruction!(YokoProgramInstruction, Deposit);
instruction!(YokoProgramInstruction, CreatePayout);
instruction!(YokoProgramInstruction, ClaimPayout);
instruction!(YokoProgramInstruction, Swap);
instruction!(YokoProgramInstruction, CreateFundTokenAccount);
