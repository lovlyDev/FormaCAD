//! Read-only exact authored-reference measurements; no document publication.
pub mod client;
pub mod execute;
mod solid;
pub use crate::model_measurement::schema::{MeasurementQuery, MeasurementValue};
