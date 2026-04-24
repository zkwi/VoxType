use crate::session::SessionController;
use crate::{app_log, config};
use std::{thread, time::Duration};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_opener::OpenerExt;

const OPEN_CONFIG_ID: &str = "open_config";
const EXIT_ID: &str = "exit";
const STARTUP_TOAST_LABEL: &str = "startup-toast";

pub fn setup_tray(app: &AppHandle) -> Result<(), String> {
    let open_config = MenuItem::with_id(app, OPEN_CONFIG_ID, "打开配置文件", true, None::<&str>)
        .map_err(|err| format!("创建托盘菜单失败: {}", err))?;
    let separator = PredefinedMenuItem::separator(app)
        .map_err(|err| format!("创建托盘菜单分隔线失败: {}", err))?;
    let exit = MenuItem::with_id(app, EXIT_ID, "退出", true, None::<&str>)
        .map_err(|err| format!("创建托盘菜单失败: {}", err))?;
    let menu = Menu::with_items(app, &[&open_config, &separator, &exit])
        .map_err(|err| format!("创建托盘菜单失败: {}", err))?;

    let app_for_event = app.clone();
    let mut builder = TrayIconBuilder::with_id("asr-ime")
        .tooltip("ASR语音输入")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            OPEN_CONFIG_ID => open_config_file(app),
            EXIT_ID => {
                crate::hotkey::stop_input_threads();
                let controller = app.state::<SessionController>().inner().clone();
                controller.abort_from_worker(app, "Application exiting.");
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(move |_tray, event| {
            if let TrayIconEvent::DoubleClick {
                button: MouseButton::Left,
                ..
            } = event
            {
                show_main_window(&app_for_event);
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder
        .build(app)
        .map_err(|err| format!("创建托盘图标失败: {}", err))?;
    Ok(())
}

pub fn show_startup_message(app: &AppHandle) {
    let Ok(loaded) = config::load_config() else {
        return;
    };
    if !loaded.data.tray.show_startup_message {
        return;
    }
    // Tauri v2 未内置 Windows 气泡通知；这里先写日志，后续可换成 notification 插件。
    app_log::info(format!(
        "ASR语音输入已启动，按 {} / 右Alt / 鼠标中键 开始/停止语音输入",
        loaded.data.hotkey.to_uppercase()
    ));
    show_startup_toast(
        app,
        &loaded.data.hotkey,
        loaded.data.tray.startup_message_timeout_ms,
    );
}

fn open_config_file(app: &AppHandle) {
    match config::load_config() {
        Ok(loaded) => {
            if let Err(err) = app.opener().open_path(loaded.path, None::<&str>) {
                app_log::warn(format!("打开配置文件失败: {}", err));
            }
        }
        Err(err) => app_log::warn(format!("读取配置文件路径失败: {}", err)),
    }
}

fn show_main_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    if let Err(err) = window.unminimize() {
        app_log::warn(format!("恢复主窗口失败: {}", err));
    }
    if let Err(err) = window.show() {
        app_log::warn(format!("显示主窗口失败: {}", err));
    }
    if let Err(err) = window.set_focus() {
        app_log::warn(format!("聚焦主窗口失败: {}", err));
    }
}

fn show_startup_toast(app: &AppHandle, hotkey: &str, timeout_ms: u64) {
    if let Some(window) = app.get_webview_window(STARTUP_TOAST_LABEL) {
        let _ = window.close();
    }

    let url = format!("/?toast=1&hotkey={}", encode_query_component(hotkey));
    let window =
        match WebviewWindowBuilder::new(app, STARTUP_TOAST_LABEL, WebviewUrl::App(url.into()))
            .title("ASR Startup")
            .inner_size(342.0, 70.0)
            .resizable(false)
            .decorations(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .transparent(true)
            .visible(false)
            .build()
        {
            Ok(window) => window,
            Err(err) => {
                app_log::warn(format!("创建启动提示窗口失败: {}", err));
                return;
            }
        };

    let width = 342;
    let height = 70;
    let _ = window.set_size(PhysicalSize::new(width, height));
    if let Ok(Some(monitor)) = window.primary_monitor() {
        let position = monitor.position();
        let size = monitor.size();
        let x = position.x + size.width.saturating_sub(width + 22) as i32;
        let y = position.y + size.height.saturating_sub(height + 48) as i32;
        let _ = window.set_position(PhysicalPosition::new(x, y));
    }
    let _ = window.show();

    let app = app.clone();
    let timeout_ms = timeout_ms.max(1000);
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(timeout_ms));
        if let Some(window) = app.get_webview_window(STARTUP_TOAST_LABEL) {
            let _ = window.close();
        }
    });
}

fn encode_query_component(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                vec![byte as char]
            }
            other => format!("%{other:02X}").chars().collect(),
        })
        .collect()
}
