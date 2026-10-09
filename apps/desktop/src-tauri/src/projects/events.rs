//! Preserve head-movement notifications; metadata saves do not create revisions.
use crate::models::Project;

pub(super) fn revision_changed(previous: Option<&Project>, next: &Project) -> bool {
    previous.and_then(|project| project.current_revision.as_ref()) != next.current_revision.as_ref()
}
