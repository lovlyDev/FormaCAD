//! Declarative panel views returned by `panel.render`. Only these blocks
//! exist; CAD renders them with its own components and never interprets HTML.
use super::{forms::FormField, manifest::Manifest};
use serde::{Deserialize, Serialize};

const MAX_BLOCKS: usize = 20;
const MAX_TEXT: usize = 1000;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum Block {
    Text {
        text: String,
    },
    Form {
        fields: Vec<FormField>,
        submit: Submit,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Submit {
    pub command: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct View {
    pub blocks: Vec<Block>,
}

pub fn parse(raw: &str, manifest: &Manifest) -> Result<serde_json::Value, String> {
    let view: View = serde_json::from_str(raw).map_err(|e| e.to_string())?;
    if view.blocks.len() > MAX_BLOCKS {
        return Err("A panel has at most 20 blocks".into());
    }
    for block in &view.blocks {
        match block {
            Block::Text { text } if text.chars().count() > MAX_TEXT => {
                return Err("Panel text is too long".into())
            }
            Block::Text { .. } => {}
            Block::Form { fields, submit } => {
                super::forms::check_form(fields)?;
                if fields.len() > 40
                    || submit.label.chars().count() > 60
                    || submit.label.trim().is_empty()
                {
                    return Err("Panel form is too large".into());
                }
                if !manifest
                    .contributes
                    .commands
                    .iter()
                    .any(|c| c.id == submit.command)
                {
                    return Err(format!("Panel submits unknown command {}", submit.command));
                }
            }
        }
    }
    serde_json::to_value(view).map_err(|e| e.to_string())
}
