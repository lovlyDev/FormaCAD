//! Shared repository boundaries; managed projects retain their existing SQL authority.
#[cfg(test)]
mod benchmark;
pub mod legacy;
mod location;
mod session;
pub use legacy::{capture_history_state, recheck_history_state, HistoryStateSnapshot};
