use crate::core::{AppError, Result};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Parameters {
    pub kind: String,
    pub width: f64,
    pub depth: f64,
    pub height: f64,
    pub thickness: f64,
    pub hole_diameter: f64,
    pub holes: u32,
}
impl Parameters {
    pub fn hole_limit(&self) -> f64 {
        if self.kind != "box" {
            return self.width.min(self.depth) / 4.0;
        }
        if self.holes <= 1 {
            return self.width.min(self.depth) * 0.9;
        }
        let columns = self.holes.div_ceil(2);
        (self.width * 0.36)
            .min(self.depth * 0.44)
            .min(if columns > 1 {
                self.width * 0.64 / f64::from(columns - 1) * 0.9
            } else {
                self.depth * 0.5
            })
    }
    pub fn same_geometry(&self, other: &Self) -> bool {
        fn canonical(p: &Parameters) -> Parameters {
            let mut p = p.clone();
            if p.kind == "box" || p.kind == "cylinder" {
                p.thickness = 0.0;
            }
            if p.kind == "plate" {
                p.height = 0.0;
            }
            if p.kind == "cylinder" {
                p.depth = 0.0;
                p.holes = 0;
            }
            if p.kind == "enclosure" {
                p.holes = 0;
                p.hole_diameter = 0.0;
            }
            if p.kind != "cylinder" && (p.holes == 0 || p.hole_diameter == 0.0) {
                p.holes = 0;
                p.hole_diameter = 0.0;
            }
            p
        }
        canonical(self) == canonical(other)
    }

    pub fn validate(&self) -> Result<()> {
        if !["box", "blank", "bracket", "enclosure", "plate", "cylinder"]
            .contains(&self.kind.as_str())
        {
            return Err(AppError::Invalid("Unsupported part type".into()));
        }
        if ![self.width, self.depth, self.height]
            .iter()
            .all(|v| v.is_finite() && *v >= 1.0 && *v <= 2000.0)
            || !self.thickness.is_finite()
            || self.thickness < 0.2
            || self.thickness > 100.0
            || self.thickness * 2.0 >= self.width.min(self.depth).min(self.height)
            || !self.hole_diameter.is_finite()
            || self.hole_diameter < 0.0
            || self.hole_diameter > 200.0
            || self.hole_diameter > self.hole_limit()
            || self.holes > 16
        {
            return Err(AppError::Invalid(
                "Dimensions, wall thickness or holes are outside supported bounds".into(),
            ));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Revision {
    pub id: String,
    pub parent: Option<String>,
    pub created_at: String,
    pub prompt: String,
    pub parameters: Parameters,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub program: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub program_base: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Message {
    pub id: String,
    pub role: String,
    pub text: String,
    pub created_at: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectFile {
    pub name: String,
    pub size: u64,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Export {
    pub name: String,
    pub created_at: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Project {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub units: String,
    pub agent: String,
    pub pinned: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail_revision: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub revisions: Vec<Revision>,
    pub current_revision: Option<String>,
    pub messages: Vec<Message>,
    pub files: Vec<ProjectFile>,
    pub exports: Vec<Export>,
}
impl Project {
    pub fn validate(&self) -> Result<()> {
        crate::security::valid_id(&self.id)?;
        if self.schema_version != 1
            || self.name.trim().is_empty()
            || self.name.chars().count() > 80
            || !["mm", "cm", "inch"].contains(&self.units.as_str())
            || !["codex", "claude", "custom"].contains(&self.agent.as_str())
            || self.revisions.len() > 10000
            || self.messages.len() > 10000
        {
            return Err(AppError::Invalid(
                "Unsupported or invalid project metadata".into(),
            ));
        }
        let mut ids = std::collections::HashSet::new();
        for r in &self.revisions {
            crate::security::valid_id(&r.id)?;
            if r.parent.as_ref().is_some_and(|id| !ids.contains(id)) {
                return Err(AppError::Invalid(
                    "Revision parent is missing or not an ancestor".into(),
                ));
            }
            if !ids.insert(r.id.clone()) {
                return Err(AppError::Invalid("Duplicate revision ID".into()));
            }
            r.parameters.validate()?;
            if r.program
                .as_ref()
                .is_some_and(|p| p.len() > 60000 || p.trim().is_empty())
            {
                return Err(AppError::Invalid("Invalid CAD program length".into()));
            }
            if r.prompt.len() > 32000 {
                return Err(AppError::Invalid("Revision description is too long".into()));
            }
        }
        if self
            .current_revision
            .as_ref()
            .is_some_and(|id| !ids.contains(id))
        {
            return Err(AppError::Invalid("Current revision does not exist".into()));
        }
        if self.thumbnail.as_ref().is_some_and(|value| {
            !value.starts_with("data:image/jpeg;base64,") || value.len() > 300_000
        }) || self
            .thumbnail_revision
            .as_ref()
            .is_some_and(|id| !ids.contains(id))
        {
            return Err(AppError::Invalid("Invalid project thumbnail".into()));
        }
        let mut names = std::collections::HashSet::new();
        let mut total = 0;
        for f in &self.files {
            crate::security::file_name(&f.name)?;
            if !names.insert(&f.name) {
                return Err(AppError::Invalid("Duplicate attachment filename".into()));
            }
            if f.size > crate::artifacts::MAX_FILE_BYTES as u64
                || f.sha256
                    .as_ref()
                    .is_some_and(|h| !crate::artifacts::valid_digest(h))
            {
                return Err(AppError::Invalid(
                    "Invalid attachment size or checksum".into(),
                ));
            }
            total += f.data.as_ref().map_or(0, |s| s.len());
        }
        if total > 100 * 1024 * 1024 {
            return Err(AppError::Invalid(
                "Project attachments exceed 100 MB".into(),
            ));
        }
        for r in &self.revisions {
            if [&r.source, &r.preview, &r.program_base]
                .iter()
                .any(|value| value.as_ref().is_some_and(|s| !names.contains(s)))
            {
                return Err(AppError::Invalid("Revision source file is missing".into()));
            }
        }
        Ok(())
    }
}
