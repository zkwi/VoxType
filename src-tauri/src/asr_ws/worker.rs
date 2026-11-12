use super::errors::classify_asr_error;
use super::output::{
    emit_error, emit_output_warning, emit_successful_final_text, finish_output_sent_session,
    handle_empty_transcript, record_successful_transcript_side_effects,
    should_hold_overlay_for_output_warning, AsrFinalText, ATTENTION_OVERLAY_HOLD,
};
use crate::session::{SessionController, SessionPhase};
use crate::{
    app_log, asr, asr_activity::AsrActivityReporter, asr_provider, config::AppConfig,
    llm_post_edit, overlay, screen_context, text_output,
};
use futures_util::future::{select, Either};
use std::sync::mpsc::Receiver;
use std::thread;
use std::time::{Duration, Instant};
use tauri::AppHandle;

const POST_EDITING_OVERLAY_DELAY: Duration = Duration::from_millis(450);

pub(crate) struct AsrWorkerInput {
    pub(crate) config: AppConfig,
    pub(crate) audio_rx: Receiver<Vec<u8>>,
    pub(crate) started_at: Instant,
    pub(crate) app: AppHandle,
    pub(crate) session: SessionController,
    pub(crate) generation: u64,
    pub(crate) screen_context_rx: Option<screen_context::ScreenContextReceiver>,
    pub(crate) activity: AsrActivityReporter,
}

pub fn spawn_asr_worker(input: AsrWorkerInput) {
    thread::spawn(move || {
        let AsrWorkerInput {
            config,
            audio_rx,
            started_at,
            app,
            session,
            generation,
            screen_context_rx,
            activity,
        } = input;
        app_log::info(format!("ASR worker 已启动: generation={}", generation));
        if let Some(config_error) = asr_provider::worker_configuration_error(&config) {
            let error = config_error.message;
            if session.abort_generation_from_worker_with_code(
                &app,
                generation,
                &error,
                config_error.code,
            ) {
                emit_error(&app, &session, generation, config_error.code, error);
            }
            return;
        }

        let runtime = match tokio::runtime::Runtime::new() {
            Ok(runtime) => runtime,
            Err(err) => {
                let error = format!("启动 ASR 运行时失败: {}", err);
                if session
                    .finish_generation(
                        generation,
                        Some(&app),
                        SessionPhase::Failed,
                        &error,
                        Some("ASR_NETWORK_FAILED"),
                    )
                    .is_some()
                {
                    emit_error(&app, &session, generation, "ASR_NETWORK_FAILED", error);
                }
                return;
            }
        };
        let typing = config.typing.clone();
        let remove_trailing_period = config.typing.remove_trailing_period;
        // OCR 等待不再挡在建连前面：交给 provider 在握手的同时解析，结果解析一次后复用。
        let screen_context = screen_context::PendingScreenContext::new(
            screen_context_rx,
            config.screen_context.timeout_ms,
        );
        let runtime_result = runtime.block_on(async {
            let text = asr_provider::recognize_stream(asr_provider::RecognitionInput {
                config: config.clone(),
                audio_rx,
                app: app.clone(),
                session: session.clone(),
                generation,
                screen_context: screen_context.clone(),
                activity,
            })
            .await?;
            let outcome = if !text.trim().is_empty() && llm_post_edit::should_polish(&config, &text)
            {
                // 首包等待超时的 OCR 结果到这里通常已经返回，润色仍可拿来做参考。
                // 只在确实要润色时才取：取用会记一条"已用于润色参考"的日志，不润色时不能记。
                let screen_context_text = screen_context.resolve_for_post_edit().await;
                polish_with_delayed_status(
                    &config,
                    &text,
                    screen_context_text.as_deref(),
                    &app,
                    &session,
                    generation,
                )
                .await
            } else {
                llm_post_edit::unchanged(&text)
            };
            Ok::<_, String>((text, outcome))
        });
        match runtime_result {
            Ok((original, outcome)) => {
                let original_normalized =
                    asr::normalize_final_text(&original, remove_trailing_period);
                if !session.is_current_generation(generation) {
                    app_log::info(format!(
                        "忽略过期 ASR worker 输出: generation={}, chars={}",
                        generation,
                        original_normalized.chars().count()
                    ));
                    return;
                }
                if original_normalized.trim().is_empty() {
                    handle_empty_transcript(&app, &session, generation);
                    return;
                }
                let Some(skipped) = session.begin_pasting(generation, Some(&app)) else {
                    return;
                };
                let mut outcome = if skipped {
                    llm_post_edit::unchanged(&original)
                } else {
                    outcome
                };
                let mut text = asr::normalize_final_text(&outcome.text, remove_trailing_period);
                if text.trim().is_empty() {
                    app_log::warn("LLM polish normalized to empty content, original text kept");
                    outcome = llm_post_edit::unusable_polish_fallback(&original);
                    text = original_normalized.clone();
                }
                let llm_warning = outcome.warning;
                let llm_warning_code = outcome.warning_code;
                let raw_text =
                    original_for_preview(&text, &original_normalized, llm_warning.as_deref());
                app_log::info(format!("ASR worker 返回文本长度: {}", text.chars().count()));
                overlay::update_text(&app, &text);
                let duration = started_at.elapsed().as_secs_f64();
                let output = match text_output::output_text(&text, &typing, || {
                    let preview_revision = crate::main_window::preview_revision();
                    if !finish_output_sent_session(&session, generation, Some(&app)) {
                        return;
                    }
                    // 输出层已写入剪贴板并发出粘贴键。收起动画和恢复剪贴板各自继续。
                    if llm_warning.is_none() && typing.paste_method != "clipboard_only" {
                        overlay::dismiss_after_paste(&app, &session, generation);
                    } else {
                        overlay::run_for_generation(&app, &session, generation, overlay::hide);
                    }
                    emit_successful_final_text(
                        &app,
                        &session,
                        preview_revision,
                        AsrFinalText {
                            generation,
                            text: text.clone(),
                            raw_text: raw_text.map(str::to_string),
                            error: None,
                            error_code: None,
                            warning: llm_warning.clone(),
                            warning_code: llm_warning_code.map(str::to_string),
                        },
                    );
                    record_successful_transcript_side_effects(&app, &text, duration);
                    app_log::info(format!(
                        "ASR session output sent: chars={}",
                        text.chars().count()
                    ));
                }) {
                    Ok(output) => output,
                    Err(err) => {
                        let error_code = if err.contains("剪贴板") {
                            "CLIPBOARD_WRITE_FAILED"
                        } else {
                            "PASTE_FAILED"
                        };
                        if let Some(guard_generation) = session
                            .reset_generation_from_worker_with_code(
                                &app, generation, &err, error_code,
                            )
                        {
                            emit_error(&app, &session, guard_generation, error_code, err);
                        }
                        return;
                    }
                };
                if let Some(warning) = output.warning.filter(|warning| {
                    should_hold_overlay_for_output_warning(
                        Some(warning),
                        output.warning_code.as_deref(),
                    )
                }) {
                    emit_output_warning(
                        &app,
                        &session,
                        generation,
                        &warning,
                        output.warning_code.as_deref(),
                    );
                    app_log::warn(format!("输出文本完成但存在提示: {warning}"));
                    let ui = config.ui.clone();
                    overlay::run_for_generation(&app, &session, generation, move |app| {
                        overlay::show_message(app, &ui, warning);
                    });
                    thread::sleep(ATTENTION_OVERLAY_HOLD);
                    overlay::run_for_generation(&app, &session, generation, overlay::hide);
                }
                app_log::info(format!(
                    "ASR session finished: chars={}",
                    text.chars().count()
                ));
            }
            Err(err) => {
                let error_code = classify_asr_error(&err);
                if let Some(guard_generation) = session
                    .reset_generation_from_worker_with_code(&app, generation, &err, error_code)
                {
                    emit_error(&app, &session, guard_generation, error_code, err);
                }
            }
        }
    });
}

fn original_for_preview<'a>(
    output: &str,
    original_normalized: &'a str,
    warning: Option<&str>,
) -> Option<&'a str> {
    (warning.is_none() && !output.is_empty() && output != original_normalized)
        .then_some(original_normalized)
}

async fn polish_with_delayed_status(
    config: &AppConfig,
    text: &str,
    screen_context: Option<&str>,
    app: &AppHandle,
    session: &SessionController,
    generation: u64,
) -> llm_post_edit::PolishOutcome {
    let config = config.clone();
    let original_text = text.to_string();
    let task_text = original_text.clone();
    let screen_context = screen_context.map(str::to_string);
    let mut polish_task = tokio::spawn(async move {
        llm_post_edit::polish(&config, &task_text, screen_context.as_deref()).await
    });

    let result = match tokio::time::timeout(POST_EDITING_OVERLAY_DELAY, &mut polish_task).await {
        Ok(result) => result,
        Err(_) => {
            let Some(mut skip) = session.begin_post_editing(generation, Some(app)) else {
                polish_task.abort();
                return llm_post_edit::unchanged(&original_text);
            };
            overlay::update_status(app, "post_editing", overlay::POST_EDITING_TEXT);
            match select(
                Box::pin(async move { skip.wait_for(|skipped| *skipped).await.is_ok() }),
                &mut polish_task,
            )
            .await
            {
                Either::Left(_) => {
                    polish_task.abort();
                    return llm_post_edit::unchanged(&original_text);
                }
                Either::Right((result, _)) => result,
            }
        }
    };
    match result {
        Ok(outcome) => outcome,
        Err(err) => {
            app_log::warn(format!("大模型润色任务异常: {}", err));
            llm_post_edit::PolishOutcome {
                text: original_text,
                warning: Some("大模型润色任务异常，已使用原文。".to_string()),
                warning_code: None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::original_for_preview;
    use crate::asr::normalize_final_text;

    #[test]
    fn preview_keeps_the_normalized_original_only_when_polishing_changed_it() {
        for remove_period in [true, false] {
            let original = normalize_final_text("识别原文。", remove_period);
            let expected = if remove_period {
                "识别原文"
            } else {
                "识别原文。"
            };
            assert_eq!(
                original_for_preview("润色结果", &original, None),
                Some(expected)
            );
            assert_eq!(original_for_preview(&original, &original, None), None);
            assert_eq!(
                original_for_preview("润色结果", &original, Some("润色告警")),
                None
            );
            assert_eq!(original_for_preview("", &original, None), None);
        }
    }
}
