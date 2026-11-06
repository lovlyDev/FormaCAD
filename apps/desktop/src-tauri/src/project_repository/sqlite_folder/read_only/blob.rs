//! CAS binary reads share SQLite's OS file/inode-close bookkeeping. A malicious
//! DB hardlink cannot cause an unrelated raw descriptor close to drop DB locks.
use super::{abi, invalid, source::Source, vfs};
use crate::core::Result;
use std::sync::Arc;

pub(super) fn read(source: &Arc<Source>, hash: &str, size: u64) -> Result<Vec<u8>> {
    if !crate::artifacts::valid_digest(hash)
        || size == 0
        || size > crate::artifacts::MAX_FILE_BYTES as u64
    {
        return Err(invalid());
    }
    source.check()?;
    let directory = source.binding.subdirectory("assets")?;
    if !directory.matches_directory(&source.binding.path("assets"))? {
        return Err(invalid());
    }
    let before = directory.file_record(hash)?;
    if before.size != size {
        return Err(invalid());
    }
    let alias = directory.alias(hash)?;
    let mut file = vfs::data_file(Arc::clone(source), &alias)?;
    // SAFETY: live uniquely owned base SQLite OS file, no concurrent calls.
    let methods = unsafe { &*(*file.base).pMethods };
    let lock = methods.xLock.ok_or_else(invalid)?;
    if unsafe { lock(file.base, 1) } != abi::OK {
        return Err(invalid());
    }
    if unsafe { directory.actual_file_record(file.base, hash) }? != before {
        return Err(invalid());
    }
    let mut actual_size = 0_i64;
    let get_size = methods.xFileSize.ok_or_else(invalid)?;
    if unsafe { get_size(file.base, &mut actual_size) } != abi::OK
        || actual_size < 0
        || actual_size as u64 != size
    {
        return Err(invalid());
    }
    let mut bytes = vec![0_u8; size as usize];
    let read = methods.xRead.ok_or_else(invalid)?;
    for (index, chunk) in bytes.chunks_mut(64 * 1024).enumerate() {
        let offset = (index * 64 * 1024) as i64;
        // SAFETY: live writable chunk, checked bounded size and file offset.
        if unsafe {
            read(
                file.base,
                chunk.as_mut_ptr().cast(),
                chunk.len() as i32,
                offset,
            )
        } != abi::OK
        {
            return Err(invalid());
        }
    }
    if crate::artifacts::digest(&bytes) != hash
        || unsafe { directory.actual_file_record(file.base, hash) }? != before
        || !directory.matches_directory(&source.binding.path("assets"))?
    {
        return Err(invalid());
    }
    if unsafe { get_size(file.base, &mut actual_size) } != abi::OK
        || actual_size < 0
        || actual_size as u64 != size
    {
        return Err(invalid());
    }
    // Explicit close reports errors; Drop is still idempotent on every failure.
    if unsafe { file.close() } != abi::OK {
        return Err(invalid());
    }
    source.check()?;
    Ok(bytes)
}
