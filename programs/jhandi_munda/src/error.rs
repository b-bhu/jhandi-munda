use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("The token program account is not the pinned SPL token program")]
    InvalidTokenProgram,
    #[msg("The musd_mint account does not match the mint stored in the program config")]
    MusdMintMismatch,
    #[msg("The destination account is not the wallet's MUSD associated token account")]
    InvalidAtaAddress,
}
