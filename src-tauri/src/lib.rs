mod app_log;
mod asr;
mod asr_ws;
mod audio;
mod autostart;
mod config;
mod hotkey;
mod llm_post_edit;
mod overlay;
mod protocol;
mod session;
mod setup_guide;
mod stats;
mod system_audio;
mod text_output;
mod tray;
mod update;

use config::{AppConfig, LoadedConfig};
use serde::Serialize;
use session::SessionController;
use stats::StatsSnapshot;
use tauri::{AppHandle, Manager, State, WindowEvent};

#[derive(Serialize)]
struct AppSnapshot {
    hotkey: String,
    current_version: String,
}

#[derive(Serialize)]
struct SetupStatus {
    ready: bool,
    missing_auth: bool,
    has_audio_device: bool,
    hotkey: String,
    paste_method: String,
    privacy_recent_context_enabled: bool,
    warnings: Vec<SetupWarning>,
}

#[derive(Serialize)]
struct SetupWarning {
    code: String,
    title: String,
    message: String,
    action: String,
}

#[derive(Serialize)]
struct ConnectionTestResult {
    message: String,
}

#[tauri::command]
fn get_app_snapshot() -> Result<AppSnapshot, String> {
    let loaded = config::load_config()?;

    Ok(AppSnapshot {
        hotkey: loaded.data.hotkey,
        current_version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

#[tauri::command]
fn get_setup_status() -> Result<SetupStatus, String> {
    let loaded = config::load_config()?;
    let data = loaded.data;
    let missing_auth =
        data.auth.app_key.trim().is_empty() || data.auth.access_key.trim().is_empty();
    let has_audio_device = audio::list_input_devices()
        .map(|devices| !devices.is_empty())
        .unwrap_or(false);
    let mut warnings = Vec::new();

    if missing_auth {
        warnings.push(SetupWarning {
            code: "ASR_AUTH_MISSING".to_string(),
            title: "ASR 密钥未填写".to_string(),
            message: "填写 App Key 和 Access Key 后才能开始语音识别。".to_string(),
            action: "asr_auth".to_string(),
        });
    }
    if !has_audio_device {
        warnings.push(SetupWarning {
            code: "MIC_DEVICE_NOT_FOUND".to_string(),
            title: "未检测到麦克风".to_string(),
            message: "请接入或启用麦克风，然后重新检查设备。".to_string(),
            action: "audio".to_string(),
        });
    }
    if data.triggers.middle_mouse_enabled || data.triggers.right_alt_enabled {
        warnings.push(SetupWarning {
            code: "EXTRA_TRIGGER_ENABLED".to_string(),
            title: "额外触发方式已开启".to_string(),
            message: "右 Alt 和鼠标中键可能和其他软件操作冲突，建议确认后再使用。".to_string(),
            action: "hotkey".to_string(),
        });
    }
    if data.audio.mute_system_volume_while_recording {
        warnings.push(SetupWarning {
            code: "SYSTEM_AUDIO_MUTE_ENABLED".to_string(),
            title: "录音时会静音系统声音".to_string(),
            message: "这能减少回声，但可能影响会议、视频或系统提示音。".to_string(),
            action: "audio".to_string(),
        });
    }
    if data.context.enable_recent_context {
        warnings.push(SetupWarning {
            code: "RECENT_CONTEXT_ENABLED".to_string(),
            title: "最近上下文已开启".to_string(),
            message: "最近识别片段会保存在本地，用于改善连续识别；如介意隐私可关闭。".to_string(),
            action: "privacy".to_string(),
        });
    }

    Ok(SetupStatus {
        ready: !missing_auth
            && has_audio_device
            && data.triggers.hotkey_enabled
            && !data.context.enable_recent_context,
        missing_auth,
        has_audio_device,
        hotkey: data.hotkey,
        paste_method: data.typing.paste_method,
        privacy_recent_context_enabled: data.context.enable_recent_context,
        warnings,
    })
}

#[tauri::command]
fn load_app_config() -> Result<LoadedConfig, String> {
    match config::load_config() {
        Ok(loaded) => {
            app_log::info(format!("配置加载完成: exists={}", loaded.exists));
            Ok(loaded)
        }
        Err(err) => {
            app_log::warn(format!("配置加载失败: {}", err));
            Err(err)
        }
    }
}

#[tauri::command]
fn save_app_config(config: AppConfig) -> Result<LoadedConfig, String> {
    match config::save_config(config) {
        Ok(loaded) => {
            hotkey::refresh_trigger_config_from(&loaded.data.triggers);
            if let Err(err) = autostart::apply(&loaded.data.startup) {
                app_log::warn(format!("同步开机自启动失败: {}", err));
                return Err(format!("配置已保存，但开机自启动设置失败: {}", err));
            }
            app_log::info(format!(
                "配置保存完成: hotkey_enabled={}, middle_mouse_enabled={}, right_alt_enabled={}, launch_on_startup={}, update_auto_check={}, update_repo={}, llm_enabled={}",
                loaded.data.triggers.hotkey_enabled,
                loaded.data.triggers.middle_mouse_enabled,
                loaded.data.triggers.right_alt_enabled,
                loaded.data.startup.launch_on_startup,
                loaded.data.update.auto_check_on_startup,
                loaded.data.update.github_repo,
                loaded.data.llm_post_edit.enabled
            ));
            Ok(loaded)
        }
        Err(err) => {
            app_log::warn(format!("配置保存失败: {}", err));
            Err(err)
        }
    }
}

#[tauri::command]
async fn test_asr_config(config: AppConfig) -> Result<ConnectionTestResult, String> {
    app_log::info("用户开始测试豆包 ASR 配置。");
    match asr_ws::test_connection(&config).await {
        Ok(()) => {
            app_log::info("豆包 ASR 配置测试成功。");
            Ok(ConnectionTestResult {
                message: "豆包 ASR 测试成功，当前 Key 可用。".to_string(),
            })
        }
        Err(err) => {
            app_log::warn(format!("豆包 ASR 配置测试失败: {}", err));
            Err(err)
        }
    }
}

#[tauri::command]
async fn test_llm_config(config: AppConfig) -> Result<ConnectionTestResult, String> {
    app_log::info("用户开始测试大模型配置。");
    match llm_post_edit::test_connection(&config).await {
        Ok(()) => {
            app_log::info("大模型配置测试成功。");
            Ok(ConnectionTestResult {
                message: "大模型测试成功，当前 API Key 可用。".to_string(),
            })
        }
        Err(err) => {
            app_log::warn(format!("大模型配置测试失败: {}", err));
            Err(err)
        }
    }
}

#[tauri::command]
fn open_setup_guide(app: AppHandle) -> Result<(), String> {
    app_log::info("用户打开配置指南。");
    setup_guide::open(&app).map_err(|err| {
        app_log::warn(format!("打开配置指南失败: {}", err));
        err
    })
}

#[tauri::command]
fn open_log_file(app: AppHandle) -> Result<(), String> {
    match tray::open_log_file_from_main(&app) {
        Ok(()) => Ok(()),
        Err(err) => {
            app_log::warn(err.clone());
            Err(err)
        }
    }
}

#[tauri::command]
fn log_frontend_error(message: String) {
    app_log::warn(format!("frontend error: {}", message));
}

#[tauri::command]
fn log_frontend_event(message: String) {
    app_log::info(format!("frontend event: {}", message));
}

#[tauri::command]
fn get_usage_stats() -> StatsSnapshot {
    stats::load_stats_snapshot()
}

#[tauri::command]
async fn check_for_update() -> Result<update::UpdateStatus, String> {
    app_log::info("开始检查软件更新。");
    let loaded = config::load_config()?;
    update::check_for_update(&loaded.data.update)
        .await
        .map_err(|err| {
            app_log::warn(format!("软件更新检查失败: {}", err));
            err
        })
}

#[tauri::command]
async fn download_and_install_update() -> Result<update::InstallUpdateResult, String> {
    app_log::info("开始下载并安装软件更新。");
    let loaded = config::load_config()?;
    update::download_and_install(&loaded.data.update)
        .await
        .map_err(|err| {
            app_log::warn(format!("下载并安装软件更新失败: {}", err));
            err
        })
}

#[tauri::command]
fn get_overlay_text() -> overlay::OverlayText {
    overlay::OverlayText {
        text: overlay::current_text(),
    }
}

#[tauri::command]
fn list_audio_input_devices() -> Result<Vec<audio::AudioDeviceInfo>, String> {
    audio::list_input_devices()
}

#[tauri::command]
fn get_session_state(session: State<'_, SessionController>) -> session::SessionState {
    session.current_state()
}

#[tauri::command]
fn start_recording(
    app: AppHandle,
    session: State<'_, SessionController>,
) -> Result<session::SessionState, String> {
    session.start(Some(app))
}

#[tauri::command]
fn stop_recording(
    app: AppHandle,
    session: State<'_, SessionController>,
) -> Result<session::SessionState, String> {
    session.stop(Some(app))
}

#[tauri::command]
fn toggle_recording(
    app: AppHandle,
    session: State<'_, SessionController>,
) -> Result<session::SessionState, String> {
    session.toggle(Some(app))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    std::panic::set_hook(Box::new(|panic_info| {
        let location = panic_info
            .location()
            .map(|loc| format!("{}:{}", loc.file(), loc.line()))
            .unwrap_or_else(|| "unknown location".to_string());
        let payload = panic_info
            .payload()
            .downcast_ref::<&str>()
            .map(|value| (*value).to_string())
            .or_else(|| {
                panic_info
                    .payload()
                    .downcast_ref::<String>()
                    .map(|value| value.to_string())
            })
            .unwrap_or_else(|| "unknown panic payload".to_string());
        app_log::warn(format!("panic at {}: {}", location, payload));
    }));

    if let Err(err) = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            app_log::info("检测到重复启动，已唤起现有主窗口。");
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .manage(SessionController::default())
        .invoke_handler(tauri::generate_handler![
            get_app_snapshot,
            get_setup_status,
            load_app_config,
            save_app_config,
            test_asr_config,
            test_llm_config,
            open_setup_guide,
            open_log_file,
            log_frontend_error,
            log_frontend_event,
            get_usage_stats,
            check_for_update,
            download_and_install_update,
            get_overlay_text,
            list_audio_input_devices,
            get_session_state,
            start_recording,
            stop_recording,
            toggle_recording
        ])
        .setup(|app| {
            app_log::info(format!(
                "VoxType Tauri client started. version={}",
                env!("CARGO_PKG_VERSION")
            ));
            app_log::info("startup stage: create overlay begin");
            let _ = overlay::create_overlay_window(app.handle());
            app_log::info("startup stage: create overlay done");
            app_log::info("startup stage: setup tray begin");
            if let Err(err) = tray::setup_tray(app.handle()) {
                app_log::warn(err);
            }
            app_log::info("startup stage: setup tray done");
            app_log::info("startup stage: startup message begin");
            tray::show_startup_message();
            app_log::info("startup stage: startup message done");
            app_log::info("startup stage: setup guide check begin");
            setup_guide::open_if_config_missing(app.handle());
            app_log::info("startup stage: setup guide check done");
            if let Ok(loaded) = config::load_config() {
                if let Err(err) = autostart::apply(&loaded.data.startup) {
                    app_log::warn(format!("启动时同步开机自启动失败: {}", err));
                }
            }
            app_log::info("startup stage: global hotkey thread start");
            hotkey::start_global_hotkey_thread(app.handle().clone());
            app_log::info("startup stage: input hook thread start");
            hotkey::start_input_hook_thread(app.handle().clone());
            app_log::info("startup stage: setup complete");
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }

            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                if let Err(err) = window.hide() {
                    app_log::warn(format!("隐藏主窗口失败: {}", err));
                } else {
                    app_log::info("主窗口已隐藏到托盘。");
                }
            }
        })
        .run(tauri::generate_context!())
    {
        app_log::warn(format!("Tauri application exited with error: {}", err));
    }
}
