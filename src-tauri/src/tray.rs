use crate::session::SessionController;
use crate::{app_log, config, main_window};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, TrayIconBuilder, TrayIconEvent};
use tauri::{image::Image, AppHandle, Emitter, Manager};
use tauri_plugin_opener::OpenerExt;
use windows::core::BOOL;
use windows::Win32::Foundation::{HWND, LPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{EnumThreadWindows, GetClassNameW, KillTimer};

const TRAY_ID: &str = "voxtype";
/// 托盘库 tray-icon 放托盘图标的隐藏窗口的窗口类名，以及它检测"鼠标离开图标"的定时器编号。
/// 两者都是该库的内部常量（按 tray-icon 0.24.1 和 2026-10 的上游 dev 分支核对），
/// 升级 Tauri 带动 tray-icon 变化后需要复核；对不上时下面的清理只是不起作用，不会误伤别的窗口。
const TRAY_LIBRARY_WINDOW_CLASS: &str = "tray_icon_app";
const TRAY_LIBRARY_LEAVE_TIMER_ID: usize = 6008;
/// 托盘图标安静这么久之后才清理残留定时器：鼠标还在图标上移动时，库自己的离开检测照常工作。
const TRAY_QUIET_BEFORE_TIMER_CLEANUP: Duration = Duration::from_secs(2);
static TRAY_LAST_EVENT: Mutex<Option<Instant>> = Mutex::new(None);
static TRAY_TIMER_CLEANUP_PENDING: AtomicBool = AtomicBool::new(false);
const TRAY_ICON_SIZE: usize = 32;
const TRAY_ICON_RGBA: &[u8] = include_bytes!("../icons/32x32.rgba");

const OPEN_CONFIG_ID: &str = "open_config";
const OPEN_LOG_ID: &str = "open_log";
const OPEN_SETUP_GUIDE_ID: &str = "open_setup_guide";
const OPEN_ISSUES_ID: &str = "open_issues";
const CHECK_UPDATE_ID: &str = "check_update";
const RESTART_ID: &str = "restart";
const EXIT_ID: &str = "exit";
const CHECK_UPDATE_EVENT: &str = "check-update-requested";
const ISSUES_URL: &str = "https://github.com/zkwi/VoxType/issues/new/choose";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TrayLabels {
    open_config: &'static str,
    open_log: &'static str,
    open_setup_guide: &'static str,
    open_issues: &'static str,
    check_update: &'static str,
    restart: &'static str,
    exit: &'static str,
}

pub fn setup_tray(app: &AppHandle) -> Result<(), String> {
    let language = current_tray_language();
    let menu = build_tray_menu(app, tray_labels(language))?;

    let app_for_event = app.clone();
    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip(tray_tooltip(language, false))
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
            OPEN_ISSUES_ID => {
                if let Err(err) = open_issues_page(app) {
                    app_log::warn(err);
                }
            }
            CHECK_UPDATE_ID => request_update_check(app),
            RESTART_ID => restart_app(app),
            EXIT_ID => request_exit(app),
            _ => {}
        })
        .on_tray_icon_event(move |_tray, event| {
            schedule_tray_timer_cleanup(&app_for_event);
            match event {
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    ..
                }
                | TrayIconEvent::DoubleClick {
                    button: MouseButton::Left,
                    ..
                } => show_main_window(&app_for_event),
                _ => {}
            }
        });
    let icon = normal_tray_icon();
    builder = builder.icon(icon);
    builder
        .build(app)
        .map_err(|err| format!("创建托盘图标失败: {}", err))?;
    Ok(())
}

pub fn set_language(app: &AppHandle, language: &str) -> Result<(), String> {
    let language = normalize_tray_language(language);
    if let Ok(mut current) = tray_language().lock() {
        *current = language;
    }

    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return Ok(());
    };
    let menu = build_tray_menu(app, tray_labels(language))?;
    tray.set_menu(Some(menu))
        .map_err(|err| format!("更新托盘菜单语言失败: {}", err))?;
    tray.set_tooltip(Some(tray_tooltip(language, current_tray_active())))
        .map_err(|err| format!("更新托盘提示语言失败: {}", err))?;
    Ok(())
}

fn build_tray_menu(app: &AppHandle, labels: TrayLabels) -> Result<Menu<tauri::Wry>, String> {
    let open_config =
        MenuItem::with_id(app, OPEN_CONFIG_ID, labels.open_config, true, None::<&str>)
            .map_err(|err| format!("创建托盘菜单失败: {}", err))?;
    let open_log = MenuItem::with_id(app, OPEN_LOG_ID, labels.open_log, true, None::<&str>)
        .map_err(|err| format!("创建托盘菜单失败: {}", err))?;
    let open_setup_guide = MenuItem::with_id(
        app,
        OPEN_SETUP_GUIDE_ID,
        labels.open_setup_guide,
        true,
        None::<&str>,
    )
    .map_err(|err| format!("创建托盘菜单失败: {}", err))?;
    let open_issues =
        MenuItem::with_id(app, OPEN_ISSUES_ID, labels.open_issues, true, None::<&str>)
            .map_err(|err| format!("创建托盘菜单失败: {}", err))?;
    let check_update = MenuItem::with_id(
        app,
        CHECK_UPDATE_ID,
        labels.check_update,
        true,
        None::<&str>,
    )
    .map_err(|err| format!("创建托盘菜单失败: {}", err))?;
    let restart = MenuItem::with_id(app, RESTART_ID, labels.restart, true, None::<&str>)
        .map_err(|err| format!("创建托盘菜单失败: {}", err))?;
    let separator = PredefinedMenuItem::separator(app)
        .map_err(|err| format!("创建托盘菜单分隔线失败: {}", err))?;
    let exit = MenuItem::with_id(app, EXIT_ID, labels.exit, true, None::<&str>)
        .map_err(|err| format!("创建托盘菜单失败: {}", err))?;
    Menu::with_items(
        app,
        &[
            &open_config,
            &open_log,
            &open_setup_guide,
            &open_issues,
            &check_update,
            &restart,
            &separator,
            &exit,
        ],
    )
    .map_err(|err| format!("创建托盘菜单失败: {}", err))
}

pub fn set_input_active(app: &AppHandle, active: bool) {
    if let Ok(mut current) = tray_active().lock() {
        *current = active;
    }
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
    let tooltip = tray_tooltip(current_tray_language(), active);
    if let Err(err) = tray.set_tooltip(Some(tooltip)) {
        app_log::warn(format!("更新托盘提示失败: {}", err));
    }
}

fn tray_labels(language: &str) -> TrayLabels {
    match normalize_tray_language(language) {
        "en" => TrayLabels {
            open_config: "Open config file",
            open_log: "View logs",
            open_setup_guide: "Setup guide",
            open_issues: "Report issue",
            check_update: "Check updates",
            restart: "Restart app",
            exit: "Exit",
        },
        "zh-TW" => TrayLabels {
            open_config: "打開配置檔",
            open_log: "查看日誌",
            open_setup_guide: "配置指南",
            open_issues: "問題回報",
            check_update: "檢查更新",
            restart: "重新啟動程式",
            exit: "退出",
        },
        _ => TrayLabels {
            open_config: "打开配置文件",
            open_log: "查看日志",
            open_setup_guide: "配置指南",
            open_issues: "问题反馈",
            check_update: "检查更新",
            restart: "重启程序",
            exit: "退出",
        },
    }
}

fn tray_tooltip(language: &str, active: bool) -> &'static str {
    match (normalize_tray_language(language), active) {
        ("en", true) => "VoxType · Listening",
        ("en", false) => "VoxType",
        ("zh-TW", true) => "聲寫 · 輸入中",
        ("zh-TW", false) => "聲寫",
        (_, true) => "声写 · 输入中",
        (_, false) => "声写",
    }
}

fn normalize_tray_language(language: &str) -> &'static str {
    match language {
        "en" => "en",
        "zh-TW" => "zh-TW",
        _ => "zh-CN",
    }
}

fn current_tray_language() -> &'static str {
    tray_language()
        .lock()
        .map(|language| *language)
        .unwrap_or("zh-CN")
}

fn current_tray_active() -> bool {
    tray_active().lock().map(|active| *active).unwrap_or(false)
}

fn tray_language() -> &'static Mutex<&'static str> {
    static TRAY_LANGUAGE: OnceLock<Mutex<&'static str>> = OnceLock::new();
    TRAY_LANGUAGE.get_or_init(|| Mutex::new("zh-CN"))
}

fn tray_active() -> &'static Mutex<bool> {
    static TRAY_ACTIVE: OnceLock<Mutex<bool>> = OnceLock::new();
    TRAY_ACTIVE.get_or_init(|| Mutex::new(false))
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

fn request_exit(app: &AppHandle) {
    if main_window::request_config_exit_guard(app, "exit") {
        app_log::info("配置保存失败，已在退出前请求用户确认。");
        return;
    }
    exit_app(app);
}

fn restart_app(app: &AppHandle) {
    app_log::info("用户从托盘菜单重启程序。");
    crate::hotkey::stop_input_threads();
    let controller = app.state::<SessionController>().inner().clone();
    controller.abort_from_worker(app, "Application restarting.");
    main_window::mark_user_restart();
    app.request_restart();
}

fn open_issues_page(app: &AppHandle) -> Result<(), String> {
    app_log::info("用户从托盘菜单打开问题反馈页面。");
    app.opener()
        .open_url(ISSUES_URL, None::<&str>)
        .map_err(|err| format!("打开问题反馈页面失败: {}", err))
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
    main_window::show_existing(app, "托盘菜单");
}

fn request_update_check(app: &AppHandle) {
    show_main_window(app);
    if let Err(err) = app.emit(CHECK_UPDATE_EVENT, ()) {
        app_log::warn(format!("发送托盘检查更新事件失败: {}", err));
    }
}

/// 托盘图标安静一段时间后，替托盘库停掉它可能残留的"鼠标离开检测"定时器。
///
/// tray-icon 在鼠标滑过图标时启动一个 15ms 定时器，用来判断鼠标何时离开。它只在"最近一次移动之后
/// 的第一个节拍"检查一次位置：如果那一刻鼠标还停在图标上（比如正在点击），之后鼠标没有在图标范围内
/// 再移动就离开了，这个定时器就再也不会停。主线程因此每秒被唤醒约 64 次，常驻约 0.75% 单核，
/// 直到进程退出；实测从托盘打开过主窗口之后应用就处于这个状态。排查记录见 0.14.0 发布审计。
///
/// 停掉它是安全的：定时器只在鼠标移动后的下一个节拍有用，鼠标再次滑过图标时库会重新启动它。
fn schedule_tray_timer_cleanup(app: &AppHandle) {
    if let Ok(mut last_event) = TRAY_LAST_EVENT.lock() {
        *last_event = Some(Instant::now());
    }
    if TRAY_TIMER_CLEANUP_PENDING.swap(true, Ordering::AcqRel) {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        loop {
            let since_last_event = TRAY_LAST_EVENT
                .lock()
                .ok()
                .and_then(|last_event| *last_event)
                .map(|at| at.elapsed());
            match remaining_tray_quiet_time(since_last_event, TRAY_QUIET_BEFORE_TIMER_CLEANUP) {
                Some(wait) => std::thread::sleep(wait),
                None => break,
            }
        }
        TRAY_TIMER_CLEANUP_PENDING.store(false, Ordering::Release);
        // 定时器属于创建托盘窗口的主线程，只能在主线程上停。
        if let Err(err) = app.run_on_main_thread(|| {
            let stopped = stop_timer_on_current_thread_windows(
                TRAY_LIBRARY_WINDOW_CLASS,
                TRAY_LIBRARY_LEAVE_TIMER_ID,
            );
            if stopped > 0 {
                app_log::info("已停止托盘图标残留的鼠标离开检测定时器。");
            }
        }) {
            app_log::warn(format!("清理托盘定时器失败: {}", err));
        }
    });
}

/// 距离"托盘图标已经安静够久"还差多久；`None` 表示现在就可以清理。
fn remaining_tray_quiet_time(
    since_last_event: Option<Duration>,
    quiet: Duration,
) -> Option<Duration> {
    let elapsed = since_last_event?;
    (elapsed < quiet).then(|| quiet - elapsed)
}

/// 在当前线程创建的顶层窗口里，找出窗口类名匹配的那些并停掉指定编号的定时器；返回实际停掉的个数。
fn stop_timer_on_current_thread_windows(class_name: &str, timer_id: usize) -> usize {
    struct Request<'a> {
        class_name: &'a str,
        timer_id: usize,
        stopped: usize,
    }

    unsafe extern "system" fn visit(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let request = unsafe { &mut *(lparam.0 as *mut Request<'_>) };
        let mut buffer = [0u16; 64];
        let length = unsafe { GetClassNameW(hwnd, &mut buffer) }.max(0) as usize;
        if String::from_utf16_lossy(&buffer[..length]) == request.class_name
            && unsafe { KillTimer(Some(hwnd), request.timer_id) }.is_ok()
        {
            request.stopped += 1;
        }
        BOOL(1)
    }

    let mut request = Request {
        class_name,
        timer_id,
        stopped: 0,
    };
    unsafe {
        let _ = EnumThreadWindows(
            GetCurrentThreadId(),
            Some(visit),
            LPARAM(&mut request as *mut Request<'_> as isize),
        );
    }
    request.stopped
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
    let center_x = 24_i32;
    let center_y = 24_i32;
    let border_radius_sq = 64_i32;
    let dot_radius_sq = 36_i32;
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
                [34, 197, 94, 255]
            } else {
                [255, 255, 255, 255]
            };
            rgba[offset..offset + 4].copy_from_slice(&color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        remaining_tray_quiet_time, stop_timer_on_current_thread_windows, tray_labels, tray_tooltip,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{Duration, Instant};
    use windows::core::w;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, DispatchMessageW, GetClassNameW, PeekMessageW, SetTimer,
        MSG, PM_REMOVE, WINDOW_EX_STYLE, WINDOW_STYLE,
    };

    #[test]
    fn tray_timer_cleanup_waits_for_the_icon_to_go_quiet() {
        let quiet = Duration::from_secs(2);
        // 刚收到过托盘事件：等满剩下的时间再清理。
        assert_eq!(
            remaining_tray_quiet_time(Some(Duration::from_millis(500)), quiet),
            Some(Duration::from_millis(1500))
        );
        assert_eq!(remaining_tray_quiet_time(Some(quiet), quiet), None);
        assert_eq!(
            remaining_tray_quiet_time(Some(Duration::from_secs(5)), quiet),
            None
        );
        assert_eq!(remaining_tray_quiet_time(None, quiet), None);
    }

    static TEST_TIMER_TICKS: AtomicUsize = AtomicUsize::new(0);

    unsafe extern "system" fn count_tick(_: HWND, _: u32, _: usize, _: u32) {
        TEST_TIMER_TICKS.fetch_add(1, Ordering::SeqCst);
    }

    fn pump_messages_for(duration: Duration) {
        let deadline = Instant::now() + duration;
        let mut message = MSG::default();
        while Instant::now() < deadline {
            while unsafe { PeekMessageW(&mut message, None, 0, 0, PM_REMOVE) }.as_bool() {
                unsafe { DispatchMessageW(&message) };
            }
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    #[test]
    fn stops_a_running_timer_on_matching_windows_of_this_thread() {
        const TIMER_ID: usize = 4242;
        // 隐藏的顶层窗口，和托盘库的托盘窗口是同一种形态（不是 message-only，线程窗口枚举能找到）。
        let hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("STATIC"),
                w!(""),
                WINDOW_STYLE(0),
                0,
                0,
                0,
                0,
                None,
                None,
                None,
                None,
            )
        }
        .expect("create hidden window");
        let mut buffer = [0u16; 64];
        let length = unsafe { GetClassNameW(hwnd, &mut buffer) } as usize;
        let class_name = String::from_utf16_lossy(&buffer[..length]);

        assert_ne!(
            unsafe { SetTimer(Some(hwnd), TIMER_ID, 15, Some(count_tick)) },
            0
        );
        pump_messages_for(Duration::from_millis(150));
        assert!(
            TEST_TIMER_TICKS.load(Ordering::SeqCst) > 0,
            "timer should be ticking before the cleanup"
        );

        // 窗口类名对不上的窗口不受影响。
        assert_eq!(
            stop_timer_on_current_thread_windows("no_such_class", TIMER_ID),
            0
        );
        assert_eq!(
            stop_timer_on_current_thread_windows(&class_name, TIMER_ID),
            1
        );

        TEST_TIMER_TICKS.store(0, Ordering::SeqCst);
        pump_messages_for(Duration::from_millis(150));
        assert_eq!(TEST_TIMER_TICKS.load(Ordering::SeqCst), 0);
        // 定时器已经不在时再清理一次，什么都不做。
        assert_eq!(
            stop_timer_on_current_thread_windows(&class_name, TIMER_ID),
            0
        );

        unsafe { DestroyWindow(hwnd) }.expect("destroy window");
    }

    #[test]
    fn tray_labels_follow_selected_language() {
        let labels = tray_labels("en");

        assert_eq!(labels.open_config, "Open config file");
        assert_eq!(labels.open_log, "View logs");
        assert_eq!(labels.open_setup_guide, "Setup guide");
        assert_eq!(labels.open_issues, "Report issue");
        assert_eq!(labels.check_update, "Check updates");
        assert_eq!(labels.restart, "Restart app");
        assert_eq!(labels.exit, "Exit");
    }

    #[test]
    fn tray_labels_support_traditional_chinese() {
        let labels = tray_labels("zh-TW");

        assert_eq!(labels.open_config, "打開配置檔");
        assert_eq!(labels.open_log, "查看日誌");
        assert_eq!(labels.open_setup_guide, "配置指南");
        assert_eq!(labels.open_issues, "問題回報");
        assert_eq!(labels.check_update, "檢查更新");
        assert_eq!(labels.restart, "重新啟動程式");
        assert_eq!(labels.exit, "退出");
    }

    #[test]
    fn tray_language_falls_back_to_simplified_chinese() {
        let labels = tray_labels("fr");

        assert_eq!(labels.open_config, "打开配置文件");
        assert_eq!(labels.open_issues, "问题反馈");
        assert_eq!(labels.restart, "重启程序");
        assert_eq!(tray_tooltip("fr", false), "声写");
        assert_eq!(tray_tooltip("en", true), "VoxType · Listening");
    }
}
