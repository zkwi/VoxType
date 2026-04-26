use crate::app_log;
use tauri::{AppHandle, Manager, WebviewWindow};

pub const MAIN_LABEL: &str = "main";

pub fn center_existing(app: &AppHandle, source: &str) {
    let Some(window) = app.get_webview_window(MAIN_LABEL) else {
        app_log::warn(format!("{}居中主窗口失败：找不到主窗口。", source));
        return;
    };
    center_window(&window, source);
}

pub fn show_centered(app: &AppHandle, source: &str) {
    let Some(window) = app.get_webview_window(MAIN_LABEL) else {
        app_log::warn(format!("{}显示主窗口失败：找不到主窗口。", source));
        return;
    };
    if let Err(err) = window.unminimize() {
        app_log::warn(format!("{}恢复主窗口失败: {}", source, err));
    }
    center_window(&window, source);
    if let Err(err) = window.show() {
        app_log::warn(format!("{}显示主窗口失败: {}", source, err));
    }
    if let Err(err) = window.set_focus() {
        app_log::warn(format!("{}聚焦主窗口失败: {}", source, err));
    }
}

fn center_window(window: &WebviewWindow, source: &str) {
    if let Err(err) = window.center() {
        app_log::warn(format!("{}居中主窗口失败: {}", source, err));
    }
}
