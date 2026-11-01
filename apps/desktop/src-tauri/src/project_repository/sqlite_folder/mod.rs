//! Independent folder storage foundations under verification. Production routing
//! is enabled only after the read session and writer lifecycle are integrated.
#[cfg(any(
    windows,
    all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    )
))]
mod cas;
mod format;
mod read_only;
#[cfg(any(target_os = "linux", windows))]
mod session;
#[cfg(any(
    windows,
    all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    )
))]
mod write_connection;
