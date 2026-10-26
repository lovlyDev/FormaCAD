//! Local plugin journal (`plugins/journal.jsonl`). Never sent anywhere.
//! Rotates once at 1 MB, keeping one previous file.
use serde::{Deserialize, Serialize};
use std::{
    io::{BufRead, Write},
    path::{Path, PathBuf},
};

const MAX_BYTES: u64 = 1024 * 1024;
const MAX_DETAIL: usize = 2000;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub at: String,
    pub plugin_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// install, update, uninstall, enable, disable, call, log, refused.
    pub event: String,
    pub ok: bool,
    pub detail: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

impl Entry {
    pub fn new(plugin_id: &str, version: Option<&str>, event: &str, ok: bool, detail: impl Into<String>) -> Self {
        let mut detail = detail.into();
        if detail.len() > MAX_DETAIL {
            let mut end = MAX_DETAIL;
            while !detail.is_char_boundary(end) {
                end -= 1;
            }
            detail.truncate(end);
        }
        Self {
            at: chrono::Utc::now().to_rfc3339(),
            plugin_id: plugin_id.to_owned(),
            version: version.map(str::to_owned),
            event: event.to_owned(),
            ok,
            detail,
            duration_ms: None,
        }
    }
}

pub struct Journal {
    path: PathBuf,
    lock: std::sync::Mutex<()>,
}

impl Journal {
    pub fn new(root: &Path) -> Self {
        Self {
            path: root.join("journal.jsonl"),
            lock: Default::default(),
        }
    }

    /// Best effort: a full disk must not turn into a plugin failure.
    pub fn append(&self, entry: &Entry) {
        let _guard = self.lock.lock().unwrap_or_else(|poison| poison.into_inner());
        if std::fs::metadata(&self.path).is_ok_and(|meta| meta.len() > MAX_BYTES) {
            let _ = std::fs::rename(&self.path, self.path.with_extension("1.jsonl"));
        }
        let Ok(line) = serde_json::to_string(entry) else { return };
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&self.path) {
            let _ = writeln!(file, "{line}");
        }
        tracing::info!(target: "forma_core::plugins", plugin = %entry.plugin_id, event = %entry.event, ok = entry.ok);
    }

    /// Newest first, optionally for one plugin.
    pub fn recent(&self, plugin_id: Option<&str>, limit: usize) -> Vec<Entry> {
        let _guard = self.lock.lock().unwrap_or_else(|poison| poison.into_inner());
        let Ok(file) = std::fs::File::open(&self.path) else { return vec![] };
        let mut entries: Vec<Entry> = std::io::BufReader::new(file)
            .lines()
            .map_while(|line| line.ok())
            .filter_map(|line| serde_json::from_str(&line).ok())
            .filter(|entry: &Entry| plugin_id.is_none_or(|id| entry.plugin_id == id))
            .collect();
        entries.reverse();
        entries.truncate(limit);
        entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appends_filters_and_truncates() {
        let dir = tempfile::tempdir().unwrap();
        let journal = Journal::new(dir.path());
        journal.append(&Entry::new("a", Some("1.0.0"), "install", true, "ok"));
        journal.append(&Entry::new("b", None, "call", false, "ж".repeat(3000)));
        let all = journal.recent(None, 10);
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].plugin_id, "b");
        assert!(all[0].detail.len() <= MAX_DETAIL);
        assert_eq!(journal.recent(Some("a"), 10).len(), 1);
    }
}
