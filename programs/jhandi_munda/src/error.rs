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
    #[msg("A pool withdraw must pay out a non-zero amount of MUSD")]
    ZeroWithdraw,
    #[msg("The withdrawer does not hold that many pool shares")]
    InsufficientShares,
    #[msg("A bet must be at least the table minimum (1 MUSD)")]
    BetBelowMinimum,
    #[msg("A bet is above the table maximum")]
    BetAboveMaximum,
    #[msg("A bet must name one die face, 1 to 6")]
    InvalidSymbol,
    #[msg("The table already has its maximum of 10 players this round")]
    TableFull,
    #[msg("The 90-second join window for this round has closed")]
    BettingClosed,
    #[msg("The join window is still open; the roll can be requested once it closes")]
    WindowStillOpen,
    #[msg("There is no open round to roll")]
    NoRoundToRoll,
    #[msg("A VRF roll is already pending for this round")]
    RollAlreadyPending,
}
