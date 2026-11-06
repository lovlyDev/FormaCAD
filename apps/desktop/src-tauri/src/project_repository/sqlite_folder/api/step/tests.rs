//! Public selected-source capability checks; synthetic bytes are not STEP geometry.
use super::*;
use crate::cad_tasks::CadTaskGate;
use std::collections::HashMap;
use tokio::sync::Mutex;

fn intent() -> StepProjectIntent {
    StepProjectIntent {
        name: "Explicit transferred project".into(),
        agent: "codex".into(),
        prompt: "Selected STEP".into(),
    }
}

#[tokio::test]
async fn public_transfer_refuses_another_opening_and_late_foreign_destination_data() {
    let root = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let path = root.path().join("last.step");
    let bytes = b"synthetic selected input, not geometry";
    std::fs::write(&path, bytes).unwrap();
    let first = SelectedStepFile::open(&path).unwrap();
    let original = first.capture().unwrap();
    assert_eq!(original.descriptor().size, bytes.len() as u64);
    assert_eq!(first.read(&original).unwrap(), bytes);
    let second = SelectedStepFile::open(&path).unwrap();
    let other = second.capture().unwrap();
    assert!(second.read(&original).is_err());
    let tasks = Mutex::new(HashMap::new());
    let gate = CadTaskGate::default();
    let runtime = || ModelRuntime {
        tasks: &tasks,
        gate: &gate,
        runtime_root: work.path(),
        transient_root: work.path(),
        executable: None,
    };
    let approval = first.approve_destination(&original, target.path()).unwrap();
    assert!(second
        .transfer_to_folder(&other, approval, intent(), runtime())
        .await
        .is_err());
    assert_eq!(std::fs::read_dir(target.path()).unwrap().count(), 0);
    let approval = first.approve_destination(&original, target.path()).unwrap();
    let foreign = target.path().join("keep.txt");
    std::fs::write(&foreign, b"foreign data").unwrap();
    assert!(first
        .transfer_to_folder(&original, approval, intent(), runtime())
        .await
        .is_err());
    assert_eq!(std::fs::read(foreign).unwrap(), b"foreign data");
    assert_eq!(std::fs::read_dir(target.path()).unwrap().count(), 1);
    assert_eq!(std::fs::read_dir(work.path()).unwrap().count(), 0);
    assert!(tasks.lock().await.is_empty());
    first.close();
    assert!(first.read(&original).is_err());
    second.recheck(&other).unwrap();
    assert_eq!(std::fs::read(path).unwrap(), bytes);
}
