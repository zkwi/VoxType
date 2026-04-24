use crate::config::AppConfig;
use serde_json::{json, Value};
use std::time::Duration;

pub async fn polish(config: &AppConfig, text: &str) -> String {
    let settings = &config.llm_post_edit;
    if !settings.enabled || text.trim().chars().count() < settings.min_chars {
        return text.to_string();
    }
    let api_key = settings.api_key.trim();
    let base_url = settings.base_url.trim().trim_end_matches('/');
    let model = settings.model.trim();
    if api_key.is_empty() || base_url.is_empty() || model.is_empty() {
        return text.to_string();
    }

    let mut user_prompt = settings.user_prompt_template.replace("{text}", text);
    let hotwords: Vec<String> = config
        .context
        .hotwords
        .iter()
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect();
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

    match call_openai_compatible(config, base_url, api_key, model, &user_prompt).await {
        Ok(polished) if !polished.trim().is_empty() => polished.trim().to_string(),
        _ => text.to_string(),
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
        .json(&json!({
            "model": model,
            "messages": [
                {"role": "system", "content": config.llm_post_edit.system_prompt},
                {"role": "user", "content": user_prompt}
            ],
            "enable_thinking": config.llm_post_edit.enable_thinking
        }))
        .send()
        .await
        .map_err(|err| format!("调用 LLM 失败: {}", err))?;
    if !response.status().is_success() {
        return Err(format!("LLM 返回状态码: {}", response.status()));
    }
    let value: Value = response
        .json()
        .await
        .map_err(|err| format!("解析 LLM 响应失败: {}", err))?;
    Ok(value
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"))
        .and_then(|message| message.get("content"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string())
}
