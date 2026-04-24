mod app_log;
mod asr;
mod asr_ws;
mod audio;
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

use config::{AppConfig, LoadedConfig};
use serde::Serialize;
use session::SessionController;
use stats::StatsSnapshot;
use tauri::{AppHandle, Manager, State, WindowEvent};

#[derive(Serialize)]
struct AppSnapshot {
    hotkey: String,
}

#[tauri::command]
fn get_app_snapshot() -> Result<AppSnapshot, String> {
    let loaded = config::load_config()?;

    Ok(AppSnapshot {
        hotkey: loaded.data.hotkey,
    })
}

#[tauri::command]
fn load_app_config() -> Result<LoadedConfig, String> {
    config::load_config()
}

#[tauri::command]
fn save_app_config(config: AppConfig) -> Result<LoadedConfig, String> {
    config::save_config(config)
}

#[tauri::command]
fn open_setup_guide(app: AppHandle) -> Result<(), String> {
    setup_guide::open(&app)
}

#[tauri::command]
fn log_frontend_error(message: String) {
    app_log::warn(format!("frontend error: {}", message));
}

#[tauri::command]
fn get_usage_stats() -> StatsSnapshot {
    stats::load_stats_snapshot()
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
            load_app_config,
            save_app_config,
            open_setup_guide,
            log_frontend_error,
            get_usage_stats,
            get_overlay_text,
            list_audio_input_devices,
            get_session_state,
            start_recording,
            stop_recording,
            toggle_recording
        ])
        .setup(|app| {
            app_log::info("VoxType Tauri client started.");
            let _ = overlay::create_overlay_window(app.handle());
            if let Err(err) = tray::setup_tray(app.handle()) {
                app_log::warn(err);
            }
            tray::show_startup_message(app.handle());
            setup_guide::open_if_config_missing(app.handle());
            hotkey::start_global_hotkey_thread(app.handle().clone());
            hotkey::start_input_hook_thread(app.handle().clone());
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
