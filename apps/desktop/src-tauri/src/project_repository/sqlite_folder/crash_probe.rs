//! Child-process barriers compiled only in the storage test binaries.
use std::time::Duration;

pub(super) const POINT: &str = "FORMA_FCP1_CRASH_PROBE_POINT";
pub(super) const READY: &str = "FORMA_FCP1_CRASH_PROBE_READY";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Point {
    BeforeStaging,
    AfterAssetFlush,
    DuringSqlWrite,
    BeforeSqlCommit,
    AfterSqlCommit,
}

impl Point {
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::BeforeStaging => "before-staging",
            Self::AfterAssetFlush => "after-asset-flush",
            Self::DuringSqlWrite => "during-sql-write",
            Self::BeforeSqlCommit => "before-sql-commit",
            Self::AfterSqlCommit => "after-sql-commit",
        }
    }

    pub(super) fn wait_for_kill(self) {
        if std::env::var(POINT).as_deref() != Ok(self.name()) {
            return;
        }
        let ready = std::env::var_os(READY).expect("parent supplies barrier path");
        std::fs::write(ready, self.name()).unwrap();
        std::thread::sleep(Duration::from_secs(60));
        panic!("parent must terminate the child writer at {}", self.name());
    }
}
