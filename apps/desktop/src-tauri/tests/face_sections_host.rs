#![cfg(feature = "native-occt")]
// Reuse immutable bounded committed-project fixture when copied into production tests.
#[path = "reference_measurements_host/fixture.rs"]
mod fixture;
#[path = "face_sections_host/geometry.rs"]
mod geometry;
#[path = "face_sections_host/lifecycle.rs"]
mod lifecycle;
