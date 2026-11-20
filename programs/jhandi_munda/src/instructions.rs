pub mod faucet;
pub mod initialize;

// Anchor's #[program] needs the generated __client_accounts_* modules at the
// crate root. Both modules define `handler`; lib.rs calls them by module path.
#[allow(ambiguous_glob_reexports)]
pub use faucet::*;
#[allow(ambiguous_glob_reexports)]
pub use initialize::*;
