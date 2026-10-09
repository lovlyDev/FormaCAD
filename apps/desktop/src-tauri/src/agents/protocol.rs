mod command_response;
#[cfg(all(test, feature = "native-occt"))]
mod command_response_tests;
use crate::core::{AppError, Result};
use serde::Serialize;

pub fn parse_output(provider: &str, output: &str) -> Result<PlanResult> {
    parse_output_with_contract(provider, output, false, None)
}

#[cfg(feature = "native-occt")]
pub fn parse_native_output(provider: &str, output: &str) -> Result<PlanResult> {
    parse_output_with_contract(provider, output, true, None)
}

#[cfg(feature = "native-occt")]
pub(super) fn parse_native_edit_output(
    provider: &str,
    output: &str,
    current: &crate::cad_ir::Document,
) -> Result<PlanResult> {
    parse_output_with_contract(provider, output, true, Some(current))
}

fn parse_output_with_contract(
    provider: &str,
    output: &str,
    native_only: bool,
    current: Option<&crate::cad_ir::Document>,
) -> Result<PlanResult> {
    let mut final_text = (provider == "custom").then(|| output.trim().to_owned());
    for line in output.lines() {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
            if provider == "codex"
                && v["type"] == "item.completed"
                && v["item"]["type"] == "agent_message"
            {
                final_text = v["item"]["text"].as_str().map(str::to_owned);
            }
            if provider == "claude" && v["type"] == "result" {
                if v["is_error"] == true {
                    return Err(AppError::Invalid(
                        "Claude could not complete the request. Check your CLI login.".into(),
                    ));
                }
                final_text = v["result"].as_str().map(str::to_owned);
            }
        }
    }
    let raw = final_text
        .ok_or_else(|| AppError::Invalid("The agent returned no structured model result".into()))?;
    let raw = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    let value: serde_json::Value = serde_json::from_str(raw)
        .map_err(|_| AppError::Invalid("Agent response was not valid model data".into()))?;
    if value.get("commands").is_some() {
        if !native_only {
            return Err(AppError::Invalid(
                "Agent response was not valid model data".into(),
            ));
        }
        return command_response::stage(&value, current);
    }
    let message = value["message"]
        .as_str()
        .ok_or_else(|| AppError::Invalid("Agent response is missing message".into()))?
        .to_string();
    let program = if let Some(cad) = value.get("cad").filter(|v| !v.is_null()) {
        if value.get("program").is_some_and(|v| !v.is_null()) {
            return Err(AppError::Invalid(
                "Return cad or fallback program, not both".into(),
            ));
        }
        let source = serde_json::to_string_pretty(cad)?;
        if native_only {
            let document: crate::cad_ir::Document = serde_json::from_value(cad.clone())?;
            document.validate().map_err(crate::cad_ir::app_error)?;
        } else {
            crate::cad_ir::compile_json(&source)?;
        }
        if source.len() > 60000 {
            return Err(AppError::Invalid("CAD document exceeds 60000 bytes".into()));
        }
        Some(source)
    } else {
        match value.get("program") {
            Some(serde_json::Value::Null) => None,
            Some(serde_json::Value::String(source))
                if !native_only && !source.trim().is_empty() && source.len() <= 60000 =>
            {
                Some(source.trim().to_string())
            }
            _ => {
                return Err(AppError::Invalid(
                    if native_only {
                        "Native agent response must contain CAD IR v2 or program:null"
                    } else {
                        "Agent response must contain a CAD program or program:null"
                    }
                    .into(),
                ))
            }
        }
    };
    let document = program
        .as_ref()
        .and_then(|program| serde_json::from_str::<crate::cad_ir::Document>(program).ok());
    let review_plan =
        super::review_plan::parse(value.get("reviewPlan"), document.as_ref(), current)?;
    Ok(PlanResult {
        message,
        program,
        review_plan,
    })
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanResult {
    pub message: String,
    pub program: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_plan: Option<super::review_plan::ReviewPlan>,
}

pub(super) fn public_event(line: &str) -> Option<(String, String)> {
    let v: serde_json::Value = serde_json::from_str(line).ok()?;
    match v["type"].as_str()? {
        "thread.started" => Some(("connected".into(), "CLI session connected".into())),
        "turn.started" => Some(("working".into(), "Preparing a response".into())),
        "item.started" | "item.updated" | "item.completed"
            if v["item"]["type"] == "agent_message" =>
        {
            let raw = v["item"]["text"].as_str()?;
            let text = serde_json::from_str::<serde_json::Value>(raw)
                .ok()
                .and_then(|value| value["message"].as_str().map(str::to_owned));
            text.map(|text| ("message".into(), text))
        }
        "turn.completed" => Some(("completed".into(), "Response received".into())),
        "turn.failed" | "error" => Some((
            "error".into(),
            v["message"]
                .as_str()
                .or(v["error"]["message"].as_str())
                .unwrap_or("CLI request failed")
                .chars()
                .take(2000)
                .collect(),
        )),
        _ => None,
    }
}
