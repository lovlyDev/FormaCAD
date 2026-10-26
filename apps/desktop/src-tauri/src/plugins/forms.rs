//! Declarative plugin forms (`params`, `settings`): definition checks and
//! host-side validation of submitted values. The UI renders the same schema,
//! but the host never trusts UI input.
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    Length,
    Angle,
    Number,
    Integer,
    Boolean,
    Choice,
    Text,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FormField {
    pub key: String,
    #[serde(rename = "type")]
    pub kind: FieldType,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<ChoiceOption>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChoiceOption {
    pub value: String,
    pub label: String,
}

pub const MAX_TEXT: usize = 1000;
/// Same bound as CAD IR dimensions.
const MAX_LENGTH_MM: f64 = 10000.0;

/// Definition-level checks the JSON Schema cannot express.
pub fn check_form(fields: &[FormField]) -> Result<(), String> {
    let mut keys = std::collections::HashSet::new();
    for field in fields {
        if !keys.insert(field.key.as_str()) {
            return Err(format!("Duplicate form key {}", field.key));
        }
        if let (Some(min), Some(max)) = (field.min, field.max) {
            if min > max {
                return Err(format!("{}: min is greater than max", field.key));
            }
        }
        if field.kind == FieldType::Choice && field.options.is_empty() {
            return Err(format!("{}: choice needs options", field.key));
        }
        if let Some(default) = &field.default {
            check_value(field, default).map_err(|error| format!("{} default: {error}", field.key))?;
        }
    }
    Ok(())
}

/// Validate submitted values: unknown keys are rejected, missing keys take the
/// default (or the field's neutral value), every value is type and range checked.
pub fn resolve_values(fields: &[FormField], submitted: &Value) -> Result<Map<String, Value>, String> {
    let empty = Map::new();
    let submitted = match submitted {
        Value::Null => &empty,
        Value::Object(map) => map,
        _ => return Err("Form values must be an object".into()),
    };
    if let Some(unknown) = submitted.keys().find(|key| !fields.iter().any(|f| &f.key == *key)) {
        return Err(format!("Unknown form field {unknown}"));
    }
    let mut resolved = Map::new();
    for field in fields {
        let value = submitted
            .get(&field.key)
            .or(field.default.as_ref())
            .cloned()
            .unwrap_or_else(|| neutral(field));
        check_value(field, &value).map_err(|error| format!("{}: {error}", field.key))?;
        resolved.insert(field.key.clone(), value);
    }
    Ok(resolved)
}

fn neutral(field: &FormField) -> Value {
    match field.kind {
        FieldType::Boolean => Value::Bool(false),
        FieldType::Text => Value::String(String::new()),
        FieldType::Choice => Value::String(field.options[0].value.clone()),
        _ => serde_json::json!(field.min.unwrap_or(0.0).max(0.0)),
    }
}

fn check_value(field: &FormField, value: &Value) -> Result<(), String> {
    match field.kind {
        FieldType::Boolean => value.as_bool().map(|_| ()).ok_or_else(|| "expected true or false".into()),
        FieldType::Text => match value.as_str() {
            Some(text) if text.chars().count() <= MAX_TEXT => Ok(()),
            Some(_) => Err("text is too long".into()),
            None => Err("expected text".into()),
        },
        FieldType::Choice => match value.as_str() {
            Some(choice) if field.options.iter().any(|option| option.value == choice) => Ok(()),
            _ => Err("unknown choice".into()),
        },
        FieldType::Integer | FieldType::Number | FieldType::Length | FieldType::Angle => {
            let number = value.as_f64().filter(|n| n.is_finite()).ok_or("expected a number")?;
            if field.kind == FieldType::Integer && number.fract() != 0.0 {
                return Err("expected a whole number".into());
            }
            if field.kind == FieldType::Length && number.abs() > MAX_LENGTH_MM {
                return Err("length is out of range".into());
            }
            if field.min.is_some_and(|min| number < min) || field.max.is_some_and(|max| number > max) {
                return Err("value is out of range".into());
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fields() -> Vec<FormField> {
        serde_json::from_value(json!([
            {"key":"teeth","type":"integer","label":"Teeth","default":12,"min":6,"max":40},
            {"key":"width","type":"length","label":"Width","default":5,"min":0.5},
            {"key":"style","type":"choice","label":"Style","options":[{"value":"a","label":"A"},{"value":"b","label":"B"}]},
            {"key":"note","type":"text","label":"Note"}
        ]))
        .unwrap()
    }

    #[test]
    fn fills_defaults_and_checks_types_and_ranges() {
        let values = resolve_values(&fields(), &json!({"teeth":20})).unwrap();
        assert_eq!(values["teeth"], json!(20));
        assert_eq!(values["width"], json!(5));
        assert_eq!(values["style"], json!("a"));
        assert_eq!(values["note"], json!(""));
        for bad in [
            json!({"teeth":20.5}),
            json!({"teeth":3}),
            json!({"width":"5"}),
            json!({"style":"c"}),
            json!({"note":"x".repeat(MAX_TEXT + 1)}),
            json!({"extra":1}),
            json!([1]),
        ] {
            assert!(resolve_values(&fields(), &bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn rejects_inconsistent_definitions() {
        let mut broken = fields();
        broken[0].default = Some(json!(100));
        assert!(check_form(&broken).is_err());
        let mut duplicate = fields();
        duplicate[1].key = "teeth".into();
        assert!(check_form(&duplicate).is_err());
        assert!(check_form(&fields()).is_ok());
    }
}
