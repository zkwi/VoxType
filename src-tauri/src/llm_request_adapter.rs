use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

pub const STRATEGY_AUTO: &str = "auto";
pub const STRATEGY_DASHSCOPE_ENABLE_THINKING: &str = "dashscope_enable_thinking";
pub const STRATEGY_THINKING_DISABLED: &str = "thinking_disabled";
pub const STRATEGY_OPENROUTER_REASONING_LOW: &str = "openrouter_reasoning_low";
pub const STRATEGY_OPENROUTER_REASONING_MINIMAL: &str = "openrouter_reasoning_minimal";
pub const STRATEGY_OPENROUTER_REASONING_NONE: &str = "openrouter_reasoning_none";
pub const STRATEGY_OMIT: &str = "omit";

const ALLOWED_STRATEGIES: &[&str] = &[
    STRATEGY_AUTO,
    STRATEGY_DASHSCOPE_ENABLE_THINKING,
    STRATEGY_THINKING_DISABLED,
    STRATEGY_OPENROUTER_REASONING_LOW,
    STRATEGY_OPENROUTER_REASONING_MINIMAL,
    STRATEGY_OPENROUTER_REASONING_NONE,
    STRATEGY_OMIT,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelReasoningProfile {
    pub preferred_strategy: &'static str,
    pub uses_reasoning_tokens: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CachedModelCapability {
    Known(ModelReasoningProfile),
    Unknown { observed_reasoning_truncation: bool },
}

pub fn is_valid_thinking_strategy(strategy: &str) -> bool {
    ALLOWED_STRATEGIES.contains(&strategy)
}

pub fn is_thinking_only_model(base_url: &str, model: &str) -> bool {
    if !is_dashscope(base_url) {
        return false;
    }

    matches!(
        model.trim().to_ascii_lowercase().as_str(),
        "qwen3.7-max-preview" | "qwen3.7-max-2026-05-17"
    )
}

/// 这些固定协议官方端点已有明确关闭字段，无需额外探测 `/v1/models`。
pub fn should_probe_models_capability(base_url: &str) -> bool {
    let lower = base_url.to_ascii_lowercase();
    !(lower.contains("dashscope.aliyuncs.com")
        || lower.contains("api.deepseek.com")
        || lower.contains("api.xiaomimimo.com"))
}

pub fn effective_thinking_strategy(base_url: &str, model: &str, configured: &str) -> &'static str {
    if is_dashscope(base_url) && configured == STRATEGY_OMIT {
        return STRATEGY_DASHSCOPE_ENABLE_THINKING;
    }
    if configured != STRATEGY_AUTO {
        let normalized = normalize_thinking_strategy(configured);
        if normalized != STRATEGY_AUTO {
            return normalized;
        }
    }
    if let Some(CachedModelCapability::Known(profile)) =
        get_cached_model_capability(base_url, model)
    {
        return profile.preferred_strategy;
    }
    preferred_auto_strategy(base_url, model)
}

pub fn thinking_strategy_candidates(
    base_url: &str,
    model: &str,
    configured: &str,
) -> Vec<&'static str> {
    if configured != STRATEGY_AUTO {
        let normalized = normalize_thinking_strategy(configured);
        if normalized != STRATEGY_AUTO {
            return vec![effective_thinking_strategy(base_url, model, configured)];
        }
    }

    let base_url = base_url.to_ascii_lowercase();
    let model = model.to_ascii_lowercase();
    if base_url.contains("dashscope.aliyuncs.com") {
        return vec![STRATEGY_DASHSCOPE_ENABLE_THINKING];
    }
    if base_url.contains("api.deepseek.com") || base_url.contains("api.xiaomimimo.com") {
        return vec![
            STRATEGY_THINKING_DISABLED,
            STRATEGY_DASHSCOPE_ENABLE_THINKING,
            STRATEGY_OMIT,
        ];
    }
    if base_url.contains("openrouter.ai") {
        if model.starts_with("stepfun/") {
            return vec![
                STRATEGY_OPENROUTER_REASONING_LOW,
                STRATEGY_OPENROUTER_REASONING_MINIMAL,
                STRATEGY_OPENROUTER_REASONING_NONE,
            ];
        }
        return vec![
            STRATEGY_OPENROUTER_REASONING_NONE,
            STRATEGY_OPENROUTER_REASONING_MINIMAL,
            STRATEGY_OPENROUTER_REASONING_LOW,
        ];
    }

    vec![
        STRATEGY_DASHSCOPE_ENABLE_THINKING,
        STRATEGY_THINKING_DISABLED,
        STRATEGY_OMIT,
    ]
}

pub fn apply_thinking_strategy(
    body: &mut Value,
    base_url: &str,
    model: &str,
    enable_thinking: bool,
    configured_strategy: &str,
) -> &'static str {
    if enable_thinking {
        body["enable_thinking"] = json!(true);
        return STRATEGY_DASHSCOPE_ENABLE_THINKING;
    }

    let strategy = effective_thinking_strategy(base_url, model, configured_strategy);
    match strategy {
        STRATEGY_DASHSCOPE_ENABLE_THINKING => {
            body["enable_thinking"] = json!(false);
        }
        STRATEGY_THINKING_DISABLED => {
            body["thinking"] = json!({ "type": "disabled" });
        }
        STRATEGY_OPENROUTER_REASONING_LOW => {
            body["reasoning_effort"] = json!("low");
        }
        STRATEGY_OPENROUTER_REASONING_MINIMAL => {
            if is_openrouter(base_url) {
                body["reasoning"] = json!({ "effort": "minimal", "exclude": true });
            } else {
                body["reasoning_effort"] = json!("minimal");
            }
        }
        STRATEGY_OPENROUTER_REASONING_NONE => {
            if is_openrouter(base_url) {
                body["reasoning"] = json!({ "effort": "none", "exclude": true });
            } else {
                body["reasoning_effort"] = json!("none");
            }
        }
        STRATEGY_OMIT | STRATEGY_AUTO => {}
        _ => {}
    }
    strategy
}

/// 从 `/models` 响应解析指定模型支持的最低思考强度档位与推理开销标志。
pub fn parse_model_reasoning_profile(
    payload: &Value,
    model: &str,
) -> Option<ModelReasoningProfile> {
    let model_trimmed = model.trim();
    if model_trimmed.is_empty() {
        return None;
    }
    let entries = payload
        .get("data")
        .and_then(Value::as_array)
        .or_else(|| payload.get("models").and_then(Value::as_array))
        .or_else(|| payload.as_array())?;

    let entry = entries
        .iter()
        .find(|item| item.get("id").and_then(Value::as_str).map(str::trim) == Some(model_trimmed))
        .or_else(|| {
            entries.iter().find(|item| {
                item.get("id")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .is_some_and(|id| id.eq_ignore_ascii_case(model_trimmed))
            })
        })?;

    let (levels, has_explicit_levels_array) = extract_reasoning_levels(entry);
    if !levels.is_empty() {
        if levels
            .iter()
            .any(|level| matches!(level.as_str(), "none" | "off" | "disabled"))
        {
            return Some(ModelReasoningProfile {
                preferred_strategy: STRATEGY_OPENROUTER_REASONING_NONE,
                uses_reasoning_tokens: false,
            });
        }
        if levels.iter().any(|level| level == "minimal") {
            return Some(ModelReasoningProfile {
                preferred_strategy: STRATEGY_OPENROUTER_REASONING_MINIMAL,
                uses_reasoning_tokens: true,
            });
        }
        if levels.iter().any(|level| level == "low") {
            return Some(ModelReasoningProfile {
                preferred_strategy: STRATEGY_OPENROUTER_REASONING_LOW,
                uses_reasoning_tokens: true,
            });
        }
        return Some(ModelReasoningProfile {
            preferred_strategy: STRATEGY_OMIT,
            uses_reasoning_tokens: true,
        });
    }

    let reasoning_flag = entry.get("reasoning").and_then(Value::as_bool).or_else(|| {
        entry
            .get("capabilities")
            .and_then(|caps| caps.get("reasoning"))
            .and_then(Value::as_bool)
    });

    match reasoning_flag {
        Some(false) => Some(ModelReasoningProfile {
            preferred_strategy: STRATEGY_OPENROUTER_REASONING_NONE,
            uses_reasoning_tokens: false,
        }),
        Some(true) if has_explicit_levels_array => Some(ModelReasoningProfile {
            preferred_strategy: STRATEGY_OMIT,
            uses_reasoning_tokens: true,
        }),
        _ => None,
    }
}

pub fn get_cached_model_capability(base_url: &str, model: &str) -> Option<CachedModelCapability> {
    let key = cache_key(base_url, model);
    capability_cache().lock().ok()?.get(&key).copied()
}

pub fn store_cached_model_capability(
    base_url: &str,
    model: &str,
    capability: CachedModelCapability,
) {
    let key = cache_key(base_url, model);
    if let Ok(mut guard) = capability_cache().lock() {
        let merged = match (guard.get(&key).copied(), capability) {
            (
                Some(CachedModelCapability::Unknown {
                    observed_reasoning_truncation: true,
                }),
                CachedModelCapability::Unknown {
                    observed_reasoning_truncation: false,
                },
            ) => CachedModelCapability::Unknown {
                observed_reasoning_truncation: true,
            },
            (_, next) => next,
        };
        guard.insert(key, merged);
    }
}

pub fn mark_model_reasoning_truncation(base_url: &str, model: &str) {
    let key = cache_key(base_url, model);
    if let Ok(mut guard) = capability_cache().lock() {
        match guard.get(&key).copied() {
            Some(CachedModelCapability::Known(_)) => {}
            _ => {
                guard.insert(
                    key,
                    CachedModelCapability::Unknown {
                        observed_reasoning_truncation: true,
                    },
                );
            }
        }
    }
}

/// 判断当前润色配置是否仍会产生内部推理 token，从而需要增加输出预算余量。
pub fn strategy_uses_reasoning_tokens(
    enable_thinking: bool,
    effective_strategy: &str,
    cached: Option<CachedModelCapability>,
) -> bool {
    if enable_thinking {
        return true;
    }
    if matches!(
        cached,
        Some(CachedModelCapability::Unknown {
            observed_reasoning_truncation: true
        })
    ) {
        return true;
    }
    match effective_strategy {
        STRATEGY_OPENROUTER_REASONING_LOW | STRATEGY_OPENROUTER_REASONING_MINIMAL => true,
        STRATEGY_DASHSCOPE_ENABLE_THINKING
        | STRATEGY_THINKING_DISABLED
        | STRATEGY_OPENROUTER_REASONING_NONE => false,
        STRATEGY_OMIT | STRATEGY_AUTO => match cached {
            Some(CachedModelCapability::Known(profile)) => profile.uses_reasoning_tokens,
            Some(CachedModelCapability::Unknown {
                observed_reasoning_truncation,
            }) => observed_reasoning_truncation,
            None => false,
        },
        _ => false,
    }
}

pub fn remove_thinking_controls(body: &mut Value) {
    if let Some(object) = body.as_object_mut() {
        object.remove("enable_thinking");
        object.remove("thinking");
        object.remove("reasoning_effort");
        object.remove("reasoning");
    }
}

pub fn disabled_thinking_controls(body: &Value) -> Vec<&'static str> {
    let mut fields = Vec::new();
    if body.get("enable_thinking").and_then(Value::as_bool) == Some(false) {
        fields.push("enable_thinking");
    }
    if body
        .get("thinking")
        .and_then(|item| item.get("type"))
        .and_then(Value::as_str)
        == Some("disabled")
    {
        fields.push("thinking");
    }
    if matches!(
        body.get("reasoning_effort").and_then(Value::as_str),
        Some("none" | "minimal" | "low")
    ) {
        fields.push("reasoning_effort");
    }
    if body.get("reasoning").is_some() {
        fields.push("reasoning");
    }
    fields
}

pub fn should_retry_without_unsupported_thinking(
    base_url: &str,
    body: &Value,
    error: &str,
) -> bool {
    let controls = disabled_thinking_controls(body);
    if controls.is_empty() {
        return false;
    }
    if base_url.contains("dashscope.aliyuncs.com")
        && controls.len() == 1
        && controls[0] == "enable_thinking"
    {
        return false;
    }
    let lower = error.to_ascii_lowercase();
    if lower.contains("restricted to true")
        || lower.contains("mandatory")
        || lower.contains("cannot be disabled")
    {
        return false;
    }
    controls.iter().any(|field| lower.contains(field))
        && (lower.contains("unknown")
            || lower.contains("unrecognized")
            || lower.contains("unsupported")
            || lower.contains("not support")
            || lower.contains("unexpected")
            || lower.contains("extra")
            || lower.contains("additional")
            || lower.contains("invalid_request_error"))
}

fn extract_reasoning_levels(entry: &Value) -> (Vec<String>, bool) {
    let mut levels = Vec::new();
    let mut has_explicit_array = false;
    let sources = [
        entry.get("reasoning_levels"),
        entry.get("supported_reasoning_levels"),
        entry
            .get("capabilities")
            .and_then(|caps| caps.get("reasoning_levels")),
        entry
            .get("capabilities")
            .and_then(|caps| caps.get("supported_reasoning_levels")),
    ];

    for source in sources.into_iter().flatten() {
        let Some(items) = source.as_array() else {
            continue;
        };
        has_explicit_array = true;
        for item in items {
            let level = match item {
                Value::String(text) => Some(text.as_str()),
                Value::Object(_) => item
                    .get("effort")
                    .or_else(|| item.get("level"))
                    .or_else(|| item.get("id"))
                    .and_then(Value::as_str),
                _ => None,
            };
            if let Some(level) = level {
                let normalized = level.trim().to_ascii_lowercase();
                if !normalized.is_empty() {
                    levels.push(normalized);
                }
            }
        }
    }

    (levels, has_explicit_array)
}

fn capability_cache() -> &'static Mutex<HashMap<(String, String), CachedModelCapability>> {
    static CACHE: OnceLock<Mutex<HashMap<(String, String), CachedModelCapability>>> =
        OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn cache_key(base_url: &str, model: &str) -> (String, String) {
    (
        base_url.trim().trim_end_matches('/').to_ascii_lowercase(),
        model.trim().to_string(),
    )
}

fn preferred_auto_strategy(base_url: &str, model: &str) -> &'static str {
    let base_url = base_url.to_ascii_lowercase();
    let model = model.to_ascii_lowercase();
    if base_url.contains("dashscope.aliyuncs.com") {
        return STRATEGY_DASHSCOPE_ENABLE_THINKING;
    }
    if base_url.contains("api.deepseek.com") || base_url.contains("api.xiaomimimo.com") {
        return STRATEGY_THINKING_DISABLED;
    }
    if base_url.contains("openrouter.ai") && model.starts_with("stepfun/") {
        return STRATEGY_OPENROUTER_REASONING_LOW;
    }
    if base_url.contains("openrouter.ai") {
        return STRATEGY_OPENROUTER_REASONING_NONE;
    }
    STRATEGY_DASHSCOPE_ENABLE_THINKING
}

pub fn is_dashscope(base_url: &str) -> bool {
    base_url
        .to_ascii_lowercase()
        .contains("dashscope.aliyuncs.com")
}

fn is_openrouter(base_url: &str) -> bool {
    base_url.to_ascii_lowercase().contains("openrouter.ai")
}

fn normalize_thinking_strategy(strategy: &str) -> &'static str {
    match strategy {
        STRATEGY_DASHSCOPE_ENABLE_THINKING => STRATEGY_DASHSCOPE_ENABLE_THINKING,
        STRATEGY_THINKING_DISABLED => STRATEGY_THINKING_DISABLED,
        STRATEGY_OPENROUTER_REASONING_LOW => STRATEGY_OPENROUTER_REASONING_LOW,
        STRATEGY_OPENROUTER_REASONING_MINIMAL => STRATEGY_OPENROUTER_REASONING_MINIMAL,
        STRATEGY_OPENROUTER_REASONING_NONE => STRATEGY_OPENROUTER_REASONING_NONE,
        STRATEGY_OMIT => STRATEGY_OMIT,
        _ => STRATEGY_AUTO,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        apply_thinking_strategy, disabled_thinking_controls, effective_thinking_strategy,
        get_cached_model_capability, is_thinking_only_model, is_valid_thinking_strategy,
        mark_model_reasoning_truncation, parse_model_reasoning_profile,
        should_probe_models_capability, should_retry_without_unsupported_thinking,
        store_cached_model_capability, strategy_uses_reasoning_tokens,
        thinking_strategy_candidates, CachedModelCapability, ModelReasoningProfile, STRATEGY_AUTO,
        STRATEGY_DASHSCOPE_ENABLE_THINKING, STRATEGY_OMIT, STRATEGY_OPENROUTER_REASONING_LOW,
        STRATEGY_OPENROUTER_REASONING_MINIMAL, STRATEGY_OPENROUTER_REASONING_NONE,
        STRATEGY_THINKING_DISABLED,
    };
    use serde_json::json;

    #[test]
    fn validates_known_strategy_names() {
        assert!(is_valid_thinking_strategy(STRATEGY_AUTO));
        assert!(is_valid_thinking_strategy(STRATEGY_THINKING_DISABLED));
        assert!(!is_valid_thinking_strategy("reasoning_none"));
    }

    #[test]
    fn recognizes_dashscope_thinking_only_models() {
        let base_url = "https://dashscope.aliyuncs.com/compatible-mode/v1";
        assert!(is_thinking_only_model(base_url, "qwen3.7-max-2026-05-17"));
        assert!(is_thinking_only_model(base_url, "qwen3.7-max-preview"));
        assert!(!is_thinking_only_model(base_url, "qwen3.7-max"));
        assert!(!is_thinking_only_model(
            "https://api.example.com/v1",
            "qwen3.7-max-2026-05-17"
        ));
    }

    #[test]
    fn chooses_provider_specific_auto_strategy() {
        assert_eq!(
            effective_thinking_strategy(
                "https://dashscope.aliyuncs.com/compatible-mode/v1",
                "qwen3.7-max",
                STRATEGY_AUTO,
            ),
            STRATEGY_DASHSCOPE_ENABLE_THINKING
        );
        assert_eq!(
            effective_thinking_strategy(
                "https://api.deepseek.com",
                "deepseek-v4-pro",
                STRATEGY_AUTO,
            ),
            STRATEGY_THINKING_DISABLED
        );
        assert_eq!(
            effective_thinking_strategy(
                "https://api.xiaomimimo.com/v1",
                "mimo-v2.5-pro",
                STRATEGY_AUTO,
            ),
            STRATEGY_THINKING_DISABLED
        );
        assert_eq!(
            effective_thinking_strategy(
                "https://openrouter.ai/api/v1",
                "stepfun/step-3.7-flash",
                STRATEGY_AUTO,
            ),
            STRATEGY_OPENROUTER_REASONING_LOW
        );
        assert_eq!(
            effective_thinking_strategy(
                "https://openrouter.ai/api/v1",
                "x-ai/grok-build-0.1",
                STRATEGY_AUTO,
            ),
            STRATEGY_OPENROUTER_REASONING_NONE
        );
    }

    #[test]
    fn disables_reasoning_for_openrouter_models_that_allow_it() {
        let base_url = "https://openrouter.ai/api/v1";
        let model = "tencent/hy3";

        assert_eq!(
            effective_thinking_strategy(base_url, model, STRATEGY_AUTO),
            "openrouter_reasoning_none"
        );
        assert_eq!(
            thinking_strategy_candidates(base_url, model, STRATEGY_AUTO)[0],
            "openrouter_reasoning_none"
        );

        let mut body = json!({});
        let used = apply_thinking_strategy(&mut body, base_url, model, false, STRATEGY_AUTO);

        assert_eq!(used, "openrouter_reasoning_none");
        assert_eq!(
            body["reasoning"],
            json!({ "effort": "none", "exclude": true })
        );
    }

    #[test]
    fn uses_standard_reasoning_effort_for_non_openrouter_gateways() {
        let gateway = "http://127.0.0.1:3425/v1";
        for (strategy, expected) in [
            (STRATEGY_OPENROUTER_REASONING_NONE, "none"),
            (STRATEGY_OPENROUTER_REASONING_MINIMAL, "minimal"),
            (STRATEGY_OPENROUTER_REASONING_LOW, "low"),
        ] {
            let mut body = json!({});
            let used = apply_thinking_strategy(&mut body, gateway, "test-model", false, strategy);
            assert_eq!(used, strategy);
            assert_eq!(body["reasoning_effort"], json!(expected));
            assert!(body.get("reasoning").is_none());
        }

        let mut openrouter_minimal = json!({});
        apply_thinking_strategy(
            &mut openrouter_minimal,
            "https://openrouter.ai/api/v1",
            "test-model",
            false,
            STRATEGY_OPENROUTER_REASONING_MINIMAL,
        );
        assert_eq!(
            openrouter_minimal["reasoning"],
            json!({ "effort": "minimal", "exclude": true })
        );
    }

    #[test]
    fn keeps_explicit_manual_strategy_as_single_candidate() {
        assert_eq!(
            thinking_strategy_candidates(
                "http://127.0.0.1:3425/v1",
                "google-vertex/gemini-4-hydrogen",
                STRATEGY_OPENROUTER_REASONING_LOW
            ),
            vec![STRATEGY_OPENROUTER_REASONING_LOW]
        );
        assert_eq!(
            thinking_strategy_candidates(
                "https://api.example.com/v1",
                "custom-model",
                STRATEGY_OPENROUTER_REASONING_NONE
            ),
            vec![STRATEGY_OPENROUTER_REASONING_NONE]
        );
        assert_eq!(
            thinking_strategy_candidates(
                "https://dashscope.aliyuncs.com/compatible-mode/v1",
                "qwen3.7-max",
                STRATEGY_OMIT
            ),
            vec![STRATEGY_DASHSCOPE_ENABLE_THINKING]
        );
    }

    #[test]
    fn parses_model_reasoning_profiles_from_models_payload() {
        let payload = json!({
            "data": [
                {
                    "id": "deepseek-flash",
                    "reasoning": false,
                    "reasoning_levels": []
                },
                {
                    "id": "model-with-none",
                    "reasoning": true,
                    "reasoning_levels": ["none", "low", "high"]
                },
                {
                    "id": "kimi-k3.1-pro",
                    "reasoning": true,
                    "supported_reasoning_levels": [
                        {"effort": "minimal"},
                        {"effort": "low"}
                    ]
                },
                {
                    "id": "google-vertex/gemini-4-hydrogen",
                    "reasoning": true,
                    "reasoning_levels": ["low", "medium", "high"]
                },
                {
                    "id": "high-only-model",
                    "reasoning": true,
                    "supported_reasoning_levels": [
                        {"effort": "medium"},
                        {"effort": "high"}
                    ]
                },
                {
                    "id": "opaque-thinking-model",
                    "reasoning": true,
                    "reasoning_levels": []
                },
                {
                    "id": "plain-openai-model",
                    "object": "model"
                }
            ]
        });

        assert_eq!(
            parse_model_reasoning_profile(&payload, "deepseek-flash"),
            Some(ModelReasoningProfile {
                preferred_strategy: STRATEGY_OPENROUTER_REASONING_NONE,
                uses_reasoning_tokens: false,
            })
        );
        assert_eq!(
            parse_model_reasoning_profile(&payload, "model-with-none"),
            Some(ModelReasoningProfile {
                preferred_strategy: STRATEGY_OPENROUTER_REASONING_NONE,
                uses_reasoning_tokens: false,
            })
        );
        assert_eq!(
            parse_model_reasoning_profile(&payload, "kimi-k3.1-pro"),
            Some(ModelReasoningProfile {
                preferred_strategy: STRATEGY_OPENROUTER_REASONING_MINIMAL,
                uses_reasoning_tokens: true,
            })
        );
        assert_eq!(
            parse_model_reasoning_profile(&payload, "google-vertex/gemini-4-hydrogen"),
            Some(ModelReasoningProfile {
                preferred_strategy: STRATEGY_OPENROUTER_REASONING_LOW,
                uses_reasoning_tokens: true,
            })
        );
        assert_eq!(
            parse_model_reasoning_profile(&payload, "high-only-model"),
            Some(ModelReasoningProfile {
                preferred_strategy: STRATEGY_OMIT,
                uses_reasoning_tokens: true,
            })
        );
        assert_eq!(
            parse_model_reasoning_profile(&payload, "opaque-thinking-model"),
            Some(ModelReasoningProfile {
                preferred_strategy: STRATEGY_OMIT,
                uses_reasoning_tokens: true,
            })
        );
        assert_eq!(
            parse_model_reasoning_profile(&payload, "plain-openai-model"),
            None
        );
        assert_eq!(parse_model_reasoning_profile(&payload, "missing"), None);
    }

    #[test]
    fn probes_models_capability_only_on_non_fixed_endpoints() {
        assert!(!should_probe_models_capability(
            "https://dashscope.aliyuncs.com/compatible-mode/v1"
        ));
        assert!(!should_probe_models_capability("https://api.deepseek.com"));
        assert!(!should_probe_models_capability(
            "https://api.xiaomimimo.com/v1"
        ));
        assert!(should_probe_models_capability("http://127.0.0.1:3425/v1"));
        assert!(should_probe_models_capability(
            "https://openrouter.ai/api/v1"
        ));
    }

    #[test]
    fn caches_model_capabilities_and_detects_reasoning_budget_needs() {
        let gateway = "https://cache-unit-test.invalid/v1";
        let model = "custom-reasoning-model";
        store_cached_model_capability(
            gateway,
            model,
            CachedModelCapability::Known(ModelReasoningProfile {
                preferred_strategy: STRATEGY_OPENROUTER_REASONING_LOW,
                uses_reasoning_tokens: true,
            }),
        );
        assert_eq!(
            effective_thinking_strategy(gateway, model, STRATEGY_AUTO),
            STRATEGY_OPENROUTER_REASONING_LOW
        );
        assert!(strategy_uses_reasoning_tokens(
            false,
            STRATEGY_OPENROUTER_REASONING_LOW,
            get_cached_model_capability(gateway, model),
        ));

        let unknown_model = "unknown-truncating-model";
        store_cached_model_capability(
            gateway,
            unknown_model,
            CachedModelCapability::Unknown {
                observed_reasoning_truncation: false,
            },
        );
        assert!(!strategy_uses_reasoning_tokens(
            false,
            STRATEGY_OMIT,
            get_cached_model_capability(gateway, unknown_model),
        ));
        mark_model_reasoning_truncation(gateway, unknown_model);
        assert!(strategy_uses_reasoning_tokens(
            false,
            STRATEGY_OMIT,
            get_cached_model_capability(gateway, unknown_model),
        ));
        assert!(strategy_uses_reasoning_tokens(
            false,
            STRATEGY_THINKING_DISABLED,
            get_cached_model_capability(gateway, unknown_model),
        ));
        assert!(strategy_uses_reasoning_tokens(
            true,
            STRATEGY_OPENROUTER_REASONING_NONE,
            None,
        ));
        assert!(!strategy_uses_reasoning_tokens(
            false,
            STRATEGY_OPENROUTER_REASONING_NONE,
            None,
        ));
    }

    #[test]
    fn builds_candidate_sets_for_connection_test() {
        assert_eq!(
            thinking_strategy_candidates(
                "https://dashscope.aliyuncs.com/compatible-mode/v1",
                "qwen3.7-max",
                STRATEGY_AUTO
            ),
            vec![STRATEGY_DASHSCOPE_ENABLE_THINKING]
        );
        assert_eq!(
            thinking_strategy_candidates(
                "https://api.deepseek.com",
                "deepseek-v4-pro",
                STRATEGY_AUTO
            ),
            vec![
                STRATEGY_THINKING_DISABLED,
                STRATEGY_DASHSCOPE_ENABLE_THINKING,
                STRATEGY_OMIT
            ]
        );
        assert_eq!(
            thinking_strategy_candidates(
                "https://openrouter.ai/api/v1",
                "stepfun/step-3.7-flash",
                STRATEGY_AUTO
            )[0],
            STRATEGY_OPENROUTER_REASONING_LOW
        );
    }

    #[test]
    fn applies_disabled_strategy_to_request_body() {
        let mut body = json!({});
        let used = apply_thinking_strategy(
            &mut body,
            "https://api.deepseek.com",
            "deepseek-v4-pro",
            false,
            STRATEGY_AUTO,
        );
        assert_eq!(used, STRATEGY_THINKING_DISABLED);
        assert_eq!(body["thinking"], json!({ "type": "disabled" }));

        let mut body = json!({});
        let used = apply_thinking_strategy(
            &mut body,
            "https://dashscope.aliyuncs.com/compatible-mode/v1",
            "qwen3.7-max",
            false,
            STRATEGY_AUTO,
        );
        assert_eq!(used, STRATEGY_DASHSCOPE_ENABLE_THINKING);
        assert_eq!(body["enable_thinking"], json!(false));

        let mut body = json!({});
        let used = apply_thinking_strategy(
            &mut body,
            "https://dashscope.aliyuncs.com/compatible-mode/v1",
            "qwen3.7-max",
            false,
            STRATEGY_OMIT,
        );
        assert_eq!(used, STRATEGY_DASHSCOPE_ENABLE_THINKING);
        assert_eq!(body["enable_thinking"], json!(false));
    }

    #[test]
    fn retries_only_for_unsupported_disabled_controls() {
        let body = json!({ "thinking": { "type": "disabled" } });
        assert!(disabled_thinking_controls(&body).contains(&"thinking"));
        assert!(should_retry_without_unsupported_thinking(
            "https://api.example.com/v1",
            &body,
            "400 unknown field: thinking"
        ));
        assert!(!should_retry_without_unsupported_thinking(
            "https://openrouter.ai/api/v1",
            &json!({ "reasoning_effort": "none" }),
            "Reasoning is mandatory for this endpoint and cannot be disabled."
        ));
    }
}
