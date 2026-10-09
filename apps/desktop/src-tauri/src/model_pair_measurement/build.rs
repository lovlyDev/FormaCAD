use super::{
    failure, schema::PairMeasurementQuery, workspace::Workspace, ConfirmedPairMeasurement,
};
use crate::{
    core::{AppState, Result},
    model_measurement::capture,
};
use std::{
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

fn cancelled(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Relaxed) {
        Err(failure(
            "CAD_TASK_CANCELLED",
            "Exact reference measurement cancelled",
        ))
    } else {
        Ok(())
    }
}

/// Rust-only executable seam. No worker path or fabricated metric enters the IPC.
#[allow(clippy::too_many_arguments)]
pub async fn measure_with_executable(
    state: &AppState,
    project_id: &str,
    expected_revision: &str,
    body_id: &str,
    query: &PairMeasurementQuery,
    cancel: Arc<AtomicBool>,
    executable: &Path,
) -> Result<ConfirmedPairMeasurement> {
    measure_inner(
        state,
        project_id,
        expected_revision,
        body_id,
        query,
        cancel,
        executable,
        PhaseHook::default(),
    )
    .await
}

#[derive(Default)]
struct PhaseHook<'a> {
    #[cfg(test)]
    after_worker: Option<&'a (dyn Fn() + Send + Sync)>,
    _lifetime: std::marker::PhantomData<&'a ()>,
}

#[allow(clippy::too_many_arguments)]
async fn measure_inner(
    state: &AppState,
    project_id: &str,
    expected_revision: &str,
    body_id: &str,
    query: &PairMeasurementQuery,
    cancel: Arc<AtomicBool>,
    executable: &Path,
    hook: PhaseHook<'_>,
) -> Result<ConfirmedPairMeasurement> {
    if !query.valid() {
        return Err(failure(
            "INVALID_MEASUREMENT_QUERY",
            "Measurement reference kind or shape is invalid",
        ));
    }
    cancelled(&cancel)?;
    let captured = capture::capture(state, project_id, expected_revision, body_id).await?;
    let _permit = state
        .cad_tasks
        .acquire(project_id, "pair_measurement", cancel.clone())
        .await?;
    cancelled(&cancel)?;
    capture::recheck(state, &captured).await?;
    let workspace = Workspace::create(&state.root, project_id)?;
    crate::artifacts::immutable_write(
        &workspace.path,
        Path::new("document.json"),
        &captured.program_bytes,
    )?;
    crate::artifacts::immutable_write(
        &workspace.path,
        Path::new("source.step"),
        &capture::source_bytes(state, &captured)?,
    )?;
    crate::native::assets::stage_project_assets(
        state,
        &captured.project,
        &captured.document,
        &workspace.path,
    )?;
    cancelled(&cancel)?;
    let report = crate::native::pair_measurements::client::measure_with_executable(
        &workspace.path,
        cancel.clone(),
        executable,
        &captured.binding,
        query,
    )
    .await?;
    cancelled(&cancel)?;
    if !report.matches(&report.request_id, &captured.binding, query) {
        return Err(failure(
            "MEASUREMENT_INVALID_RESULT",
            "Verified worker report does not match host capture",
        ));
    }
    #[cfg(test)]
    if let Some(after_worker) = hook.after_worker {
        after_worker();
    }
    #[cfg(not(test))]
    let _ = hook;
    capture::recheck(state, &captured).await?;
    cancelled(&cancel)?;
    Ok(ConfirmedPairMeasurement {
        project_id: project_id.into(),
        report,
    })
}

#[cfg(test)]
#[path = "build_tests.rs"]
mod tests;
