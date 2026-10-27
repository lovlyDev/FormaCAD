//! Turntable video recording requested by plugins through `viewport` (plugin-api 0.2).
//! A plugin call only queues a validated [`spec::OrbitSpec`]; the UI renders and
//! encodes the frames and streams the encoded bytes here. Each request gets a
//! one-time ticket bound to the single destination the user picked natively.
pub mod ipc;
pub mod spec;

use std::{
    collections::HashMap,
    fs::File,
    io::{Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::Mutex,
};

/// Encoded video size limit; the UI is told the same in its error text.
pub const MAX_VIDEO_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const MAX_PENDING: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureError {
    UnknownTicket,
    NoDestination,
    TooLarge,
    Io(String),
}

impl CaptureError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::UnknownTicket => "PLUGIN_CAPTURE_TICKET",
            Self::NoDestination => "PLUGIN_CAPTURE_NO_DESTINATION",
            Self::TooLarge => "PLUGIN_CAPTURE_TOO_LARGE",
            Self::Io(_) => "PLUGIN_CAPTURE_WRITE",
        }
    }
    pub fn detail(&self) -> String {
        match self {
            Self::Io(detail) => detail.clone(),
            _ => self.code().into(),
        }
    }
}

struct Ticket {
    plugin_id: String,
    extension: &'static str,
    destination: Option<(PathBuf, File)>,
}

/// Pending recordings, keyed by a random token known only to the UI.
#[derive(Default)]
pub struct CaptureDesk {
    tickets: Mutex<HashMap<String, Ticket>>,
}

fn part_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".part");
    path.with_file_name(name)
}

impl CaptureDesk {
    fn tickets(&self) -> std::sync::MutexGuard<'_, HashMap<String, Ticket>> {
        self.tickets.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// Issue a ticket; an older pending request of the same plugin is dropped.
    pub fn issue(&self, plugin_id: &str, spec: &spec::OrbitSpec) -> String {
        let token = uuid::Uuid::new_v4().simple().to_string();
        let mut tickets = self.tickets();
        let stale: Vec<_> = tickets
            .iter()
            .filter(|(_, t)| t.plugin_id == plugin_id)
            .map(|(k, _)| k.clone())
            .collect();
        for key in stale {
            if let Some(ticket) = tickets.remove(&key) {
                discard(ticket);
            }
        }
        if tickets.len() >= MAX_PENDING {
            if let Some(key) = tickets.keys().next().cloned() {
                if let Some(ticket) = tickets.remove(&key) {
                    discard(ticket);
                }
            }
        }
        tickets.insert(
            token.clone(),
            Ticket {
                plugin_id: plugin_id.to_owned(),
                extension: spec.extension(),
                destination: None,
            },
        );
        token
    }

    pub fn extension(&self, token: &str) -> Result<&'static str, CaptureError> {
        self.tickets()
            .get(token)
            .map(|t| t.extension)
            .ok_or(CaptureError::UnknownTicket)
    }

    /// Bind the path from the native save picker; the bytes go to `<path>.part`
    /// until [`CaptureDesk::finish`] succeeds.
    pub fn bind(&self, token: &str, path: PathBuf) -> Result<(), CaptureError> {
        let mut tickets = self.tickets();
        let ticket = tickets.get_mut(token).ok_or(CaptureError::UnknownTicket)?;
        let file = File::create(part_path(&path)).map_err(|e| CaptureError::Io(e.to_string()))?;
        if let Some(previous) = ticket.destination.replace((path, file)) {
            let _ = std::fs::remove_file(part_path(&previous.0));
        }
        Ok(())
    }

    pub fn write(&self, token: &str, offset: u64, bytes: &[u8]) -> Result<(), CaptureError> {
        let end = offset
            .checked_add(bytes.len() as u64)
            .ok_or(CaptureError::TooLarge)?;
        if end > MAX_VIDEO_BYTES {
            return Err(CaptureError::TooLarge);
        }
        let mut tickets = self.tickets();
        let ticket = tickets.get_mut(token).ok_or(CaptureError::UnknownTicket)?;
        let (_, file) = ticket
            .destination
            .as_mut()
            .ok_or(CaptureError::NoDestination)?;
        file.seek(SeekFrom::Start(offset))
            .and_then(|_| file.write_all(bytes))
            .map_err(|e| CaptureError::Io(e.to_string()))
    }

    /// Complete (`keep`) or abandon the recording. Returns the saved path.
    pub fn finish(&self, token: &str, keep: bool) -> Result<Option<PathBuf>, CaptureError> {
        let ticket = self
            .tickets()
            .remove(token)
            .ok_or(CaptureError::UnknownTicket)?;
        if !keep {
            discard(ticket);
            return Ok(None);
        }
        let (path, file) = ticket.destination.ok_or(CaptureError::NoDestination)?;
        file.sync_all()
            .map_err(|e| CaptureError::Io(e.to_string()))?;
        drop(file);
        std::fs::rename(part_path(&path), &path).map_err(|e| CaptureError::Io(e.to_string()))?;
        Ok(Some(path))
    }
}

fn discard(ticket: Ticket) {
    if let Some((path, file)) = ticket.destination {
        drop(file);
        let _ = std::fs::remove_file(part_path(&path));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> spec::OrbitSpec {
        spec::OrbitSpec {
            width: 640,
            height: 360,
            fps: 30,
            duration_ms: 1000,
            turns: 1.0,
            clockwise: false,
            up: "z",
            tilt_deg: 0.0,
            elevation_deg: 20.0,
            start_deg: 0.0,
            zoom: 1.0,
            background: "theme",
            format: "webm",
            bitrate_kbps: 0,
        }
    }

    #[test]
    fn writes_at_offsets_and_renames_on_finish() {
        let dir = tempfile::tempdir().unwrap();
        let desk = CaptureDesk::default();
        let token = desk.issue("orbit", &spec());
        assert_eq!(desk.extension(&token), Ok("webm"));
        assert_eq!(
            desk.write(&token, 0, b"x"),
            Err(CaptureError::NoDestination)
        );
        let path = dir.path().join("turn.webm");
        desk.bind(&token, path.clone()).unwrap();
        desk.write(&token, 0, b"hello world").unwrap();
        desk.write(&token, 0, b"HELLO").unwrap();
        assert!(!path.exists());
        assert_eq!(desk.finish(&token, true).unwrap(), Some(path.clone()));
        assert_eq!(std::fs::read(&path).unwrap(), b"HELLO world");
        assert_eq!(desk.finish(&token, true), Err(CaptureError::UnknownTicket));
    }

    #[test]
    fn abandoning_removes_the_partial_file_and_new_requests_replace_old() {
        let dir = tempfile::tempdir().unwrap();
        let desk = CaptureDesk::default();
        let first = desk.issue("orbit", &spec());
        let path = dir.path().join("turn.webm");
        desk.bind(&first, path.clone()).unwrap();
        desk.write(&first, 0, b"data").unwrap();
        let second = desk.issue("orbit", &spec());
        assert_eq!(desk.extension(&first), Err(CaptureError::UnknownTicket));
        assert!(!part_path(&path).exists());
        assert_eq!(
            desk.write(&second, MAX_VIDEO_BYTES, b"x"),
            Err(CaptureError::TooLarge)
        );
        desk.bind(&second, path.clone()).unwrap();
        assert_eq!(desk.finish(&second, false).unwrap(), None);
        assert!(!path.exists() && !part_path(&path).exists());
    }
}
