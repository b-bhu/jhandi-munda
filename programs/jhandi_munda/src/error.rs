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
    #[msg("The VRF callback does not match a pending roll (none pending, already fulfilled, or another round)")]
    UnexpectedCallback,
    #[msg("Too few usable VRF bytes to make six faces")]
    RollExhausted,
    #[msg("Settle needs the six faces of a rolled round, and runs once per round")]
    RoundNotRolled,
    #[msg("A bet account passed to settle is not an unsettled bet of this round")]
    BetAccountMismatch,
    #[msg("Finish the round only after every bet of it is settled")]
    IncompleteSettle,
    #[msg("This bet is already settled")]
    BetAlreadySettled,
    #[msg("A claim can only be opened for a settled bet")]
    BetNotSettled,
    #[msg("This claim is still locked; Wait pays after its unlock time")]
    ClaimLocked,
    #[msg("This claim was already paid out")]
    ClaimAlreadyRedeemed,
    #[msg("The pool does not hold enough MUSD to pay this now (the queue is Step 8)")]
    InsufficientPoolFunds,
    #[msg("The insurance fund does not hold enough MUSD for this Take now")]
    InsufficientInsuranceFunds,
    #[msg("There is nothing to collect from the pool for this claim")]
    NothingToCollect,
    #[msg("A chip purchase or cash-out must be greater than zero")]
    ZeroChips,
    #[msg("The VRF queue must be the base-layer or the ephemeral queue")]
    InvalidQueue,
    #[msg("Only the owner of this chip stack can delegate it")]
    NotChipOwner,
    #[msg("Not enough chips for this bet or cash-out")]
    InsufficientChips,
    #[msg("Only the table authority (the config admin) can delegate the round")]
    NotTableAuthority,
    #[msg("A token account does not have the expected layout")]
    InvalidTokenAccount,
    #[msg("The payout queue is full")]
    QueueFull,
    #[msg("Withdrawals wait until every queued winner is paid (D30)")]
    QueueNotEmpty,
    #[msg("Queue payouts must be passed in strict first-in, first-out order")]
    QueueOrderMismatch,
    #[msg("This claim is waiting in the payout queue")]
    ClaimAlreadyQueued,
    #[msg("A stake must be greater than zero")]
    ZeroStake,
    #[msg("The stake is smaller than the amount to unstake")]
    InsufficientStake,
    #[msg("Pass every stake account, exactly once, to distribute the stakers' slice")]
    StakerSetMismatch,
    #[msg("There is nothing to claim")]
    NothingToClaim,
    #[msg("This wallet already has the maximum number of stake lots; unstake to free one")]
    TooManyLots,
    #[msg("A payout or fee amount overflowed")]
    MathOverflow,
}
