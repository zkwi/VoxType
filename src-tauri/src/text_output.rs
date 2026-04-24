use crate::config::TypingConfig;
use std::mem::size_of;
use std::thread;
use std::time::Duration;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
};
use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY,
    VK_CONTROL, VK_INSERT, VK_SHIFT, VK_V,
};

const CF_UNICODETEXT: u32 = 13;

pub fn output_text(text: &str, typing: &TypingConfig) -> Result<(), String> {
    if text.trim().is_empty() {
        return Ok(());
    }
    write_clipboard_text(text)?;
    if typing.paste_method == "clipboard_only" {
        return Ok(());
    }
    thread::sleep(Duration::from_millis(typing.paste_delay_ms));
    match typing.paste_method.as_str() {
        "shift_insert" => send_shortcut(VK_SHIFT, VK_INSERT),
        _ => send_shortcut(VK_CONTROL, VK_V),
    }
}

fn write_clipboard_text(text: &str) -> Result<(), String> {
    let mut utf16: Vec<u16> = text.encode_utf16().collect();
    utf16.push(0);
    let byte_len = utf16.len() * size_of::<u16>();
    unsafe {
        let memory = GlobalAlloc(GMEM_MOVEABLE, byte_len)
            .map_err(|err| format!("分配剪贴板内存失败: {}", err))?;
        let locked = GlobalLock(memory) as *mut u16;
        if locked.is_null() {
            return Err("锁定剪贴板内存失败".to_string());
        }
        std::ptr::copy_nonoverlapping(utf16.as_ptr(), locked, utf16.len());
        let _ = GlobalUnlock(memory);

        OpenClipboard(None).map_err(|err| format!("打开剪贴板失败: {}", err))?;
        let result = (|| {
            EmptyClipboard().map_err(|err| format!("清空剪贴板失败: {}", err))?;
            SetClipboardData(CF_UNICODETEXT, Some(HANDLE(memory.0)))
                .map_err(|err| format!("写入剪贴板失败: {}", err))?;
            Ok(())
        })();
        let _ = CloseClipboard();
        result
    }
}

fn send_shortcut(modifier: VIRTUAL_KEY, key: VIRTUAL_KEY) -> Result<(), String> {
    let inputs = [
        key_input(modifier, false),
        key_input(key, false),
        key_input(key, true),
        key_input(modifier, true),
    ];
    let sent = unsafe { SendInput(&inputs, size_of::<INPUT>() as i32) };
    if sent == inputs.len() as u32 {
        Ok(())
    } else {
        Err("模拟粘贴快捷键失败".to_string())
    }
}

fn key_input(key: VIRTUAL_KEY, key_up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: key,
                wScan: 0,
                dwFlags: if key_up {
                    KEYEVENTF_KEYUP
                } else {
                    Default::default()
                },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}
