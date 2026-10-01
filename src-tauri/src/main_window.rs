use crate::app_log;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, Manager};

pub const MAIN_LABEL: &str = "main";
/// 开机自启动注册表项携带的参数：带着它启动，说明是系统登录时拉起的，而不是用户手动打开。
pub const AUTOSTART_ARG: &str = "--autostart";
static CONFIG_EXIT_GUARD: AtomicBool = AtomicBool::new(false);

pub fn launched_by_autostart() -> bool {
    std::env::args().any(|arg| arg == AUTOSTART_ARG)
}

/// 开机自启动且已经可以直接使用时，主窗口留在托盘，不打断登录后的操作。
///
/// 尚未配置好时仍然显示主窗口，否则用户看不到需要先完成设置的提示。
pub fn should_start_hidden(launched_by_autostart: bool, setup_ready: bool) -> bool {
    launched_by_autostart && setup_ready
}

pub fn is_visible(app: &AppHandle) -> bool {
    app.get_webview_window(MAIN_LABEL)
        .and_then(|window| window.is_visible().ok())
        .unwrap_or(true)
}

pub fn set_config_exit_guard(active: bool) {
    CONFIG_EXIT_GUARD.store(active, Ordering::SeqCst);
}

pub fn config_exit_guard_active() -> bool {
    CONFIG_EXIT_GUARD.load(Ordering::SeqCst)
}

#[derive(Debug, Clone, Serialize)]
pub struct ConfigExitGuardRequest {
    pub action: String,
}

pub fn request_config_exit_guard(app: &AppHandle, action: &str) -> bool {
    if !config_exit_guard_active() {
        return false;
    }
    show_existing(app, "配置保存失败时");
    let Some(window) = app.get_webview_window(MAIN_LABEL) else {
        return false;
    };
    if let Err(err) = window.emit(
        "config-exit-guard-requested",
        ConfigExitGuardRequest {
            action: action.to_string(),
        },
    ) {
        app_log::warn(format!("发送配置退出保护事件失败: {}", err));
        return false;
    }
    true
}

pub fn show_existing(app: &AppHandle, source: &str) {
    let Some(window) = app.get_webview_window(MAIN_LABEL) else {
        app_log::warn(format!("{}显示主窗口失败：找不到主窗口。", source));
        return;
    };
    if let Err(err) = window.unminimize() {
        app_log::warn(format!("{}恢复主窗口失败: {}", source, err));
    }
    if let Err(err) = window.show() {
        app_log::warn(format!("{}显示主窗口失败: {}", source, err));
    }
    // 主窗口隐藏期间前端会暂停动画和电平更新，重新显示时要通知它恢复。
    let _ = window.emit("main-window-shown", ());
    if let Err(err) = window.set_focus() {
        app_log::warn(format!("{}聚焦主窗口失败: {}", source, err));
    }
}

#[cfg(test)]
mod tests {
    use super::{config_exit_guard_active, set_config_exit_guard, should_start_hidden};

    #[test]
    fn config_exit_guard_tracks_failed_unsaved_changes() {
        set_config_exit_guard(true);
        assert!(config_exit_guard_active());

        set_config_exit_guard(false);
        assert!(!config_exit_guard_active());
    }

    #[test]
    fn only_a_ready_autostart_launch_stays_in_the_tray() {
        assert!(should_start_hidden(true, true));
        // 手动打开时用户就是想看到主窗口。
        assert!(!should_start_hidden(false, true));
        // 还没配置好时必须显示主窗口，否则首次使用的提示无处可见。
        assert!(!should_start_hidden(true, false));
        assert!(!should_start_hidden(false, false));
    }
}
