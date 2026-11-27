use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

pub(super) struct Fixture {
    pub(super) step: PathBuf,
    pub(super) project_id: String,
    root: PathBuf,
    initial_files: BTreeMap<PathBuf, Option<Vec<u8>>>,
    historical_writer: bool,
}

pub(super) fn prepare(root: &Path, step: &Path) -> Fixture {
    let project_id = uuid::Uuid::new_v4().to_string();
    let bytes = fs::read(step).unwrap();
    let generator = std::env::var_os("FORMA_LEGACY_1_1_GENERATOR");
    let selected_step = if let Some(generator) = &generator {
        let output = Command::new(generator)
            .args([
                "--ignored",
                "--exact",
                "write_legacy_1_1_project",
                "--nocapture",
            ])
            .env("FORMA_LEGACY_1_1_DATA", root)
            .env("FORMA_LEGACY_1_1_STEP", step)
            .env("FORMA_LEGACY_1_1_ID", &project_id)
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success(),
            "{stdout}\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            stdout
                .lines()
                .any(|line| line == "FORMA_LEGACY_1_1_FIXTURE_OK"),
            "{stdout}"
        );
        assert!(root.join("forma.sqlite").is_file());
        root.join("projects")
            .join(&project_id)
            .join("attachments")
            .join(format!("{}.step", crate::artifacts::digest(&bytes)))
    } else {
        // Local host validation without a historical build uses a synthetic source.
        let selected = root.join("last.step");
        fs::write(&selected, &bytes).unwrap();
        fs::write(root.join("old-project-sentinel"), &project_id).unwrap();
        selected
    };
    assert_eq!(fs::read(&selected_step).unwrap(), bytes);
    Fixture {
        step: selected_step,
        project_id,
        root: root.into(),
        initial_files: tree(root),
        historical_writer: generator.is_some(),
    }
}

impl Fixture {
    pub(super) fn assert_unchanged(&self) {
        assert_eq!(tree(&self.root), self.initial_files);
        if self.historical_writer {
            println!("\nFORMA_LEGACY_1_1_SOURCE_UNCHANGED");
        }
    }
}

fn tree(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    fn walk(root: &Path, directory: &Path, files: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
        for entry in fs::read_dir(directory).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            let relative = path.strip_prefix(root).unwrap().to_owned();
            if entry.file_type().unwrap().is_dir() {
                files.insert(relative, None);
                walk(root, &path, files);
            } else {
                files.insert(relative, Some(fs::read(path).unwrap()));
            }
        }
    }
    let mut files = BTreeMap::new();
    walk(root, root, &mut files);
    files
}
