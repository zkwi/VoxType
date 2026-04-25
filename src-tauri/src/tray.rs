use crate::session::SessionController;
use crate::{app_log, config};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, TrayIconBuilder, TrayIconEvent};
use tauri::{image::Image, AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;

const OPEN_CONFIG_ID: &str = "open_config";
const OPEN_LOG_ID: &str = "open_log";
const OPEN_SETUP_GUIDE_ID: &str = "open_setup_guide";
const EXIT_ID: &str = "exit";

pub fn setup_tray(app: &AppHandle) -> Result<(), String> {
    let open_config = MenuItem::with_id(app, OPEN_CONFIG_ID, "打开配置文件", true, None::<&str>)
        .map_err(|err| format!("创建托盘菜单失败: {}", err))?;
    let open_log = MenuItem::with_id(app, OPEN_LOG_ID, "查看日志", true, None::<&str>)
        .map_err(|err| format!("创建托盘菜单失败: {}", err))?;
    let open_setup_guide =
        MenuItem::with_id(app, OPEN_SETUP_GUIDE_ID, "配置指南", true, None::<&str>)
            .map_err(|err| format!("创建托盘菜单失败: {}", err))?;
    let separator = PredefinedMenuItem::separator(app)
        .map_err(|err| format!("创建托盘菜单分隔线失败: {}", err))?;
    let exit = MenuItem::with_id(app, EXIT_ID, "退出", true, None::<&str>)
        .map_err(|err| format!("创建托盘菜单失败: {}", err))?;
    let menu = Menu::with_items(
        app,
        &[
            &open_config,
            &open_log,
            &open_setup_guide,
            &separator,
            &exit,
        ],
    )
    .map_err(|err| format!("创建托盘菜单失败: {}", err))?;

    let app_for_event = app.clone();
    let mut builder = TrayIconBuilder::with_id("voxtype")
        .tooltip("声写")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            OPEN_CONFIG_ID => open_config_file(app),
            OPEN_LOG_ID => {
                if let Err(err) = open_log_file(app) {
                    app_log::warn(err);
                }
            }
            OPEN_SETUP_GUIDE_ID => {
                if let Err(err) = crate::setup_guide::open(app) {
                    app_log::warn(err);
                }
            }
            EXIT_ID => exit_app(app),
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
    let icon = Image::new(include_bytes!("../icons/32x32.rgba"), 32, 32);
    builder = builder.icon(icon);
    builder
        .build(app)
        .map_err(|err| format!("创建托盘图标失败: {}", err))?;
    Ok(())
}

pub fn show_startup_message() {
    let Ok(loaded) = config::load_config() else {
        return;
    };
    if !loaded.data.tray.show_startup_message {
        return;
    }
    // Tauri v2 未内置 Windows 气泡通知；这里先写日志，后续可换成 notification 插件。
    app_log::info(format!(
        "声写已启动，按 {} / 右Alt / 鼠标中键 开始/停止语音输入",
        loaded.data.hotkey.to_uppercase()
    ));
}

fn open_config_file(app: &AppHandle) {
    match config::load_config() {
        Ok(loaded) => {
            let path = loaded.path.clone();
            if !loaded.exists {
                match config::save_config(loaded.data) {
                    Ok(created) => app_log::info(format!("已创建默认配置文件: {}", created.path)),
                    Err(err) => {
                        app_log::warn(format!("创建默认配置文件失败: {}", err));
                        return;
                    }
                }
            }
            if let Err(err) = app.opener().open_path(path, None::<&str>) {
                app_log::warn(format!("打开配置文件失败: {}", err));
            }
        }
        Err(err) => app_log::warn(format!("读取配置文件路径失败: {}", err)),
    }
}

pub fn open_log_file(app: &AppHandle) -> Result<(), String> {
    open_log_file_with_source(app, "托盘")
}

pub fn open_log_file_from_main(app: &AppHandle) -> Result<(), String> {
    open_log_file_with_source(app, "主窗口")
}

pub fn exit_app(app: &AppHandle) {
    crate::hotkey::stop_input_threads();
    let controller = app.state::<SessionController>().inner().clone();
    controller.abort_from_worker(app, "Application exiting.");
    app.exit(0);
}

fn open_log_file_with_source(app: &AppHandle, source: &str) -> Result<(), String> {
    app_log::info(format!("用户从{}打开日志文件。", source));
    let path = app_log::log_path();
    app.opener()
        .open_path(path.to_string_lossy().to_string(), None::<&str>)
        .map_err(|err| format!("打开日志文件失败: {}", err))?;
    Ok(())
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
