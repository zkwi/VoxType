use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Mutex, OnceLock};
use std::thread;
use tauri::{AppHandle, Manager};
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN,
    VIRTUAL_KEY, VK_A, VK_B, VK_C, VK_D, VK_E, VK_F, VK_G, VK_H, VK_I, VK_J, VK_K, VK_L, VK_M,
    VK_N, VK_O, VK_P, VK_Q, VK_R, VK_RMENU, VK_S, VK_T, VK_U, VK_V, VK_W, VK_X, VK_Y, VK_Z,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, PostThreadMessageW, SetWindowsHookExW,
    TranslateMessage, UnhookWindowsHookEx, HC_ACTION, KBDLLHOOKSTRUCT, LLKHF_INJECTED,
    LLMHF_INJECTED, MSG, MSLLHOOKSTRUCT, WH_KEYBOARD_LL, WH_MOUSE_LL, WM_HOTKEY, WM_KEYUP,
    WM_MBUTTONDOWN, WM_MBUTTONUP, WM_QUIT, WM_SYSKEYUP,
};

use crate::app_log;
use crate::config;
use crate::session::SessionController;

const HOTKEY_ID: i32 = 1;
static GLOBAL_HOTKEY_THREAD_ID: OnceLock<Mutex<Option<u32>>> = OnceLock::new();
static INPUT_HOOK_THREAD_ID: OnceLock<Mutex<Option<u32>>> = OnceLock::new();
static HOOK_TRIGGER_TX: OnceLock<Sender<&'static str>> = OnceLock::new();
static TOGGLE_IN_FLIGHT: AtomicBool = AtomicBool::new(false);

pub fn start_global_hotkey_thread(app: AppHandle) {
    thread::spawn(move || {
        if let Err(err) = run_global_hotkey_loop(app.clone()) {
            app_log::warn(format!(
                "global hotkey thread failed: {}。如果提示热键已注册，通常是已有 VoxType 实例或其他软件占用了该快捷键；右 Alt / 鼠标中键仍会继续工作。",
                err
            ));
        }
    });
}

pub fn start_input_hook_thread(app: AppHandle) {
    let (tx, rx) = mpsc::channel::<&'static str>();
    if HOOK_TRIGGER_TX.set(tx).is_err() {
        app_log::warn("输入钩子触发通道已初始化，本次启动请求被忽略。");
        return;
    }

    let app_for_dispatch = app.clone();
    thread::spawn(move || {
        for source in rx {
            dispatch_toggle(app_for_dispatch.clone(), source);
        }
    });

    thread::spawn(|| {
        if let Err(err) = run_input_hook_loop() {
            app_log::warn(format!("input hook thread failed: {}", err));
        }
    });
}

pub fn stop_input_threads() {
    post_quit(&GLOBAL_HOTKEY_THREAD_ID);
    post_quit(&INPUT_HOOK_THREAD_ID);
}

fn run_global_hotkey_loop(app: AppHandle) -> Result<(), String> {
    let _thread_id = ThreadIdGuard::new(&GLOBAL_HOTKEY_THREAD_ID);
    let loaded = config::load_config()?;
    let hotkey_text = loaded.data.hotkey.clone();
    let (modifiers, key) = parse_hotkey(&loaded.data.hotkey)?;
    unsafe {
        RegisterHotKey(None, HOTKEY_ID, modifiers, key.0 as u32)
            .map_err(|err| format!("注册全局热键失败: {}，hotkey={}", err, hotkey_text))?;
    }
    app_log::info(format!("全局热键已注册: {}", hotkey_text.to_uppercase()));

    let mut msg = MSG::default();
    loop {
        let result = unsafe { GetMessageW(&mut msg, None, 0, 0) };
        if result.0 == -1 {
            unsafe {
                let _ = UnregisterHotKey(None, HOTKEY_ID);
            }
            return Err("读取热键消息失败".to_string());
        }
        if result.0 == 0 {
            break;
        }
        if msg.message == WM_HOTKEY && msg.wParam.0 as i32 == HOTKEY_ID {
            dispatch_toggle(app.clone(), "全局热键");
            continue;
        }
        unsafe {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    unsafe {
        let _ = UnregisterHotKey(None, HOTKEY_ID);
    }
    Ok(())
}

fn parse_hotkey(value: &str) -> Result<(HOT_KEY_MODIFIERS, VIRTUAL_KEY), String> {
    let parts: Vec<String> = value
        .split('+')
        .map(|part| part.trim().to_ascii_lowercase())
        .filter(|part| !part.is_empty())
        .collect();
    if parts.is_empty() {
        return Err("热键不能为空".to_string());
    }

    let mut modifiers = HOT_KEY_MODIFIERS(0);
    for part in &parts[..parts.len() - 1] {
        match part.as_str() {
            "ctrl" | "control" => modifiers |= MOD_CONTROL,
            "alt" => modifiers |= MOD_ALT,
            "shift" => modifiers |= MOD_SHIFT,
            "win" => modifiers |= MOD_WIN,
            other => return Err(format!("不支持的热键修饰键: {}", other)),
        }
    }

    let key = match parts.last().map(String::as_str) {
        Some("a") => VK_A,
        Some("b") => VK_B,
        Some("c") => VK_C,
        Some("d") => VK_D,
        Some("e") => VK_E,
        Some("f") => VK_F,
        Some("g") => VK_G,
        Some("h") => VK_H,
        Some("i") => VK_I,
        Some("j") => VK_J,
        Some("k") => VK_K,
        Some("l") => VK_L,
        Some("m") => VK_M,
        Some("n") => VK_N,
        Some("o") => VK_O,
        Some("p") => VK_P,
        Some("q") => VK_Q,
        Some("r") => VK_R,
        Some("s") => VK_S,
        Some("t") => VK_T,
        Some("u") => VK_U,
        Some("v") => VK_V,
        Some("w") => VK_W,
        Some("x") => VK_X,
        Some("y") => VK_Y,
        Some("z") => VK_Z,
        Some(other) => return Err(format!("暂不支持的热键按键: {}", other)),
        None => return Err("热键不能为空".to_string()),
    };

    Ok((modifiers, key))
}

fn run_input_hook_loop() -> Result<(), String> {
    let _thread_id = ThreadIdGuard::new(&INPUT_HOOK_THREAD_ID);
    let keyboard_hook =
        unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook_proc), None, 0) }
            .map_err(|err| format!("注册键盘钩子失败: {}", err))?;
    let mouse_hook = match unsafe { SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook_proc), None, 0) }
    {
        Ok(hook) => hook,
        Err(err) => {
            unsafe {
                let _ = UnhookWindowsHookEx(keyboard_hook);
            }
            return Err(format!("注册鼠标钩子失败: {}", err));
        }
    };
    app_log::info("输入钩子已注册: 右 Alt / 鼠标中键。");

    let mut msg = MSG::default();
    loop {
        let result = unsafe { GetMessageW(&mut msg, None, 0, 0) };
        if result.0 == -1 {
            unsafe {
                let _ = UnhookWindowsHookEx(keyboard_hook);
                let _ = UnhookWindowsHookEx(mouse_hook);
            }
            return Err("读取输入钩子消息失败".to_string());
        }
        if result.0 == 0 {
            break;
        }
        unsafe {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    unsafe {
        let _ = UnhookWindowsHookEx(keyboard_hook);
        let _ = UnhookWindowsHookEx(mouse_hook);
    }
    Ok(())
}

unsafe extern "system" fn keyboard_hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 && (wparam.0 as u32 == WM_KEYUP || wparam.0 as u32 == WM_SYSKEYUP) {
        let event = unsafe { *(lparam.0 as *const KBDLLHOOKSTRUCT) };
        if event.vkCode == VK_RMENU.0 as u32 && !event.flags.contains(LLKHF_INJECTED) {
            queue_hook_trigger("右 Alt");
        }
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

unsafe extern "system" fn mouse_hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let message = wparam.0 as u32;
    if code == HC_ACTION as i32 && (message == WM_MBUTTONDOWN || message == WM_MBUTTONUP) {
        let event = unsafe { *(lparam.0 as *const MSLLHOOKSTRUCT) };
        if event.flags & LLMHF_INJECTED == 0 {
            if message == WM_MBUTTONDOWN {
                queue_hook_trigger("鼠标中键");
            }
            return LRESULT(1);
        }
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

fn queue_hook_trigger(source: &'static str) {
    if let Some(tx) = HOOK_TRIGGER_TX.get() {
        let _ = tx.send(source);
    }
}

fn dispatch_toggle(app: AppHandle, source: &'static str) {
    if TOGGLE_IN_FLIGHT.swap(true, Ordering::AcqRel) {
        app_log::warn(format!("忽略{}触发：上一轮切换仍在处理中", source));
        return;
    }

    thread::spawn(move || {
        let _in_flight = ToggleInFlightGuard;
        app_log::info(format!("输入触发来源: {}", source));
        let controller = app.state::<SessionController>().inner().clone();
        if let Err(err) = controller.toggle(Some(app.clone())) {
            if is_config_error(&err) {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.unminimize();
                    let _ = window.show();
                    let _ = window.set_focus();
                }
                if err.contains("config.toml") {
                    crate::setup_guide::open_if_config_missing(&app);
                }
            }
            app_log::warn(format!("{}触发失败: {}", source, err));
        }
    });
}

fn is_config_error(message: &str) -> bool {
    message.contains("config.toml")
        || message.contains("app_key")
        || message.contains("access_key")
        || message.contains("豆包认证")
}

struct ToggleInFlightGuard;

impl Drop for ToggleInFlightGuard {
    fn drop(&mut self) {
        TOGGLE_IN_FLIGHT.store(false, Ordering::Release);
    }
}

struct ThreadIdGuard(&'static OnceLock<Mutex<Option<u32>>>);

impl ThreadIdGuard {
    fn new(slot: &'static OnceLock<Mutex<Option<u32>>>) -> Self {
        set_thread_id(slot, Some(unsafe { GetCurrentThreadId() }));
        Self(slot)
    }
}

impl Drop for ThreadIdGuard {
    fn drop(&mut self) {
        set_thread_id(self.0, None);
    }
}

fn set_thread_id(slot: &'static OnceLock<Mutex<Option<u32>>>, value: Option<u32>) {
    if let Ok(mut thread_id) = slot.get_or_init(|| Mutex::new(None)).lock() {
        *thread_id = value;
    }
}

fn post_quit(slot: &'static OnceLock<Mutex<Option<u32>>>) {
    let thread_id = slot
        .get()
        .and_then(|lock| lock.lock().ok().and_then(|guard| *guard));
    if let Some(thread_id) = thread_id {
        unsafe {
            let _ = PostThreadMessageW(thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
        }
    }
}
