//! A write reply describes what the publisher knows about the original source.
use super::ApprovalSource;
use crate::{
    core::AppError,
    models::Project,
    project_repository::sqlite_folder::api::{ModelChange, Publication},
};
use serde::{Serialize, Serializer};

pub struct FolderWriteReply {
    source_id: String,
    project_id: String,
    base_commit_id: String,
    outcome: ModelChange,
}

impl FolderWriteReply {
    pub(super) fn new(source: ApprovalSource<'_>, outcome: ModelChange) -> Self {
        Self {
            source_id: source.source_id.into(),
            project_id: source.project_id.into(),
            base_commit_id: source.commit_id.into(),
            outcome,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Reply<'a> {
    source_id: &'a str,
    project_id: &'a str,
    base_commit_id: &'a str,
    #[serde(flatten)]
    outcome: Outcome<'a>,
}

#[derive(Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum Outcome<'a> {
    NoOp {
        project: &'a Project,
    },
    Committed {
        commit_id: &'a str,
        project: &'a Project,
    },
    RejectedBefore {
        error: &'a AppError,
    },
    Indeterminate {
        commit_id: &'a str,
        error: &'a AppError,
    },
}

impl Serialize for FolderWriteReply {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let outcome = match &self.outcome {
            ModelChange::NoOp(project) => Outcome::NoOp { project },
            ModelChange::Published(Publication::Committed(saved)) => Outcome::Committed {
                commit_id: saved.commit_id(),
                project: saved.project(),
            },
            ModelChange::Published(Publication::RejectedBefore(error)) => {
                Outcome::RejectedBefore { error }
            }
            ModelChange::Published(Publication::Indeterminate { commit_id, error }) => {
                Outcome::Indeterminate { commit_id, error }
            }
        };
        Reply {
            source_id: &self.source_id,
            project_id: &self.project_id,
            base_commit_id: &self.base_commit_id,
            outcome,
        }
        .serialize(serializer)
    }
}
