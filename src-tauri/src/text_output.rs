use crate::{app_log, config::TypingConfig};
use std::mem::size_of;
use std::thread;
use std::time::Duration;
use windows::Win32::Foundation::{HANDLE, HGLOBAL};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
    SetClipboardData,
};
use windows::Win32::System::Memory::{
    GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    keybd_event, MapVirtualKeyW, KEYBD_EVENT_FLAGS, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP,
    MAPVK_VK_TO_VSC, VIRTUAL_KEY, VK_CONTROL, VK_INSERT, VK_SHIFT, VK_V,
};

const CF_UNICODETEXT: u32 = 13;
const KEY_INTERVAL: Duration = Duration::from_millis(10);

pub struct OutputResult {
    pub warning: Option<String>,
}

pub fn output_text(text: &str, typing: &TypingConfig) -> Result<OutputResult, String> {
    if text.trim().is_empty() {
        return Ok(OutputResult { warning: None });
    }
    app_log::info(format!(
        "准备输出文本: chars={}, method={}, restore_clipboard={}",
        text.chars().count(),
        typing.paste_method,
        typing.restore_clipboard_after_paste
    ));
    let original_clipboard =
        if typing.restore_clipboard_after_paste && typing.paste_method != "clipboard_only" {
            match read_clipboard_text_with_retry(typing) {
                Ok(value) => value,
                Err(err) => {
                    app_log::warn(format!("备份剪贴板文本失败，将继续输出: {}", err));
                    None
                }
            }
        } else {
            None
        };

    write_clipboard_text_with_retry(text, typing)?;
    if typing.paste_method == "clipboard_only" {
        app_log::info("文本已写入剪贴板: method=clipboard_only");
        return Ok(OutputResult { warning: None });
    }
    thread::sleep(Duration::from_millis(typing.paste_delay_ms));
    let result = match typing.paste_method.as_str() {
        "shift_insert" => send_shortcut(VK_SHIFT, VK_INSERT, true),
        _ => send_shortcut(VK_CONTROL, VK_V, false),
    };
    if let Err(err) = result {
        app_log::warn(format!(
            "发送粘贴快捷键失败，识别文本已保留在剪贴板: {}",
            err
        ));
        return Ok(OutputResult {
            warning: Some("粘贴失败，文本已复制，可手动 Ctrl+V。".to_string()),
        });
    }

    app_log::info(format!(
        "粘贴快捷键已发送: method={}, delay_ms={}",
        typing.paste_method, typing.paste_delay_ms
    ));
    if let Some(original) = original_clipboard {
        thread::sleep(clipboard_restore_delay(typing));
        match write_clipboard_text_with_retry(&original, typing) {
            Ok(()) => {
                app_log::info("自动粘贴后已恢复原剪贴板文本");
                Ok(OutputResult { warning: None })
            }
            Err(err) => {
                app_log::warn(format!("恢复原剪贴板文本失败: {}", err));
                Ok(OutputResult {
                    warning: Some("文本已粘贴，但恢复原剪贴板失败。".to_string()),
                })
            }
        }
    } else {
        Ok(OutputResult { warning: None })
    }
}

fn read_clipboard_text_with_retry(typing: &TypingConfig) -> Result<Option<String>, String> {
    with_clipboard_retry(typing, read_clipboard_text)
}

fn write_clipboard_text_with_retry(text: &str, typing: &TypingConfig) -> Result<(), String> {
    with_clipboard_retry(typing, || write_clipboard_text(text))
}

fn with_clipboard_retry<T>(
    typing: &TypingConfig,
    operation: impl Fn() -> Result<T, String>,
) -> Result<T, String> {
    let attempts = typing.clipboard_open_retry_count.max(1);
    let interval = Duration::from_millis(typing.clipboard_open_retry_interval_ms);
    let mut last_error = String::new();
    for attempt in 0..attempts {
        match operation() {
            Ok(value) => return Ok(value),
            Err(err) => {
                last_error = err;
                if attempt + 1 < attempts {
                    thread::sleep(interval);
                }
            }
        }
    }
    Err(last_error)
}

fn read_clipboard_text() -> Result<Option<String>, String> {
    unsafe {
        OpenClipboard(None).map_err(|err| format!("打开剪贴板失败: {}", err))?;
        let result = (|| {
            if IsClipboardFormatAvailable(CF_UNICODETEXT).is_err() {
                return Ok(None);
            }
            let handle = GetClipboardData(CF_UNICODETEXT)
                .map_err(|err| format!("读取剪贴板失败: {}", err))?;
            if handle.is_invalid() {
                return Ok(None);
            }
            let memory = HGLOBAL(handle.0);
            let size = GlobalSize(memory);
            let locked = GlobalLock(memory) as *const u16;
            if locked.is_null() {
                return Err("锁定剪贴板内存失败".to_string());
            }
            let units = size / size_of::<u16>();
            let slice = std::slice::from_raw_parts(locked, units);
            let len = slice.iter().position(|value| *value == 0).unwrap_or(units);
            let text = String::from_utf16_lossy(&slice[..len]);
            let _ = GlobalUnlock(memory);
            Ok(Some(text))
        })();
        let _ = CloseClipboard();
        result
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

fn clipboard_restore_delay(typing: &TypingConfig) -> Duration {
    Duration::from_millis(typing.paste_delay_ms.max(200))
}

fn send_shortcut(
    modifier: VIRTUAL_KEY,
    key: VIRTUAL_KEY,
    key_extended: bool,
) -> Result<(), String> {
    send_key_event(modifier, false, false);
    thread::sleep(KEY_INTERVAL);
    send_key_event(key, false, key_extended);
    thread::sleep(KEY_INTERVAL);
    send_key_event(key, true, key_extended);
    thread::sleep(KEY_INTERVAL);
    send_key_event(modifier, true, false);
    Ok(())
}

fn send_key_event(key: VIRTUAL_KEY, key_up: bool, extended: bool) {
    let scan_code = unsafe { MapVirtualKeyW(key.0 as u32, MAPVK_VK_TO_VSC) as u8 };
    let mut flags = KEYBD_EVENT_FLAGS(0);
    if extended {
        flags |= KEYEVENTF_EXTENDEDKEY;
    }
    if key_up {
        flags |= KEYEVENTF_KEYUP;
    }
    unsafe {
        keybd_event(key.0 as u8, scan_code, flags, 0);
    }
}

#[cfg(test)]
mod tests {
    use super::clipboard_restore_delay;
    use crate::config::TypingConfig;
    use std::time::Duration;

    #[test]
    fn restore_delay_leaves_time_for_target_app_to_read_clipboard() {
        let typing = TypingConfig {
            paste_delay_ms: 0,
            ..TypingConfig::default()
        };
        assert_eq!(clipboard_restore_delay(&typing), Duration::from_millis(200));
        let typing = TypingConfig {
            paste_delay_ms: 350,
            ..TypingConfig::default()
        };
        assert_eq!(clipboard_restore_delay(&typing), Duration::from_millis(350));
    }
}
