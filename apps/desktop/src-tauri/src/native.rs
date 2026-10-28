//! Native worker clients; the OCCT bridge is compiled only into kernel builds.
mod error;
pub use error::NativeError;
#[cfg(feature = "native-occt")]
mod kernel;
#[cfg(feature = "native-occt")]
pub(super) use kernel::ffi;
#[cfg(feature = "native-occt")]
pub use kernel::{box_volume, Solid};
#[cfg(feature = "native-occt")]
mod affine_transform;
pub mod assets;
#[cfg(feature = "native-occt")]
mod chamfer;
#[cfg(feature = "native-occt")]
mod document;
pub mod face_section;
#[cfg(feature = "native-occt")]
mod fillet;
#[cfg(feature = "native-occt")]
pub mod glb_merge;
pub mod health;
#[cfg(feature = "native-occt")]
mod hole;
pub mod inspection;
#[cfg(feature = "native-occt")]
pub mod job;
pub mod pair_measurements;
pub mod reference_measurements;
#[cfg(feature = "native-occt")]
mod revolution;
pub mod section;
#[cfg(feature = "native-occt")]
pub mod step_import;
#[cfg(feature = "native-occt")]
mod topology_refs;
#[cfg(feature = "native-occt")]
mod transforms;
pub mod worker;
#[cfg(feature = "native-occt")]
pub use document::{build_body, build_body_with_assets};
#[cfg(all(test, feature = "native-occt"))]
mod tests;
