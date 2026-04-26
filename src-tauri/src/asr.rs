use crate::config::AppConfig;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct AsrRequestPreview {
    pub ws_url: String,
    pub headers: BTreeMap<String, String>,
    pub payload: Value,
    pub context: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DefiniteSegment {
    pub start_time: i64,
    pub end_time: i64,
    pub text: String,
}

pub fn build_request_preview(config: &AppConfig) -> AsrRequestPreview {
    let context = build_context_payload(config);
    AsrRequestPreview {
        ws_url: config.request.ws_url.clone(),
        headers: build_headers(config),
        payload: build_request_payload(config, context.clone()),
        context,
    }
}

pub fn build_headers(config: &AppConfig) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("X-Api-App-Key".to_string(), config.auth.app_key.clone()),
        (
            "X-Api-Access-Key".to_string(),
            config.auth.access_key.clone(),
        ),
        (
            "X-Api-Resource-Id".to_string(),
            config.auth.resource_id.clone(),
        ),
        ("X-Api-Connect-Id".to_string(), Uuid::new_v4().to_string()),
    ])
}

pub fn build_request_payload(config: &AppConfig, context_payload: Option<String>) -> Value {
    let mut request = serde_json::Map::new();
    request.insert("model_name".to_string(), json!(config.request.model_name));
    request.insert(
        "enable_nonstream".to_string(),
        json!(config.request.enable_nonstream),
    );
    request.insert("enable_itn".to_string(), json!(config.request.enable_itn));
    request.insert("enable_punc".to_string(), json!(config.request.enable_punc));
    request.insert("enable_ddc".to_string(), json!(config.request.enable_ddc));
    request.insert(
        "show_utterances".to_string(),
        json!(config.request.show_utterances),
    );
    request.insert("result_type".to_string(), json!(config.request.result_type));

    if let Some(value) = config.request.enable_accelerate_text {
        request.insert("enable_accelerate_text".to_string(), json!(value));
    }
    if let Some(value) = config.request.accelerate_score {
        request.insert("accelerate_score".to_string(), json!(value));
    }
    if let Some(value) = config.request.end_window_size {
        request.insert("end_window_size".to_string(), json!(value));
    }
    if let Some(value) = config.request.force_to_speech_time {
        request.insert("force_to_speech_time".to_string(), json!(value));
    }
    if let Some(context) = context_payload {
        request.insert("corpus".to_string(), json!({ "context": context }));
    }

    json!({
        "user": { "uid": "desktop-input" },
        "audio": {
            "format": "pcm",
            "codec": "raw",
            "rate": config.audio.sample_rate,
            "bits": 16,
            "channel": config.audio.channels,
        },
        "request": Value::Object(request),
    })
}

pub fn build_context_payload(config: &AppConfig) -> Option<String> {
    let mut payload = serde_json::Map::new();
    let hotwords: Vec<Value> = config
        .context
        .hotwords
        .iter()
        .map(|word| word.trim())
        .filter(|word| !word.is_empty())
        .map(|word| json!({ "word": word }))
        .collect();
    if !hotwords.is_empty() {
        payload.insert("hotwords".to_string(), Value::Array(hotwords));
    }

    let mut context_data = Vec::new();
    if config.context.enable_recent_context {
        for item in &config.context.recent_context {
            let text = item.text.trim();
            if !text.is_empty() {
                context_data.push(json!({ "text": text }));
            }
        }
    }
    for item in &config.context.prompt_context {
        let text = item.text.trim();
        if !text.is_empty() {
            context_data.push(json!({ "text": text }));
        }
    }
    if !context_data.is_empty() {
        payload.insert("context_type".to_string(), json!("dialog_ctx"));
        payload.insert(
            "context_data".to_string(),
            Value::Array(context_data.into_iter().take(20).collect()),
        );
    }

    if payload.is_empty() {
        return None;
    }
    serde_json::to_string(&Value::Object(payload)).ok()
}

pub fn extract_display_text(payload_msg: Option<&Value>) -> String {
    let Some(payload) = payload_msg else {
        return String::new();
    };
    let Some(result) = payload.get("result") else {
        return String::new();
    };

    let direct_text = result
        .get("text")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    if !direct_text.is_empty() {
        return direct_text.to_string();
    }

    result
        .get("utterances")
        .and_then(Value::as_array)
        .map(|utterances| {
            utterances
                .iter()
                .filter_map(|item| item.get("text").and_then(Value::as_str))
                .map(str::trim)
                .filter(|text| !text.is_empty())
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default()
}

pub fn extract_definite_segments(payload_msg: Option<&Value>) -> Vec<DefiniteSegment> {
    let Some(utterances) = payload_msg
        .and_then(|payload| payload.get("result"))
        .and_then(|result| result.get("utterances"))
        .and_then(Value::as_array)
    else {
        return Vec::new();
    };

    utterances
        .iter()
        .filter(|item| {
            item.get("definite")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .filter_map(|item| {
            let text = item.get("text").and_then(Value::as_str)?.trim().to_string();
            if text.is_empty() {
                return None;
            }
            Some(DefiniteSegment {
                start_time: item.get("start_time").and_then(Value::as_i64).unwrap_or(0),
                end_time: item.get("end_time").and_then(Value::as_i64).unwrap_or(0),
                text,
            })
        })
        .collect()
}

pub fn normalize_final_text(text: &str) -> String {
    let mut result = text.trim().to_string();
    if result.ends_with('。') || result.ends_with('.') {
        result.pop();
        result = result.trim_end().to_string();
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AppConfig, TextContext};

    #[test]
    fn builds_context_payload_in_expected_order() {
        let mut config = AppConfig::default();
        config.context.hotwords = vec!["ASR".to_string()];
        config.context.enable_recent_context = true;
        config.context.recent_context = vec![TextContext {
            text: "recent".to_string(),
        }];
        config.context.prompt_context = vec![TextContext {
            text: "prompt".to_string(),
        }];
        let context = build_context_payload(&config).unwrap();
        let value: Value = serde_json::from_str(&context).unwrap();
        assert_eq!(value["hotwords"][0]["word"], "ASR");
        assert_eq!(value["context_data"][0]["text"], "recent");
        assert_eq!(value["context_data"][1]["text"], "prompt");
    }

    #[test]
    fn extracts_definite_segments() {
        let payload = json!({
            "result": {
                "utterances": [
                    {"definite": true, "start_time": 0, "end_time": 10, "text": "你好"},
                    {"definite": false, "text": "忽略"}
                ]
            }
        });
        let segments = extract_definite_segments(Some(&payload));
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].text, "你好");
    }

    #[test]
    fn extracts_display_text_from_utterances_when_result_text_is_missing() {
        let payload = json!({
            "result": {
                "utterances": [
                    {"definite": false, "text": "实时"},
                    {"definite": false, "text": "字幕"}
                ]
            }
        });
        assert_eq!(extract_display_text(Some(&payload)), "实时字幕");
    }

    #[test]
    fn trims_final_period() {
        assert_eq!(normalize_final_text("测试。"), "测试");
        assert_eq!(normalize_final_text("test."), "test");
    }
}
