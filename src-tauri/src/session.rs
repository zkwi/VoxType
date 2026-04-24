use serde::Serialize;
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
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AudioLevel {
    pub level: f32,
}

#[derive(Default)]
struct InnerSession {
    recording: bool,
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
                message: "Session state is unavailable.".to_string(),
            };
        };
        SessionState {
            recording: inner.recording,
            message: if let Some(audio) = &inner.audio_capture {
                let info = audio.info();
                format!(
                    "Recording from {} at {} Hz / {} channel(s), {} PCM bytes captured.",
                    info.device_name, info.sample_rate, info.channels, info.pcm_bytes
                )
            } else if inner.recording {
                "Recording is active, waiting for audio stream.".to_string()
            } else {
                "Recording is idle.".to_string()
            },
        }
    }

    pub fn start(&self, app: Option<AppHandle>) -> Result<SessionState, String> {
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
            app_log::warn(format!(
                "录音启动被拦截: config_exists={}, auth_ready=false",
                loaded.exists
            ));
            let state = SessionState {
                recording: false,
                message: message.to_string(),
            };
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
            if inner.recording {
                return Ok(SessionState {
                    recording: true,
                    message: "Recording is already active.".to_string(),
                });
            }
            inner.recording = true;
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
                message: "Recording is starting.".to_string(),
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
                system_audio::safe_restore(volume_state);
                let state = self.force_stop_generation(generation, "Recording failed to start.");
                if let Some(app) = app.as_ref() {
                    overlay::update_text(app, format!("启动录音失败: {}", err));
                    overlay::hide(app);
                    emit_state(
                        Some(app),
                        &state.unwrap_or(SessionState {
                            recording: false,
                            message: format!("Recording failed: {}", err),
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
                true
            }
        };
        if !started {
            system_audio::safe_restore(volume_state);
            return Ok(SessionState {
                recording: false,
                message: "Recording is already idle.".to_string(),
            });
        }

        let state = SessionState {
            recording: true,
            message: "Recording started.".to_string(),
        };
        app_log::info("录音会话已开始");
        emit_state(app.as_ref(), &state);
        if let Some(app) = app.clone() {
            asr_ws::spawn_asr_worker(runtime_config, audio_rx, started_at, app, self.clone());
        }

        let controller = self.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_secs(max_seconds.max(1)));
            let stopped = controller.force_stop_generation(
                generation,
                "Recording reached the configured maximum duration.",
            );
            if let (Some(app), Some(state)) = (app, stopped) {
                emit_state(Some(&app), &state);
            }
        });

        Ok(state)
    }

    pub fn stop(&self, app: Option<AppHandle>) -> Result<SessionState, String> {
        let loaded = config::load_config()?;
        let grace_ms = loaded.data.audio.stop_grace_ms;
        if grace_ms == 0 {
            let state = self.force_stop("Recording stopped.");
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
                    message: "Recording is already idle.".to_string(),
                });
            }
            inner.generation
        };
        app_log::info(format!("收到停止录音请求，等待 {} ms 收尾", grace_ms));

        let controller = self.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(grace_ms));
            let stopped = controller
                .force_stop_generation(generation, "Recording stopped after grace period.");
            if let (Some(app), Some(state)) = (app, stopped) {
                emit_state(Some(&app), &state);
            }
        });

        Ok(SessionState {
            recording: true,
            message: "Recording will stop after the configured grace period.".to_string(),
        })
    }

    pub fn toggle(&self, app: Option<AppHandle>) -> Result<SessionState, String> {
        if self.current_state().recording {
            self.stop(app)
        } else {
            self.start(app)
        }
    }

    fn force_stop(&self, message: &str) -> SessionState {
        let Ok(mut inner) = self.inner.lock() else {
            app_log::warn("停止会话失败：session mutex poisoned");
            return SessionState {
                recording: false,
                message: message.to_string(),
            };
        };
        inner.recording = false;
        inner.generation = inner.generation.wrapping_add(1);
        system_audio::safe_restore(inner.volume_state.take());
        inner.audio_capture = None;
        app_log::info(message);
        SessionState {
            recording: false,
            message: message.to_string(),
        }
    }

    fn force_stop_generation(&self, generation: u64, message: &str) -> Option<SessionState> {
        let Ok(mut inner) = self.inner.lock() else {
            app_log::warn("停止指定会话失败：session mutex poisoned");
            return None;
        };
        if !inner.recording || inner.generation != generation {
            return None;
        }
        inner.recording = false;
        inner.generation = inner.generation.wrapping_add(1);
        system_audio::safe_restore(inner.volume_state.take());
        inner.audio_capture = None;
        app_log::info(message);
        Some(SessionState {
            recording: false,
            message: message.to_string(),
        })
    }

    pub fn abort_from_worker(&self, app: &AppHandle, message: &str) {
        let state = self.force_stop(message);
        overlay::hide(app);
        emit_state(Some(app), &state);
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
