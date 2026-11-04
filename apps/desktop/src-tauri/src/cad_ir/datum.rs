use super::{ErrorCode, Operation, Result, ValidationError, MAX_DIMENSION_MM};

const MIN_SIZE: f64 = 1e-7;

#[cfg(feature = "native-worker-client")]
impl super::Document {
    pub(crate) fn is_datum_only(&self) -> bool {
        self.bodies.is_empty()
            && !self.features.is_empty()
            && self
                .features
                .iter()
                .all(|feature| feature.operation.datum_packet().is_some())
    }
}

fn bounded(values: &[f64], limit: f64) -> bool {
    values
        .iter()
        .all(|value| value.is_finite() && value.abs() <= limit)
}

impl Operation {
    pub(crate) fn datum_packet(&self) -> Option<(i32, [f64; 7], usize)> {
        let (kind, origin, direction, span, len) = match self {
            Self::DatumPlane {
                origin_mm,
                normal,
                side_mm,
            } => (0, *origin_mm, *normal, *side_mm, 7),
            Self::DatumAxis {
                origin_mm,
                direction,
                length_mm,
            } => (1, *origin_mm, *direction, *length_mm, 7),
            Self::DatumPoint { origin_mm } => (2, *origin_mm, [0.; 3], 0., 3),
            _ => return None,
        };
        Some((
            kind,
            [
                origin[0],
                origin[1],
                origin[2],
                direction[0],
                direction[1],
                direction[2],
                span,
            ],
            len,
        ))
    }

    pub(super) fn validate_datum(&self, id: &str) -> Result<()> {
        let Some((kind, data, _)) = self.datum_packet() else {
            return Ok(());
        };
        if bounded(&data[..3], MAX_DIMENSION_MM)
            && (kind == 2
                || (bounded(&data[3..6], 1e6)
                    && data[3].hypot(data[4]).hypot(data[5]) > MIN_SIZE
                    && data[6] > MIN_SIZE
                    && data[6] <= MAX_DIMENSION_MM))
        {
            return Ok(());
        }
        Err(ValidationError::new(
            ErrorCode::InvalidValue,
            Some(id.to_owned()),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cad_ir::{apply_commands, Body, Command, Document, Feature, SCHEMA_VERSION};

    #[test]
    fn datum_commands_reject_invalid_axes_and_cannot_create_solid_bodies() {
        let base = Document {
            schema_version: SCHEMA_VERSION,
            revision_id: "base".into(),
            parameters: vec![],
            features: vec![],
            bodies: vec![],
        };
        let axis = |direction| Command::AddFeature {
            feature: Feature {
                id: "axis".into(),
                name: "Axis".into(),
                suppressed: false,
                operation: Operation::DatumAxis {
                    origin_mm: [1., 2., 3.],
                    direction,
                    length_mm: 10.,
                },
            },
        };
        let before = base.clone();
        assert_eq!(
            apply_commands(&base, "base", "next", &[axis([0.; 3])])
                .unwrap_err()
                .code,
            ErrorCode::InvalidValue
        );
        let body = Command::AddBody {
            body: Body {
                id: "body".into(),
                name: "Body".into(),
                source_feature_id: "axis".into(),
            },
        };
        assert_eq!(
            apply_commands(&base, "base", "next", &[axis([0., 0., 1.]), body])
                .unwrap_err()
                .code,
            ErrorCode::InvalidBody
        );
        assert_eq!(base, before);
        let next = apply_commands(&base, "base", "next", &[axis([0., 0., 1.])]).unwrap();
        assert_eq!(next.features.len(), 1);
        assert!(next.bodies.is_empty());
    }
}
