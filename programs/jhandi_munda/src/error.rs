use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("The token program account is not the pinned SPL token program")]
    InvalidTokenProgram,
    #[msg("The musd_mint account does not match the mint stored in the program config")]
    MusdMintMismatch,
    #[msg("The destination account is not the wallet's MUSD associated token account")]
    InvalidAtaAddress,
    #[msg("This wallet already claimed MUSD in the last hour — wait for the cooldown to pass")]
    FaucetCooldown,
    #[msg("A pool deposit must be greater than zero")]
    ZeroDeposit,
    #[msg("The pool is in a state its own instructions cannot create (shares without vaulted MUSD, or the reverse)")]
    InvalidPoolState,
    #[msg("This deposit is too small at the current NAV and would mint zero shares")]
    ZeroShareMint,
    #[msg("This deposit would mint more pool shares than a u64 can hold")]
    ShareMintOverflow,
}
