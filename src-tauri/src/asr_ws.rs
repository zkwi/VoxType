use crate::session::{SessionController, SessionPhase};
use crate::{
    app_log, asr, config, config::AppConfig, llm_post_edit, overlay, protocol, stats, text_output,
};
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::{HeaderName, HeaderValue};
use tokio_tungstenite::tungstenite::Message;

#[derive(Debug, Clone, Serialize)]
pub struct AsrFinalText {
    pub text: String,
    pub error: Option<String>,
    pub warning: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AsrPartialText {
    pub text: String,
}

pub fn spawn_asr_worker(
    config: AppConfig,
    audio_rx: Receiver<Vec<u8>>,
    started_at: Instant,
    app: AppHandle,
    session: SessionController,
) {
    thread::spawn(move || {
        app_log::info("ASR worker 已启动");
        if config.auth.app_key.trim().is_empty() || config.auth.access_key.trim().is_empty() {
            let error = "ASR skipped: app_key/access_key is not configured.".to_string();
            session.abort_from_worker(&app, &error);
            emit_error(&app, error);
            return;
        }

        let runtime = match tokio::runtime::Runtime::new() {
            Ok(runtime) => runtime,
            Err(err) => {
                let error = format!("启动 ASR 运行时失败: {}", err);
                session.set_phase(
                    Some(&app),
                    SessionPhase::Failed,
                    &error,
                    Some("ASR_RUNTIME_FAILED"),
                );
                emit_error(&app, error);
                return;
            }
        };
        let typing = config.typing.clone();
        let runtime_result = runtime.block_on(async {
            let text =
                run_websocket_session(config.clone(), audio_rx, app.clone(), session.clone())
                    .await?;
            session.set_phase(
                Some(&app),
                SessionPhase::PostEditing,
                "Post-editing transcript.",
                None,
            );
            Ok::<llm_post_edit::PolishOutcome, String>(llm_post_edit::polish(&config, &text).await)
        });
        match runtime_result {
            Ok(outcome) => {
                let text = outcome.text;
                let mut output_warning = None;
                app_log::info(format!("ASR worker 返回文本长度: {}", text.chars().count()));
                if !text.trim().is_empty() {
                    overlay::update_text(&app, &text);
                    let duration = started_at.elapsed().as_secs_f64();
                    if let Err(err) = config::remember_recent_context(&text) {
                        app_log::warn(format!("写入 recent context 失败: {}", err));
                    }
                    if let Err(err) = stats::append_event(&text, duration) {
                        app_log::warn(err);
                    } else if let Err(err) =
                        app.emit("usage-stats-updated", stats::load_stats_snapshot())
                    {
                        app_log::warn(format!("刷新统计事件发送失败: {}", err));
                    }
                    session.set_phase(
                        Some(&app),
                        SessionPhase::Pasting,
                        "Pasting transcript.",
                        None,
                    );
                    output_warning = match text_output::output_text(&text, &typing) {
                        Ok(result) => result.warning,
                        Err(err) => {
                            session.set_phase(
                                Some(&app),
                                SessionPhase::Failed,
                                &err,
                                Some("PASTE_FAILED"),
                            );
                            emit_error(&app, err);
                            return;
                        }
                    };
                    if output_warning.is_some() {
                        app_log::warn(format!(
                            "输出文本完成但存在提示: {}",
                            output_warning.as_deref().unwrap_or_default()
                        ));
                    }
                    app_log::info(format!(
                        "ASR session finished: chars={}",
                        text.chars().count()
                    ));
                }
                if text.trim().is_empty() {
                    app_log::info("ASR session finished: empty transcript");
                }
                overlay::hide(&app);
                session.set_phase(
                    Some(&app),
                    SessionPhase::Succeeded,
                    "Transcript pasted.",
                    None,
                );
                let _ = app.emit(
                    "asr-final-text",
                    AsrFinalText {
                        text,
                        error: None,
                        warning: outcome.warning.or(output_warning),
                    },
                );
            }
            Err(err) => {
                session.abort_from_worker(&app, &err);
                emit_error(&app, err);
            }
        }
    });
}

pub async fn test_connection(config: &AppConfig) -> Result<(), String> {
    if config.auth.app_key.trim().is_empty() || config.auth.access_key.trim().is_empty() {
        return Err("请先填写豆包 App Key 和 Access Key。".to_string());
    }
    if config.auth.resource_id.trim().is_empty() {
        return Err("请先填写豆包 Resource ID。".to_string());
    }

    let mut test_config = config.clone();
    test_config.context.hotwords.clear();
    test_config.context.prompt_context.clear();
    test_config.context.recent_context.clear();
    test_config.context.image_url = None;
    let preview = asr::build_request_preview(&test_config);
    let mut request = preview
        .ws_url
        .as_str()
        .into_client_request()
        .map_err(|err| format!("创建豆包 ASR 测试请求失败: {}", err))?;
    for (name, value) in preview.headers {
        let name = HeaderName::from_bytes(name.as_bytes())
            .map_err(|err| format!("豆包 ASR header 名称无效: {}", err))?;
        let value = HeaderValue::from_str(&value)
            .map_err(|err| format!("豆包 ASR header 值无效: {}", err))?;
        request.headers_mut().insert(name, value);
    }

    let (mut websocket, _) = tokio::time::timeout(Duration::from_secs(20), connect_async(request))
        .await
        .map_err(|_| "连接豆包 ASR 测试超时，请检查网络或代理设置。".to_string())?
        .map_err(|err| friendly_asr_connection_error(&err.to_string()))?;
    websocket
        .send(Message::Binary(protocol::build_full_request(
            &preview.payload,
            1,
        )?))
        .await
        .map_err(|err| format!("发送豆包 ASR 测试首包失败: {}", err))?;
    let test_audio = silent_test_audio(&test_config);
    websocket
        .send(Message::Binary(protocol::build_audio_request(
            2,
            &test_audio,
            false,
        )?))
        .await
        .map_err(|err| format!("发送豆包 ASR 测试音频包失败: {}", err))?;
    websocket
        .send(Message::Binary(protocol::build_audio_request(
            3,
            &[],
            true,
        )?))
        .await
        .map_err(|err| format!("发送豆包 ASR 测试结束包失败: {}", err))?;

    let response = tokio::time::timeout(Duration::from_secs(8), websocket.next())
        .await
        .map_err(|_| "豆包 ASR 已连接，但未收到测试响应，请稍后重试。".to_string())?;
    let Some(response) = response else {
        return Err("豆包 ASR 连接已关闭，未收到测试响应。".to_string());
    };
    match response {
        Ok(Message::Binary(data)) => {
            let parsed = protocol::parse_response(&data)?;
            if is_success_code(parsed.code) {
                let _ = websocket.close(None).await;
                Ok(())
            } else {
                Err(friendly_asr_service_error(parsed.code))
            }
        }
        Ok(Message::Close(_)) => Err("豆包 ASR 连接已关闭，未收到有效测试响应。".to_string()),
        Ok(_) => Err("豆包 ASR 返回了非预期测试响应。".to_string()),
        Err(err) => Err(format!("接收豆包 ASR 测试响应失败: {}", err)),
    }
}

fn silent_test_audio(config: &AppConfig) -> Vec<u8> {
    let bytes_per_second = config.audio.sample_rate as usize * config.audio.channels as usize * 2;
    let requested = bytes_per_second.saturating_mul(config.audio.segment_ms as usize) / 1000;
    let byte_len = requested.clamp(3_200, 32_000);
    vec![0; byte_len]
}

async fn run_websocket_session(
    config: AppConfig,
    audio_rx: Receiver<Vec<u8>>,
    app: AppHandle,
    session: SessionController,
) -> Result<String, String> {
    let preview = asr::build_request_preview(&config);
    let mut request = preview
        .ws_url
        .as_str()
        .into_client_request()
        .map_err(|err| format!("创建 ASR WebSocket 请求失败: {}", err))?;
    for (name, value) in preview.headers {
        let name = HeaderName::from_bytes(name.as_bytes())
            .map_err(|err| format!("ASR header 名称无效: {}", err))?;
        let value =
            HeaderValue::from_str(&value).map_err(|err| format!("ASR header 值无效: {}", err))?;
        request.headers_mut().insert(name, value);
    }

    let (mut websocket, _) = connect_async(request).await.map_err(|err| {
        let detail = err.to_string();
        let message = friendly_asr_connection_error(&detail);
        app_log::warn(format!(
            "连接 ASR WebSocket 失败: {}; user_message={}",
            detail, message
        ));
        message
    })?;
    app_log::info("ASR WebSocket 已连接");
    websocket
        .send(Message::Binary(protocol::build_full_request(
            &preview.payload,
            1,
        )?))
        .await
        .map_err(|err| format!("发送 ASR 首包失败: {}", err))?;
    app_log::info("ASR 首包已发送");

    let mut seq = 2;
    let mut audio_finished = false;
    let mut final_wait_started: Option<Instant> = None;
    let final_timeout =
        Duration::from_secs_f64(config.request.final_result_timeout_seconds.max(0.5));
    let mut display_text = String::new();
    let mut definitive_segments = Vec::new();

    loop {
        if !audio_finished {
            match audio_rx.try_recv() {
                Ok(chunk) => {
                    websocket
                        .send(Message::Binary(protocol::build_audio_request(
                            seq, &chunk, false,
                        )?))
                        .await
                        .map_err(|err| format!("发送 ASR 音频包失败: {}", err))?;
                    seq += 1;
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    websocket
                        .send(Message::Binary(protocol::build_audio_request(
                            seq,
                            &[],
                            true,
                        )?))
                        .await
                        .map_err(|err| format!("发送 ASR 结束包失败: {}", err))?;
                    audio_finished = true;
                    final_wait_started = Some(Instant::now());
                    session.set_phase(
                        Some(&app),
                        SessionPhase::WaitingFinalResult,
                        "Waiting for final ASR result.",
                        None,
                    );
                    app_log::info("ASR 音频结束包已发送");
                }
            }
        }

        match tokio::time::timeout(Duration::from_millis(40), websocket.next()).await {
            Ok(Some(Ok(Message::Binary(data)))) => {
                let parsed = protocol::parse_response(&data)?;
                if !is_success_code(parsed.code) {
                    return Err(friendly_asr_service_error(parsed.code));
                }
                let partial =
                    normalize_live_text(&asr::extract_display_text(parsed.payload_msg.as_ref()));
                if !partial.is_empty() && partial != display_text {
                    display_text = partial;
                    emit_partial_text(&app, &display_text);
                }
                for segment in asr::extract_definite_segments(parsed.payload_msg.as_ref()) {
                    if !definitive_segments
                        .iter()
                        .any(|item: &asr::DefiniteSegment| {
                            item.start_time == segment.start_time
                                && item.end_time == segment.end_time
                        })
                    {
                        definitive_segments.push(segment);
                        definitive_segments.sort_by_key(|item| (item.start_time, item.end_time));
                        let text = definitive_segments
                            .iter()
                            .map(|item| item.text.as_str())
                            .collect::<Vec<_>>()
                            .join("");
                        if !text.trim().is_empty() {
                            let normalized = asr::normalize_final_text(&text);
                            emit_partial_text(&app, &normalized);
                        }
                    }
                }
                if parsed.is_last_package {
                    break;
                }
            }
            Ok(Some(Ok(Message::Close(_)))) => break,
            Ok(Some(Ok(_))) | Err(_) => {}
            Ok(Some(Err(err))) => return Err(format!("接收 ASR 响应失败: {}", err)),
            Ok(None) => break,
        }

        if let Some(started) = final_wait_started {
            if started.elapsed() >= final_timeout {
                break;
            }
        }
    }

    if definitive_segments.is_empty() {
        return Ok(asr::normalize_final_text(&display_text));
    }
    definitive_segments.sort_by_key(|item| (item.start_time, item.end_time));
    let final_text = definitive_segments
        .into_iter()
        .map(|item| item.text)
        .collect::<Vec<_>>()
        .join("");
    let final_text = asr::normalize_final_text(&final_text);
    if final_text.is_empty() {
        Ok(asr::normalize_final_text(&display_text))
    } else {
        Ok(final_text)
    }
}

fn emit_partial_text(app: &AppHandle, text: &str) {
    if text.trim().is_empty() {
        return;
    }
    let text = text.to_string();
    let _ = app.emit("asr-partial-text", AsrPartialText { text: text.clone() });
    overlay::update_text(app, text);
}

fn normalize_live_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn emit_error(app: &AppHandle, error: String) {
    app_log::warn(&error);
    overlay::update_text(app, format!("识别失败: {}", error));
    overlay::hide(app);
    let _ = app.emit(
        "asr-final-text",
        AsrFinalText {
            text: String::new(),
            error: Some(error),
            warning: None,
        },
    );
}

fn is_success_code(code: i32) -> bool {
    code == 0 || code == 20_000_000
}

fn friendly_asr_connection_error(error: &str) -> String {
    let lower = error.to_ascii_lowercase();
    if lower.contains("401")
        || lower.contains("403")
        || lower.contains("unauthorized")
        || lower.contains("forbidden")
    {
        "豆包 ASR 认证失败，请检查 App Key、Access Key 和 Resource ID。".to_string()
    } else if lower.contains("dns")
        || lower.contains("resolve")
        || lower.contains("timeout")
        || lower.contains("timed out")
        || lower.contains("connection")
        || lower.contains("connect")
        || lower.contains("proxy")
        || lower.contains("tls")
    {
        "无法连接豆包 ASR 服务，请检查网络、代理或防火墙设置。".to_string()
    } else {
        "连接豆包 ASR 失败，请检查网络环境和豆包认证配置。".to_string()
    }
}

fn friendly_asr_service_error(code: i32) -> String {
    if (400..500).contains(&code) || (40_000_000..50_000_000).contains(&code) {
        format!(
            "豆包 ASR 认证或权限校验失败，错误码 {}。请检查 App Key、Access Key、Resource ID 和服务权限。",
            code
        )
    } else {
        format!(
            "豆包 ASR 服务返回错误码 {}。请稍后重试，或检查网络与豆包控制台配置。",
            code
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{
        friendly_asr_connection_error, friendly_asr_service_error, is_success_code,
        silent_test_audio,
    };
    use crate::config::AppConfig;

    #[test]
    fn accepts_doubao_success_codes() {
        assert!(is_success_code(0));
        assert!(is_success_code(20_000_000));
        assert!(!is_success_code(400));
    }

    #[test]
    fn explains_common_asr_failures() {
        assert!(friendly_asr_connection_error("HTTP error: 401 Unauthorized").contains("认证失败"));
        assert!(friendly_asr_connection_error("dns error").contains("无法连接"));
        assert!(friendly_asr_service_error(40_000_001).contains("权限"));
    }

    #[test]
    fn silent_test_audio_is_small_and_non_empty() {
        let config = AppConfig::default();
        let audio = silent_test_audio(&config);
        assert!(!audio.is_empty());
        assert!(audio.len() >= 3_200);
        assert!(audio.len() <= 32_000);
        assert!(audio.iter().all(|value| *value == 0));
    }
}
