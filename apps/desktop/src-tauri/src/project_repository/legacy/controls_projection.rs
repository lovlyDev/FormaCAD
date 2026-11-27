//! Model-only controls input. Ignored JSON is traversed, never decoded as chat,
//! thumbnail, prompt or embedded attachment strings. Not a full Project reader.
use crate::{
    core::{AppError, Result},
    models::Parameters,
    project_history::model_context::{HistoryModelContext, HistoryRevision},
};
use serde::{de::IgnoredAny, Deserialize};
use std::collections::HashSet;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Projection {
    schema_version: u32,
    pub id: String,
    name: String,
    units: String,
    agent: String,
    #[serde(rename = "pinned")]
    _pinned: bool,
    #[serde(default, rename = "thumbnail")]
    _thumbnail: IgnoredAny,
    thumbnail_revision: Option<String>,
    #[serde(rename = "createdAt")]
    _created_at: IgnoredAny,
    #[serde(rename = "updatedAt")]
    _updated_at: IgnoredAny,
    revisions: Vec<ModelRevision>,
    pub current_revision: Option<String>,
    #[serde(rename = "messages", deserialize_with = "skip_messages")]
    _messages: (),
    pub files: Vec<ModelFile>,
    #[serde(rename = "exports")]
    _exports: IgnoredAny,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ModelRevision {
    id: String,
    parent: Option<String>,
    #[serde(rename = "createdAt")]
    _created_at: IgnoredAny,
    #[serde(rename = "prompt")]
    _prompt: IgnoredAny,
    parameters: Parameters,
    source: Option<String>,
    preview: Option<String>,
    program: Option<String>,
    program_base: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ModelFile {
    pub name: String,
    pub size: u64,
    #[serde(rename = "kind")]
    _kind: String,
    pub data: Option<IgnoredAny>,
    pub sha256: Option<String>,
}

fn skip_messages<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<(), D::Error> {
    struct Count;
    impl<'de> serde::de::Visitor<'de> for Count {
        type Value = ();
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("a bounded messages array")
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(
            self,
            mut seq: A,
        ) -> std::result::Result<(), A::Error> {
            let mut count = 0usize;
            while seq.next_element::<IgnoredAny>()?.is_some() {
                count += 1;
                if count > 10000 {
                    return Err(serde::de::Error::custom(
                        "Unsupported or invalid project metadata",
                    ));
                }
            }
            Ok(())
        }
    }
    deserializer.deserialize_seq(Count)
}

fn invalid() -> AppError {
    AppError::Invalid("HISTORY_INVALID_STATE".into())
}

impl Projection {
    pub fn context(&self) -> HistoryModelContext<'_> {
        HistoryModelContext {
            current_revision: self.current_revision.as_deref(),
            revisions: self
                .revisions
                .iter()
                .map(|r| HistoryRevision {
                    id: &r.id,
                    parent: r.parent.as_deref(),
                    parameters: &r.parameters,
                    source: &r.source,
                    preview: &r.preview,
                    program_base: &r.program_base,
                    program: &r.program,
                })
                .collect(),
        }
    }

    pub fn supported(&self) -> bool {
        self.files
            .iter()
            .all(|file| file.data.is_none() && file.sha256.is_some())
    }

    pub fn validate(&self) -> Result<()> {
        crate::security::valid_id(&self.id)?;
        if self.schema_version != 1
            || self.name.trim().is_empty()
            || self.name.chars().count() > 80
            || !["mm", "cm", "inch"].contains(&self.units.as_str())
            || !["codex", "claude", "custom"].contains(&self.agent.as_str())
            || self.revisions.len() > 10000
        {
            return Err(invalid());
        }
        let mut ids = HashSet::new();
        for r in &self.revisions {
            crate::security::valid_id(&r.id)?;
            if r.parent.as_deref().is_some_and(|id| !ids.contains(id)) || !ids.insert(r.id.as_str())
            {
                return Err(invalid());
            }
            r.parameters.validate()?;
            if r.program
                .as_ref()
                .is_some_and(|p| p.len() > 60000 || p.trim().is_empty())
            {
                return Err(invalid());
            }
        }
        if self
            .current_revision
            .as_deref()
            .is_some_and(|id| !ids.contains(id))
            || self
                .thumbnail_revision
                .as_deref()
                .is_some_and(|id| !ids.contains(id))
        {
            return Err(invalid());
        }
        let mut names = HashSet::new();
        for file in &self.files {
            crate::security::file_name(&file.name)?;
            if !names.insert(file.name.as_str())
                || file.size > crate::artifacts::MAX_FILE_BYTES as u64
                || file
                    .sha256
                    .as_deref()
                    .is_some_and(|hash| !crate::artifacts::valid_digest(hash))
            {
                return Err(invalid());
            }
        }
        for r in &self.revisions {
            if [&r.source, &r.preview, &r.program_base]
                .iter()
                .any(|name| name.as_deref().is_some_and(|name| !names.contains(name)))
            {
                return Err(invalid());
            }
        }
        Ok(())
    }
}
