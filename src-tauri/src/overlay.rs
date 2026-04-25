use crate::config::UiConfig;
use serde::Serialize;
use std::sync::{Mutex, OnceLock};
use tauri::{
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, Monitor, WebviewUrl,
    WebviewWindowBuilder,
};
use windows::Win32::Foundation::POINT;
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

const OVERLAY_LABEL: &str = "caption-overlay";
pub const RECORDING_TEXT: &str = "正在听你说话...";
pub const STOPPING_TEXT: &str = "正在收尾...";
pub const WAITING_FINAL_TEXT: &str = "正在等待最后一句识别完成...";
pub const POST_EDITING_TEXT: &str = "正在润色...";
pub const PASTING_TEXT: &str = "正在粘贴...";
pub const PASTED_TEXT: &str = "已粘贴";
pub const PASTE_FAILED_TEXT: &str = "粘贴失败，文本已复制，可手动 Ctrl+V。";
const DEFAULT_TEXT: &str = RECORDING_TEXT;
static OVERLAY_TEXT: OnceLock<Mutex<String>> = OnceLock::new();

#[derive(Debug, Clone, Serialize)]
pub struct OverlayText {
    pub text: String,
}

pub fn create_overlay_window(app: &AppHandle) -> Result<(), String> {
    if app.get_webview_window(OVERLAY_LABEL).is_some() {
        return Ok(());
    }
    WebviewWindowBuilder::new(app, OVERLAY_LABEL, WebviewUrl::App("/?overlay=1".into()))
        .title("ASR Caption")
        .inner_size(350.0, 64.0)
        .resizable(false)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .transparent(false)
        .visible(false)
        .build()
        .map_err(|err| format!("创建悬浮字幕窗失败: {}", err))?;
    crate::app_log::info("悬浮字幕窗已创建");
    Ok(())
}

pub fn show_for_recording(app: &AppHandle, ui: &UiConfig) {
    if let Err(err) = create_overlay_window(app) {
        crate::app_log::warn(err);
        return;
    }
    let Some(window) = app.get_webview_window(OVERLAY_LABEL) else {
        return;
    };
    let _ = window.set_size(LogicalSize::new(ui.width as f64, ui.height as f64));
    if let Some(monitor) = current_monitor(app).or_else(|| window.primary_monitor().ok().flatten())
    {
        let position = monitor.position();
        let size = monitor.size();
        let scale = monitor.scale_factor().max(1.0);
        let monitor_x = position.x as f64 / scale;
        let monitor_y = position.y as f64 / scale;
        let monitor_width = size.width as f64 / scale;
        let monitor_height = size.height as f64 / scale;
        let x = monitor_x + ((monitor_width - ui.width as f64).max(0.0) / 2.0);
        let y = monitor_y + (monitor_height - ui.height as f64 - ui.margin_bottom as f64).max(0.0);
        let _ = window.set_position(LogicalPosition::new(x, y));
    }
    update_text(app, DEFAULT_TEXT);
    if let Err(err) = window.show() {
        crate::app_log::warn(format!("显示悬浮字幕窗失败: {}", err));
    } else {
        crate::app_log::info("悬浮字幕窗已显示");
    }
}

fn current_monitor(app: &AppHandle) -> Option<Monitor> {
    let cursor = cursor_position()?;
    let monitors = app.available_monitors().ok()?;
    monitors.into_iter().find(|monitor| {
        let position = monitor.position();
        let size = monitor.size();
        let left = position.x;
        let top = position.y;
        let right = left + size.width as i32;
        let bottom = top + size.height as i32;
        cursor.x >= left && cursor.x < right && cursor.y >= top && cursor.y < bottom
    })
}

fn cursor_position() -> Option<POINT> {
    let mut point = POINT::default();
    unsafe { GetCursorPos(&mut point).ok()? };
    Some(point)
}

pub fn update_text(app: &AppHandle, text: impl Into<String>) {
    let payload = OverlayText { text: text.into() };
    set_current_text(payload.text.clone());
    if let Some(window) = app.get_webview_window(OVERLAY_LABEL) {
        let _ = window.emit("overlay-text", payload.clone());
    }
    let _ = app.emit_to(OVERLAY_LABEL, "overlay-text", payload.clone());
    let _ = app.emit("overlay-text", payload);
}

pub fn hide(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(OVERLAY_LABEL) {
        let _ = window.hide();
    }
}

pub fn current_text() -> String {
    OVERLAY_TEXT
        .get_or_init(|| Mutex::new(DEFAULT_TEXT.to_string()))
        .lock()
        .map(|text| text.clone())
        .unwrap_or_else(|_| DEFAULT_TEXT.to_string())
}

fn set_current_text(text: String) {
    if let Ok(mut current) = OVERLAY_TEXT
        .get_or_init(|| Mutex::new(DEFAULT_TEXT.to_string()))
        .lock()
    {
        *current = if text.trim().is_empty() {
            DEFAULT_TEXT.to_string()
        } else {
            text
        };
    }
}
