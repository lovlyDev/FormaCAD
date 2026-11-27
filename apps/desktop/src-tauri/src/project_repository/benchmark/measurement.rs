use serde_json::json;
use std::{
    path::Path,
    time::{Duration, Instant},
};

pub(super) fn row(size: &str, backend: &str, operation: &str, elapsed: Duration, note: &str) {
    println!(
        "STORAGE_BENCHMARK {}",
        json!({"kind":"measurement","size":size,"backend":backend,
        "operation":operation,"elapsed_ns":elapsed.as_nanos(),"note":note,"samples":1})
    );
}

pub(super) fn measured<T>(size: &str, backend: &str, operation: &str, action: impl FnOnce() -> T) {
    let start = Instant::now();
    std::hint::black_box(action());
    row(
        size,
        backend,
        operation,
        start.elapsed(),
        "single observation; filesystem cache not flushed",
    );
}

pub(super) fn bytes_on_disk(root: &Path) -> u64 {
    std::fs::read_dir(root)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            let metadata = entry.metadata().unwrap();
            if metadata.is_dir() {
                bytes_on_disk(&entry.path())
            } else {
                metadata.len()
            }
        })
        .sum()
}
