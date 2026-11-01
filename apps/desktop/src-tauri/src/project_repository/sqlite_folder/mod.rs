//! Independent folder storage foundations under verification. Production routing
//! is enabled only after the read session and writer lifecycle are integrated.
mod format;
mod read_only;
#[cfg(any(target_os = "linux", windows))]
mod session;
