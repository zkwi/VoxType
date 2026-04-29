use crate::{
    app_log,
    config::{effective_hotwords, AppConfig},
    hotword_history,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotwordCandidate {
    pub word: String,
    pub category: String,
    pub reason: String,
    pub confidence: f64,
    pub source_count: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct HotwordGenerationResult {
    pub candidates: Vec<HotwordCandidate>,
    pub used_chars: usize,
    pub warning: Option<String>,
}

#[derive(Debug, Deserialize)]
struct HotwordCandidateEnvelope {
    candidates: Vec<HotwordCandidate>,
}

pub async fn generate_candidates(config: AppConfig) -> Result<HotwordGenerationResult, String> {
    ensure_llm_api_ready(&config)?;
    let history_text = hotword_history::load_recent_text(config.auto_hotwords.max_history_chars)?;
    if history_text.trim().is_empty() {
        return Err("暂无可用于生成热词的历史文本。".to_string());
    }

    let used_chars = history_text.chars().count();
    let redacted_text = redact_sensitive_text(&history_text);
    let system_prompt = hotword_system_prompt();
    let existing_hotwords = effective_hotwords(&config);
    let user_prompt = hotword_user_prompt(
        &redacted_text,
        &existing_hotwords,
        &config.auto_hotwords.ignored_hotwords,
        config.auto_hotwords.max_candidates,
    );

    app_log::info(format!(
        "自动热词候选生成开始: used_chars={}, max_candidates={}, model={}",
        used_chars,
        config.auto_hotwords.max_candidates,
        config.llm_post_edit.model.trim()
    ));
    let raw = call_openai_compatible_for_hotwords(&config, &system_prompt, &user_prompt).await?;
    let parsed = parse_candidates(&raw)?;
    let candidates = filter_candidates(parsed, &config);
    app_log::info(format!(
        "自动热词候选生成完成: candidate_count={}",
        candidates.len()
    ));

    Ok(HotwordGenerationResult {
        candidates,
        used_chars,
        warning: None,
    })
}

fn ensure_llm_api_ready(config: &AppConfig) -> Result<(), String> {
    let settings = &config.llm_post_edit;
    if settings.base_url.trim().is_empty()
        || settings.api_key.trim().is_empty()
        || settings.model.trim().is_empty()
    {
        return Err("请先在 API 配置页填写大模型 Base URL、API Key 和模型名称。".to_string());
    }
    Ok(())
}

async fn call_openai_compatible_for_hotwords(
    config: &AppConfig,
    system_prompt: &str,
    user_prompt: &str,
) -> Result<String, String> {
    let settings = &config.llm_post_edit;
    let base_url = settings.base_url.trim().trim_end_matches('/');
    let api_key = settings.api_key.trim();
    let model = settings.model.trim();
    let timeout = Duration::from_secs_f64(settings.timeout_seconds.clamp(1.0, 300.0));
    let client = reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|err| format!("创建自动热词生成客户端失败: {}", err))?;
    let response = client
        .post(format!("{}/chat/completions", base_url))
        .bearer_auth(api_key)
        .json(&chat_body(
            model,
            system_prompt,
            user_prompt,
            thinking_flag(base_url, settings.enable_thinking),
        ))
        .send()
        .await
        .map_err(|err| friendly_generation_error(&format!("调用 LLM 失败: {}", err)))?;
    handle_generation_response(response).await
}

fn chat_body(
    model: &str,
    system_prompt: &str,
    user_prompt: &str,
    enable_thinking: Option<bool>,
) -> Value {
    let mut body = json!({
            "model": model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_prompt}
            ],
            "temperature": 0.2
    });
    if let Some(enable_thinking) = enable_thinking {
        body["enable_thinking"] = json!(enable_thinking);
    }
    body
}

async fn handle_generation_response(response: reqwest::Response) -> Result<String, String> {
    if !response.status().is_success() {
        let status = response.status();
        return Err(friendly_generation_error(&format!(
            "LLM 返回状态码: {}",
            status
        )));
    }
    let value: Value = response
        .json()
        .await
        .map_err(|err| format!("解析自动热词生成响应失败: {}", err))?;
    if let Some(error) = value.get("error") {
        return Err(friendly_generation_error(&format!(
            "LLM 返回错误: {}",
            error
        )));
    }
    if response_was_truncated(&value) {
        return Err("大模型返回的热词候选被截断。请减少历史文本上限或候选数量后重试。".to_string());
    }
    let content = extract_message_content(&value);
    if content.trim().is_empty() {
        return Err("大模型没有返回热词候选内容，请稍后重试或检查模型配置。".to_string());
    }
    Ok(content)
}

fn parse_candidates(raw: &str) -> Result<Vec<HotwordCandidate>, String> {
    let cleaned = strip_code_fence(raw);
    let envelope: HotwordCandidateEnvelope = serde_json::from_str(&cleaned).map_err(|err| {
        if err.is_eof() {
            "大模型返回的热词候选不完整。请减少历史文本上限或候选数量后重试。".to_string()
        } else {
            format!("解析热词候选 JSON 失败: {}", err)
        }
    })?;
    Ok(envelope.candidates)
}

fn filter_candidates(
    candidates: Vec<HotwordCandidate>,
    config: &AppConfig,
) -> Vec<HotwordCandidate> {
    let existing_hotwords = effective_hotwords(config);
    let existing = normalized_set(&existing_hotwords);
    let ignored = normalized_set(&config.auto_hotwords.ignored_hotwords);
    let mut seen = HashSet::new();
    let mut filtered = Vec::new();

    for mut candidate in candidates {
        let word = candidate.word.trim().to_string();
        let normalized = normalize_word(&word);
        if normalized.is_empty()
            || existing.contains(&normalized)
            || ignored.contains(&normalized)
            || seen.contains(&normalized)
            || !is_valid_hotword(&word)
        {
            continue;
        }

        candidate.word = word;
        candidate.category = trim_for_ui(&candidate.category, 40);
        candidate.reason = trim_for_ui(&candidate.reason, 160);
        if !candidate.confidence.is_finite() {
            candidate.confidence = 0.0;
        }
        candidate.confidence = candidate.confidence.clamp(0.0, 1.0);
        seen.insert(normalized);
        filtered.push(candidate);

        if filtered.len() >= config.auto_hotwords.max_candidates {
            break;
        }
    }

    filtered
}

fn is_valid_hotword(word: &str) -> bool {
    let word = word.trim();
    let char_count = word.chars().count();
    if !(2..=32).contains(&char_count) {
        return false;
    }
    if word.chars().any(|ch| ch == '\n' || ch == '\r') {
        return false;
    }
    if word.chars().all(|ch| ch.is_ascii_digit()) {
        return false;
    }
    if looks_like_email(word)
        || looks_like_url(word)
        || looks_like_phone_number(word)
        || looks_like_secret(word)
    {
        return false;
    }
    let punctuation_count = word
        .chars()
        .filter(|ch| ch.is_ascii_punctuation() || "，。；：！？、，".contains(*ch))
        .count();
    punctuation_count <= 3
}

fn redact_sensitive_text(input: &str) -> String {
    input
        .split_whitespace()
        .map(|token| {
            let trimmed = token.trim_matches(|ch: char| {
                ch.is_ascii_punctuation() || "，。；：！？、（）【】《》".contains(ch)
            });
            if looks_like_email(trimmed)
                || looks_like_url(trimmed)
                || looks_like_phone_number(trimmed)
                || looks_like_secret(trimmed)
            {
                "[已脱敏]"
            } else {
                token
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn looks_like_email(word: &str) -> bool {
    let word = word.trim();
    word.contains('@') && word.contains('.') && !word.contains(' ')
}

fn looks_like_url(word: &str) -> bool {
    let lower = word.trim().to_ascii_lowercase();
    lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("www.")
        || lower.contains("://")
}

fn looks_like_phone_number(word: &str) -> bool {
    has_digit_run(word, 8)
        || (word.chars().filter(|ch| ch.is_ascii_digit()).count() >= 11
            && word
                .chars()
                .all(|ch| ch.is_ascii_digit() || matches!(ch, '+' | '-' | '(' | ')' | ' ')))
}

fn looks_like_secret(word: &str) -> bool {
    let trimmed = word.trim();
    let lower = trimmed.to_ascii_lowercase();
    if lower.contains("api_key")
        || lower.contains("apikey")
        || lower.contains("password")
        || lower.contains("passwd")
        || lower.contains("token")
        || lower.contains("secret")
        || lower.contains("验证码")
        || lower.starts_with("sk-")
    {
        return true;
    }
    let ascii_like = trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'));
    ascii_like
        && trimmed.len() >= 20
        && trimmed.chars().any(|ch| ch.is_ascii_alphabetic())
        && trimmed.chars().any(|ch| ch.is_ascii_digit())
}

fn has_digit_run(word: &str, min_run: usize) -> bool {
    let mut run = 0usize;
    for ch in word.chars() {
        if ch.is_ascii_digit() {
            run += 1;
            if run >= min_run {
                return true;
            }
        } else {
            run = 0;
        }
    }
    false
}

fn normalized_set(values: &[String]) -> HashSet<String> {
    values
        .iter()
        .map(|item| normalize_word(item))
        .filter(|item| !item.is_empty())
        .collect()
}

fn normalize_word(word: &str) -> String {
    word.trim().to_ascii_lowercase()
}

fn trim_for_ui(value: &str, max_chars: usize) -> String {
    value.trim().chars().take(max_chars).collect()
}

fn strip_code_fence(raw: &str) -> String {
    let mut text = raw.trim().to_string();
    if !text.starts_with("```") {
        return text;
    }

    text = text.trim_start_matches("```").trim_start().to_string();
    if let Some(stripped) = text.strip_prefix("json") {
        text = stripped.trim_start().to_string();
    }
    if let Some(index) = text.rfind("```") {
        text.truncate(index);
    }
    text.trim().to_string()
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

fn response_was_truncated(value: &Value) -> bool {
    value
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("finish_reason"))
        .and_then(Value::as_str)
        .map(|reason| matches!(reason, "length" | "max_tokens"))
        .unwrap_or(false)
}

fn thinking_flag(base_url: &str, enable_thinking: bool) -> Option<bool> {
    if enable_thinking || base_url.contains("dashscope.aliyuncs.com") {
        Some(enable_thinking)
    } else {
        None
    }
}

fn friendly_generation_error(error: &str) -> String {
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
        "自动热词候选生成失败，请检查 API Key、Base URL、模型名称或网络环境。".to_string()
    }
}

fn hotword_system_prompt() -> String {
    r#"你是一个语音识别热词提取器。你的任务是从用户最近的语音输入文本中提取对 ASR 识别有帮助的专有词、术语、名称和缩写。

只提取以下类型：
- 人名
- 公司名
- 产品名
- 项目名
- 地名
- 品牌名
- 技术术语
- 英文缩写
- 业务专有词
- 经常被语音识别误写的短语

不要提取：
- 普通高频词
- 完整句子
- 长段文本
- 手机号
- 邮箱
- URL
- 详细地址
- 身份证号
- 银行卡号
- 密码
- API Key
- Token
- 验证码
- 订单号
- 纯数字
- 明显敏感信息

必须只输出 JSON，不要输出 Markdown，不要解释。"#
        .to_string()
}

fn hotword_user_prompt(
    text: &str,
    existing_hotwords: &[String],
    ignored_hotwords: &[String],
    max_candidates: usize,
) -> String {
    let existing = if existing_hotwords.is_empty() {
        "无".to_string()
    } else {
        existing_hotwords.join("\n")
    };
    let ignored = if ignored_hotwords.is_empty() {
        "无".to_string()
    } else {
        ignored_hotwords.join("\n")
    };
    format!(
        r#"请从下面的最近语音输入文本中提取最多 {max_candidates} 个 ASR 热词候选。

已有热词：
{existing}

用户忽略过的候选词：
{ignored}

最近语音输入文本：
{text}

输出格式必须为：
{{
  "candidates": [
    {{
      "word": "VoxType",
      "category": "project",
      "reason": "项目名，可能被 ASR 误识别",
      "confidence": 0.86,
      "source_count": 2
    }}
  ]
}}"#
    )
}

#[cfg(test)]
mod tests {
    use super::{
        filter_candidates, friendly_generation_error, is_valid_hotword, parse_candidates,
        redact_sensitive_text, response_was_truncated, HotwordCandidate,
    };
    use crate::config::AppConfig;
    use serde_json::json;

    fn candidate(word: &str) -> HotwordCandidate {
        HotwordCandidate {
            word: word.to_string(),
            category: "term".to_string(),
            reason: "可能被识别错误".to_string(),
            confidence: 0.9,
            source_count: 1,
        }
    }

    #[test]
    fn filters_sensitive_and_low_quality_words() {
        assert!(!is_valid_hotword("name@example.com"));
        assert!(!is_valid_hotword("https://example.com"));
        assert!(!is_valid_hotword("13800138000"));
        assert!(!is_valid_hotword("123456"));
        assert!(!is_valid_hotword("api_key_example_value"));
        assert!(!is_valid_hotword("abcdef1234567890abcdef"));
        assert!(is_valid_hotword("VoxType"));
        assert!(is_valid_hotword("豆包 ASR"));
    }

    #[test]
    fn removes_existing_ignored_and_duplicate_candidates() {
        let mut config = AppConfig::default();
        config.context.hotwords = vec!["VoxType".to_string()];
        config.auto_hotwords.accepted_hotwords = vec!["自动热词".to_string()];
        config.auto_hotwords.ignored_hotwords = vec!["豆包 ASR".to_string()];
        config.auto_hotwords.max_candidates = 30;
        let result = filter_candidates(
            vec![
                candidate("VoxType"),
                candidate("自动热词"),
                candidate("豆包 ASR"),
                candidate("OpenAI Compatible"),
                candidate("OpenAI Compatible"),
            ],
            &config,
        );

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].word, "OpenAI Compatible");
    }

    #[test]
    fn parses_json_inside_markdown_fence() {
        let raw = r#"```json
{"candidates":[{"word":"VoxType","category":"project","reason":"项目名","confidence":0.8,"source_count":2}]}
```"#;
        let result = parse_candidates(raw).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].word, "VoxType");
    }

    #[test]
    fn detects_truncated_llm_response() {
        let value = json!({
            "choices": [{
                "finish_reason": "length",
                "message": {"content": "{\"candidates\":["}
            }]
        });

        assert!(response_was_truncated(&value));
    }

    #[test]
    fn truncated_json_gets_actionable_error() {
        let message = parse_candidates(r#"{"candidates":[{"word":"VoxType""#)
            .expect_err("incomplete JSON should fail");

        assert!(message.contains("不完整"));
        assert!(message.contains("减少历史文本上限或候选数量"));
    }

    #[test]
    fn redacts_sensitive_tokens_before_llm_prompt() {
        let redacted = redact_sensitive_text(
            "联系 name@example.com，网址 https://example.com，电话 13800138000，token api_key_example_value。",
        );

        assert!(!redacted.contains("name@example.com"));
        assert!(!redacted.contains("https://example.com"));
        assert!(!redacted.contains("13800138000"));
        assert!(!redacted.contains("api_key_example_value"));
    }

    #[test]
    fn friendly_error_does_not_echo_raw_service_body() {
        let message =
            friendly_generation_error("LLM 返回状态码: 400; api_key_example_value password token");

        assert!(!message.contains("api_key_example_value"));
        assert!(!message.contains("password"));
        assert!(!message.contains("token"));
    }
}
