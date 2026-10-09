//! One native CAD job: build every named body, then publish STEP and GLB artifacts.
use super::{
    build_body_with_assets,
    glb_merge::{merge_body_glbs, BodyGlb},
    NativeError, Solid,
};
use crate::cad_ir::Document;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub struct JobResult {
    pub volume_mm3: f64,
    pub area_mm2: f64,
    pub face_count: i32,
    pub edge_count: i32,
    pub bounds_mm: [f64; 3],
}

impl JobResult {
    pub fn from_solid(solid: &Solid) -> Self {
        Self {
            volume_mm3: solid.volume_mm3(),
            area_mm2: solid.surface_area_mm2(),
            face_count: solid.face_count(),
            edge_count: solid.edge_count(),
            bounds_mm: solid.bounds_mm(),
        }
    }
}

pub fn execute(
    document: &Document,
    selected_body_id: Option<&str>,
    cwd: &Path,
) -> Result<JobResult, NativeError> {
    super::assets::verify_inputs(document, cwd)?;
    let bodies: Vec<_> = document
        .bodies
        .iter()
        .filter(|body| selected_body_id.is_none_or(|id| body.id == id))
        .collect();
    if bodies.is_empty() {
        return Err(NativeError {
            code: "MISSING_BODY".into(),
            detail: "No matching body in CAD document".into(),
        });
    }
    let mut shapes = Vec::<Solid>::with_capacity(bodies.len());
    let mut paths = Vec::<PathBuf>::with_capacity(bodies.len());
    for (index, body) in bodies.iter().enumerate() {
        let shape = build_body_with_assets(document, &body.id, Some(cwd))?;
        let path = cwd.join(format!("body-{index}.glb"));
        shape.write_glb(&path)?;
        shapes.push(shape);
        paths.push(path);
    }
    let glbs: Vec<_> = bodies
        .iter()
        .zip(&paths)
        .map(|(body, path)| BodyGlb {
            id: &body.id,
            name: &body.name,
            path,
        })
        .collect();
    let mut shapes = shapes.into_iter();
    let mut aggregate = shapes.next().ok_or_else(|| NativeError {
        code: "MISSING_BODY".into(),
        detail: "No solid body was built".into(),
    })?;
    for shape in shapes {
        aggregate = aggregate.compound(&shape)?;
    }
    aggregate.write_step(&cwd.join("model.step"))?;
    merge_body_glbs(&glbs, &cwd.join("preview.glb"))?;
    for path in paths {
        let _ = fs::remove_file(path);
    }
    Ok(JobResult::from_solid(&aggregate))
}
