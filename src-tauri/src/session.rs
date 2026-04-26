use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

use crate::app_log;
use crate::asr_ws;
use crate::audio::{self, AudioCapture};
use crate::config;
use crate::overlay;
use crate::system_audio::{self, VolumeState};

#[derive(Debug, Clone, Serialize)]
pub struct SessionState {
    pub recording: bool,
    pub phase: SessionPhase,
    pub message: String,
    pub error_code: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionPhase {
    #[default]
    Idle,
    Starting,
    Recording,
    Stopping,
    WaitingFinalResult,
    PostEditing,
    Pasting,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, Serialize)]
pub struct AudioLevel {
    pub level: f32,
}

#[derive(Default)]
struct InnerSession {
    recording: bool,
    phase: SessionPhase,
    message: String,
    error_code: Option<String>,
    generation: u64,
    audio_capture: Option<AudioCapture>,
    volume_state: Option<VolumeState>,
}

#[derive(Clone, Default)]
pub struct SessionController {
    inner: Arc<Mutex<InnerSession>>,
}

impl SessionController {
    pub fn current_state(&self) -> SessionState {
        let Ok(inner) = self.inner.lock() else {
            app_log::warn("读取会话状态失败：session mutex poisoned");
            return SessionState {
                recording: false,
                phase: SessionPhase::Failed,
                message: "Session state is unavailable.".to_string(),
                error_code: Some("SESSION_STATE_UNAVAILABLE".to_string()),
            };
        };
        state_from_inner(&inner)
    }

    /// 启动一轮录音会话，并把后续 ASR worker 绑定到当前 generation。
    ///
    /// 已进入等待最终结果、润色或粘贴阶段时不会重新启动录音，调用方会拿到当前状态。
    /// 这是全局热键、右 Alt、鼠标中键和托盘入口共用的保护边界。
    pub fn start(&self, app: Option<AppHandle>) -> Result<SessionState, String> {
        let current = self.current_state();
        if is_processing_phase(current.phase) {
            emit_state(app.as_ref(), &current);
            return Ok(current);
        }
        let loaded = config::load_config()?;
        let max_seconds = loaded.data.audio.max_record_seconds;
        if loaded.data.auth.app_key.trim().is_empty()
            || loaded.data.auth.access_key.trim().is_empty()
        {
            let message = if loaded.exists {
                "ASR 未配置 app_key/access_key，请先在配置页填写豆包认证信息并保存。"
            } else {
                "未找到 config.toml。请先在配置页填写豆包认证信息并保存，或复制 config.example.toml 为 config.toml 后手动编辑。"
            };
            let error_code = if loaded.exists {
                "ASR_AUTH_MISSING"
            } else {
                "CONFIG_MISSING"
            };
            app_log::warn(format!(
                "录音启动被拦截: config_exists={}, auth_ready=false",
                loaded.exists
            ));
            let state = SessionState {
                recording: false,
                phase: SessionPhase::Failed,
                message: message.to_string(),
                error_code: Some(error_code.to_string()),
            };
            self.set_state_values(false, SessionPhase::Failed, message, Some(error_code));
            if let Some(app) = app.as_ref() {
                emit_state(Some(app), &state);
            }
            return Err(message.to_string());
        }
        let generation = {
            let mut inner = self
                .inner
                .lock()
                .map_err(|_| "session mutex poisoned".to_string())?;
            if inner.recording || is_processing_phase(inner.phase) {
                let state = state_from_inner(&inner);
                drop(inner);
                emit_state(app.as_ref(), &state);
                return Ok(state);
            }
            inner.recording = true;
            inner.phase = SessionPhase::Starting;
            inner.message = "Recording is starting.".to_string();
            inner.error_code = None;
            inner.generation = inner.generation.wrapping_add(1);
            inner.audio_capture = None;
            inner.volume_state = None;
            inner.generation
        };

        let (audio_tx, audio_rx) = std::sync::mpsc::channel();
        let (level_tx, level_rx) = if app.is_some() {
            let (tx, rx) = std::sync::mpsc::channel();
            (Some(tx), Some(rx))
        } else {
            (None, None)
        };
        let started_at = Instant::now();
        app_log::info(format!(
            "录音启动请求: max_seconds={}, stop_grace_ms={}, mute_system_volume={}",
            max_seconds,
            loaded.data.audio.stop_grace_ms,
            loaded.data.audio.mute_system_volume_while_recording
        ));
        if let Some(app) = app.as_ref() {
            overlay::show_for_recording(app, &loaded.data.ui);
            let starting = SessionState {
                recording: true,
                phase: SessionPhase::Starting,
                message: "Recording is starting.".to_string(),
                error_code: None,
            };
            emit_state(Some(app), &starting);
        }
        let volume_state = if loaded.data.audio.mute_system_volume_while_recording {
            system_audio::safe_mute_and_save()
        } else {
            None
        };
        let audio_capture = match audio::start_capture(&loaded.data.audio, Some(audio_tx), level_tx)
        {
            Ok(capture) => capture,
            Err(err) => {
                let error_code =
                    if err.contains("未找到") || err.contains("找不到") || err.contains("没有可用")
                    {
                        "MIC_DEVICE_NOT_FOUND"
                    } else {
                        "MIC_START_FAILED"
                    };
                system_audio::safe_restore(volume_state);
                let state = self.force_stop_generation(
                    generation,
                    SessionPhase::Failed,
                    "Recording failed to start.",
                    Some(error_code),
                );
                if let Some(app) = app.as_ref() {
                    overlay::update_text(app, format!("启动录音失败: {}", err));
                    overlay::hide(app);
                    emit_state(
                        Some(app),
                        &state.unwrap_or(SessionState {
                            recording: false,
                            phase: SessionPhase::Failed,
                            message: format!("Recording failed: {}", err),
                            error_code: Some(error_code.to_string()),
                        }),
                    );
                }
                app_log::warn(format!("启动麦克风失败: {}", err));
                return Err(err);
            }
        };
        let audio_info = audio_capture.info();
        app_log::info(format!(
            "麦克风采集已启动: device=\"{}\", rate={}Hz, channels={}",
            audio_info.device_name, audio_info.sample_rate, audio_info.channels
        ));
        if let (Some(app_for_level), Some(level_rx)) = (app.clone(), level_rx) {
            spawn_audio_level_emitter(app_for_level, level_rx);
        }
        let mut runtime_config = loaded.data.clone();
        runtime_config.audio.sample_rate = audio_info.sample_rate;
        runtime_config.audio.channels = audio_info.channels;
        let mut audio_capture = Some(audio_capture);
        let mut volume_state = volume_state;
        let started = {
            let mut inner = match self.inner.lock() {
                Ok(inner) => inner,
                Err(_) => {
                    system_audio::safe_restore(volume_state);
                    return Err("session mutex poisoned".to_string());
                }
            };
            if !inner.recording || inner.generation != generation {
                false
            } else {
                inner.audio_capture = audio_capture.take();
                inner.volume_state = volume_state.take();
                inner.phase = SessionPhase::Recording;
                inner.message = "Recording started.".to_string();
                inner.error_code = None;
                true
            }
        };
        if !started {
            system_audio::safe_restore(volume_state);
            return Ok(SessionState {
                recording: false,
                phase: SessionPhase::Idle,
                message: "Recording is already idle.".to_string(),
                error_code: None,
            });
        }

        let state = SessionState {
            recording: true,
            phase: SessionPhase::Recording,
            message: "Recording started.".to_string(),
            error_code: None,
        };
        app_log::info("录音会话已开始");
        if let Some(app) = app.as_ref() {
            overlay::update_text(app, overlay::RECORDING_TEXT);
        }
        emit_state(app.as_ref(), &state);
        if let Some(app) = app.clone() {
            asr_ws::spawn_asr_worker(
                runtime_config,
                audio_rx,
                started_at,
                app,
                self.clone(),
                generation,
            );
        }

        let controller = self.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_secs(max_seconds.max(1)));
            let stopped = controller.force_stop_generation(
                generation,
                SessionPhase::WaitingFinalResult,
                "Recording reached the configured maximum duration.",
                None,
            );
            if let (Some(app), Some(state)) = (app, stopped) {
                emit_state(Some(&app), &state);
            }
        });

        Ok(state)
    }

    /// 停止当前录音会话。
    ///
    /// 若配置了尾音保留，会先进入 `Stopping`，短暂等待后再切到等待最终结果；
    /// generation 必须保持不变，避免旧 ASR worker 覆盖新会话状态。
    pub fn stop(&self, app: Option<AppHandle>) -> Result<SessionState, String> {
        let loaded = config::load_config()?;
        let grace_ms = loaded.data.audio.stop_grace_ms;
        if grace_ms == 0 {
            let state =
                self.force_stop(SessionPhase::WaitingFinalResult, "Recording stopped.", None);
            emit_state(app.as_ref(), &state);
            return Ok(state);
        }

        let generation = {
            let inner = self
                .inner
                .lock()
                .map_err(|_| "session mutex poisoned".to_string())?;
            if !inner.recording {
                return Ok(SessionState {
                    recording: false,
                    phase: inner.phase,
                    message: "Recording is already idle.".to_string(),
                    error_code: inner.error_code.clone(),
                });
            }
            drop(inner);
            self.set_phase(
                app.as_ref(),
                SessionPhase::Stopping,
                "Recording is stopping.",
                None,
            );
            let inner = self
                .inner
                .lock()
                .map_err(|_| "session mutex poisoned".to_string())?;
            inner.generation
        };
        app_log::info(format!("收到停止录音请求，等待 {} ms 收尾", grace_ms));

        let controller = self.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(grace_ms));
            let stopped = controller.force_stop_generation(
                generation,
                SessionPhase::WaitingFinalResult,
                "Recording stopped after grace period.",
                None,
            );
            if let (Some(app), Some(state)) = (app, stopped) {
                emit_state(Some(&app), &state);
            }
        });

        Ok(SessionState {
            recording: true,
            phase: SessionPhase::Stopping,
            message: "Recording will stop after the configured grace period.".to_string(),
            error_code: None,
        })
    }

    pub fn toggle(&self, app: Option<AppHandle>) -> Result<SessionState, String> {
        let current = self.current_state();
        match current.phase {
            SessionPhase::Starting | SessionPhase::Stopping => {
                emit_state(app.as_ref(), &current);
                Ok(current)
            }
            SessionPhase::Recording => self.stop(app),
            SessionPhase::WaitingFinalResult
            | SessionPhase::PostEditing
            | SessionPhase::Pasting => {
                emit_state(app.as_ref(), &current);
                Ok(current)
            }
            _ => self.start(app),
        }
    }

    fn force_stop(
        &self,
        phase: SessionPhase,
        message: &str,
        error_code: Option<&str>,
    ) -> SessionState {
        let Ok(mut inner) = self.inner.lock() else {
            app_log::warn("停止会话失败：session mutex poisoned");
            return SessionState {
                recording: false,
                phase: SessionPhase::Failed,
                message: message.to_string(),
                error_code: Some("SESSION_STOP_FAILED".to_string()),
            };
        };
        inner.recording = false;
        inner.phase = phase;
        inner.message = message.to_string();
        inner.error_code = error_code.map(str::to_string);
        system_audio::safe_restore(inner.volume_state.take());
        inner.audio_capture = None;
        app_log::info(message);
        SessionState {
            recording: false,
            phase,
            message: message.to_string(),
            error_code: error_code.map(str::to_string),
        }
    }

    fn force_stop_generation(
        &self,
        generation: u64,
        phase: SessionPhase,
        message: &str,
        error_code: Option<&str>,
    ) -> Option<SessionState> {
        let Ok(mut inner) = self.inner.lock() else {
            app_log::warn("停止指定会话失败：session mutex poisoned");
            return None;
        };
        if !inner.recording || inner.generation != generation {
            return None;
        }
        inner.recording = false;
        inner.phase = phase;
        inner.message = message.to_string();
        inner.error_code = error_code.map(str::to_string);
        system_audio::safe_restore(inner.volume_state.take());
        inner.audio_capture = None;
        app_log::info(message);
        Some(SessionState {
            recording: false,
            phase,
            message: message.to_string(),
            error_code: error_code.map(str::to_string),
        })
    }

    pub fn abort_from_worker(&self, app: &AppHandle, message: &str) {
        self.abort_from_worker_with_code(app, message, "SESSION_FAILED");
    }

    pub fn abort_from_worker_with_code(&self, app: &AppHandle, message: &str, error_code: &str) {
        let state = self.force_stop(SessionPhase::Failed, message, Some(error_code));
        emit_state(Some(app), &state);
    }

    pub fn abort_generation_from_worker_with_code(
        &self,
        app: &AppHandle,
        generation: u64,
        message: &str,
        error_code: &str,
    ) -> bool {
        match self.finish_generation(
            generation,
            Some(app),
            SessionPhase::Failed,
            message,
            Some(error_code),
        ) {
            Some(_) => true,
            None => {
                app_log::info(format!(
                    "忽略过期 ASR worker 失败状态: generation={}, error_code={}",
                    generation, error_code
                ));
                false
            }
        }
    }

    pub fn is_current_generation(&self, generation: u64) -> bool {
        let Ok(inner) = self.inner.lock() else {
            app_log::warn("检查会话 generation 失败：session mutex poisoned");
            return false;
        };
        inner.generation == generation
    }

    pub fn set_phase_for_generation(
        &self,
        generation: u64,
        app: Option<&AppHandle>,
        phase: SessionPhase,
        message: &str,
        error_code: Option<&str>,
    ) -> Option<SessionState> {
        let state = self.set_state_values_for_generation(generation, phase, message, error_code);
        if let Some(state) = state.as_ref() {
            emit_state(app, state);
        } else {
            app_log::info(format!(
                "忽略过期会话状态更新: generation={}, phase={:?}",
                generation, phase
            ));
        }
        state
    }

    pub fn finish_generation(
        &self,
        generation: u64,
        app: Option<&AppHandle>,
        phase: SessionPhase,
        message: &str,
        error_code: Option<&str>,
    ) -> Option<SessionState> {
        let Ok(mut inner) = self.inner.lock() else {
            app_log::warn("结束指定会话失败：session mutex poisoned");
            return None;
        };
        if inner.generation != generation {
            return None;
        }
        inner.recording = false;
        inner.phase = phase;
        inner.message = message.to_string();
        inner.error_code = error_code.map(str::to_string);
        system_audio::safe_restore(inner.volume_state.take());
        inner.audio_capture = None;
        app_log::info(message);
        let state = state_from_inner(&inner);
        drop(inner);
        emit_state(app, &state);
        Some(state)
    }

    pub fn set_phase(
        &self,
        app: Option<&AppHandle>,
        phase: SessionPhase,
        message: &str,
        error_code: Option<&str>,
    ) -> SessionState {
        let recording = matches!(
            phase,
            SessionPhase::Starting | SessionPhase::Recording | SessionPhase::Stopping
        );
        self.set_state_values(recording, phase, message, error_code);
        let state = SessionState {
            recording,
            phase,
            message: message.to_string(),
            error_code: error_code.map(str::to_string),
        };
        emit_state(app, &state);
        state
    }

    fn set_state_values(
        &self,
        recording: bool,
        phase: SessionPhase,
        message: &str,
        error_code: Option<&str>,
    ) {
        let Ok(mut inner) = self.inner.lock() else {
            app_log::warn("更新会话状态失败：session mutex poisoned");
            return;
        };
        inner.recording = recording;
        inner.phase = phase;
        inner.message = message.to_string();
        inner.error_code = error_code.map(str::to_string);
    }

    fn set_state_values_for_generation(
        &self,
        generation: u64,
        phase: SessionPhase,
        message: &str,
        error_code: Option<&str>,
    ) -> Option<SessionState> {
        let Ok(mut inner) = self.inner.lock() else {
            app_log::warn("更新指定会话状态失败：session mutex poisoned");
            return None;
        };
        if inner.generation != generation {
            return None;
        }
        inner.recording = matches!(
            phase,
            SessionPhase::Starting | SessionPhase::Recording | SessionPhase::Stopping
        );
        inner.phase = phase;
        inner.message = message.to_string();
        inner.error_code = error_code.map(str::to_string);
        Some(state_from_inner(&inner))
    }
}

pub fn emit_state(app: Option<&AppHandle>, state: &SessionState) {
    if let Some(app) = app {
        let _ = app.emit("session-state-changed", state);
    }
}

fn spawn_audio_level_emitter(app: AppHandle, level_rx: std::sync::mpsc::Receiver<f32>) {
    thread::spawn(move || {
        let mut last_emit = Instant::now()
            .checked_sub(Duration::from_millis(100))
            .unwrap_or_else(Instant::now);
        while let Ok(level) = level_rx.recv() {
            if last_emit.elapsed() < Duration::from_millis(80) {
                continue;
            }
            let _ = app.emit(
                "audio-level",
                AudioLevel {
                    level: level.clamp(0.0, 1.0),
                },
            );
            last_emit = Instant::now();
        }
        let _ = app.emit("audio-level", AudioLevel { level: 0.0 });
    });
}

fn is_processing_phase(phase: SessionPhase) -> bool {
    matches!(
        phase,
        SessionPhase::WaitingFinalResult | SessionPhase::PostEditing | SessionPhase::Pasting
    )
}

fn state_from_inner(inner: &InnerSession) -> SessionState {
    SessionState {
        recording: inner.recording,
        phase: inner.phase,
        message: if matches!(inner.phase, SessionPhase::Recording) {
            if let Some(audio) = &inner.audio_capture {
                let info = audio.info();
                format!(
                    "Recording from {} at {} Hz / {} channel(s), {} PCM bytes captured.",
                    info.device_name, info.sample_rate, info.channels, info.pcm_bytes
                )
            } else {
                "Recording is active, waiting for audio stream.".to_string()
            }
        } else {
            inner.message.clone()
        },
        error_code: inner.error_code.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::{SessionController, SessionPhase};

    #[test]
    fn stop_keeps_generation_valid_for_post_processing() {
        let controller = SessionController::default();
        {
            let mut inner = controller.inner.lock().unwrap();
            inner.recording = true;
            inner.phase = SessionPhase::Recording;
            inner.generation = 7;
        }

        let stopped = controller.force_stop_generation(
            7,
            SessionPhase::WaitingFinalResult,
            "Recording stopped.",
            None,
        );

        assert!(stopped.is_some());
        assert!(controller.is_current_generation(7));
        assert!(controller
            .set_phase_for_generation(7, None, SessionPhase::PostEditing, "Post-editing.", None)
            .is_some());
        assert!(controller
            .finish_generation(7, None, SessionPhase::Succeeded, "Done.", None)
            .is_some());
    }

    #[test]
    fn stale_worker_cannot_mutate_new_session() {
        let controller = SessionController::default();
        {
            let mut inner = controller.inner.lock().unwrap();
            inner.recording = true;
            inner.phase = SessionPhase::Recording;
            inner.message = "Recording started.".to_string();
            inner.generation = 2;
        }

        assert!(controller
            .set_phase_for_generation(1, None, SessionPhase::PostEditing, "Stale update.", None)
            .is_none());
        assert!(controller
            .finish_generation(
                1,
                None,
                SessionPhase::Failed,
                "Stale failure.",
                Some("STALE")
            )
            .is_none());

        let state = controller.current_state();
        assert!(state.recording);
        assert_eq!(state.phase, SessionPhase::Recording);
        assert_eq!(state.error_code, None);
    }

    #[test]
    fn processing_phases_block_new_start_attempts() {
        assert!(super::is_processing_phase(SessionPhase::WaitingFinalResult));
        assert!(super::is_processing_phase(SessionPhase::PostEditing));
        assert!(super::is_processing_phase(SessionPhase::Pasting));
        assert!(!super::is_processing_phase(SessionPhase::Idle));
        assert!(!super::is_processing_phase(SessionPhase::Recording));
    }

    #[test]
    fn toggle_ignores_starting_phase() {
        let controller = SessionController::default();
        {
            let mut inner = controller.inner.lock().unwrap();
            inner.recording = true;
            inner.phase = SessionPhase::Starting;
            inner.message = "Recording is starting.".to_string();
            inner.generation = 3;
        }

        let state = controller.toggle(None).unwrap();

        assert!(state.recording);
        assert_eq!(state.phase, SessionPhase::Starting);
        assert!(controller.is_current_generation(3));
    }

    #[test]
    fn toggle_ignores_stopping_phase() {
        let controller = SessionController::default();
        {
            let mut inner = controller.inner.lock().unwrap();
            inner.recording = true;
            inner.phase = SessionPhase::Stopping;
            inner.message = "Recording is stopping.".to_string();
            inner.generation = 5;
        }

        let state = controller.toggle(None).unwrap();

        assert!(state.recording);
        assert_eq!(state.phase, SessionPhase::Stopping);
        assert!(controller.is_current_generation(5));
    }

    #[test]
    fn toggle_recording_phase_still_allows_stop() {
        let controller = SessionController::default();
        {
            let mut inner = controller.inner.lock().unwrap();
            inner.recording = true;
            inner.phase = SessionPhase::Recording;
            inner.message = "Recording started.".to_string();
            inner.generation = 8;
        }

        let state = controller.toggle(None).unwrap();

        assert!(matches!(
            state.phase,
            SessionPhase::Stopping | SessionPhase::WaitingFinalResult
        ));
    }
}
