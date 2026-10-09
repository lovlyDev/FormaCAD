pub fn provider_diagnostic(output: &str, errors: &str) -> Option<String> {
    let mut message = None;
    for line in output.lines() {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
            if v["type"] == "error" && v["code"] == "GEOMETRY_BUILD_FAILED" {
                // Preserve the worker's feature identity through the existing string IPC error.
                return Some(serde_json::json!({
                    "code": v["code"], "featureId": v["featureId"],
                    "operation": v["operation"],
                    "message": v["message"].as_str().unwrap_or("CAD build failed").chars().take(1500).collect::<String>()
                }).to_string());
            }
            if v["type"] == "error" || v["type"] == "turn.failed" || v["is_error"] == true {
                if let Some(text) = v["message"]
                    .as_str()
                    .or(v["error"]["message"].as_str())
                    .or(v["result"].as_str())
                {
                    if text.to_lowercase().contains("usage limit")
                        || text.to_lowercase().contains("quota")
                    {
                        return Some(text.chars().take(4000).collect());
                    }
                    message = Some(text.chars().take(4000).collect::<String>());
                }
            }
        }
    }
    if message.is_some() {
        return message;
    }
    for line in errors.lines().chain(output.lines()) {
        let lower = line.to_lowercase();
        if lower.contains("usage limit") || lower.contains("rate limit") || lower.contains("quota")
        {
            return Some(line.chars().take(4000).collect());
        }
    }
    let diagnostic = safe_diagnostic(errors);
    (!diagnostic.is_empty()).then_some(diagnostic)
}
fn safe_diagnostic(s: &str) -> String {
    let lower = s.to_lowercase();
    if lower.contains("not logged")
        || lower.contains("authentication")
        || lower.contains("unauthorized")
    {
        "Authentication is required in your CLI.".into()
    } else if lower.contains("no module named") {
        "The selected Python environment is missing the CAD kernel (cadquery).".into()
    } else if lower.contains("unexpected argument") || lower.contains("unknown option") {
        "This CLI version does not support the required restricted mode. Update the CLI.".into()
    } else {
        String::new()
    }
}
