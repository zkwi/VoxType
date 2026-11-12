use std::cell::Cell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN,
    VIRTUAL_KEY, VK_A, VK_B, VK_C, VK_D, VK_E, VK_F, VK_G, VK_H, VK_I, VK_J, VK_K, VK_L, VK_M,
    VK_N, VK_O, VK_P, VK_Q, VK_R, VK_RCONTROL, VK_RMENU, VK_S, VK_T, VK_U, VK_V, VK_W, VK_X, VK_Y,
    VK_Z,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, PostThreadMessageW, SetWindowsHookExW,
    TranslateMessage, UnhookWindowsHookEx, HC_ACTION, KBDLLHOOKSTRUCT, LLKHF_INJECTED,
    LLMHF_INJECTED, MSG, MSLLHOOKSTRUCT, WH_KEYBOARD_LL, WH_MOUSE_LL, WM_HOTKEY, WM_KEYDOWN,
    WM_KEYUP, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_QUIT, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

use crate::app_log;
use crate::config::{self, TriggerConfig};
use crate::session::SessionController;

const HOTKEY_ID: i32 = 1;
const HOTKEY_TEST_ID: i32 = 991;
static GLOBAL_HOTKEY_THREAD_ID: OnceLock<Mutex<Option<u32>>> = OnceLock::new();
static INPUT_HOOK_THREAD_ID: OnceLock<Mutex<Option<u32>>> = OnceLock::new();
static HOOK_TRIGGER_TX: OnceLock<Sender<&'static str>> = OnceLock::new();
static TOGGLE_IN_FLIGHT: AtomicBool = AtomicBool::new(false);
static HOTKEY_TRIGGER_ENABLED: AtomicBool = AtomicBool::new(true);
static MIDDLE_MOUSE_TRIGGER_ENABLED: AtomicBool = AtomicBool::new(false);
static RIGHT_ALT_TRIGGER_ENABLED: AtomicBool = AtomicBool::new(false);
static RIGHT_CTRL_TRIGGER_ENABLED: AtomicBool = AtomicBool::new(false);
static HOTKEY_CAPTURE_ACTIVE: AtomicBool = AtomicBool::new(false);

const CAPTURE_RIGHT_ALT: u8 = 1;
const CAPTURE_RIGHT_CTRL: u8 = 2;
const CAPTURE_MIDDLE_MOUSE: u8 = 4;

thread_local! {
    static CAPTURE_FORWARDED_PRESSES: Cell<u8> = const { Cell::new(0) };
}

pub fn set_hotkey_capture_active(active: bool) {
    HOTKEY_CAPTURE_ACTIVE.store(active, Ordering::Release);
}

fn forward_capture_press(held: &Cell<u8>, key: u8, pressed: bool, active: bool) -> bool {
    let previous = held.get();
    if pressed && active {
        held.set(previous | key);
    } else if !pressed {
        held.set(previous & !key);
    }
    // 录制组合键成功时修饰键可能仍按着。它的松开事件要与按下一同放行。
    active || previous & key != 0
}

fn forward_capture_event(key: u8, pressed: bool) -> bool {
    CAPTURE_FORWARDED_PRESSES.with(|held| {
        forward_capture_press(
            held,
            key,
            pressed,
            HOTKEY_CAPTURE_ACTIVE.load(Ordering::Acquire),
        )
    })
}

pub fn refresh_trigger_config() {
    match config::load_config() {
        Ok(loaded) => refresh_trigger_config_from(&loaded.data.triggers),
        Err(err) => {
            app_log::warn(format!("读取启动方式配置失败，使用保守触发默认值: {}", err));
            apply_conservative_trigger_defaults();
        }
    }
}

pub fn refresh_trigger_config_from(triggers: &TriggerConfig) {
    HOTKEY_TRIGGER_ENABLED.store(triggers.hotkey_enabled, Ordering::Release);
    MIDDLE_MOUSE_TRIGGER_ENABLED.store(triggers.middle_mouse_enabled, Ordering::Release);
    RIGHT_ALT_TRIGGER_ENABLED.store(triggers.right_alt_enabled, Ordering::Release);
    RIGHT_CTRL_TRIGGER_ENABLED.store(triggers.right_ctrl_enabled, Ordering::Release);
}

fn apply_conservative_trigger_defaults() {
    HOTKEY_TRIGGER_ENABLED.store(true, Ordering::Release);
    MIDDLE_MOUSE_TRIGGER_ENABLED.store(false, Ordering::Release);
    RIGHT_ALT_TRIGGER_ENABLED.store(false, Ordering::Release);
    RIGHT_CTRL_TRIGGER_ENABLED.store(false, Ordering::Release);
}

pub fn start_global_hotkey_thread(app: AppHandle) {
    start_global_hotkey_thread_with_ready(app, None);
}

fn start_global_hotkey_thread_with_ready(
    app: AppHandle,
    ready: Option<Sender<Result<(), String>>>,
) {
    thread::spawn(move || {
        if let Err(err) = run_global_hotkey_loop(app.clone(), ready) {
            app_log::warn(format!(
                "HOTKEY_REGISTER_FAILED: global hotkey thread failed: {}。如果提示热键已注册，通常是已有 VoxType 实例或其他软件占用了该快捷键；已开启的右 Alt / 右 Ctrl / 鼠标中键触发仍可使用。",
                err
            ));
        }
    });
}

pub fn restart_global_hotkey_thread(app: AppHandle) -> Result<(), String> {
    post_quit(&GLOBAL_HOTKEY_THREAD_ID);
    let (ready_tx, ready_rx) = mpsc::channel();
    thread::spawn(move || {
        if !wait_for_thread_to_stop(&GLOBAL_HOTKEY_THREAD_ID, Duration::from_millis(800)) {
            app_log::warn("等待旧全局热键线程退出超时，继续尝试重新注册。");
        }
        start_global_hotkey_thread_with_ready(app, Some(ready_tx));
    });
    ready_rx
        .recv_timeout(Duration::from_millis(1200))
        .map_err(|_| "等待快捷键重新注册超时，请稍后再试或重启应用。".to_string())?
}

pub fn can_register_hotkey(value: &str) -> Result<(), String> {
    let (modifiers, key) = parse_hotkey(value)?;
    unsafe {
        RegisterHotKey(None, HOTKEY_TEST_ID, modifiers, key.0 as u32)
            .map_err(|err| format!("该快捷键可能已被其他程序占用: {}", err))?;
        let _ = UnregisterHotKey(None, HOTKEY_TEST_ID);
    }
    Ok(())
}

pub fn start_input_hook_thread(app: AppHandle) {
    refresh_trigger_config();
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

fn run_global_hotkey_loop(
    app: AppHandle,
    mut ready: Option<Sender<Result<(), String>>>,
) -> Result<(), String> {
    let _thread_id = ThreadIdGuard::new(&GLOBAL_HOTKEY_THREAD_ID);
    let loaded = match config::load_config() {
        Ok(loaded) => loaded,
        Err(err) => {
            send_hotkey_ready(&mut ready, Err(err.clone()));
            return Err(err);
        }
    };
    refresh_trigger_config_from(&loaded.data.triggers);
    if !loaded.data.triggers.hotkey_enabled {
        app_log::info("主快捷键触发已关闭，跳过全局热键注册。");
        send_hotkey_ready(&mut ready, Ok(()));
        return Ok(());
    }
    let hotkey_text = loaded.data.hotkey.clone();
    let (modifiers, key) = match parse_hotkey(&loaded.data.hotkey) {
        Ok(parsed) => parsed,
        Err(err) => {
            send_hotkey_ready(&mut ready, Err(err.clone()));
            return Err(err);
        }
    };
    unsafe {
        if let Err(err) = RegisterHotKey(None, HOTKEY_ID, modifiers, key.0 as u32) {
            let message = format!("注册全局热键失败: {}，hotkey={}", err, hotkey_text);
            send_hotkey_ready(&mut ready, Err(message.clone()));
            return Err(message);
        }
    }
    app_log::info(format!("全局热键已注册: {}", hotkey_text.to_uppercase()));
    send_hotkey_ready(&mut ready, Ok(()));

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

fn send_hotkey_ready(ready: &mut Option<Sender<Result<(), String>>>, result: Result<(), String>) {
    if let Some(tx) = ready.take() {
        let _ = tx.send(result);
    }
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
            "win" | "meta" => modifiers |= MOD_WIN,
            other => return Err(format!("不支持的热键修饰键: {}", other)),
        }
    }
    if modifiers.0 == 0 {
        return Err("热键至少需要一个修饰键".to_string());
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
        Some("0") => VIRTUAL_KEY(0x30),
        Some("1") => VIRTUAL_KEY(0x31),
        Some("2") => VIRTUAL_KEY(0x32),
        Some("3") => VIRTUAL_KEY(0x33),
        Some("4") => VIRTUAL_KEY(0x34),
        Some("5") => VIRTUAL_KEY(0x35),
        Some("6") => VIRTUAL_KEY(0x36),
        Some("7") => VIRTUAL_KEY(0x37),
        Some("8") => VIRTUAL_KEY(0x38),
        Some("9") => VIRTUAL_KEY(0x39),
        Some("space") => VIRTUAL_KEY(0x20),
        Some("enter") => VIRTUAL_KEY(0x0d),
        Some("tab") => VIRTUAL_KEY(0x09),
        Some("f1") => VIRTUAL_KEY(0x70),
        Some("f2") => VIRTUAL_KEY(0x71),
        Some("f3") => VIRTUAL_KEY(0x72),
        Some("f4") => VIRTUAL_KEY(0x73),
        Some("f5") => VIRTUAL_KEY(0x74),
        Some("f6") => VIRTUAL_KEY(0x75),
        Some("f7") => VIRTUAL_KEY(0x76),
        Some("f8") => VIRTUAL_KEY(0x77),
        Some("f9") => VIRTUAL_KEY(0x78),
        Some("f10") => VIRTUAL_KEY(0x79),
        Some("f11") => VIRTUAL_KEY(0x7a),
        Some("f12") => VIRTUAL_KEY(0x7b),
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
    app_log::info("输入钩子已注册: 右 Alt / 右 Ctrl / 鼠标中键。");

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModifierHookAction {
    /// 原样交给前台程序。
    Forward,
    /// 拦下但不触发录音。
    Swallow,
    /// 拦下并排队触发录音。
    SwallowAndTrigger(&'static str),
}

/// 开启触发后成对拦截按下和松开，避免前台程序收到不成对的修饰键事件。
/// 注入事件一律放行，VoxType 自己模拟的粘贴按键不受影响。
fn modifier_hook_action(
    message: u32,
    vk_code: u32,
    injected: bool,
    right_alt_trigger_enabled: bool,
    right_ctrl_trigger_enabled: bool,
) -> ModifierHookAction {
    if injected {
        return ModifierHookAction::Forward;
    }
    let source = match vk_code {
        key if key == u32::from(VK_RMENU.0) && right_alt_trigger_enabled => "右 Alt",
        key if key == u32::from(VK_RCONTROL.0) && right_ctrl_trigger_enabled => "右 Ctrl",
        _ => return ModifierHookAction::Forward,
    };
    match message {
        WM_KEYUP | WM_SYSKEYUP => ModifierHookAction::SwallowAndTrigger(source),
        WM_KEYDOWN | WM_SYSKEYDOWN => ModifierHookAction::Swallow,
        _ => ModifierHookAction::Forward,
    }
}

unsafe extern "system" fn keyboard_hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let event = unsafe { *(lparam.0 as *const KBDLLHOOKSTRUCT) };
        let message = wparam.0 as u32;
        let injected = event.flags.contains(LLKHF_INJECTED);
        let capture_key = match event.vkCode {
            key if key == u32::from(VK_RMENU.0) => CAPTURE_RIGHT_ALT,
            key if key == u32::from(VK_RCONTROL.0) => CAPTURE_RIGHT_CTRL,
            _ => 0,
        };
        if !injected
            && capture_key != 0
            && matches!(message, WM_KEYDOWN | WM_SYSKEYDOWN | WM_KEYUP | WM_SYSKEYUP)
            && forward_capture_event(capture_key, matches!(message, WM_KEYDOWN | WM_SYSKEYDOWN))
        {
            return unsafe { CallNextHookEx(None, code, wparam, lparam) };
        }
        match modifier_hook_action(
            message,
            event.vkCode,
            injected,
            trigger_enabled("right_alt"),
            trigger_enabled("right_ctrl"),
        ) {
            ModifierHookAction::SwallowAndTrigger(source) => {
                queue_hook_trigger(source);
                return LRESULT(1);
            }
            ModifierHookAction::Swallow => return LRESULT(1),
            ModifierHookAction::Forward => {}
        }
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

unsafe extern "system" fn mouse_hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let message = wparam.0 as u32;
    if code == HC_ACTION as i32 && (message == WM_MBUTTONDOWN || message == WM_MBUTTONUP) {
        let event = unsafe { *(lparam.0 as *const MSLLHOOKSTRUCT) };
        if event.flags & LLMHF_INJECTED == 0
            && !forward_capture_event(CAPTURE_MIDDLE_MOUSE, message == WM_MBUTTONDOWN)
            && trigger_enabled("middle_mouse")
        {
            if message == WM_MBUTTONDOWN {
                queue_hook_trigger("鼠标中键");
            }
            return LRESULT(1);
        }
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

fn trigger_enabled(kind: &'static str) -> bool {
    match kind {
        "hotkey" => HOTKEY_TRIGGER_ENABLED.load(Ordering::Acquire),
        "right_alt" => RIGHT_ALT_TRIGGER_ENABLED.load(Ordering::Acquire),
        "right_ctrl" => RIGHT_CTRL_TRIGGER_ENABLED.load(Ordering::Acquire),
        "middle_mouse" => MIDDLE_MOUSE_TRIGGER_ENABLED.load(Ordering::Acquire),
        _ => true,
    }
}

fn queue_hook_trigger(source: &'static str) {
    if let Some(tx) = HOOK_TRIGGER_TX.get() {
        let _ = tx.send(source);
    }
}

fn dispatch_toggle(app: AppHandle, source: &'static str) {
    if HOTKEY_CAPTURE_ACTIVE.load(Ordering::Acquire) {
        return;
    }
    if source == "全局热键" && !trigger_enabled("hotkey") {
        app_log::info("忽略全局热键触发：该启动方式已关闭。");
        return;
    }
    if TOGGLE_IN_FLIGHT.swap(true, Ordering::AcqRel) {
        app_log::warn(format!("忽略{}触发：上一轮切换仍在处理中", source));
        return;
    }

    thread::spawn(move || {
        let _in_flight = ToggleInFlightGuard;
        app_log::info(format!("输入触发来源: {}", source));
        let controller = app.state::<SessionController>().inner().clone();
        if let Err(err) = controller.toggle(Some(app.clone())) {
            app_log::warn(format!("{}触发失败: {}", source, err));
        }
    });
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

fn wait_for_thread_to_stop(slot: &'static OnceLock<Mutex<Option<u32>>>, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        let running = slot
            .get()
            .and_then(|lock| lock.lock().ok().and_then(|guard| *guard))
            .is_some();
        if !running {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        thread::sleep(Duration::from_millis(20));
    }
}

#[cfg(test)]
mod tests {
    use super::{
        apply_conservative_trigger_defaults, forward_capture_press, modifier_hook_action,
        parse_hotkey, refresh_trigger_config_from, trigger_enabled, ModifierHookAction,
        CAPTURE_MIDDLE_MOUSE, CAPTURE_RIGHT_ALT, CAPTURE_RIGHT_CTRL, HOTKEY_TRIGGER_ENABLED,
        MIDDLE_MOUSE_TRIGGER_ENABLED, RIGHT_ALT_TRIGGER_ENABLED, RIGHT_CTRL_TRIGGER_ENABLED,
    };
    use crate::config::TriggerConfig;
    use std::cell::Cell;
    use std::sync::atomic::Ordering;
    use std::sync::Mutex;
    use windows::Win32::UI::Input::KeyboardAndMouse::{VK_LCONTROL, VK_RCONTROL, VK_RMENU, VK_TAB};
    use windows::Win32::UI::WindowsAndMessaging::{
        WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
    };

    const RIGHT_ALT: u32 = VK_RMENU.0 as u32;
    const RIGHT_CTRL: u32 = VK_RCONTROL.0 as u32;
    static TRIGGER_STATE_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn capture_passes_both_halves_of_a_press_even_after_recording_the_shortcut() {
        let held = Cell::new(0);
        for key in [CAPTURE_RIGHT_ALT, CAPTURE_RIGHT_CTRL, CAPTURE_MIDDLE_MOUSE] {
            assert!(forward_capture_press(&held, key, true, true));
            assert!(forward_capture_press(&held, key, true, false));
            assert!(forward_capture_press(&held, key, false, false));
            assert!(!forward_capture_press(&held, key, true, false));
            assert!(!forward_capture_press(&held, key, false, false));
        }
    }

    #[test]
    fn meta_and_win_parse_as_the_same_modifier() {
        assert_eq!(
            parse_hotkey("Meta + K").unwrap(),
            parse_hotkey("Win + K").unwrap()
        );
    }

    #[test]
    fn enabled_right_alt_and_right_ctrl_take_over_both_key_down_and_key_up() {
        for (key, source, alt_enabled, ctrl_enabled) in [
            (RIGHT_ALT, "右 Alt", true, false),
            (RIGHT_CTRL, "右 Ctrl", false, true),
        ] {
            for message in [WM_KEYDOWN, WM_SYSKEYDOWN] {
                assert_eq!(
                    modifier_hook_action(message, key, false, alt_enabled, ctrl_enabled),
                    ModifierHookAction::Swallow,
                );
            }
            for message in [WM_KEYUP, WM_SYSKEYUP] {
                assert_eq!(
                    modifier_hook_action(message, key, false, alt_enabled, ctrl_enabled),
                    ModifierHookAction::SwallowAndTrigger(source),
                );
            }
        }
    }

    #[test]
    fn right_modifier_triggers_respect_independent_switches() {
        for message in [WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP] {
            assert_eq!(
                modifier_hook_action(message, RIGHT_ALT, false, false, true),
                ModifierHookAction::Forward
            );
            assert_eq!(
                modifier_hook_action(message, RIGHT_CTRL, false, true, false),
                ModifierHookAction::Forward
            );
        }
    }

    #[test]
    fn left_ctrl_other_keys_and_injected_modifiers_pass_through() {
        for (key, injected) in [
            (RIGHT_ALT, true),
            (RIGHT_CTRL, true),
            (u32::from(VK_LCONTROL.0), false),
            (u32::from(VK_TAB.0), false),
        ] {
            for message in [WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP] {
                assert_eq!(
                    modifier_hook_action(message, key, injected, true, true),
                    ModifierHookAction::Forward
                );
            }
        }
    }

    struct TriggerStateGuard {
        hotkey: bool,
        middle_mouse: bool,
        right_alt: bool,
        right_ctrl: bool,
    }

    impl TriggerStateGuard {
        fn new() -> Self {
            Self {
                hotkey: HOTKEY_TRIGGER_ENABLED.load(Ordering::Acquire),
                middle_mouse: MIDDLE_MOUSE_TRIGGER_ENABLED.load(Ordering::Acquire),
                right_alt: RIGHT_ALT_TRIGGER_ENABLED.load(Ordering::Acquire),
                right_ctrl: RIGHT_CTRL_TRIGGER_ENABLED.load(Ordering::Acquire),
            }
        }
    }

    impl Drop for TriggerStateGuard {
        fn drop(&mut self) {
            HOTKEY_TRIGGER_ENABLED.store(self.hotkey, Ordering::Release);
            MIDDLE_MOUSE_TRIGGER_ENABLED.store(self.middle_mouse, Ordering::Release);
            RIGHT_ALT_TRIGGER_ENABLED.store(self.right_alt, Ordering::Release);
            RIGHT_CTRL_TRIGGER_ENABLED.store(self.right_ctrl, Ordering::Release);
        }
    }

    #[test]
    fn trigger_static_defaults_are_conservative() {
        let _lock = TRIGGER_STATE_LOCK.lock().unwrap();
        assert!(trigger_enabled("hotkey"));
        assert!(!trigger_enabled("middle_mouse"));
        assert!(!trigger_enabled("right_alt"));
        assert!(!trigger_enabled("right_ctrl"));
    }

    #[test]
    fn can_restore_conservative_trigger_state() {
        let _lock = TRIGGER_STATE_LOCK.lock().unwrap();
        let _guard = TriggerStateGuard::new();
        refresh_trigger_config_from(&TriggerConfig {
            hotkey_enabled: false,
            middle_mouse_enabled: true,
            right_alt_enabled: true,
            right_ctrl_enabled: true,
        });
        assert!(trigger_enabled("right_ctrl"));

        apply_conservative_trigger_defaults();

        assert!(trigger_enabled("hotkey"));
        assert!(!trigger_enabled("middle_mouse"));
        assert!(!trigger_enabled("right_alt"));
        assert!(!trigger_enabled("right_ctrl"));
    }
}
