use crate::{app_log, config::TypingConfig};
use std::mem::size_of;
use std::thread;
use std::time::{Duration, Instant};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{GlobalFree, HANDLE, HGLOBAL, HWND};
use windows::Win32::System::DataExchange::{
    CloseClipboard, CountClipboardFormats, EmptyClipboard, EnumClipboardFormats, GetClipboardData,
    IsClipboardFormatAvailable, OpenClipboard, SetClipboardData,
};
use windows::Win32::System::Memory::{
    GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    keybd_event, MapVirtualKeyW, KEYBD_EVENT_FLAGS, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP,
    MAPVK_VK_TO_VSC, VIRTUAL_KEY, VK_CONTROL, VK_INSERT, VK_SHIFT, VK_V,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, HWND_MESSAGE, WINDOW_EX_STYLE, WINDOW_STYLE,
};

const CF_TEXT: u32 = 1;
const CF_BITMAP: u32 = 2;
const CF_METAFILEPICT: u32 = 3;
const CF_OEMTEXT: u32 = 7;
const CF_DIB: u32 = 8;
const CF_PALETTE: u32 = 9;
const CF_UNICODETEXT: u32 = 13;
const CF_ENHMETAFILE: u32 = 14;
const CF_LOCALE: u32 = 16;
const CF_DIBV5: u32 = 17;
const CF_OWNERDISPLAY: u32 = 0x0080;
const CF_DSPBITMAP: u32 = 0x0082;
const CF_DSPMETAFILEPICT: u32 = 0x0083;
const CF_DSPENHMETAFILE: u32 = 0x008E;
const CF_GDIOBJFIRST: u32 = 0x0300;
const CF_GDIOBJLAST: u32 = 0x03FF;
const KEY_INTERVAL: Duration = Duration::from_millis(10);
const MIN_RESTORE_DELAY_AFTER_PASTE_MS: u64 = 500;
// 剪贴板被其他程序短暂占用时先做几次短间隔重试，再交给按配置的慢速重试。
// 我们刚写完剪贴板，监听剪贴板的程序（远程桌面、剪贴板历史等）正好会来读，占用通常只有几毫秒。
const CLIPBOARD_OPEN_QUICK_ATTEMPTS: u32 = 5;
const CLIPBOARD_OPEN_QUICK_INTERVAL: Duration = Duration::from_millis(5);
pub const WARNING_CLIPBOARD_PARTIAL_RESTORE: &str = "CLIPBOARD_PARTIAL_RESTORE";
pub const WARNING_CLIPBOARD_NON_RESTORABLE: &str = "CLIPBOARD_NON_RESTORABLE";
pub const WARNING_CLIPBOARD_RESTORE_FAILED: &str = "CLIPBOARD_RESTORE_FAILED";

pub struct OutputResult {
    pub warning: Option<String>,
    pub warning_code: Option<String>,
}

impl OutputResult {
    fn ok() -> Self {
        Self {
            warning: None,
            warning_code: None,
        }
    }

    fn warning(message: impl Into<String>, code: &'static str) -> Self {
        Self {
            warning: Some(message.into()),
            warning_code: Some(code.to_string()),
        }
    }
}

enum ClipboardBackup {
    Snapshot(ClipboardSnapshot),
    NonRestorable,
    Empty,
}

struct ClipboardSnapshot {
    formats: Vec<ClipboardFormatBackup>,
    /// 恢复后确实拿不回来的格式数；系统能从已备份格式重新合成的不计入。
    skipped_formats: usize,
    /// 没有备份、但恢复后系统会自动合成的格式数（例如备份了 DIB 时的 CF_BITMAP）。
    resynthesized_formats: usize,
    total_bytes: usize,
}

struct ClipboardFormatBackup {
    format: u32,
    bytes: Vec<u8>,
}

/// 打开剪贴板时使用的隐藏消息窗口。
///
/// `OpenClipboard(NULL)` 不是独占的：其他同样传 NULL 的线程可以在我们持有期间再次"打开成功"
/// 并夺走剪贴板，此后本线程的 `GetClipboardData` 会以 `ERROR_CLIPBOARD_NOT_OPEN` 失败。
/// 写入后立刻读回校验时最容易撞上：监听剪贴板的程序正好在这时来读我们刚写的内容。
/// 带上属于本线程的窗口句柄后，系统才会真正拒绝并发打开，冲突表现为可重试的"拒绝访问"。
struct ClipboardOwnerWindow(HWND);

impl ClipboardOwnerWindow {
    fn create() -> Option<Self> {
        // 消息窗口不会出现在屏幕和任务栏上，也收不到广播消息，不需要消息循环。
        unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("STATIC"),
                PCWSTR::null(),
                WINDOW_STYLE(0),
                0,
                0,
                0,
                0,
                Some(HWND_MESSAGE),
                None,
                None,
                None,
            )
        }
        .ok()
        .map(Self)
    }
}

impl Drop for ClipboardOwnerWindow {
    fn drop(&mut self) {
        // 写入的是真实数据而非延迟渲染，窗口销毁后剪贴板内容保留，只是不再有属主。
        unsafe {
            let _ = DestroyWindow(self.0);
        }
    }
}

struct ClipboardGuard {
    // 字段在 `Drop::drop` 之后才析构：先关闭剪贴板，再销毁属主窗口。
    _owner: Option<ClipboardOwnerWindow>,
}

impl ClipboardGuard {
    fn open() -> Result<Self, String> {
        // 窗口创建失败时退回无属主打开，至少保持旧行为可用。
        let owner = ClipboardOwnerWindow::create();
        let owner_hwnd = owner.as_ref().map(|window| window.0);
        let mut last_error = String::new();
        for attempt in 0..CLIPBOARD_OPEN_QUICK_ATTEMPTS {
            match unsafe { OpenClipboard(owner_hwnd) } {
                Ok(()) => return Ok(Self { _owner: owner }),
                Err(err) => {
                    last_error = err.to_string();
                    if attempt + 1 < CLIPBOARD_OPEN_QUICK_ATTEMPTS {
                        thread::sleep(CLIPBOARD_OPEN_QUICK_INTERVAL);
                    }
                }
            }
        }
        Err(format!("打开剪贴板失败: {}", last_error))
    }
}

impl Drop for ClipboardGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseClipboard();
        }
    }
}

struct OwnedGlobalMemory {
    handle: Option<HGLOBAL>,
}

impl OwnedGlobalMemory {
    fn alloc(byte_len: usize) -> Result<Self, String> {
        let handle = unsafe {
            GlobalAlloc(GMEM_MOVEABLE, byte_len)
                .map_err(|err| format!("分配剪贴板内存失败: {}", err))?
        };
        Ok(Self {
            handle: Some(handle),
        })
    }

    fn handle(&self) -> HGLOBAL {
        *self.handle.as_ref().expect("global memory handle exists")
    }

    fn clipboard_handle(&self) -> HANDLE {
        HANDLE(self.handle().0)
    }

    /// `SetClipboardData` 成功后系统接管 `HGLOBAL`，此后本对象不能再释放它。
    fn transfer_to_clipboard(mut self) {
        self.handle = None;
    }
}

impl Drop for OwnedGlobalMemory {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            unsafe {
                let _ = GlobalFree(Some(handle));
            }
        }
    }
}

struct LockedMemory<T> {
    memory: HGLOBAL,
    ptr: *mut T,
}

impl<T> LockedMemory<T> {
    unsafe fn lock(memory: HGLOBAL) -> Result<Self, String> {
        let ptr = unsafe { GlobalLock(memory) } as *mut T;
        if ptr.is_null() {
            return Err("锁定剪贴板内存失败".to_string());
        }
        Ok(Self { memory, ptr })
    }

    fn as_ptr(&self) -> *const T {
        self.ptr as *const T
    }

    fn as_mut_ptr(&self) -> *mut T {
        self.ptr
    }
}

impl<T> Drop for LockedMemory<T> {
    fn drop(&mut self) {
        unsafe {
            let _ = GlobalUnlock(self.memory);
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum ClipboardFormatSnapshotAction {
    TakeFormat,
    SkipFormat,
    StopSnapshot,
}

/// 将最终文本写入剪贴板，并按配置决定是否发送粘贴快捷键。
///
/// 该函数会短暂占用系统剪贴板。调用方必须保留最终文本的用户兜底提示：
/// 如果自动粘贴失败或原剪贴板无法完整恢复，用户仍应知道文本已经复制，可手动粘贴。
pub fn output_text(
    text: &str,
    typing: &TypingConfig,
    mut on_output_sent: impl FnMut(),
) -> Result<OutputResult, String> {
    if text.trim().is_empty() {
        return Ok(OutputResult::ok());
    }
    app_log::info(format!(
        "准备输出文本: chars={}, method={}, restore_clipboard={}, clipboard_restore_delay_ms={}, effective_restore_delay_ms={}, clipboard_snapshot_max_bytes={}",
        text.chars().count(),
        typing.paste_method,
        typing.restore_clipboard_after_paste,
        typing.clipboard_restore_delay_ms,
        effective_clipboard_restore_delay_ms(typing),
        typing.clipboard_snapshot_max_bytes
    ));
    let original_clipboard =
        if typing.restore_clipboard_after_paste && typing.paste_method != "clipboard_only" {
            match read_clipboard_backup_with_retry(typing) {
                Ok(value) => value,
                Err(err) => {
                    app_log::warn(format!("备份剪贴板失败，将继续输出: {}", err));
                    ClipboardBackup::Empty
                }
            }
        } else {
            ClipboardBackup::Empty
        };
    if let ClipboardBackup::Snapshot(snapshot) = &original_clipboard {
        app_log::info(format!(
            "剪贴板快照完成: clipboard_snapshot_formats={}, clipboard_skipped_formats={}, clipboard_resynthesized_formats={}, clipboard_snapshot_bytes={}, clipboard_snapshot_max_bytes={}",
            snapshot.formats.len(),
            snapshot.skipped_formats,
            snapshot.resynthesized_formats,
            snapshot.total_bytes,
            typing.clipboard_snapshot_max_bytes
        ));
    }

    let written_at = write_clipboard_text_with_retry(text, typing)?;
    if typing.paste_method == "clipboard_only" {
        app_log::info("文本已写入剪贴板: method=clipboard_only");
        on_output_sent();
        return Ok(OutputResult::ok());
    }
    // 粘贴延迟从文本写入剪贴板那一刻算起；读回校验遇到占用而重试的时间不再额外叠加。
    thread::sleep(remaining_paste_delay(
        typing.paste_delay_ms,
        written_at.elapsed(),
    ));
    ensure_clipboard_text_ready_for_paste(text, typing)?;
    match typing.paste_method.as_str() {
        "shift_insert" => send_shortcut(VK_SHIFT, VK_INSERT, true),
        _ => send_shortcut(VK_CONTROL, VK_V, false),
    }

    app_log::info(format!(
        "粘贴快捷键已发送: method={}, delay_ms={}",
        typing.paste_method, typing.paste_delay_ms
    ));
    on_output_sent();
    if let ClipboardBackup::Snapshot(original) = original_clipboard {
        thread::sleep(clipboard_restore_delay(typing));
        match write_clipboard_snapshot_with_retry(&original, typing) {
            Ok(()) => {
                app_log::info(format!(
                    "发送粘贴快捷键后已恢复原剪贴板: clipboard_snapshot_formats={}, clipboard_skipped_formats={}, clipboard_resynthesized_formats={}, clipboard_snapshot_bytes={}, clipboard_restore_delay_ms={}",
                    original.formats.len(),
                    original.skipped_formats,
                    original.resynthesized_formats,
                    original.total_bytes,
                    typing.clipboard_restore_delay_ms
                ));
                if original.skipped_formats > 0 {
                    let warning =
                        "原剪贴板内容较大或包含特殊格式，已恢复可备份部分，部分格式未备份。"
                            .to_string();
                    // 静默提示：识别文本已正常粘贴，不打断用户，也不按告警级别记录。
                    app_log::info(&warning);
                    Ok(OutputResult::warning(
                        warning,
                        WARNING_CLIPBOARD_PARTIAL_RESTORE,
                    ))
                } else {
                    Ok(OutputResult::ok())
                }
            }
            Err(err) => {
                app_log::warn(format!("恢复原剪贴板失败: {}", err));
                Ok(OutputResult::warning(
                    "已发送粘贴快捷键，但恢复原剪贴板失败。",
                    WARNING_CLIPBOARD_RESTORE_FAILED,
                ))
            }
        }
    } else if matches!(original_clipboard, ClipboardBackup::NonRestorable) {
        let warning =
            "已发送粘贴快捷键；原剪贴板内容较大或包含暂不支持恢复的格式，当前剪贴板保留识别文本。"
                .to_string();
        app_log::warn(&warning);
        Ok(OutputResult::warning(
            warning,
            WARNING_CLIPBOARD_NON_RESTORABLE,
        ))
    } else {
        Ok(OutputResult::ok())
    }
}

pub fn is_quiet_output_warning_code(code: Option<&str>) -> bool {
    matches!(code, Some(WARNING_CLIPBOARD_PARTIAL_RESTORE))
}

/// 仅复制文本到剪贴板，不发送任何粘贴快捷键。
///
/// 用于自动粘贴失败后的兜底路径，避免在不确定目标窗口状态时继续模拟按键。
pub fn copy_text_to_clipboard(text: &str) -> Result<(), String> {
    write_clipboard_text_with_retry(text, &TypingConfig::default()).map(|_| ())
}

fn read_clipboard_backup_with_retry(typing: &TypingConfig) -> Result<ClipboardBackup, String> {
    with_clipboard_retry(typing, || read_clipboard_backup(typing))
}

/// 写入并校验文本，返回文本实际写入剪贴板的时刻。
fn write_clipboard_text_with_retry(text: &str, typing: &TypingConfig) -> Result<Instant, String> {
    with_clipboard_retry(typing, || write_clipboard_text_verified(text))
}

fn read_clipboard_text_with_retry(typing: &TypingConfig) -> Result<String, String> {
    with_clipboard_retry(typing, read_clipboard_text)
}

fn write_clipboard_snapshot_with_retry(
    snapshot: &ClipboardSnapshot,
    typing: &TypingConfig,
) -> Result<(), String> {
    with_clipboard_retry(typing, || write_clipboard_snapshot(snapshot))
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

fn ensure_clipboard_text_ready_for_paste(text: &str, typing: &TypingConfig) -> Result<(), String> {
    match read_clipboard_text_with_retry(typing) {
        Ok(actual) if clipboard_text_ready_for_paste(text, &actual) => Ok(()),
        Ok(_) => {
            app_log::warn("粘贴前剪贴板内容已变化，重新写入本次识别文本。");
            write_clipboard_text_with_retry(text, typing).map(|_| ())
        }
        Err(err) => {
            app_log::warn(format!(
                "粘贴前读取剪贴板失败，将重新写入本次识别文本: {}",
                err
            ));
            write_clipboard_text_with_retry(text, typing).map(|_| ())
        }
    }
}

fn remaining_paste_delay(paste_delay_ms: u64, elapsed_since_write: Duration) -> Duration {
    Duration::from_millis(paste_delay_ms).saturating_sub(elapsed_since_write)
}

fn clipboard_text_ready_for_paste(expected: &str, actual: &str) -> bool {
    actual == expected
}

fn read_clipboard_backup(typing: &TypingConfig) -> Result<ClipboardBackup, String> {
    let _clipboard = ClipboardGuard::open()?;
    let format_count = unsafe { CountClipboardFormats() };
    if format_count == 0 {
        return Ok(ClipboardBackup::Empty);
    }
    let mut formats = Vec::new();
    let mut skipped_format_ids = Vec::new();
    let mut total_bytes = 0usize;
    let max_bytes = usize::try_from(typing.clipboard_snapshot_max_bytes).unwrap_or(usize::MAX);
    let mut budget_exhausted = false;
    let mut format = 0;
    loop {
        format = unsafe { EnumClipboardFormats(format) };
        if format == 0 {
            break;
        }
        // 额度用完后不再读取数据，但继续记下剩余格式，才能判断它们是否可由系统重新合成。
        if budget_exhausted || is_known_non_memory_clipboard_format(format) {
            skipped_format_ids.push(format);
            continue;
        }
        let Some((memory, size)) = clipboard_format_memory(format) else {
            skipped_format_ids.push(format);
            continue;
        };
        match clipboard_format_snapshot_action(total_bytes, size, max_bytes) {
            ClipboardFormatSnapshotAction::TakeFormat => match copy_global_memory(memory, size) {
                Some(bytes) => {
                    total_bytes += bytes.len();
                    formats.push(ClipboardFormatBackup { format, bytes });
                }
                None => skipped_format_ids.push(format),
            },
            ClipboardFormatSnapshotAction::SkipFormat => skipped_format_ids.push(format),
            ClipboardFormatSnapshotAction::StopSnapshot => {
                skipped_format_ids.push(format);
                budget_exhausted = true;
            }
        }
    }

    if formats.is_empty() {
        return Ok(ClipboardBackup::NonRestorable);
    }
    let captured_format_ids = formats.iter().map(|item| item.format).collect::<Vec<_>>();
    let resynthesized_formats = skipped_format_ids
        .iter()
        .filter(|format| is_resynthesized_after_restore(**format, &captured_format_ids))
        .count();
    Ok(ClipboardBackup::Snapshot(ClipboardSnapshot {
        formats,
        skipped_formats: skipped_format_ids.len() - resynthesized_formats,
        resynthesized_formats,
        total_bytes,
    }))
}

/// 没有备份的格式里，哪些在恢复后会由系统从已备份格式自动合成，因而不算丢失。
///
/// Windows 会在位图族（CF_BITMAP / CF_DIB / CF_DIBV5，以及由它们派生的 CF_PALETTE）和
/// 文本族（CF_TEXT / CF_OEMTEXT / CF_UNICODETEXT / CF_LOCALE）内部互相合成。
/// 剪贴板里有截图时最常见：DIB 已备份，句柄型的 CF_BITMAP 没法按内存复制，但恢复后照样可用。
/// 把它们算成"部分格式未备份"会让每次输入都带上一条并不成立的提示。
///
/// 文本族只认"已备份 Unicode 文本"这一个方向：从 ANSI 文本反推 Unicode 会丢掉代码页之外的字符，
/// 那种情况仍按丢失计。
fn is_resynthesized_after_restore(skipped_format: u32, captured_formats: &[u32]) -> bool {
    let captured = |candidates: &[u32]| {
        candidates
            .iter()
            .any(|item| captured_formats.contains(item))
    };
    match skipped_format {
        CF_BITMAP | CF_PALETTE | CF_DIB | CF_DIBV5 => captured(&[CF_DIB, CF_DIBV5]),
        CF_TEXT | CF_OEMTEXT | CF_LOCALE => captured(&[CF_UNICODETEXT]),
        _ => false,
    }
}

fn read_clipboard_text() -> Result<String, String> {
    let _clipboard = ClipboardGuard::open()?;
    unsafe {
        if IsClipboardFormatAvailable(CF_UNICODETEXT).is_err() {
            return Ok(String::new());
        }
        let handle =
            GetClipboardData(CF_UNICODETEXT).map_err(|err| format!("读取剪贴板失败: {}", err))?;
        read_clipboard_text_from_handle(handle)
    }
}

/// 取出某个格式的全局内存句柄和大小。
///
/// 每个格式只调用一次 `GetClipboardData`：合成格式在取数据时才真正渲染，
/// 剪贴板里是大截图时重复调用会把几兆数据白白转换两遍。
fn clipboard_format_memory(format: u32) -> Option<(HGLOBAL, usize)> {
    let handle = unsafe { GetClipboardData(format) }.ok()?;
    if handle.is_invalid() {
        return None;
    }
    let memory = HGLOBAL(handle.0);
    let size = unsafe { GlobalSize(memory) };
    if size == 0 {
        return None;
    }
    Some((memory, size))
}

fn copy_global_memory(memory: HGLOBAL, size: usize) -> Option<Vec<u8>> {
    let locked = unsafe { LockedMemory::<u8>::lock(memory) }.ok()?;
    Some(unsafe { std::slice::from_raw_parts(locked.as_ptr(), size) }.to_vec())
}

fn clipboard_format_snapshot_action(
    current_total_bytes: usize,
    format_bytes: usize,
    max_total_bytes: usize,
) -> ClipboardFormatSnapshotAction {
    if format_bytes > max_total_bytes {
        return ClipboardFormatSnapshotAction::SkipFormat;
    }
    if current_total_bytes.saturating_add(format_bytes) > max_total_bytes {
        return ClipboardFormatSnapshotAction::StopSnapshot;
    }
    ClipboardFormatSnapshotAction::TakeFormat
}

fn read_clipboard_text_from_handle(handle: HANDLE) -> Result<String, String> {
    if handle.is_invalid() {
        return Ok(String::new());
    }
    unsafe {
        let memory = HGLOBAL(handle.0);
        let size = GlobalSize(memory);
        let locked = LockedMemory::<u16>::lock(memory)?;
        let units = size / size_of::<u16>();
        let slice = std::slice::from_raw_parts(locked.as_ptr(), units);
        let len = slice.iter().position(|value| *value == 0).unwrap_or(units);
        Ok(String::from_utf16_lossy(&slice[..len]))
    }
}

fn is_known_non_memory_clipboard_format(format: u32) -> bool {
    matches!(
        format,
        CF_BITMAP
            | CF_METAFILEPICT
            | CF_PALETTE
            | CF_ENHMETAFILE
            | CF_OWNERDISPLAY
            | CF_DSPBITMAP
            | CF_DSPMETAFILEPICT
            | CF_DSPENHMETAFILE
    ) || (CF_GDIOBJFIRST..=CF_GDIOBJLAST).contains(&format)
}

fn write_clipboard_text(text: &str) -> Result<(), String> {
    let mut utf16: Vec<u16> = text.encode_utf16().collect();
    utf16.push(0);
    let byte_len = utf16.len() * size_of::<u16>();
    unsafe {
        let memory = OwnedGlobalMemory::alloc(byte_len)?;
        {
            let locked = LockedMemory::<u16>::lock(memory.handle())?;
            std::ptr::copy_nonoverlapping(utf16.as_ptr(), locked.as_mut_ptr(), utf16.len());
        }

        let _clipboard = ClipboardGuard::open()?;
        EmptyClipboard().map_err(|err| format!("清空剪贴板失败: {}", err))?;
        SetClipboardData(CF_UNICODETEXT, Some(memory.clipboard_handle()))
            .map_err(|err| format!("写入剪贴板失败: {}", err))?;
        memory.transfer_to_clipboard();
        Ok(())
    }
}

fn write_clipboard_snapshot(snapshot: &ClipboardSnapshot) -> Result<(), String> {
    if snapshot.formats.is_empty() {
        return Err("没有可恢复的剪贴板格式".to_string());
    }
    unsafe {
        let _clipboard = ClipboardGuard::open()?;
        EmptyClipboard().map_err(|err| format!("清空剪贴板失败: {}", err))?;
        for item in &snapshot.formats {
            set_clipboard_memory(item.format, &item.bytes)?;
        }
        Ok(())
    }
}

fn write_clipboard_text_verified(text: &str) -> Result<Instant, String> {
    write_clipboard_text(text)?;
    let written_at = Instant::now();
    match clipboard_write_readback_verdict(text, read_clipboard_text())? {
        true => {
            app_log::info(format!(
                "剪贴板写入已确认: chars={}, readback_verified=true",
                text.chars().count()
            ));
        }
        false => {
            app_log::warn("剪贴板写入后读回校验失败，已接受写入结果继续输出。");
        }
    }
    Ok(written_at)
}

fn clipboard_write_readback_verdict(
    expected: &str,
    readback: Result<String, String>,
) -> Result<bool, String> {
    match readback {
        Ok(actual) if actual == expected => Ok(true),
        Ok(_) => Err("剪贴板写入后校验失败：读取内容与目标文本不一致".to_string()),
        Err(err) => {
            app_log::warn(format!("剪贴板写入后读回失败: {}", err));
            Ok(false)
        }
    }
}

fn set_clipboard_memory(format: u32, bytes: &[u8]) -> Result<(), String> {
    if bytes.is_empty() {
        return Err("剪贴板格式内容为空，无法恢复".to_string());
    }
    unsafe {
        let memory = OwnedGlobalMemory::alloc(bytes.len())?;
        {
            let locked = LockedMemory::<u8>::lock(memory.handle())?;
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), locked.as_mut_ptr(), bytes.len());
        }

        SetClipboardData(format, Some(memory.clipboard_handle()))
            .map_err(|err| format!("恢复剪贴板格式失败: {}", err))?;
        memory.transfer_to_clipboard();
        Ok(())
    }
}

fn clipboard_restore_delay(typing: &TypingConfig) -> Duration {
    Duration::from_millis(effective_clipboard_restore_delay_ms(typing))
}

fn effective_clipboard_restore_delay_ms(typing: &TypingConfig) -> u64 {
    typing
        .clipboard_restore_delay_ms
        .max(MIN_RESTORE_DELAY_AFTER_PASTE_MS)
}

fn send_shortcut(modifier: VIRTUAL_KEY, key: VIRTUAL_KEY, key_extended: bool) {
    send_key_event(modifier, false, false);
    thread::sleep(KEY_INTERVAL);
    send_key_event(key, false, key_extended);
    thread::sleep(KEY_INTERVAL);
    send_key_event(key, true, key_extended);
    thread::sleep(KEY_INTERVAL);
    send_key_event(modifier, true, false);
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
    use super::{
        clipboard_format_snapshot_action, clipboard_restore_delay, clipboard_text_ready_for_paste,
        clipboard_write_readback_verdict, effective_clipboard_restore_delay_ms,
        is_known_non_memory_clipboard_format, is_quiet_output_warning_code,
        is_resynthesized_after_restore, output_text, read_clipboard_backup_with_retry,
        read_clipboard_text, remaining_paste_delay, write_clipboard_snapshot_with_retry,
        write_clipboard_text_verified, ClipboardBackup, ClipboardFormatBackup,
        ClipboardFormatSnapshotAction, ClipboardGuard, ClipboardSnapshot, LockedMemory,
        OwnedGlobalMemory, CF_BITMAP, CF_DIB, CF_DIBV5, CF_ENHMETAFILE, CF_PALETTE, CF_TEXT,
        CF_UNICODETEXT, MIN_RESTORE_DELAY_AFTER_PASTE_MS, WARNING_CLIPBOARD_NON_RESTORABLE,
        WARNING_CLIPBOARD_PARTIAL_RESTORE,
    };
    use crate::config::TypingConfig;
    use std::time::Duration;
    use windows::Win32::Foundation::GlobalFree;
    use windows::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, GetClipboardData, GetClipboardSequenceNumber,
        IsClipboardFormatAvailable, OpenClipboard,
    };

    #[test]
    fn restore_delay_uses_independent_clipboard_restore_setting() {
        let typing = TypingConfig {
            paste_delay_ms: 0,
            clipboard_restore_delay_ms: 300,
            ..TypingConfig::default()
        };
        assert_eq!(
            clipboard_restore_delay(&typing),
            Duration::from_millis(MIN_RESTORE_DELAY_AFTER_PASTE_MS)
        );
        let typing = TypingConfig {
            paste_delay_ms: 120,
            clipboard_restore_delay_ms: 2_400,
            ..TypingConfig::default()
        };
        assert_eq!(
            clipboard_restore_delay(&typing),
            Duration::from_millis(2_400)
        );
    }

    #[test]
    fn effective_restore_delay_never_goes_below_paste_safety_floor() {
        let typing = TypingConfig {
            clipboard_restore_delay_ms: 0,
            ..TypingConfig::default()
        };
        assert_eq!(
            effective_clipboard_restore_delay_ms(&typing),
            MIN_RESTORE_DELAY_AFTER_PASTE_MS
        );
    }

    #[test]
    fn owned_global_memory_can_transfer_ownership_after_locking() {
        unsafe {
            let memory = OwnedGlobalMemory::alloc(2).expect("allocate global memory");
            {
                let locked = LockedMemory::<u16>::lock(memory.handle()).expect("lock memory");
                std::ptr::write(locked.as_mut_ptr(), 42);
            }

            let handle = memory.handle();
            memory.transfer_to_clipboard();
            {
                let locked = LockedMemory::<u16>::lock(handle).expect("lock transferred memory");
                assert_eq!(*locked.as_ptr(), 42);
            }
            let _ = GlobalFree(Some(handle));
        }
    }

    #[test]
    fn clipboard_text_must_still_match_before_paste() {
        assert!(clipboard_text_ready_for_paste("识别文本", "识别文本"));
        assert!(!clipboard_text_ready_for_paste("识别文本", "旧剪贴板文本"));
    }

    #[test]
    fn readback_failure_after_successful_write_is_non_fatal() {
        let result = clipboard_write_readback_verdict(
            "识别文本",
            Err("读取剪贴板失败: Thread does not have a clipboard open. (0x8007058A)".to_string()),
        );

        assert_eq!(result, Ok(false));
    }

    #[test]
    fn readback_mismatch_after_successful_write_stays_fatal() {
        let result = clipboard_write_readback_verdict("识别文本", Ok("其他剪贴板文本".to_string()));

        assert!(result.is_err());
    }

    #[test]
    fn empty_output_does_not_signal_output_sent() {
        let mut called = false;
        let result = output_text("", &TypingConfig::default(), || called = true);

        assert!(result.is_ok());
        assert!(!called);
    }

    #[test]
    fn snapshot_limit_skips_large_formats_and_stops_when_total_is_full() {
        assert_eq!(
            clipboard_format_snapshot_action(0, 9 * 1024 * 1024, 8 * 1024 * 1024),
            ClipboardFormatSnapshotAction::SkipFormat
        );
        assert_eq!(
            clipboard_format_snapshot_action(7 * 1024 * 1024, 2 * 1024 * 1024, 8 * 1024 * 1024),
            ClipboardFormatSnapshotAction::StopSnapshot
        );
        assert_eq!(
            clipboard_format_snapshot_action(2 * 1024 * 1024, 512 * 1024, 8 * 1024 * 1024),
            ClipboardFormatSnapshotAction::TakeFormat
        );
    }

    #[test]
    fn known_handle_clipboard_formats_are_not_memory_snapshotted() {
        assert!(is_known_non_memory_clipboard_format(2));
        assert!(is_known_non_memory_clipboard_format(9));
        assert!(is_known_non_memory_clipboard_format(14));
        assert!(!is_known_non_memory_clipboard_format(13));
        assert!(!is_known_non_memory_clipboard_format(15));
        assert!(!is_known_non_memory_clipboard_format(49350));
    }

    #[test]
    fn gdi_object_clipboard_formats_are_not_memory_snapshotted() {
        // CF_GDIOBJFIRST..=CF_GDIOBJLAST 里放的是 GDI 对象句柄，不能当全局内存读取。
        assert!(is_known_non_memory_clipboard_format(0x0300));
        assert!(is_known_non_memory_clipboard_format(0x03FF));
        assert!(!is_known_non_memory_clipboard_format(0x02FF));
        assert!(!is_known_non_memory_clipboard_format(0x0400));
    }

    #[test]
    fn formats_windows_resynthesizes_are_not_reported_as_lost() {
        // 剪贴板里是截图：DIB 已按内存备份，句柄型的 CF_BITMAP/CF_PALETTE 以及
        // 因额度没再读取的 CF_DIBV5，恢复后都能由系统从 DIB 重新合成。
        for skipped in [CF_BITMAP, CF_PALETTE, CF_DIBV5] {
            assert!(is_resynthesized_after_restore(skipped, &[CF_DIB]));
        }
        assert!(is_resynthesized_after_restore(CF_DIB, &[CF_DIBV5]));
        assert!(is_resynthesized_after_restore(CF_TEXT, &[CF_UNICODETEXT]));
    }

    #[test]
    fn formats_without_a_captured_source_still_count_as_lost() {
        // 只备份了文本时，位图族拿不回来。
        assert!(!is_resynthesized_after_restore(
            CF_BITMAP,
            &[CF_UNICODETEXT]
        ));
        // 图元文件互相合成的两种格式都是句柄，一个也备份不了。
        assert!(!is_resynthesized_after_restore(CF_ENHMETAFILE, &[CF_DIB]));
        // 从 ANSI 文本反推 Unicode 会丢字符，不能当作无损恢复。
        assert!(!is_resynthesized_after_restore(CF_UNICODETEXT, &[CF_TEXT]));
        // 应用私有的注册格式没有合成来源。
        assert!(!is_resynthesized_after_restore(
            49350,
            &[CF_DIB, CF_UNICODETEXT]
        ));
    }

    #[test]
    fn paste_delay_counts_from_the_clipboard_write() {
        assert_eq!(
            remaining_paste_delay(120, Duration::from_millis(20)),
            Duration::from_millis(100)
        );
        // 读回校验因剪贴板被占用而重试得比延迟还久时，不再额外等待。
        assert_eq!(
            remaining_paste_delay(120, Duration::from_millis(500)),
            Duration::ZERO
        );
        assert_eq!(remaining_paste_delay(0, Duration::ZERO), Duration::ZERO);
    }

    /// 手工回归：会短暂改写真实系统剪贴板（结束时恢复原内容），因此默认不运行。
    ///
    /// 运行：`cargo test --lib real_clipboard -- --ignored --nocapture`
    #[test]
    #[ignore = "touches the real system clipboard; run manually"]
    fn real_clipboard_open_is_exclusive_and_text_survives_owner_window() {
        let typing = TypingConfig::default();
        let original = read_clipboard_backup_with_retry(&typing).expect("snapshot clipboard");
        let fully_restorable = match &original {
            ClipboardBackup::Empty => true,
            ClipboardBackup::Snapshot(snapshot) => snapshot.skipped_formats == 0,
            ClipboardBackup::NonRestorable => false,
        };
        if !fully_restorable {
            eprintln!("skipped: 当前剪贴板内容无法完整恢复，不在其上运行测试。");
            return;
        }

        let text = "VoxType 剪贴板回归 clipboard regression";
        write_clipboard_text_verified(text).expect("write and verify text");
        let sequence = unsafe { GetClipboardSequenceNumber() };
        // 写入用的属主窗口此时已经销毁：内容必须还在，且销毁本身不产生新的剪贴板变更。
        assert_eq!(read_clipboard_text().expect("read text back"), text);
        assert_eq!(unsafe { GetClipboardSequenceNumber() }, sequence);

        {
            let _guard = ClipboardGuard::open().expect("open clipboard with owner window");
            let opened_by_other_thread = std::thread::spawn(|| unsafe {
                let opened = OpenClipboard(None).is_ok();
                if opened {
                    let _ = CloseClipboard();
                }
                opened
            })
            .join()
            .expect("contending thread");
            // 回归：用 NULL 打开时，这里会"打开成功"并夺走剪贴板，随后本线程读取报
            // ERROR_CLIPBOARD_NOT_OPEN (0x8007058A)。
            assert!(!opened_by_other_thread);
            assert!(unsafe { GetClipboardData(CF_UNICODETEXT) }.is_ok());
        }

        restore_real_clipboard(&original, &typing);
    }

    /// 手工回归：验证"截图在剪贴板里"时快照不再误报丢失，且恢复后位图句柄格式仍可用。
    ///
    /// 运行：`cargo test --lib real_clipboard -- --ignored --nocapture`
    #[test]
    #[ignore = "touches the real system clipboard; run manually"]
    fn real_clipboard_image_snapshot_restores_bitmap_through_synthesis() {
        let typing = TypingConfig::default();
        let original = read_clipboard_backup_with_retry(&typing).expect("snapshot clipboard");
        let fully_restorable = match &original {
            ClipboardBackup::Empty => true,
            ClipboardBackup::Snapshot(snapshot) => snapshot.skipped_formats == 0,
            ClipboardBackup::NonRestorable => false,
        };
        if !fully_restorable {
            eprintln!("skipped: 当前剪贴板内容无法完整恢复，不在其上运行测试。");
            return;
        }

        // 2x2 的 32 位 DIB：40 字节 BITMAPINFOHEADER + 16 字节像素。
        let mut dib = Vec::new();
        dib.extend_from_slice(&40u32.to_le_bytes());
        dib.extend_from_slice(&2i32.to_le_bytes());
        dib.extend_from_slice(&2i32.to_le_bytes());
        dib.extend_from_slice(&1u16.to_le_bytes());
        dib.extend_from_slice(&32u16.to_le_bytes());
        dib.extend_from_slice(&0u32.to_le_bytes());
        dib.extend_from_slice(&16u32.to_le_bytes());
        dib.extend_from_slice(&[0u8; 16]);
        dib.extend_from_slice(&[0x7Fu8; 16]);
        let image_only = ClipboardSnapshot {
            total_bytes: dib.len(),
            formats: vec![ClipboardFormatBackup {
                format: CF_DIB,
                bytes: dib,
            }],
            skipped_formats: 0,
            resynthesized_formats: 0,
        };
        write_clipboard_snapshot_with_retry(&image_only, &typing).expect("put image on clipboard");

        let ClipboardBackup::Snapshot(snapshot) =
            read_clipboard_backup_with_retry(&typing).expect("snapshot image clipboard")
        else {
            panic!("image clipboard should produce a snapshot");
        };
        // 系统枚举时会带上合成出来的 CF_BITMAP；它是句柄，备份不了，但不应算丢失。
        assert!(snapshot.formats.iter().any(|item| item.format == CF_DIB));
        assert!(snapshot.resynthesized_formats >= 1);
        assert_eq!(snapshot.skipped_formats, 0);

        write_clipboard_snapshot_with_retry(&snapshot, &typing).expect("restore image snapshot");
        {
            let _guard = ClipboardGuard::open().expect("open clipboard");
            assert!(unsafe { IsClipboardFormatAvailable(CF_BITMAP) }.is_ok());
            assert!(unsafe { GetClipboardData(CF_BITMAP) }.is_ok());
        }

        restore_real_clipboard(&original, &typing);
    }

    fn restore_real_clipboard(original: &ClipboardBackup, typing: &TypingConfig) {
        match original {
            ClipboardBackup::Snapshot(snapshot) => {
                write_clipboard_snapshot_with_retry(snapshot, typing).expect("restore clipboard");
            }
            _ => {
                let _guard = ClipboardGuard::open().expect("open clipboard to clear");
                unsafe { EmptyClipboard() }.expect("clear clipboard");
            }
        }
    }

    #[test]
    fn only_partial_restore_warning_is_quiet() {
        assert!(is_quiet_output_warning_code(Some(
            WARNING_CLIPBOARD_PARTIAL_RESTORE
        )));
        assert!(!is_quiet_output_warning_code(Some(
            WARNING_CLIPBOARD_NON_RESTORABLE
        )));
        assert!(!is_quiet_output_warning_code(None));
    }
}
