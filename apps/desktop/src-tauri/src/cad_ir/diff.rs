//! Structural changes between two validated CAD IR revisions.
//!
//! IDs, rather than positions in the feature tree, identify entities. Positions
//! are retained in each snapshot because feature order is meaningful to rebuilds.
use super::{Body, Document, Feature, Parameter, Result};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot<T> {
    pub index: usize,
    pub value: T,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityChange<T> {
    pub id: String,
    pub before: Option<Snapshot<T>>,
    pub after: Option<Snapshot<T>>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentDiff {
    pub from_revision_id: String,
    pub to_revision_id: String,
    pub parameters: Vec<EntityChange<Parameter>>,
    pub features: Vec<EntityChange<Feature>>,
    pub bodies: Vec<EntityChange<Body>>,
}

impl DocumentDiff {
    pub fn is_empty(&self) -> bool {
        self.parameters.is_empty() && self.features.is_empty() && self.bodies.is_empty()
    }
}

/// Compare document intent only. Geometric differences require a successful
/// rebuild and native BREP inspection; this function makes no geometry claims.
pub fn diff_documents(before: &Document, after: &Document) -> Result<DocumentDiff> {
    before.validate()?;
    after.validate()?;
    Ok(DocumentDiff {
        from_revision_id: before.revision_id.clone(),
        to_revision_id: after.revision_id.clone(),
        parameters: changes(&before.parameters, &after.parameters, |p| &p.id),
        features: changes(&before.features, &after.features, |f| &f.id),
        bodies: changes(&before.bodies, &after.bodies, |b| &b.id),
    })
}

fn changes<T: Clone + PartialEq>(
    before: &[T],
    after: &[T],
    id: impl Fn(&T) -> &str,
) -> Vec<EntityChange<T>> {
    let previous: HashMap<&str, (usize, &T)> = before
        .iter()
        .enumerate()
        .map(|(index, value)| (id(value), (index, value)))
        .collect();
    let current: HashMap<&str, (usize, &T)> = after
        .iter()
        .enumerate()
        .map(|(index, value)| (id(value), (index, value)))
        .collect();
    let mut result = Vec::new();
    // Existing entities keep their old order; additions follow their new order.
    for (index, value) in before.iter().enumerate() {
        let key = id(value);
        let next = current.get(key);
        if next.is_some_and(|(next_index, next_value)| *next_index == index && *next_value == value)
        {
            continue;
        }
        result.push(EntityChange {
            id: key.to_owned(),
            before: Some(Snapshot {
                index,
                value: value.clone(),
            }),
            after: next.map(|(next_index, next_value)| Snapshot {
                index: *next_index,
                value: (*next_value).clone(),
            }),
        });
    }
    for (index, value) in after.iter().enumerate() {
        if !previous.contains_key(id(value)) {
            result.push(EntityChange {
                id: id(value).to_owned(),
                before: None,
                after: Some(Snapshot {
                    index,
                    value: value.clone(),
                }),
            });
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cad_ir::{Dimension, Operation, SCHEMA_VERSION};

    fn document() -> Document {
        Document {
            schema_version: SCHEMA_VERSION,
            revision_id: "first".into(),
            parameters: vec![Parameter {
                id: "width".into(),
                name: "Width".into(),
                value_mm: 10.0,
            }],
            features: vec![Feature {
                id: "circle".into(),
                name: "Circle".into(),
                suppressed: false,
                operation: Operation::Circle {
                    radius: Dimension::Literal { mm: 2.0 },
                },
            }],
            bodies: vec![],
        }
    }

    #[test]
    fn records_changes_by_id_and_ignores_revision_only_changes() {
        let first = document();
        let mut second = first.clone();
        second.revision_id = "second".into();
        assert!(diff_documents(&first, &second).unwrap().is_empty());
        second.parameters[0].value_mm = 12.0;
        second.features[0].name = "Renamed".into();
        let diff = diff_documents(&first, &second).unwrap();
        assert_eq!(diff.parameters[0].id, "width");
        assert_eq!(
            diff.parameters[0].before.as_ref().unwrap().value.value_mm,
            10.0
        );
        assert_eq!(
            diff.parameters[0].after.as_ref().unwrap().value.value_mm,
            12.0
        );
        assert_eq!(diff.features[0].id, "circle");
    }

    #[test]
    fn records_additions_removals_and_order_changes() {
        let first = document();
        let mut second = first.clone();
        second.revision_id = "second".into();
        second.parameters.clear();
        second.features.insert(
            0,
            Feature {
                id: "rectangle".into(),
                name: "Rectangle".into(),
                suppressed: false,
                operation: Operation::Rectangle {
                    width: Dimension::Literal { mm: 5.0 },
                    depth: Dimension::Literal { mm: 5.0 },
                },
            },
        );
        let diff = diff_documents(&first, &second).unwrap();
        assert!(diff.parameters[0].after.is_none());
        assert_eq!(diff.features[0].after.as_ref().unwrap().index, 1);
        assert!(diff.features[1].before.is_none());
    }

    #[test]
    fn rejects_invalid_documents() {
        let first = document();
        let mut invalid = first.clone();
        invalid.parameters[0].value_mm = f64::NAN;
        assert!(diff_documents(&first, &invalid).is_err());
    }
}
