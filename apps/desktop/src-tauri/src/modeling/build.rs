//! Managed adapter for the shared CAD executor.
use super::{
    execution::{self, AssetSource, BuildRuntime},
    program::PreparedProgram,
};
use crate::{
    agents::review_plan::ReviewPlan,
    cad_ir::Document,
    core::{AppState, Result},
    models::{Project, ProjectFile},
};
use std::{
    path::Path,
    sync::{atomic::AtomicBool, Arc},
};

pub(crate) struct GeneratedModel {
    pub step: Vec<u8>,
    pub preview: Vec<u8>,
    pub program_base: Option<String>,
}

struct ManagedAssets<'a> {
    state: &'a AppState,
    project: &'a Project,
}
impl AssetSource for ManagedAssets<'_> {
    fn read(&mut self, file: &ProjectFile) -> Result<Vec<u8>> {
        crate::artifacts::read(self.state, &self.project.id, file)
    }
    #[cfg(feature = "native-worker-client")]
    fn stage_native(&mut self, project: &Project, document: &Document, cwd: &Path) -> Result<()> {
        // Keep the existing managed preflight and its error classification.
        crate::native::assets::stage_project_assets(self.state, project, document, cwd)
    }
}

pub(super) async fn build(
    state: &AppState,
    original: &Project,
    program: &str,
    prepared: &PreparedProgram,
    review: Option<(&ReviewPlan, &Document)>,
    cancel: Arc<AtomicBool>,
    executable: Option<&Path>,
) -> Result<GeneratedModel> {
    execution::build(
        BuildRuntime {
            runtime_root: &state.root,
            transient_root: &state.root,
            executable,
        },
        &mut ManagedAssets {
            state,
            project: original,
        },
        original,
        program,
        prepared,
        review,
        cancel,
    )
    .await
}
