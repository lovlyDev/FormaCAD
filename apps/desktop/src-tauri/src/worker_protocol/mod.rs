//! Isolated worker protocol. Legacy handlers preserved; new operations separate.
mod dispatch;
mod face_section;
mod measurement;
mod pair_measurement;
mod schema;
use std::{
    fs,
    io::{self, Read},
    path::Path,
};

pub fn run(input: impl Read, cwd: &Path) -> io::Result<()> {
    let mut bytes = Vec::new();
    input
        .take(schema::MAX_REQUEST_BYTES + 1)
        .read_to_end(&mut bytes)?;
    let response = if bytes.len() as u64 > schema::MAX_REQUEST_BYTES {
        schema::failure(
            String::new(),
            "REQUEST_TOO_LARGE",
            "CAD worker input exceeded the limit",
        )
    } else {
        dispatch::handle(&bytes, cwd)
    };
    let bytes = serde_json::to_vec(&response)?;
    let temporary =
        crate::security::guarded(cwd, Path::new("result.json.tmp")).map_err(io::Error::other)?;
    let target =
        crate::security::guarded(cwd, Path::new("result.json")).map_err(io::Error::other)?;
    fs::write(&temporary, bytes)?;
    fs::rename(temporary, target)?;
    Ok(())
}

#[cfg(test)]
#[path = "protocol_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "face_section_tests.rs"]
mod face_section_tests;

#[cfg(test)]
mod pair_measurement_tests;
