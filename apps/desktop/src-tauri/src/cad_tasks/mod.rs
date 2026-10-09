//! One heavy CAD worker per application instance; CLI planning does not take a permit.
mod gate;
pub mod ipc;
#[cfg(test)]
mod tests;
pub use gate::{CadTaskGate, TaskInfo, TaskPermit};
