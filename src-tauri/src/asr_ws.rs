use crate::session::SessionController;
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
                emit_error(&app, format!("启动 ASR 运行时失败: {}", err));
                return;
            }
        };
        let typing = config.typing.clone();
        let runtime_result = runtime.block_on(async {
            let text = run_websocket_session(config.clone(), audio_rx, app.clone()).await?;
            Ok::<String, String>(llm_post_edit::polish(&config, &text).await)
        });
        match runtime_result {
            Ok(text) => {
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
                    if let Err(err) = text_output::output_text(&text, &typing) {
                        emit_error(&app, err);
                        return;
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
                let _ = app.emit("asr-final-text", AsrFinalText { text, error: None });
            }
            Err(err) => {
                session.abort_from_worker(&app, &err);
                emit_error(&app, err);
            }
        }
    });
}

async fn run_websocket_session(
    config: AppConfig,
    audio_rx: Receiver<Vec<u8>>,
    app: AppHandle,
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

    let (mut websocket, _) = connect_async(request)
        .await
        .map_err(|err| format!("连接 ASR WebSocket 失败: {}", err))?;
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
                    app_log::info("ASR 音频结束包已发送");
                }
            }
        }

        match tokio::time::timeout(Duration::from_millis(40), websocket.next()).await {
            Ok(Some(Ok(Message::Binary(data)))) => {
                let parsed = protocol::parse_response(&data)?;
                if !is_success_code(parsed.code) {
                    return Err(format!("ASR 服务返回错误码: {}", parsed.code));
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
        },
    );
}

fn is_success_code(code: i32) -> bool {
    code == 0 || code == 20_000_000
}

#[cfg(test)]
mod tests {
    use super::is_success_code;

    #[test]
    fn accepts_doubao_success_codes() {
        assert!(is_success_code(0));
        assert!(is_success_code(20_000_000));
        assert!(!is_success_code(400));
    }
}
