#![cfg(feature = "native-occt")]
//! Actual host apply/commit route, real worker and isolated storage; no UI runtime.
#[path = "model_apply_transaction/fixture.rs"]
mod fixture;
#[path = "model_apply_transaction/geometry.rs"]
mod geometry;
#[path = "model_apply_transaction/inputs.rs"]
mod inputs;
#[path = "model_apply_transaction/lifecycle.rs"]
mod lifecycle;
