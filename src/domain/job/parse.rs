/// Unwrap a JSON string literal to get the inner content
pub(super) fn unwrap_json_string_literal(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if !(trimmed.starts_with('"') && trimmed.ends_with('"')) {
        return None;
    }

    serde_json::from_str::<String>(trimmed).ok()
}

/// Convert a YAML value to an optional String
pub(super) fn value_to_string(value: &serde_yaml::Value) -> Option<String> {
    match value {
        serde_yaml::Value::String(s) => Some(s.clone()),
        serde_yaml::Value::Number(n) => Some(n.to_string()),
        serde_yaml::Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// Convert a YAML value to a JSON value (for next_context)
pub(super) fn yaml_to_json(value: &serde_yaml::Value) -> Option<serde_json::Value> {
    match value {
        serde_yaml::Value::Null => Some(serde_json::Value::Null),
        serde_yaml::Value::Bool(b) => Some(serde_json::Value::Bool(*b)),
        serde_yaml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Some(serde_json::Value::Number(i.into()))
            } else if let Some(f) = n.as_f64() {
                serde_json::Number::from_f64(f).map(serde_json::Value::Number)
            } else {
                None
            }
        }
        serde_yaml::Value::String(s) => Some(serde_json::Value::String(s.clone())),
        serde_yaml::Value::Sequence(seq) => {
            let json_arr: Option<Vec<_>> = seq.iter().map(yaml_to_json).collect();
            json_arr.map(serde_json::Value::Array)
        }
        serde_yaml::Value::Mapping(map) => {
            let mut json_obj = serde_json::Map::new();
            for (k, v) in map {
                if let serde_yaml::Value::String(key) = k {
                    if let Some(json_v) = yaml_to_json(v) {
                        json_obj.insert(key.clone(), json_v);
                    }
                }
            }
            Some(serde_json::Value::Object(json_obj))
        }
        serde_yaml::Value::Tagged(tagged) => yaml_to_json(&tagged.value),
    }
}
