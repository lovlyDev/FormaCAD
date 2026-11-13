use super::{MeasurementQuery, MeasurementValue};
use crate::{
    model_measurement::schema::TopologySelectionMode,
    native::{ffi, NativeError, Solid},
};

pub(super) fn error(code: &str, detail: impl Into<String>) -> NativeError {
    NativeError {
        code: code.into(),
        detail: detail.into(),
    }
}

fn vector(values: &cxx::CxxVector<f64>) -> Result<[f64; 3], NativeError> {
    if values.len() != 3 {
        return Err(error(
            "MEASUREMENT_INVALID_RESULT",
            "Kernel vector length is invalid",
        ));
    }
    values
        .iter()
        .copied()
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|_| {
            error(
                "MEASUREMENT_INVALID_RESULT",
                "Kernel vector length is invalid",
            )
        })
}

impl Solid {
    pub fn measure_reference(
        &self,
        query: &MeasurementQuery,
    ) -> Result<MeasurementValue, NativeError> {
        if !query.valid() {
            return Err(error(
                "INVALID_TOPOLOGY_REFERENCE",
                "Invalid measurement query reference",
            ));
        }
        if let MeasurementQuery::TopologySelection { mode, seed } = query {
            let kind = match mode {
                TopologySelectionMode::SimilarFaces => 0,
                TopologySelectionMode::SimilarEdges => 1,
                TopologySelectionMode::TangentChain => 2,
            };
            let result = ffi::smart_select(self.as_ref(), kind, *seed);
            let value = MeasurementValue::TopologySelection {
                ordinals: result
                    .as_ref()
                    .into_iter()
                    .flat_map(|values| values.iter().copied())
                    .collect(),
            };
            if value.valid_for(query) {
                return Ok(value);
            }
            return Err(error(
                "SMART_SELECTION_UNAVAILABLE",
                "Selection is outside the supported BREP topology",
            ));
        }
        if let MeasurementQuery::EdgeEndpoints { reference } = query {
            let (start_mm, end_mm) = self.variable_fillet_endpoints(reference)?;
            let value = MeasurementValue::EdgeEndpoints { start_mm, end_mm };
            return if value.valid_for(query) {
                Ok(value)
            } else {
                Err(error(
                    "MEASUREMENT_INVALID_RESULT",
                    "Invalid exact edge endpoints",
                ))
            };
        }
        let reference = query.reference();
        cxx::let_cxx_string!(owner = reference.map_or("", |r| r.owner_feature_id.as_str()));
        cxx::let_cxx_string!(role = reference.map_or("", |r| r.role.as_str()));
        cxx::let_cxx_string!(
            path = reference.map_or_else(String::new, |r| r.occurrence_path.join("/"))
        );
        let result = ffi::measure_reference(self.as_ref(), query.tag(), &owner, &role, &path);
        let result = result.as_ref().ok_or_else(|| {
            error(
                "NATIVE_ALLOCATION_FAILED",
                "Kernel measurement allocation failed",
            )
        })?;
        if !result.measurement_valid() {
            return Err(error(
                &result.measurement_error_code().to_string_lossy(),
                result.measurement_error_message().to_string_lossy(),
            ));
        }
        if result.query_kind() != query.tag() {
            return Err(error(
                "MEASUREMENT_INVALID_RESULT",
                "Kernel returned another query kind",
            ));
        }
        let value = match query {
            MeasurementQuery::BodyMetrics {} => MeasurementValue::BodyMetrics {
                volume_mm3: result.measurement_volume_mm3(),
                area_mm2: result.area_mm2(),
                face_count: result
                    .measurement_face_count()
                    .try_into()
                    .map_err(|_| error("MEASUREMENT_INVALID_RESULT", "Negative face count"))?,
                edge_count: result
                    .measurement_edge_count()
                    .try_into()
                    .map_err(|_| error("MEASUREMENT_INVALID_RESULT", "Negative edge count"))?,
                extents_mm: vector(result.extents_mm())?,
            },
            MeasurementQuery::EdgeLength { .. } => MeasurementValue::EdgeLength {
                length_mm: result.length_mm(),
            },
            MeasurementQuery::EdgeRadius { .. } => MeasurementValue::EdgeRadius {
                radius_mm: result.radius_mm(),
            },
            MeasurementQuery::EdgeDiameter { .. } => MeasurementValue::EdgeDiameter {
                diameter_mm: result.diameter_mm(),
            },
            MeasurementQuery::FaceArea { .. } => MeasurementValue::FaceArea {
                area_mm2: result.area_mm2(),
            },
            MeasurementQuery::PlanarFace { .. } => MeasurementValue::PlanarFace {
                area_mm2: result.area_mm2(),
                origin_mm: vector(result.origin_mm())?,
                normal: vector(result.normal())?,
            },
            MeasurementQuery::TopologySelection { .. } | MeasurementQuery::EdgeEndpoints { .. } => {
                unreachable!("handled above")
            }
        };
        if !value.valid_for(query) {
            return Err(error(
                "MEASUREMENT_INVALID_RESULT",
                "Kernel values are outside the measurement contract",
            ));
        }
        Ok(value)
    }
}
