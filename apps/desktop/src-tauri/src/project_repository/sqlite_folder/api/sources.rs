//! Window-owned source registry. Tokens identify live readers, never writer grants.
use super::{FolderProject, ReadView};
use crate::core::{AppError, Result};
use std::{
    collections::{HashMap, HashSet},
    fmt,
    path::Path,
    sync::{Arc, Mutex},
};

pub const MAX_FOLDER_SOURCES: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FolderSourceId {
    scope: uuid::Uuid,
    sequence: u64,
    nonce: uuid::Uuid,
}
impl fmt::Display for FolderSourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", self.scope, self.sequence, self.nonce)
    }
}

pub struct OpenedFolder {
    pub id: FolderSourceId,
    pub snapshot: ReadView,
}

struct State {
    active: bool,
    next: u64,
    pending: HashSet<u64>,
    sources: HashMap<u64, (uuid::Uuid, Arc<FolderProject>)>,
}

/// The host owns one instance per window. It persists neither paths nor projects.
pub struct FolderSources {
    scope: uuid::Uuid,
    state: Mutex<State>,
}
impl Default for FolderSources {
    fn default() -> Self {
        Self {
            scope: uuid::Uuid::new_v4(),
            state: Mutex::new(State {
                active: true,
                next: 0,
                pending: HashSet::new(),
                sources: HashMap::new(),
            }),
        }
    }
}
fn stale() -> AppError {
    AppError::Invalid("PROJECT_SNAPSHOT_CHANGED".into())
}
fn limit() -> AppError {
    AppError::Invalid("PROJECT_SOURCE_LIMIT".into())
}

struct Opening<'a> {
    owner: &'a FolderSources,
    id: FolderSourceId,
    published: Option<Arc<FolderProject>>,
    keep: bool,
}
impl FolderSources {
    fn reserve(&self) -> Result<Opening<'_>> {
        let mut state = self.state.lock().map_err(|_| stale())?;
        if !state.active {
            return Err(stale());
        }
        if state.sources.len() + state.pending.len() >= MAX_FOLDER_SOURCES {
            return Err(limit());
        }
        let sequence = state.next.checked_add(1).ok_or_else(limit)?;
        state.next = sequence;
        state.pending.insert(sequence);
        Ok(Opening {
            owner: self,
            id: FolderSourceId {
                scope: self.scope,
                sequence,
                nonce: uuid::Uuid::new_v4(),
            },
            published: None,
            keep: false,
        })
    }

    /// A trusted host invokes this only for an explicitly selected folder.
    /// Capacity is reserved before IO; no registry mutex crosses an await.
    pub async fn open(&self, root: &Path) -> Result<OpenedFolder> {
        let mut opening = self.reserve()?;
        let source = Arc::new(FolderProject::open(root).await?);
        let snapshot = source.capture_metadata().await?;
        opening.publish(Arc::clone(&source))?;
        source.recheck(&snapshot).await?;
        let id = opening.finish();
        Ok(OpenedFolder { id, snapshot })
    }

    fn parse(&self, text: &str) -> Result<(u64, uuid::Uuid)> {
        if text.len() > 128 {
            return Err(stale());
        }
        let (scope, rest) = text.split_once(':').ok_or_else(stale)?;
        let (sequence, nonce) = rest.split_once(':').ok_or_else(stale)?;
        if uuid::Uuid::parse_str(scope).map_err(|_| stale())? != self.scope {
            return Err(stale());
        }
        let value: u64 = sequence.parse().map_err(|_| stale())?;
        if value == 0 || value.to_string() != sequence {
            return Err(stale());
        }
        Ok((value, uuid::Uuid::parse_str(nonce).map_err(|_| stale())?))
    }

    /// Returning an existing Arc preserves its original opening; never reopen by path.
    pub fn lookup(&self, text: &str) -> Result<Arc<FolderProject>> {
        let (sequence, nonce) = self.parse(text)?;
        let state = self.state.lock().map_err(|_| stale())?;
        if !state.active {
            return Err(stale());
        }
        state
            .sources
            .get(&sequence)
            .filter(|(expected, _)| *expected == nonce)
            .map(|(_, source)| Arc::clone(source))
            .ok_or_else(stale)
    }

    /// Logical revocation is immediate. Actual connection drop happens outside the map lock.
    pub fn close(&self, text: &str) -> Result<bool> {
        let (sequence, nonce) = self.parse(text)?;
        let source = {
            let mut state = self.state.lock().map_err(|_| stale())?;
            let source = if state
                .sources
                .get(&sequence)
                .is_some_and(|(expected, _)| *expected == nonce)
            {
                state.sources.remove(&sequence).map(|(_, source)| source)
            } else {
                None
            };
            if let Some(source) = &source {
                source.close();
            }
            source
        };
        Ok(source.is_some())
    }

    /// Permanent window close: pending openings cannot publish afterward.
    pub fn close_all(&self) -> Result<()> {
        let sources: Vec<_> = {
            let mut state = self.state.lock().map_err(|_| stale())?;
            state.active = false;
            state.pending.clear();
            state
                .sources
                .drain()
                .map(|(_, (_, source))| {
                    source.close();
                    source
                })
                .collect()
        };
        drop(sources);
        Ok(())
    }
}

impl Opening<'_> {
    fn publish(&mut self, source: Arc<FolderProject>) -> Result<()> {
        let mut state = self.owner.state.lock().map_err(|_| stale())?;
        if !state.active || !state.pending.remove(&self.id.sequence) {
            return Err(stale());
        }
        state
            .sources
            .insert(self.id.sequence, (self.id.nonce, Arc::clone(&source)));
        self.published = Some(source);
        Ok(())
    }
    fn finish(mut self) -> FolderSourceId {
        self.keep = true;
        self.id
    }
}
impl Drop for Opening<'_> {
    fn drop(&mut self) {
        if self.keep {
            return;
        }
        let removed = {
            // Poison recovery only revokes/releases this operation; it grants nothing.
            let mut state = self
                .owner
                .state
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            state.pending.remove(&self.id.sequence);
            if let Some(source) = &self.published {
                source.close();
                if state
                    .sources
                    .get(&self.id.sequence)
                    .is_some_and(|(nonce, current)| {
                        *nonce == self.id.nonce && Arc::ptr_eq(current, source)
                    })
                {
                    state
                        .sources
                        .remove(&self.id.sequence)
                        .map(|(_, source)| source)
                } else {
                    None
                }
            } else {
                None
            }
        };
        drop(removed);
    }
}
impl Drop for FolderSources {
    fn drop(&mut self) {
        let state = self
            .state
            .get_mut()
            .unwrap_or_else(|error| error.into_inner());
        state.active = false;
        state.pending.clear();
        for (_, source) in state.sources.values() {
            source.close();
        }
    }
}

#[cfg(test)]
mod tests;
