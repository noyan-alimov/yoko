use steel::*;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq, IntoPrimitive)]
#[repr(u32)]
pub enum YokoProgramError {
    #[error("Invalid account")]
    InvalidAccount = 0,
    #[error("Invalid amount")]
    InvalidAmount = 1,
    #[error("Error inserting other mint")]
    ErrorInsertingOtherMint = 2,
    #[error("Error removing other mint")]
    ErrorRemovingOtherMint = 3,
}

error!(YokoProgramError);
