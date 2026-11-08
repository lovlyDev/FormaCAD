//! Durable publication reports are data, never a newly minted read/write session.
use super::format;
use crate::{core::AppError, models::Project};

pub struct SavedCommit {
    saved: Box<format::Snapshot>,
}
impl SavedCommit {
    pub fn project(&self) -> &Project {
        self.saved.project()
    }
    pub fn commit_id(&self) -> &str {
        self.saved.expectation().commit_id()
    }
    pub fn sequence(&self) -> u64 {
        self.saved.expectation().sequence()
    }
}

pub enum Publication {
    Committed(SavedCommit),
    RejectedBefore(AppError),
    Indeterminate { commit_id: String, error: AppError },
}
impl From<format::CommitOutcome> for Publication {
    fn from(outcome: format::CommitOutcome) -> Self {
        match outcome {
            format::CommitOutcome::Committed(saved) => Self::Committed(SavedCommit { saved }),
            format::CommitOutcome::RejectedBefore(error) => Self::RejectedBefore(error),
            format::CommitOutcome::Indeterminate { commit_id, error } => {
                Self::Indeterminate { commit_id, error }
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum CommitPresence {
    Present { is_head: bool },
    Absent,
}
