//! Independent FCP1 Rust host API. UI/IPC routing is a separate integration gate.
#[cfg(any(
    windows,
    all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    )
))]
pub mod api;
#[cfg(any(
    windows,
    all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    )
))]
mod cas;
#[cfg(any(
    windows,
    all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    )
))]
mod copy;
#[cfg(any(
    windows,
    all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    )
))]
mod creation;
mod format;
#[cfg(any(target_os = "linux", windows))]
mod migration;
#[cfg(any(target_os = "linux", windows))]
mod publication_verification;
mod read_only;
#[cfg(any(target_os = "linux", windows))]
mod session;
#[cfg(all(
    test,
    feature = "native-worker-client",
    any(target_os = "linux", windows)
))]
mod step_transfer;
#[cfg(any(
    windows,
    all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    )
))]
mod write_connection;
