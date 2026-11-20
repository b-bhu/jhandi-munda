use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("The token program account is not the pinned SPL token program")]
    InvalidTokenProgram,
}
