//! Explicit plane side: never inferred from OCCT enumeration order.
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SplitSide {
    Positive,
    Negative,
}
