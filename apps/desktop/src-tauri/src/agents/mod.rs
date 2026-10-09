mod cli_spec;
mod context;
pub mod custom;
pub(crate) mod health;
mod protocol;
#[cfg(feature = "native-occt")]
mod repair;
pub mod review_plan;
pub mod selection;
pub(crate) mod session;
#[cfg(test)]
mod tests;

pub use health::{check_cad, choose_cad_python, detect_environment, Health};
pub use protocol::{parse_output, PlanResult};
pub use session::{cancel_task, plan_model};
