use crate::{
    cad_ir::TopologyKind,
    model_pair_measurement::schema::{PairMeasurementQuery, PairMeasurementValue},
    native::{ffi, NativeError, Solid},
};
fn failure(code: &str, detail: impl Into<String>) -> NativeError {
    NativeError {
        code: code.into(),
        detail: detail.into(),
    }
}
fn point(values: &cxx::CxxVector<f64>) -> Result<[f64; 3], NativeError> {
    if values.len() != 3 {
        return Err(failure(
            "MEASUREMENT_INVALID_RESULT",
            "Pair witness requires three coordinates",
        ));
    }
    Ok([
        *values.get(0).unwrap(),
        *values.get(1).unwrap(),
        *values.get(2).unwrap(),
    ])
}
impl Solid {
    pub fn measure_reference_pair(
        &self,
        query: &PairMeasurementQuery,
    ) -> Result<PairMeasurementValue, NativeError> {
        if !query.valid() {
            return Err(failure(
                "INVALID_MEASUREMENT_QUERY",
                "Pair reference kinds or identities are invalid",
            ));
        }
        let (a, b) = query.references();
        cxx::let_cxx_string!(owner_a = &a.owner_feature_id);
        cxx::let_cxx_string!(role_a = &a.role);
        cxx::let_cxx_string!(path_a = a.occurrence_path.join("/"));
        cxx::let_cxx_string!(owner_b = &b.owner_feature_id);
        cxx::let_cxx_string!(role_b = &b.role);
        cxx::let_cxx_string!(path_b = b.occurrence_path.join("/"));
        let kind = |value: TopologyKind| match value {
            TopologyKind::Edge => 0,
            TopologyKind::Face => 1,
        };
        let result = ffi::measure_reference_pair(
            self.as_ref(),
            query.tag(),
            kind(a.kind),
            &owner_a,
            &role_a,
            &path_a,
            kind(b.kind),
            &owner_b,
            &role_b,
            &path_b,
        );
        let result = result.as_ref().ok_or_else(|| {
            failure(
                "NATIVE_ALLOCATION_FAILED",
                "Kernel pair result allocation failed",
            )
        })?;
        if !result.pair_measurement_valid() {
            return Err(failure(
                &result.pair_measurement_error_code().to_string_lossy(),
                result.pair_measurement_error_message().to_string_lossy(),
            ));
        }
        if result.pair_query_kind() != query.tag() {
            return Err(failure(
                "MEASUREMENT_INVALID_RESULT",
                "Kernel returned another pair query kind",
            ));
        }
        let value = match query {
            PairMeasurementQuery::MinimumDistance { .. } => PairMeasurementValue::MinimumDistance {
                distance_mm: result.pair_distance_mm(),
                point_a_mm: point(result.pair_point_a_mm())?,
                point_b_mm: point(result.pair_point_b_mm())?,
            },
            PairMeasurementQuery::FaceNormalAngle { .. } => PairMeasurementValue::FaceNormalAngle {
                angle_deg: result.pair_angle_deg(),
            },
            PairMeasurementQuery::EdgeAcuteAngle { .. } => PairMeasurementValue::EdgeAcuteAngle {
                angle_deg: result.pair_angle_deg(),
            },
        };
        if !value.valid_for(query) {
            return Err(failure(
                "MEASUREMENT_INVALID_RESULT",
                "Kernel pair values exceed the captured query contract",
            ));
        }
        Ok(value)
    }
}
