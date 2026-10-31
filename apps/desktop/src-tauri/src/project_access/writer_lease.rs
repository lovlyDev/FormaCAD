//! One logical writer lease, released before the final owning file is closed.
//! Duplicated OS descriptors must not extend the parent registry's ownership.
use std::fs::File;

#[derive(Debug)]
pub(super) struct WriterLease {
    pub(super) file: File,
}

impl WriterLease {
    /// The caller has already acquired this file's exclusive OS lock.
    pub(super) fn new(file: File) -> Self {
        Self { file }
    }
}

impl Drop for WriterLease {
    fn drop(&mut self) {
        // Closing alone can leave an OS lock held by a duplicated descriptor,
        // including one inherited by a concurrent child before exec. All parent
        // guards share this wrapper, so only their final owner unlocks it.
        if let Err(error) = self.file.unlock() {
            tracing::warn!(%error, "Project writer lease unlock failed");
        }
    }
}
