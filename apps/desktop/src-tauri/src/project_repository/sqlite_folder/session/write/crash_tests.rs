//! Real process death at explicit FCP1 publication boundaries.
use super::*;
use crate::project_history;

#[tokio::test]
async fn abrupt_exit_exposes_an_intact_head_or_read_only_recovery_state() {
    for point in [
        Point::BeforeStaging,
        Point::AfterAssetFlush,
        Point::DuringSqlWrite,
        Point::BeforeSqlCommit,
        Point::AfterSqlCommit,
    ] {
        let root = tempfile::tempdir().unwrap();
        let control = tempfile::tempdir().unwrap();
        let Publication::Committed(original) =
            FolderProject::create(root.path(), project(), vec![], None)
                .await
                .unwrap()
        else {
            panic!("baseline for {point:?} must be committed");
        };
        let db = root.path().join("project.sqlite");
        let journal = root.path().join("project.sqlite-journal");
        let original_db = fs::read(&db).unwrap();
        let mut writer =
            ParkedWriter::start(root.path(), &control.path().join("ready"), point).await;
        writer.kill();

        let asset = root
            .path()
            .join("assets")
            .join(crate::artifacts::digest(PENDING_BYTES));
        if point == Point::BeforeStaging {
            assert!(!asset.exists());
        } else {
            assert_eq!(fs::read(asset).unwrap(), PENDING_BYTES, "{point:?}");
        }
        if matches!(point, Point::DuringSqlWrite | Point::BeforeSqlCommit) {
            let before = fs::read(&db).unwrap();
            let journal_bytes =
                fs::read(&journal).expect("the real writer created its rollback journal");
            assert!(matches!(
                FolderProject::open(root.path()).await,
                Err(AppError::Invalid(reason)) if reason == "PROJECT_SNAPSHOT_RECOVERY_REQUIRED"
            ));
            assert_eq!(fs::read(&db).unwrap(), before);
            assert_eq!(fs::read(&journal).unwrap(), journal_bytes);
            continue;
        }
        assert!(!journal.exists(), "{point:?}");
        let reopened = FolderProject::open(root.path()).await.unwrap();
        let actual = reopened.capture().await.unwrap();
        let history = reopened.read_history(&actual).await.unwrap();
        project_history::validate_archive(actual.project(), &history).unwrap();
        if point == Point::AfterSqlCommit {
            assert_ne!(actual.commit_id(), original.commit_id());
            assert_eq!(actual.project().name, CANDIDATE_NAME);
            assert_eq!(
                reopened
                    .read_asset(&actual, &actual.project().files[0])
                    .await
                    .unwrap(),
                PENDING_BYTES
            );
        } else {
            assert_eq!(actual.commit_id(), original.commit_id(), "{point:?}");
            assert!(actual.project().files.is_empty());
            assert_eq!(fs::read(&db).unwrap(), original_db, "{point:?}");
        }
    }
}
