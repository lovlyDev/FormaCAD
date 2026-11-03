//! Shared repository boundaries; managed projects retain their existing SQL authority.
pub mod backend;
#[cfg(test)]
mod benchmark;
pub mod catalog;
pub mod folder;
pub mod legacy;
mod location;
#[cfg(all(test, any(target_os = "linux", windows)))]
pub(crate) mod selected_step;
mod session;
#[cfg(any(
    test,
    windows,
    all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    )
))]
pub mod sqlite_folder;
pub use legacy::{capture_history_state, recheck_history_state, HistoryStateSnapshot};
