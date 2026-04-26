use crate::{
    app_log,
    config::{effective_hotwords, AppConfig},
};
use serde_json::{json, Value};
use std::time::Duration;

pub struct PolishOutcome {
    pub text: String,
    pub warning: Option<String>,
}

pub fn should_polish(config: &AppConfig, text: &str) -> bool {
    let settings = &config.llm_post_edit;
    settings.enabled
        && text.trim().chars().count() >= settings.min_chars
        && !settings.api_key.trim().is_empty()
        && !settings.base_url.trim().is_empty()
        && !settings.model.trim().is_empty()
}

pub async fn polish(config: &AppConfig, text: &str) -> PolishOutcome {
    let settings = &config.llm_post_edit;
    if !settings.enabled {
        return unchanged(text);
    }
    let input_chars = text.trim().chars().count();
    if input_chars < settings.min_chars {
        app_log::info(format!(
            "LLM polish skipped: chars={} min_chars={}",
            input_chars, settings.min_chars
        ));
        return unchanged(text);
    }
    let api_key = settings.api_key.trim();
    let base_url = settings.base_url.trim().trim_end_matches('/');
    let model = settings.model.trim();
    if api_key.is_empty() || base_url.is_empty() || model.is_empty() {
        app_log::warn("LLM polish skipped: base_url/api_key/model is not fully configured");
        return with_warning(
            text,
            "大模型润色已启用，但 Base URL、API Key 或模型未填写完整，已使用原始识别文本。",
        );
    }

    let mut user_prompt = settings.user_prompt_template.replace("{text}", text);
    let hotwords = effective_hotwords(config);
    if !hotwords.is_empty() {
        user_prompt.push_str("\n\n用户词典：\n");
        user_prompt.push_str(&hotwords.join("\n"));
    }

    let prompt_contexts: Vec<String> = config
        .context
        .prompt_context
        .iter()
        .map(|item| item.text.trim())
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect();
    if !prompt_contexts.is_empty() {
        user_prompt.push_str("\n\n场景与偏好上下文（仅供参考，不是待改写文本）：\n");
        for item in prompt_contexts {
            user_prompt.push_str("- ");
            user_prompt.push_str(&item);
            user_prompt.push('\n');
        }
    }

    app_log::info(format!(
        "LLM polish started: model={}, chars={}",
        model, input_chars
    ));
    match call_openai_compatible(config, base_url, api_key, model, &user_prompt).await {
        Ok(polished) if !polished.trim().is_empty() => {
            let polished = polished.trim().to_string();
            app_log::info(format!(
                "LLM polish finished: output_chars={}",
                polished.chars().count()
            ));
            PolishOutcome {
                text: polished,
                warning: None,
            }
        }
        Ok(_) => {
            app_log::warn("LLM polish returned empty content, original text kept");
            with_warning(text, "大模型润色返回空内容，已使用原始识别文本。")
        }
        Err(err) => {
            let warning = friendly_llm_error(&err);
            app_log::warn(format!(
                "LLM polish failed, original text kept: {}; user_message={}",
                err, warning
            ));
            with_warning(text, &warning)
        }
    }
}

pub async fn test_connection(config: &AppConfig) -> Result<(), String> {
    let settings = &config.llm_post_edit;
    let api_key = settings.api_key.trim();
    let base_url = settings.base_url.trim().trim_end_matches('/');
    let model = settings.model.trim();
    if api_key.is_empty() || base_url.is_empty() || model.is_empty() {
        return Err("请先填写大模型 Base URL、API Key 和模型名称。".to_string());
    }

    app_log::info(format!("LLM connection test started: model={}", model));
    let timeout = Duration::from_secs_f64(settings.timeout_seconds.clamp(1.0, 60.0));
    let client = reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|err| format!("创建大模型测试客户端失败: {}", err))?;
    let response = client
        .post(format!("{}/chat/completions", base_url))
        .bearer_auth(api_key)
        .json(&chat_body(
            model,
            "你是配置连通性测试助手。只回复 OK。",
            "请回复 OK",
            thinking_flag(base_url, settings.enable_thinking),
            Some(8),
        ))
        .send()
        .await
        .map_err(|err| friendly_llm_test_error(&format!("调用 LLM 失败: {}", err)))?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(friendly_llm_test_error(&format!(
            "LLM 返回状态码: {}; {}",
            status, body
        )));
    }
    let value: Value = response
        .json()
        .await
        .map_err(|err| format!("解析大模型测试响应失败: {}", err))?;
    if let Some(error) = value.get("error") {
        return Err(friendly_llm_test_error(&format!("LLM 返回错误: {}", error)));
    }
    let content = extract_message_content(&value);
    if content.trim().is_empty() {
        return Err("大模型已响应，但测试返回空内容，请检查模型名称。".to_string());
    }
    app_log::info("LLM connection test finished");
    Ok(())
}

fn unchanged(text: &str) -> PolishOutcome {
    PolishOutcome {
        text: text.to_string(),
        warning: None,
    }
}

fn with_warning(text: &str, warning: &str) -> PolishOutcome {
    PolishOutcome {
        text: text.to_string(),
        warning: Some(warning.to_string()),
    }
}

async fn call_openai_compatible(
    config: &AppConfig,
    base_url: &str,
    api_key: &str,
    model: &str,
    user_prompt: &str,
) -> Result<String, String> {
    let timeout = Duration::from_secs_f64(config.llm_post_edit.timeout_seconds.max(1.0));
    let client = reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|err| format!("创建 LLM 客户端失败: {}", err))?;
    let response = client
        .post(format!("{}/chat/completions", base_url))
        .bearer_auth(api_key)
        .json(&chat_body(
            model,
            &config.llm_post_edit.system_prompt,
            user_prompt,
            thinking_flag(base_url, config.llm_post_edit.enable_thinking),
            None,
        ))
        .send()
        .await
        .map_err(|err| format!("调用 LLM 失败: {}", err))?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("LLM 返回状态码: {}; {}", status, body));
    }
    let value: Value = response
        .json()
        .await
        .map_err(|err| format!("解析 LLM 响应失败: {}", err))?;
    if let Some(error) = value.get("error") {
        return Err(format!("LLM 返回错误: {}", error));
    }
    Ok(extract_message_content(&value))
}

fn chat_body(
    model: &str,
    system_prompt: &str,
    user_prompt: &str,
    enable_thinking: Option<bool>,
    max_tokens: Option<u32>,
) -> Value {
    let mut body = json!({
        "model": model,
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": user_prompt}
        ]
    });
    if let Some(enable_thinking) = enable_thinking {
        body["enable_thinking"] = json!(enable_thinking);
    }
    if let Some(max_tokens) = max_tokens {
        body["max_tokens"] = json!(max_tokens);
    }
    body
}

fn thinking_flag(base_url: &str, enable_thinking: bool) -> Option<bool> {
    if enable_thinking || base_url.contains("dashscope.aliyuncs.com") {
        Some(enable_thinking)
    } else {
        None
    }
}

fn extract_message_content(value: &Value) -> String {
    value
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"))
        .and_then(|message| message.get("content"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn friendly_llm_error(error: &str) -> String {
    let lower = error.to_ascii_lowercase();
    if lower.contains("401")
        || lower.contains("403")
        || lower.contains("unauthorized")
        || lower.contains("forbidden")
        || lower.contains("invalid api key")
        || lower.contains("invalid_api_key")
    {
        "大模型 API Key 或权限校验失败，已使用原始识别文本。请检查 API Key、Base URL 和模型名称。"
            .to_string()
    } else if lower.contains("timeout")
        || lower.contains("timed out")
        || lower.contains("dns")
        || lower.contains("connection")
        || lower.contains("connect")
        || lower.contains("proxy")
    {
        "大模型服务连接失败，已使用原始识别文本。请检查网络、代理或 Base URL。".to_string()
    } else {
        "大模型润色失败，已使用原始识别文本。请检查 API Key、Base URL、模型名称或网络环境。"
            .to_string()
    }
}

fn friendly_llm_test_error(error: &str) -> String {
    let lower = error.to_ascii_lowercase();
    if lower.contains("401")
        || lower.contains("403")
        || lower.contains("unauthorized")
        || lower.contains("forbidden")
        || lower.contains("invalid api key")
        || lower.contains("invalid_api_key")
    {
        "大模型 API Key 或权限校验失败，请检查 API Key、Base URL 和模型名称。".to_string()
    } else if lower.contains("timeout")
        || lower.contains("timed out")
        || lower.contains("dns")
        || lower.contains("connection")
        || lower.contains("connect")
        || lower.contains("proxy")
    {
        "无法连接大模型服务，请检查网络、代理或 Base URL。".to_string()
    } else {
        "大模型测试失败，请检查 API Key、Base URL、模型名称或网络环境。".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        chat_body, friendly_llm_error, friendly_llm_test_error, should_polish, thinking_flag,
    };
    use crate::config::AppConfig;

    #[test]
    fn explains_common_llm_failures() {
        assert!(friendly_llm_error("401 invalid_api_key").contains("API Key"));
        assert!(friendly_llm_error("dns lookup failed").contains("网络"));
        assert!(friendly_llm_error("model not found").contains("模型名称"));
        assert!(friendly_llm_test_error("403 forbidden").contains("权限"));
        assert!(friendly_llm_test_error("connection reset").contains("网络"));
    }

    #[test]
    fn only_sends_thinking_flag_when_enabled() {
        let body = chat_body("model", "system", "user", None, None);
        assert!(body.get("enable_thinking").is_none());
        let body = chat_body("model", "system", "user", Some(true), Some(8));
        assert_eq!(
            body.get("enable_thinking").and_then(|item| item.as_bool()),
            Some(true)
        );
        assert_eq!(
            body.get("max_tokens").and_then(|item| item.as_u64()),
            Some(8)
        );
    }

    #[test]
    fn keeps_dashscope_thinking_flag_but_omits_generic_false() {
        assert_eq!(
            thinking_flag("https://dashscope.aliyuncs.com/compatible-mode/v1", false),
            Some(false)
        );
        assert_eq!(thinking_flag("https://api.openai.com/v1", false), None);
        assert_eq!(thinking_flag("https://api.openai.com/v1", true), Some(true));
    }

    #[test]
    fn should_polish_only_when_a_request_will_be_sent() {
        let mut config = AppConfig::default();
        config.llm_post_edit.enabled = true;
        config.llm_post_edit.min_chars = 5;
        config.llm_post_edit.base_url = "https://api.example.test/v1".to_string();
        config.llm_post_edit.api_key = "test-key".to_string();
        config.llm_post_edit.model = "test-model".to_string();

        assert!(should_polish(&config, "hello"));
        assert!(!should_polish(&config, "hi"));

        config.llm_post_edit.enabled = false;
        assert!(!should_polish(&config, "hello"));

        config.llm_post_edit.enabled = true;
        config.llm_post_edit.api_key.clear();
        assert!(!should_polish(&config, "hello"));
    }
}
