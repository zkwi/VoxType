use crate::session::SessionController;
use crate::{app_log, config, main_window};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, TrayIconBuilder, TrayIconEvent};
use tauri::{image::Image, AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;

const TRAY_ID: &str = "voxtype";
const TRAY_TOOLTIP_IDLE: &str = "声写";
const TRAY_TOOLTIP_INPUT: &str = "声写 · 输入中";
const TRAY_ICON_SIZE: usize = 32;
const TRAY_ICON_RGBA: &[u8] = include_bytes!("../icons/32x32.rgba");

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
    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip(TRAY_TOOLTIP_IDLE)
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
    let icon = normal_tray_icon();
    builder = builder.icon(icon);
    builder
        .build(app)
        .map_err(|err| format!("创建托盘图标失败: {}", err))?;
    Ok(())
}

pub fn set_input_active(app: &AppHandle, active: bool) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    let icon = if active {
        active_tray_icon()
    } else {
        normal_tray_icon()
    };
    if let Err(err) = tray.set_icon(Some(icon)) {
        app_log::warn(format!("更新托盘图标失败: {}", err));
    }
    let tooltip = if active {
        TRAY_TOOLTIP_INPUT
    } else {
        TRAY_TOOLTIP_IDLE
    };
    if let Err(err) = tray.set_tooltip(Some(tooltip)) {
        app_log::warn(format!("更新托盘提示失败: {}", err));
    }
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
    main_window::show_centered(app, "托盘菜单");
}

fn normal_tray_icon() -> Image<'static> {
    Image::new(TRAY_ICON_RGBA, TRAY_ICON_SIZE as u32, TRAY_ICON_SIZE as u32)
}

fn active_tray_icon() -> Image<'static> {
    let mut rgba = TRAY_ICON_RGBA.to_vec();
    paint_status_dot(&mut rgba);
    Image::new_owned(rgba, TRAY_ICON_SIZE as u32, TRAY_ICON_SIZE as u32)
}

fn paint_status_dot(rgba: &mut [u8]) {
    let center_x = 23_i32;
    let center_y = 23_i32;
    let border_radius_sq = 81_i32;
    let dot_radius_sq = 49_i32;
    for y in 0..TRAY_ICON_SIZE {
        for x in 0..TRAY_ICON_SIZE {
            let dx = x as i32 - center_x;
            let dy = y as i32 - center_y;
            let distance_sq = dx * dx + dy * dy;
            if distance_sq > border_radius_sq {
                continue;
            }
            let offset = (y * TRAY_ICON_SIZE + x) * 4;
            let color = if distance_sq <= dot_radius_sq {
                [239, 68, 68, 255]
            } else {
                [255, 255, 255, 255]
            };
            rgba[offset..offset + 4].copy_from_slice(&color);
        }
    }
}
