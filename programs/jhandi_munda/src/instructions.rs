pub mod faucet;
pub mod initialize;
pub mod pool_deposit;
pub mod pool_init;
pub mod pool_withdraw;

// Anchor's #[program] needs the generated __client_accounts_* modules at the
// crate root. All five modules define `handler`; lib.rs calls them by module
// path.
#[allow(ambiguous_glob_reexports)]
pub use faucet::*;
#[allow(ambiguous_glob_reexports)]
pub use initialize::*;
#[allow(ambiguous_glob_reexports)]
pub use pool_deposit::*;
#[allow(ambiguous_glob_reexports)]
pub use pool_init::*;
#[allow(ambiguous_glob_reexports)]
pub use pool_withdraw::*;
