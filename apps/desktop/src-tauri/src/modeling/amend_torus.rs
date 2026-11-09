//! Replace the latest torus creation without adding an Undo slot.
use super::{amend_transform::read_document, apply::ApplyRequest};
use crate::{
    cad_ir::{Dimension, Document, Operation},
    core::{AppError, AppState, Result},
    models::Project,
    project_history::AmendCapture,
};
use serde::{Deserialize, Serialize};

pub mod ipc;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TorusRadii {
    pub major_radius_mm: f64,
    pub minor_radius_mm: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LastTorusStatus {
    pub expected_revision: String,
    pub expected_generation: u64,
    pub transaction_id: String,
    pub radii: TorusRadii,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TorusReplacement {
    pub expected_revision: String,
    pub expected_generation: u64,
    pub transaction_id: String,
    pub radii: TorusRadii,
}

fn stale() -> AppError {
    AppError::Invalid("AMEND_STALE".into())
}

fn definition(project: &Project, capture: &AmendCapture) -> Option<(Document, TorusRadii)> {
    let current = read_document(project, &capture.expected_revision)?;
    let (existing_features, existing_bodies) = match &capture.base_revision {
        Some(revision) => {
            let before = read_document(project, revision)?;
            if before.parameters != current.parameters
                || !current.features.starts_with(&before.features)
                || !current.bodies.starts_with(&before.bodies)
            {
                return None;
            }
            (before.features.len(), before.bodies.len())
        }
        None if current.parameters.is_empty() => (0, 0),
        None => return None,
    };
    if current.features.len() != existing_features + 1
        || current.bodies.len() != existing_bodies + 1
    {
        return None;
    }
    let feature = &current.features[existing_features];
    let body = &current.bodies[existing_bodies];
    let Operation::Torus {
        major_radius: Dimension::Literal {
            mm: major_radius_mm,
        },
        minor_radius: Dimension::Literal {
            mm: minor_radius_mm,
        },
    } = &feature.operation
    else {
        return None;
    };
    if feature.suppressed || body.source_feature_id != feature.id {
        return None;
    }
    let radii = TorusRadii {
        major_radius_mm: *major_radius_mm,
        minor_radius_mm: *minor_radius_mm,
    };
    Some((current, radii))
}

pub async fn status(state: &AppState, project_id: &str) -> Result<Option<LastTorusStatus>> {
    let project = crate::projects::get(state, project_id).await?;
    let history = crate::project_history::load_state(&state.pool, &project).await?;
    let Some(capture) = history.capture_amend(&project)? else {
        return Ok(None);
    };
    Ok(
        definition(&project, &capture).map(|(_, radii)| LastTorusStatus {
            expected_revision: capture.expected_revision,
            expected_generation: capture.generation,
            transaction_id: capture.transaction_id,
            radii,
        }),
    )
}

pub async fn prepare(
    state: &AppState,
    project_id: String,
    replacement: TorusReplacement,
) -> Result<ApplyRequest> {
    let project = crate::projects::get(state, &project_id).await?;
    let history = crate::project_history::load_state(&state.pool, &project).await?;
    let capture = history.capture_amend(&project)?.ok_or_else(stale)?;
    if capture.expected_revision != replacement.expected_revision
        || capture.generation != replacement.expected_generation
        || capture.transaction_id != replacement.transaction_id
    {
        return Err(stale());
    }
    let (mut next, previous) = definition(&project, &capture).ok_or_else(stale)?;
    let radii = replacement.radii;
    if radii.major_radius_mm == previous.major_radius_mm
        && radii.minor_radius_mm == previous.minor_radius_mm
    {
        return Err(AppError::Invalid("NO_CHANGE".into()));
    }
    next.features.last_mut().ok_or_else(stale)?.operation = Operation::Torus {
        major_radius: Dimension::Literal {
            mm: radii.major_radius_mm,
        },
        minor_radius: Dimension::Literal {
            mm: radii.minor_radius_mm,
        },
    };
    next.validate().map_err(crate::cad_ir::app_error)?;
    let revision = uuid::Uuid::new_v4().to_string();
    next.revision_id = revision.clone();
    let mut request = ApplyRequest::new(
        project_id,
        serde_json::to_string(&next)?,
        format!("history:amend:{}", capture.expected_revision),
        Some(capture.expected_revision.clone()),
    );
    request.next_revision = Some(revision);
    request.history_amend = Some(capture);
    Ok(request)
}
