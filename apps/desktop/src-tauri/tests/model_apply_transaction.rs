#![cfg(feature = "native-occt")]
//! Actual host apply/commit route, real worker and isolated storage; no UI runtime.
#[path = "model_apply_transaction/amend.rs"]
mod amend;
#[path = "model_apply_transaction/chamfer_edge.rs"]
mod chamfer_edge;
#[path = "model_apply_transaction/datum.rs"]
mod datum;
#[path = "model_apply_transaction/duplicate_body.rs"]
mod duplicate_body;
#[path = "model_apply_transaction/extrude_face.rs"]
mod extrude_face;
#[path = "model_apply_transaction/fixture.rs"]
mod fixture;
#[path = "model_apply_transaction/geometry.rs"]
mod geometry;
#[path = "model_apply_transaction/groups.rs"]
mod groups;
#[path = "model_apply_transaction/inputs.rs"]
mod inputs;
#[path = "model_apply_transaction/lifecycle.rs"]
mod lifecycle;
#[path = "model_apply_transaction/pattern.rs"]
mod pattern;
#[path = "model_apply_transaction/separate_solids.rs"]
mod separate_solids;
#[path = "model_apply_transaction/shell.rs"]
mod shell;
#[path = "model_apply_transaction/torus.rs"]
mod torus;
