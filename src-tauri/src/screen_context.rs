use crate::{app_log, config::ScreenContextConfig};
use serde::Serialize;
use std::{
    borrow::Cow,
    ffi::c_void,
    fmt, mem,
    sync::{
        mpsc::{self, Receiver, RecvTimeoutError, TryRecvError},
        Arc, Mutex, OnceLock,
    },
    thread,
    time::{Duration, Instant},
};
use tokio::sync::OnceCell;
use windows::{
    core::{Error as WindowsError, HSTRING},
    Foundation::{PropertyType, PropertyValue},
    Globalization::Language,
    Graphics::Imaging::{
        BitmapAlphaMode, BitmapEncoder, BitmapPixelFormat, BitmapPropertySet, BitmapTypedValue,
        SoftwareBitmap,
    },
    Media::Ocr::OcrEngine,
    Storage::Streams::{DataReader, DataWriter, InMemoryRandomAccessStream},
    Win32::{
        Foundation::{RECT, RPC_E_CHANGED_MODE},
        Graphics::Gdi::{
            BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC,
            GetDIBits, GetMonitorInfoW, MonitorFromWindow, ReleaseDC, SelectObject, BITMAPINFO,
            BITMAPINFOHEADER, BI_RGB, CAPTUREBLT, DIB_RGB_COLORS, HBITMAP, HDC, HGDIOBJ,
            MONITORINFO, MONITOR_DEFAULTTONEAREST, SRCCOPY,
        },
        System::WinRT::{RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED},
        UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowRect},
    },
};

const SCREEN_IMAGE_MAX_EDGE: i32 = 1_600;
const SCREEN_IMAGE_JPEG_QUALITY: f32 = 0.80;
const SCREEN_IMAGE_FALLBACK_QUALITY: f32 = 0.65;
const SCREEN_IMAGE_FALLBACK_EDGE: i32 = 1_200;
const SCREEN_IMAGE_MAX_BYTES: usize = 1_500_000;

pub const WARNING_SCREEN_OCR_EMPTY: &str = "SCREEN_OCR_EMPTY";
pub const WARNING_SCREEN_OCR_FAILED_WITH_SCREENSHOT: &str = "SCREEN_OCR_FAILED_WITH_SCREENSHOT";
pub const WARNING_SCREEN_SCREENSHOT_ENCODE_FAILED: &str = "SCREEN_SCREENSHOT_ENCODE_FAILED";
pub const WARNING_SCREEN_SCREENSHOT_TOO_LARGE: &str = "SCREEN_SCREENSHOT_TOO_LARGE";

pub type ScreenContextReceiver = Receiver<Result<ScreenContextSnapshot, String>>;
pub type ScreenImageReceiver = Receiver<Result<EncodedScreenImage, String>>;

pub struct ScreenContextReceivers {
    pub ocr_rx: ScreenContextReceiver,
    pub image_rx: Option<ScreenImageReceiver>,
}

#[derive(Clone)]
pub struct ScreenContextSnapshot {
    pub text: String,
    pub elapsed_ms: u128,
    pub selected_language: String,
    pub available_languages: Vec<String>,
    pub image_width: i32,
    pub image_height: i32,
}

impl fmt::Debug for ScreenContextSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScreenContextSnapshot")
            .field("text", &"<redacted>")
            .field("text_chars", &self.text.chars().count())
            .field("elapsed_ms", &self.elapsed_ms)
            .field("selected_language", &self.selected_language)
            .field("available_languages", &self.available_languages)
            .field("image_width", &self.image_width)
            .field("image_height", &self.image_height)
            .finish()
    }
}

#[derive(Clone)]
pub struct EncodedScreenImage {
    pub mime_type: String,
    pub data_base64: String,
    pub bytes_len: usize,
    pub width: i32,
    pub height: i32,
    pub source_width: i32,
    pub source_height: i32,
    pub elapsed_ms: u128,
}

impl EncodedScreenImage {
    pub fn data_url(&self) -> String {
        format!("data:{};base64,{}", self.mime_type, self.data_base64)
    }
}

impl fmt::Debug for EncodedScreenImage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EncodedScreenImage")
            .field("mime_type", &self.mime_type)
            .field("data_base64", &"<redacted>")
            .field("bytes_len", &self.bytes_len)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("source_width", &self.source_width)
            .field("source_height", &self.source_height)
            .field("elapsed_ms", &self.elapsed_ms)
            .finish()
    }
}

#[derive(Clone, Default)]
pub struct PostEditScreenContext {
    pub ocr_text: Option<String>,
    pub image: Option<EncodedScreenImage>,
}

impl fmt::Debug for PostEditScreenContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PostEditScreenContext")
            .field(
                "ocr_text_chars",
                &self.ocr_text.as_ref().map(|text| text.chars().count()),
            )
            .field("image", &self.image)
            .finish()
    }
}

#[derive(Clone, Serialize)]
pub struct ScreenContextTestResult {
    pub available_languages: Vec<String>,
    pub selected_language: Option<String>,
    pub elapsed_ms: u128,
    pub text: String,
    pub text_chars: usize,
    pub image_width: i32,
    pub image_height: i32,
    pub warning: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning_code: Option<String>,
    pub llm_mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screenshot_data_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screenshot_mime: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screenshot_bytes: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screenshot_width: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screenshot_height: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screenshot_elapsed_ms: Option<u128>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screenshot_warning: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screenshot_warning_code: Option<String>,
}

impl fmt::Debug for ScreenContextTestResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScreenContextTestResult")
            .field("available_languages", &self.available_languages)
            .field("selected_language", &self.selected_language)
            .field("elapsed_ms", &self.elapsed_ms)
            .field("text", &"<redacted>")
            .field("text_chars", &self.text_chars)
            .field("image_width", &self.image_width)
            .field("image_height", &self.image_height)
            .field("warning", &self.warning)
            .field("warning_code", &self.warning_code)
            .field("llm_mode", &self.llm_mode)
            .field(
                "screenshot_data_url",
                &self.screenshot_data_url.as_ref().map(|_| "<redacted>"),
            )
            .field("screenshot_mime", &self.screenshot_mime)
            .field("screenshot_bytes", &self.screenshot_bytes)
            .field("screenshot_width", &self.screenshot_width)
            .field("screenshot_height", &self.screenshot_height)
            .field("screenshot_elapsed_ms", &self.screenshot_elapsed_ms)
            .field("screenshot_warning", &self.screenshot_warning)
            .field("screenshot_warning_code", &self.screenshot_warning_code)
            .finish()
    }
}

#[derive(Clone)]
struct CapturedImage {
    pixels: Vec<u8>,
    width: i32,
    height: i32,
}

pub fn spawn_capture(
    config: &ScreenContextConfig,
    capture_image_for_llm: bool,
) -> Option<ScreenContextReceivers> {
    if !config.enabled {
        return None;
    }
    let started_at = Instant::now();
    let config = config.clone();
    let (ocr_tx, ocr_rx) = mpsc::channel();
    let (image_tx, image_rx) = if capture_image_for_llm {
        let (tx, rx) = mpsc::channel();
        (Some(tx), Some(rx))
    } else {
        (None, None)
    };
    thread::spawn(move || {
        let captured = match capture_context_bitmap(&config) {
            Ok(image) => Arc::new(image),
            Err(err) => {
                app_log::warn(format!("屏幕上下文捕获失败，已跳过: {}", err));
                let _ = ocr_tx.send(Err(err.clone()));
                if let Some(tx) = image_tx {
                    let _ = tx.send(Err(err));
                }
                return;
            }
        };

        if let Some(tx) = image_tx {
            let image_for_encode = Arc::clone(&captured);
            thread::spawn(move || {
                let encoded = encode_screen_image(&image_for_encode, started_at);
                match &encoded {
                    Ok(image) => app_log::info(format!(
                        "屏幕截图已编码: mime={}, bytes={}, size={}x{}, source={}x{}, elapsed_ms={}",
                        image.mime_type,
                        image.bytes_len,
                        image.width,
                        image.height,
                        image.source_width,
                        image.source_height,
                        image.elapsed_ms
                    )),
                    Err(err) => app_log::warn(format!("屏幕截图编码失败，已跳过: {}", err)),
                }
                let _ = tx.send(encoded);
            });
        }

        let ocr_result = recognize_image_context(&captured, &config, started_at);
        match &ocr_result {
            Ok(snapshot) => app_log::info(format!(
                "屏幕 OCR 上下文已生成: chars={}, elapsed_ms={}, language={}, image={}x{}",
                snapshot.text.chars().count(),
                snapshot.elapsed_ms,
                snapshot.selected_language,
                snapshot.image_width,
                snapshot.image_height
            )),
            Err(err) => app_log::warn(format!("屏幕 OCR 识别失败，已跳过: {}", err)),
        }
        let _ = ocr_tx.send(ocr_result);
    });
    Some(ScreenContextReceivers { ocr_rx, image_rx })
}

/// 延后解析的屏幕上下文（ASR 使用 OCR 文字；LLM 润色按设置使用压缩截图或 OCR 文字）。
///
/// OCR 只影响首包 payload，不影响 WebSocket 握手，所以建连可以和 OCR 等待并行；
/// 同一次截图并行跑 OCR 与图片编码，不增加 ASR 首包等待时间。
pub struct PendingScreenContext {
    ocr_receiver: Mutex<Option<ScreenContextReceiver>>,
    image_receiver: Mutex<Option<ScreenImageReceiver>>,
    timeout_ms: u64,
    resolved_ocr: OnceCell<Option<String>>,
    late_ocr: OnceLock<Option<String>>,
    resolved_image: OnceLock<Option<EncodedScreenImage>>,
}

/// 首包阶段等待 OCR 的结果。
enum ContextWait {
    /// 在等待上限内有了结论；`None` 表示识别为空或失败。
    Finished(Option<ScreenContextSnapshot>),
    /// 等待上限已到但 OCR 还在跑，结果稍后仍可能到达。
    TimedOut,
}

impl PendingScreenContext {
    pub fn new(receivers: Option<ScreenContextReceivers>, timeout_ms: u64) -> Arc<Self> {
        let (ocr_receiver, image_receiver) = match receivers {
            Some(receivers) => (Some(receivers.ocr_rx), receivers.image_rx),
            None => (None, None),
        };
        Arc::new(Self {
            ocr_receiver: Mutex::new(ocr_receiver),
            image_receiver: Mutex::new(image_receiver),
            timeout_ms,
            resolved_ocr: OnceCell::new(),
            late_ocr: OnceLock::new(),
            resolved_image: OnceLock::new(),
        })
    }

    /// ASR 首包使用：最多等待配置的上限，只等一次。
    pub async fn resolve(&self) -> Option<String> {
        self.resolved_ocr
            .get_or_init(|| async {
                let receiver = self.take_ocr_receiver()?;
                let timeout_ms = self.timeout_ms;
                // OCR 等待是阻塞的 recv_timeout，放到 blocking 线程，避免占住 ASR 的 runtime。
                let (wait, receiver) = tokio::task::spawn_blocking(move || {
                    let wait = wait_for_context(&receiver, timeout_ms);
                    (wait, receiver)
                })
                .await
                .ok()?;
                match wait {
                    ContextWait::Finished(snapshot) => snapshot.map(|snapshot| snapshot.text),
                    ContextWait::TimedOut => {
                        // 迟到的结果赶不上首包，但润色发生在说完话之后，留着接收端给它用。
                        if let Ok(mut slot) = self.ocr_receiver.lock() {
                            *slot = Some(receiver);
                        }
                        None
                    }
                }
            })
            .await
            .clone()
    }

    /// LLM 润色使用：取已就绪的压缩截图和 OCR 文字（首包超时后迟到的 OCR 一并取用）。
    ///
    /// 这里不做任何额外等待。大画幅截图的 OCR 与图片编码通常只花几百毫秒，
    /// 而润色在说完话之后才开始，无需阻塞等待。
    pub async fn resolve_for_post_edit(&self) -> PostEditScreenContext {
        let ocr_text = self.resolve_ocr_for_post_edit().await;
        let image = self.resolve_image_for_post_edit();
        PostEditScreenContext { ocr_text, image }
    }

    async fn resolve_ocr_for_post_edit(&self) -> Option<String> {
        if let Some(text) = self.resolve().await {
            return Some(text);
        }
        self.late_ocr
            .get_or_init(|| {
                let receiver = self.take_ocr_receiver()?;
                match receiver.try_recv() {
                    Ok(Ok(snapshot)) if !snapshot.text.trim().is_empty() => {
                        app_log::info(format!(
                            "屏幕 OCR 迟到结果已用于润色参考: chars={}, elapsed_ms={}",
                            snapshot.text.chars().count(),
                            snapshot.elapsed_ms
                        ));
                        Some(snapshot.text)
                    }
                    _ => None,
                }
            })
            .clone()
    }

    fn resolve_image_for_post_edit(&self) -> Option<EncodedScreenImage> {
        self.resolved_image
            .get_or_init(|| {
                let receiver = self.take_image_receiver()?;
                match receiver.try_recv() {
                    Ok(Ok(image)) => {
                        app_log::info(format!(
                            "屏幕截图已用于润色参考: mime={}, bytes={}, size={}x{}, source={}x{}, elapsed_ms={}",
                            image.mime_type,
                            image.bytes_len,
                            image.width,
                            image.height,
                            image.source_width,
                            image.source_height,
                            image.elapsed_ms
                        ));
                        Some(image)
                    }
                    Ok(Err(err)) => {
                        app_log::warn(format!("屏幕截图编码不可用，将回退 OCR 文字参考: {}", err));
                        None
                    }
                    Err(TryRecvError::Empty) => {
                        app_log::info("屏幕截图编码尚未完成，润色将回退 OCR 文字参考。");
                        None
                    }
                    Err(TryRecvError::Disconnected) => {
                        app_log::warn("屏幕截图编码线程提前结束，已跳过。");
                        None
                    }
                }
            })
            .clone()
    }

    fn take_ocr_receiver(&self) -> Option<ScreenContextReceiver> {
        self.ocr_receiver
            .lock()
            .ok()
            .and_then(|mut slot| slot.take())
    }

    fn take_image_receiver(&self) -> Option<ScreenImageReceiver> {
        self.image_receiver
            .lock()
            .ok()
            .and_then(|mut slot| slot.take())
    }
}

fn wait_for_context(receiver: &ScreenContextReceiver, timeout_ms: u64) -> ContextWait {
    // 500ms 是近期实测的命中率/首字延迟折中：OCR 超时只跳过上下文，不应阻断录音或最终输出。
    // 维护依据见 docs/asr-quality-latency-guardrails.md。
    match receiver.recv_timeout(Duration::from_millis(timeout_ms.max(1))) {
        Ok(Ok(snapshot)) if !snapshot.text.trim().is_empty() => {
            ContextWait::Finished(Some(snapshot))
        }
        Ok(Ok(snapshot)) => {
            app_log::info(format!(
                "屏幕 OCR 上下文为空，已跳过: elapsed_ms={}, language={}, image={}x{}",
                snapshot.elapsed_ms,
                snapshot.selected_language,
                snapshot.image_width,
                snapshot.image_height
            ));
            ContextWait::Finished(None)
        }
        Ok(Err(err)) => {
            app_log::warn(format!("屏幕 OCR 上下文不可用，已跳过: {}", err));
            ContextWait::Finished(None)
        }
        Err(RecvTimeoutError::Timeout) => {
            app_log::info(format!(
                "屏幕 OCR 超过 {}ms 未返回，ASR 首包不带屏幕上下文。",
                timeout_ms
            ));
            ContextWait::TimedOut
        }
        Err(RecvTimeoutError::Disconnected) => {
            app_log::warn("屏幕 OCR 线程提前结束，已跳过。");
            ContextWait::Finished(None)
        }
    }
}

fn screenshot_encode_warning_code(err: &str) -> &'static str {
    if err.contains("超过体积上限") {
        WARNING_SCREEN_SCREENSHOT_TOO_LARGE
    } else {
        WARNING_SCREEN_SCREENSHOT_ENCODE_FAILED
    }
}

pub fn test_capture(config: &ScreenContextConfig) -> Result<ScreenContextTestResult, String> {
    let started_at = Instant::now();
    let image = Arc::new(capture_context_bitmap(config)?);
    let llm_mode = config.llm_mode.clone();

    if config.uses_screenshot_for_llm() {
        let image_for_encode = Arc::clone(&image);
        let encode_handle =
            thread::spawn(move || encode_screen_image(&image_for_encode, started_at));
        let ocr_result = recognize_image_context(&image, config, started_at);
        let encode_result = encode_handle
            .join()
            .unwrap_or_else(|_| Err("屏幕截图编码线程异常结束。".to_string()));

        return match (ocr_result, encode_result) {
            (Ok(recognized), Ok(encoded)) => {
                let text_chars = recognized.text.chars().count();
                let warning_code = if recognized.text.trim().is_empty() {
                    Some(WARNING_SCREEN_OCR_EMPTY.to_string())
                } else {
                    None
                };
                Ok(ScreenContextTestResult {
                    available_languages: recognized.available_languages,
                    selected_language: Some(recognized.selected_language),
                    elapsed_ms: recognized.elapsed_ms,
                    text: recognized.text,
                    text_chars,
                    image_width: recognized.image_width,
                    image_height: recognized.image_height,
                    warning: None,
                    warning_code,
                    llm_mode,
                    screenshot_data_url: Some(encoded.data_url()),
                    screenshot_mime: Some(encoded.mime_type),
                    screenshot_bytes: Some(encoded.bytes_len),
                    screenshot_width: Some(encoded.width),
                    screenshot_height: Some(encoded.height),
                    screenshot_elapsed_ms: Some(encoded.elapsed_ms),
                    screenshot_warning: None,
                    screenshot_warning_code: None,
                })
            }
            (Err(ocr_err), Ok(encoded)) => {
                app_log::warn(format!("屏幕上下文测试文字识别失败: {}", ocr_err));
                Ok(ScreenContextTestResult {
                    available_languages: Vec::new(),
                    selected_language: None,
                    elapsed_ms: encoded.elapsed_ms,
                    text: String::new(),
                    text_chars: 0,
                    image_width: image.width,
                    image_height: image.height,
                    warning: None,
                    warning_code: Some(WARNING_SCREEN_OCR_FAILED_WITH_SCREENSHOT.to_string()),
                    llm_mode,
                    screenshot_data_url: Some(encoded.data_url()),
                    screenshot_mime: Some(encoded.mime_type),
                    screenshot_bytes: Some(encoded.bytes_len),
                    screenshot_width: Some(encoded.width),
                    screenshot_height: Some(encoded.height),
                    screenshot_elapsed_ms: Some(encoded.elapsed_ms),
                    screenshot_warning: None,
                    screenshot_warning_code: None,
                })
            }
            (Ok(recognized), Err(img_err)) => {
                app_log::warn(format!("屏幕上下文测试截图编码失败: {}", img_err));
                let text_chars = recognized.text.chars().count();
                let warning_code = if recognized.text.trim().is_empty() {
                    Some(WARNING_SCREEN_OCR_EMPTY.to_string())
                } else {
                    None
                };
                let screenshot_warning_code =
                    Some(screenshot_encode_warning_code(&img_err).to_string());
                Ok(ScreenContextTestResult {
                    available_languages: recognized.available_languages,
                    selected_language: Some(recognized.selected_language),
                    elapsed_ms: recognized.elapsed_ms,
                    text: recognized.text,
                    text_chars,
                    image_width: recognized.image_width,
                    image_height: recognized.image_height,
                    warning: None,
                    warning_code,
                    llm_mode,
                    screenshot_data_url: None,
                    screenshot_mime: None,
                    screenshot_bytes: None,
                    screenshot_width: None,
                    screenshot_height: None,
                    screenshot_elapsed_ms: None,
                    screenshot_warning: None,
                    screenshot_warning_code,
                })
            }
            (Err(ocr_err), Err(_)) => Err(ocr_err),
        };
    }

    let recognized = recognize_image_context(&image, config, started_at)?;
    let text_chars = recognized.text.chars().count();
    Ok(ScreenContextTestResult {
        available_languages: recognized.available_languages,
        selected_language: Some(recognized.selected_language),
        elapsed_ms: recognized.elapsed_ms,
        warning: None,
        warning_code: if recognized.text.trim().is_empty() {
            Some(WARNING_SCREEN_OCR_EMPTY.to_string())
        } else {
            None
        },
        text: recognized.text,
        text_chars,
        image_width: recognized.image_width,
        image_height: recognized.image_height,
        llm_mode,
        screenshot_data_url: None,
        screenshot_mime: None,
        screenshot_bytes: None,
        screenshot_width: None,
        screenshot_height: None,
        screenshot_elapsed_ms: None,
        screenshot_warning: None,
        screenshot_warning_code: None,
    })
}

pub fn test_capture_on_worker(
    config: &ScreenContextConfig,
) -> Result<ScreenContextTestResult, String> {
    let config = config.clone();
    thread::spawn(move || test_capture(&config))
        .join()
        .map_err(|_| "屏幕 OCR 测试线程异常结束。".to_string())?
}

fn recognize_image_context(
    image: &CapturedImage,
    config: &ScreenContextConfig,
    started_at: Instant,
) -> Result<ScreenContextSnapshot, String> {
    let _apartment = WinRtApartment::init_mta()?;
    let available_languages = available_ocr_language_tags()?;
    let (engine, selected_language) = create_ocr_engine(&available_languages)?;
    let max_dimension = OcrEngine::MaxImageDimension()
        .map(|value| value as i32)
        .unwrap_or(2_600);
    let image = resize_to_max_dimension(image, max_dimension);
    let bitmap = software_bitmap_from_bgra(image.as_ref(), BitmapAlphaMode::Premultiplied)?;
    let result = engine
        .RecognizeAsync(&bitmap)
        .map_err(|err| windows_error("启动 Windows OCR 识别失败", err))?
        .join()
        .map_err(|err| windows_error("等待 Windows OCR 识别结果失败", err))?;
    let raw_text = result
        .Text()
        .map_err(|err| windows_error("读取 Windows OCR 文本失败", err))?
        .to_string();
    let _ = bitmap.Close();
    let text = normalize_screen_context_text(&raw_text, config.max_chars);
    Ok(ScreenContextSnapshot {
        text,
        elapsed_ms: started_at.elapsed().as_millis(),
        selected_language,
        available_languages,
        image_width: image.width,
        image_height: image.height,
    })
}

fn encode_screen_image(
    image: &CapturedImage,
    started_at: Instant,
) -> Result<EncodedScreenImage, String> {
    let _apartment = WinRtApartment::init_mta()?;
    let source_width = image.width;
    let source_height = image.height;
    let longest = source_width.max(source_height);
    let was_downscaled = longest > SCREEN_IMAGE_MAX_EDGE;
    let resized_storage = if was_downscaled {
        Some(resize_to_max_dimension(image, SCREEN_IMAGE_MAX_EDGE).into_owned())
    } else {
        None
    };
    let target = resized_storage.as_ref().unwrap_or(image);
    let mut final_width = target.width;
    let mut final_height = target.height;

    let bitmap = software_bitmap_from_bgra(target, BitmapAlphaMode::Ignore)?;
    let jpeg_bytes = encode_bitmap_jpeg(&bitmap, SCREEN_IMAGE_JPEG_QUALITY);
    let (mut mime_type, mut bytes) = match jpeg_bytes {
        Ok(jpeg) => {
            if !was_downscaled {
                match encode_bitmap_png(&bitmap) {
                    Ok(png) if png.len() <= jpeg.len() => ("image/png".to_string(), png),
                    _ => ("image/jpeg".to_string(), jpeg),
                }
            } else {
                ("image/jpeg".to_string(), jpeg)
            }
        }
        Err(jpeg_err) => {
            let png = encode_bitmap_png(&bitmap).map_err(|_| jpeg_err)?;
            ("image/png".to_string(), png)
        }
    };

    if bytes.len() > SCREEN_IMAGE_MAX_BYTES {
        if let Ok(smaller_jpeg) = encode_bitmap_jpeg(&bitmap, SCREEN_IMAGE_FALLBACK_QUALITY) {
            mime_type = "image/jpeg".to_string();
            bytes = smaller_jpeg;
        }
    }
    let _ = bitmap.Close();

    if bytes.len() > SCREEN_IMAGE_MAX_BYTES
        && target.width.max(target.height) > SCREEN_IMAGE_FALLBACK_EDGE
    {
        let smaller = resize_to_max_dimension(target, SCREEN_IMAGE_FALLBACK_EDGE);
        let retry_bitmap = software_bitmap_from_bgra(smaller.as_ref(), BitmapAlphaMode::Ignore)?;
        if let Ok(smaller_jpeg) = encode_bitmap_jpeg(&retry_bitmap, SCREEN_IMAGE_FALLBACK_QUALITY) {
            mime_type = "image/jpeg".to_string();
            bytes = smaller_jpeg;
            final_width = smaller.width;
            final_height = smaller.height;
        }
        let _ = retry_bitmap.Close();
    }

    if bytes.len() > SCREEN_IMAGE_MAX_BYTES {
        return Err(format!(
            "屏幕截图压缩后仍超过体积上限: {} bytes > {} bytes",
            bytes.len(),
            SCREEN_IMAGE_MAX_BYTES
        ));
    }

    let bytes_len = bytes.len();
    let data_base64 = encode_base64(&bytes);
    Ok(EncodedScreenImage {
        mime_type,
        data_base64,
        bytes_len,
        width: final_width,
        height: final_height,
        source_width,
        source_height,
        elapsed_ms: started_at.elapsed().as_millis(),
    })
}

fn encode_bitmap_jpeg(bitmap: &SoftwareBitmap, quality: f32) -> Result<Vec<u8>, String> {
    let stream = InMemoryRandomAccessStream::new()
        .map_err(|err| windows_error("创建 JPEG 编码流失败", err))?;
    let props =
        BitmapPropertySet::new().map_err(|err| windows_error("创建 JPEG 编码参数失败", err))?;
    let quality_value = PropertyValue::CreateSingle(quality)
        .map_err(|err| windows_error("创建 JPEG 质量参数失败", err))?;
    let typed_quality = BitmapTypedValue::Create(&quality_value, PropertyType::Single)
        .map_err(|err| windows_error("包装 JPEG 质量参数失败", err))?;
    props
        .Insert(&HSTRING::from("ImageQuality"), &typed_quality)
        .map_err(|err| windows_error("设置 JPEG 质量参数失败", err))?;

    let encoder_id = BitmapEncoder::JpegEncoderId()
        .map_err(|err| windows_error("获取 JPEG 编码器 ID 失败", err))?;
    let encoder = BitmapEncoder::CreateWithEncodingOptionsAsync(encoder_id, &stream, &props)
        .map_err(|err| windows_error("创建 JPEG 编码器失败", err))?
        .join()
        .map_err(|err| windows_error("等待 JPEG 编码器就绪失败", err))?;
    encode_with_bitmap_encoder(encoder, bitmap, stream)
}

fn encode_bitmap_png(bitmap: &SoftwareBitmap) -> Result<Vec<u8>, String> {
    let stream = InMemoryRandomAccessStream::new()
        .map_err(|err| windows_error("创建 PNG 编码流失败", err))?;
    let encoder_id = BitmapEncoder::PngEncoderId()
        .map_err(|err| windows_error("获取 PNG 编码器 ID 失败", err))?;
    let encoder = BitmapEncoder::CreateAsync(encoder_id, &stream)
        .map_err(|err| windows_error("创建 PNG 编码器失败", err))?
        .join()
        .map_err(|err| windows_error("等待 PNG 编码器就绪失败", err))?;
    encode_with_bitmap_encoder(encoder, bitmap, stream)
}

fn encode_with_bitmap_encoder(
    encoder: BitmapEncoder,
    bitmap: &SoftwareBitmap,
    stream: InMemoryRandomAccessStream,
) -> Result<Vec<u8>, String> {
    encoder
        .SetSoftwareBitmap(bitmap)
        .map_err(|err| windows_error("写入编码位图失败", err))?;
    encoder
        .FlushAsync()
        .map_err(|err| windows_error("提交图片编码失败", err))?
        .join()
        .map_err(|err| windows_error("等待图片编码完成失败", err))?;

    let size = stream
        .Size()
        .map_err(|err| windows_error("读取编码图片大小失败", err))?;
    let byte_len = usize::try_from(size).map_err(|_| "编码图片超出内存范围。".to_string())?;
    let reader_len = u32::try_from(size).map_err(|_| "编码图片超出读取范围。".to_string())?;
    stream
        .Seek(0)
        .map_err(|err| windows_error("重置图片读取位置失败", err))?;
    let input = stream
        .GetInputStreamAt(0)
        .map_err(|err| windows_error("打开编码图片输入流失败", err))?;
    let reader = DataReader::CreateDataReader(&input)
        .map_err(|err| windows_error("创建编码图片读取器失败", err))?;
    reader
        .LoadAsync(reader_len)
        .map_err(|err| windows_error("加载编码图片数据失败", err))?
        .join()
        .map_err(|err| windows_error("等待读取编码图片数据失败", err))?;
    let mut bytes = vec![0u8; byte_len];
    reader
        .ReadBytes(&mut bytes)
        .map_err(|err| windows_error("读取编码图片字节失败", err))?;
    let _ = reader.Close();
    let _ = input.Close();
    let _ = stream.Close();
    Ok(bytes)
}

fn encode_base64(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = *chunk.get(1).unwrap_or(&0) as usize;
        let b2 = *chunk.get(2).unwrap_or(&0) as usize;
        let triple = (b0 << 16) | (b1 << 8) | b2;

        out.push(TABLE[(triple >> 18) & 0x3F] as char);
        out.push(TABLE[(triple >> 12) & 0x3F] as char);
        out.push(if chunk.len() > 1 {
            TABLE[(triple >> 6) & 0x3F] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[triple & 0x3F] as char
        } else {
            '='
        });
    }
    out
}

fn available_ocr_language_tags() -> Result<Vec<String>, String> {
    let languages = OcrEngine::AvailableRecognizerLanguages()
        .map_err(|err| windows_error("读取 Windows OCR 语言列表失败", err))?;
    let mut tags = Vec::new();
    for index in 0..languages
        .Size()
        .map_err(|err| windows_error("读取 Windows OCR 语言数量失败", err))?
    {
        let language = languages
            .GetAt(index)
            .map_err(|err| windows_error("读取 Windows OCR 语言失败", err))?;
        tags.push(
            language
                .LanguageTag()
                .map_err(|err| windows_error("读取 Windows OCR 语言标签失败", err))?
                .to_string(),
        );
    }
    Ok(tags)
}

fn create_ocr_engine(available_languages: &[String]) -> Result<(OcrEngine, String), String> {
    for preferred in ["zh-Hans-CN", "zh-CN", "zh-Hans", "en-US", "en"] {
        if !available_languages
            .iter()
            .any(|tag| tag.eq_ignore_ascii_case(preferred))
        {
            continue;
        }
        let language = Language::CreateLanguage(&HSTRING::from(preferred))
            .map_err(|err| windows_error("创建 Windows OCR 语言失败", err))?;
        if OcrEngine::IsLanguageSupported(&language)
            .map_err(|err| windows_error("检查 Windows OCR 语言支持失败", err))?
        {
            let engine = OcrEngine::TryCreateFromLanguage(&language)
                .map_err(|err| windows_error("创建 Windows OCR 引擎失败", err))?;
            return Ok((engine, preferred.to_string()));
        }
    }

    let engine = OcrEngine::TryCreateFromUserProfileLanguages()
        .map_err(|err| windows_error("创建 Windows OCR 用户语言引擎失败", err))?;
    let selected_language = engine
        .RecognizerLanguage()
        .ok()
        .and_then(|language| language.LanguageTag().ok())
        .map(|tag| tag.to_string())
        .unwrap_or_else(|| "user-profile".to_string());
    Ok((engine, selected_language))
}

fn software_bitmap_from_bgra(
    image: &CapturedImage,
    alpha_mode: BitmapAlphaMode,
) -> Result<SoftwareBitmap, String> {
    let writer = DataWriter::new().map_err(|err| windows_error("创建截图缓冲失败", err))?;
    writer
        .WriteBytes(&image.pixels)
        .map_err(|err| windows_error("写入截图缓冲失败", err))?;
    let buffer = writer
        .DetachBuffer()
        .map_err(|err| windows_error("生成截图缓冲失败", err))?;
    let bitmap = SoftwareBitmap::CreateCopyWithAlphaFromBuffer(
        &buffer,
        BitmapPixelFormat::Bgra8,
        image.width,
        image.height,
        alpha_mode,
    )
    .map_err(|err| windows_error("创建位图失败", err))?;
    let _ = writer.Close();
    Ok(bitmap)
}

fn resize_to_max_dimension<'a>(
    image: &'a CapturedImage,
    max_dimension: i32,
) -> Cow<'a, CapturedImage> {
    if max_dimension <= 0 || (image.width <= max_dimension && image.height <= max_dimension) {
        return Cow::Borrowed(image);
    }
    let longest = image.width.max(image.height) as f64;
    let scale = max_dimension as f64 / longest;
    let next_width = ((image.width as f64 * scale).round() as i32).max(1);
    let next_height = ((image.height as f64 * scale).round() as i32).max(1);
    Cow::Owned(resize_bgra_box_filter_ref(image, next_width, next_height))
}

/// 缩小截图时对源区域取平均，而不是最近邻取样。
///
/// 界面文字笔画只有一两个像素宽，最近邻会整笔丢弃，OCR 识别率明显下降：本机实测把
/// 2194x1234 仅缩到 2000x1125，最近邻的词召回就掉到 68.8%，区域平均则基本保持。
/// 该路径也同时用于多模态截图长边上限缩放，保持小字与细线可读性。
fn resize_bgra_box_filter_ref(
    image: &CapturedImage,
    next_width: i32,
    next_height: i32,
) -> CapturedImage {
    let src_width = image.width as usize;
    let src_height = image.height as usize;
    let dst_width = next_width as usize;
    let dst_height = next_height as usize;
    let mut resized = vec![0u8; dst_width * dst_height * 4];

    for y in 0..dst_height {
        let src_y0 = y * src_height / dst_height;
        let src_y1 = (((y + 1) * src_height).div_ceil(dst_height)).min(src_height);
        let src_y1 = src_y1.max(src_y0 + 1);
        for x in 0..dst_width {
            let src_x0 = x * src_width / dst_width;
            let src_x1 = (((x + 1) * src_width).div_ceil(dst_width)).min(src_width);
            let src_x1 = src_x1.max(src_x0 + 1);

            let mut channel_sums = [0u32; 4];
            let mut sampled = 0u32;
            for src_y in src_y0..src_y1 {
                let row = src_y * src_width;
                for src_x in src_x0..src_x1 {
                    let src = (row + src_x) * 4;
                    for (channel, sum) in channel_sums.iter_mut().enumerate() {
                        *sum += image.pixels[src + channel] as u32;
                    }
                    sampled += 1;
                }
            }

            let dst = (y * dst_width + x) * 4;
            let sampled = sampled.max(1);
            for (channel, sum) in channel_sums.iter().enumerate() {
                resized[dst + channel] = (sum / sampled) as u8;
            }
        }
    }

    CapturedImage {
        pixels: resized,
        width: next_width,
        height: next_height,
    }
}

fn normalize_screen_context_text(text: &str, max_chars: usize) -> String {
    text.replace('\r', "\n")
        .lines()
        .map(|line| {
            merge_cjk_character_spaces(&line.split_whitespace().collect::<Vec<_>>().join(" "))
        })
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
        .chars()
        .take(max_chars)
        .collect::<String>()
        .trim()
        .to_string()
}

fn merge_cjk_character_spaces(text: &str) -> String {
    let mut merged = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == ' ' {
            let previous = merged.chars().next_back();
            let next = chars.peek().copied();
            if previous.is_some_and(is_cjk_character) && next.is_some_and(is_cjk_character) {
                continue;
            }
        }
        merged.push(ch);
    }
    merged
}

fn is_cjk_character(ch: char) -> bool {
    matches!(
        ch,
        '\u{3400}'..='\u{4DBF}' | '\u{4E00}'..='\u{9FFF}' | '\u{F900}'..='\u{FAFF}'
    )
}

fn capture_context_bitmap(config: &ScreenContextConfig) -> Result<CapturedImage, String> {
    if config.capture_scope == "window" {
        capture_foreground_window_bitmap()
    } else {
        capture_current_monitor_bitmap()
    }
}

fn capture_current_monitor_bitmap() -> Result<CapturedImage, String> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_invalid() {
            return Err("未找到当前前台窗口，无法定位当前显示器。".to_string());
        }
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        if monitor.is_invalid() {
            return Err("未找到当前显示器。".to_string());
        }
        let mut info = MONITORINFO {
            cbSize: mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !GetMonitorInfoW(monitor, &mut info).as_bool() {
            return Err("读取当前显示器区域失败。".to_string());
        }
        capture_rect_bitmap(info.rcMonitor, "当前显示器")
    }
}

fn capture_foreground_window_bitmap() -> Result<CapturedImage, String> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_invalid() {
            return Err("未找到当前前台窗口。".to_string());
        }
        let mut rect = RECT::default();
        GetWindowRect(hwnd, &mut rect).map_err(|err| windows_error("读取当前窗口区域失败", err))?;
        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;
        if width <= 0 || height <= 0 {
            return Err("当前窗口区域无效。".to_string());
        }

        capture_rect_bitmap(rect, "当前窗口")
    }
}

fn capture_rect_bitmap(rect: RECT, area_label: &str) -> Result<CapturedImage, String> {
    unsafe {
        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;
        if width <= 0 || height <= 0 {
            return Err(format!("{}区域无效。", area_label));
        }
        let screen_dc = ScreenDc::new()?;
        let memory_dc = MemoryDc::new(screen_dc.0)?;
        let bitmap = Bitmap::new(screen_dc.0, width, height)?;
        let _selection = SelectedObject::new(memory_dc.0, HGDIOBJ::from(bitmap.0))?;

        BitBlt(
            memory_dc.0,
            0,
            0,
            width,
            height,
            Some(screen_dc.0),
            rect.left,
            rect.top,
            SRCCOPY | CAPTUREBLT,
        )
        .map_err(|err| windows_error(&format!("复制{}截图失败", area_label), err))?;

        let mut info = BITMAPINFO::default();
        info.bmiHeader.biSize = mem::size_of::<BITMAPINFOHEADER>() as u32;
        info.bmiHeader.biWidth = width;
        info.bmiHeader.biHeight = -height;
        info.bmiHeader.biPlanes = 1;
        info.bmiHeader.biBitCount = 32;
        info.bmiHeader.biCompression = BI_RGB.0;

        let mut pixels = vec![0u8; width as usize * height as usize * 4];
        let copied = GetDIBits(
            memory_dc.0,
            bitmap.0,
            0,
            height as u32,
            Some(pixels.as_mut_ptr() as *mut c_void),
            &mut info,
            DIB_RGB_COLORS,
        );
        if copied == 0 {
            return Err(format!("读取{}截图像素失败。", area_label));
        }
        Ok(CapturedImage {
            pixels,
            width,
            height,
        })
    }
}

struct ScreenDc(HDC);

impl ScreenDc {
    unsafe fn new() -> Result<Self, String> {
        let hdc = GetDC(None);
        if hdc.is_invalid() {
            Err("获取屏幕 DC 失败。".to_string())
        } else {
            Ok(Self(hdc))
        }
    }
}

impl Drop for ScreenDc {
    fn drop(&mut self) {
        unsafe {
            let _ = ReleaseDC(None, self.0);
        }
    }
}

struct MemoryDc(HDC);

impl MemoryDc {
    unsafe fn new(source: HDC) -> Result<Self, String> {
        let hdc = CreateCompatibleDC(Some(source));
        if hdc.is_invalid() {
            Err("创建截图 DC 失败。".to_string())
        } else {
            Ok(Self(hdc))
        }
    }
}

impl Drop for MemoryDc {
    fn drop(&mut self) {
        unsafe {
            let _ = DeleteDC(self.0);
        }
    }
}

struct Bitmap(HBITMAP);

impl Bitmap {
    unsafe fn new(source: HDC, width: i32, height: i32) -> Result<Self, String> {
        let bitmap = CreateCompatibleBitmap(source, width, height);
        if bitmap.is_invalid() {
            Err("创建截图位图失败。".to_string())
        } else {
            Ok(Self(bitmap))
        }
    }
}

impl Drop for Bitmap {
    fn drop(&mut self) {
        unsafe {
            let _ = DeleteObject(HGDIOBJ::from(self.0));
        }
    }
}

struct SelectedObject {
    hdc: HDC,
    previous: HGDIOBJ,
}

impl SelectedObject {
    unsafe fn new(hdc: HDC, object: HGDIOBJ) -> Result<Self, String> {
        let previous = SelectObject(hdc, object);
        if previous.is_invalid() {
            Err("选择截图位图失败。".to_string())
        } else {
            Ok(Self { hdc, previous })
        }
    }
}

impl Drop for SelectedObject {
    fn drop(&mut self) {
        unsafe {
            if !self.previous.is_invalid() {
                let _ = SelectObject(self.hdc, self.previous);
            }
        }
    }
}

struct WinRtApartment {
    should_uninitialize: bool,
}

impl WinRtApartment {
    fn init_mta() -> Result<Self, String> {
        unsafe {
            match RoInitialize(RO_INIT_MULTITHREADED) {
                Ok(()) => Ok(Self {
                    should_uninitialize: true,
                }),
                Err(err) if is_changed_thread_mode(&err) => {
                    // Tauri 命令线程可能已初始化为 STA；WinRT OCR 可以沿用既有 apartment。
                    Ok(Self {
                        should_uninitialize: false,
                    })
                }
                Err(err) => Err(windows_error("初始化 Windows OCR 运行时失败", err)),
            }
        }
    }
}

fn is_changed_thread_mode(err: &WindowsError) -> bool {
    err.code() == RPC_E_CHANGED_MODE
}

impl Drop for WinRtApartment {
    fn drop(&mut self) {
        if !self.should_uninitialize {
            return;
        }
        unsafe {
            RoUninitialize();
        }
    }
}

fn windows_error(context: &str, err: WindowsError) -> String {
    format!("{}: {}", context, err)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downscale_averages_source_pixels_instead_of_dropping_them() {
        // 2x2 缩到 1x1：四个源像素应当被平均，而不是取其中一个。
        // 最近邻在这里会整块丢弃三个像素，界面文字的细笔画正是这样消失的。
        let image = CapturedImage {
            pixels: vec![
                0, 0, 0, 255, // 黑
                255, 255, 255, 255, // 白
                255, 255, 255, 255, // 白
                255, 255, 255, 255, // 白
            ],
            width: 2,
            height: 2,
        };

        let resized = resize_to_max_dimension(&image, 1);

        assert_eq!((resized.width, resized.height), (1, 1));
        assert_eq!(resized.pixels, vec![191, 191, 191, 255]);
    }

    #[test]
    fn keeps_image_untouched_when_within_ocr_limit() {
        let pixels = vec![7u8; 2 * 2 * 4];
        let image = CapturedImage {
            pixels: pixels.clone(),
            width: 2,
            height: 2,
        };

        let kept = resize_to_max_dimension(&image, 4096);

        assert!(matches!(kept, Cow::Borrowed(_)));
        assert_eq!((kept.width, kept.height), (2, 2));
        assert_eq!(kept.pixels, pixels);
    }

    #[test]
    fn normalizes_ocr_text_lightly() {
        let text = normalize_screen_context_text("  豆 包  ASR\r\n\r\n  VoxType   main  ", 1_000);
        assert_eq!(text, "豆包 ASR\nVoxType main");
    }

    #[test]
    fn removes_spaces_between_cjk_characters_only() {
        let text = normalize_screen_context_text(
            "屏 幕 OCR 上 下 文  Ctrl + Q  ASR 和 大 模 型  VoxType 首 页",
            1_000,
        );
        assert_eq!(text, "屏幕 OCR 上下文 Ctrl + Q ASR 和大模型 VoxType 首页");
    }

    #[test]
    fn limits_ocr_text_length() {
        let text = normalize_screen_context_text("abcdef", 3);
        assert_eq!(text, "abc");
    }

    fn snapshot_with_text(text: &str) -> ScreenContextSnapshot {
        ScreenContextSnapshot {
            text: text.to_string(),
            elapsed_ms: 640,
            selected_language: "zh-Hans-CN".to_string(),
            available_languages: vec!["zh-Hans-CN".to_string()],
            image_width: 3864,
            image_height: 2100,
        }
    }

    fn block_on<T>(future: impl std::future::Future<Output = T>) -> T {
        tokio::runtime::Runtime::new()
            .expect("test runtime")
            .block_on(future)
    }

    #[test]
    fn wait_for_context_times_out_without_result() {
        let (_tx, rx) = mpsc::channel::<Result<ScreenContextSnapshot, String>>();

        assert!(matches!(wait_for_context(&rx, 1), ContextWait::TimedOut));
    }

    #[test]
    fn wait_for_context_skips_worker_error() {
        let (tx, rx) = mpsc::channel();
        tx.send(Err("Windows OCR unavailable".to_string())).unwrap();

        assert!(matches!(
            wait_for_context(&rx, 500),
            ContextWait::Finished(None)
        ));
    }

    #[test]
    fn timely_result_is_shared_by_first_packet_and_post_edit() {
        let (tx, rx) = mpsc::channel();
        tx.send(Ok(snapshot_with_text("VoxType 设置页"))).unwrap();
        let pending = PendingScreenContext::new(
            Some(ScreenContextReceivers {
                ocr_rx: rx,
                image_rx: None,
            }),
            500,
        );

        block_on(async {
            assert_eq!(pending.resolve().await.as_deref(), Some("VoxType 设置页"));
            let post_edit = pending.resolve_for_post_edit().await;
            assert_eq!(post_edit.ocr_text.as_deref(), Some("VoxType 设置页"));
            assert!(post_edit.image.is_none());
        });
    }

    #[test]
    fn late_result_reaches_post_edit_without_changing_first_packet() {
        // 回归：首包等待超时后接收端被直接丢弃，稍后到达的 OCR 结果连润色也用不上。
        let (tx, rx) = mpsc::channel();
        let pending = PendingScreenContext::new(
            Some(ScreenContextReceivers {
                ocr_rx: rx,
                image_rx: None,
            }),
            5,
        );

        block_on(async {
            assert_eq!(pending.resolve().await, None);
            tx.send(Ok(snapshot_with_text("迟到的屏幕文字"))).unwrap();

            assert_eq!(
                pending.resolve_for_post_edit().await.ocr_text.as_deref(),
                Some("迟到的屏幕文字")
            );
            // 首包语义不变：超时就是不带上下文，不会因为结果迟到而回头改写。
            assert_eq!(pending.resolve().await, None);
            // 结果只取一次并缓存，重复取用得到同一份文本。
            assert_eq!(
                pending.resolve_for_post_edit().await.ocr_text.as_deref(),
                Some("迟到的屏幕文字")
            );
        });
    }

    #[test]
    fn post_edit_never_waits_for_unfinished_ocr() {
        let (_tx, rx) = mpsc::channel::<Result<ScreenContextSnapshot, String>>();
        let (_img_tx, img_rx) = mpsc::channel::<Result<EncodedScreenImage, String>>();
        let pending = PendingScreenContext::new(
            Some(ScreenContextReceivers {
                ocr_rx: rx,
                image_rx: Some(img_rx),
            }),
            5,
        );

        block_on(async {
            assert_eq!(pending.resolve().await, None);
            let started = Instant::now();
            let post_edit = pending.resolve_for_post_edit().await;
            assert_eq!(post_edit.ocr_text, None);
            assert!(post_edit.image.is_none());
            assert!(started.elapsed() < Duration::from_millis(200));
        });
    }

    #[test]
    fn disabled_screen_context_resolves_to_none_everywhere() {
        let pending = PendingScreenContext::new(None, 500);

        block_on(async {
            assert_eq!(pending.resolve().await, None);
            let post_edit = pending.resolve_for_post_edit().await;
            assert_eq!(post_edit.ocr_text, None);
            assert!(post_edit.image.is_none());
        });
    }

    #[test]
    fn post_edit_receives_encoded_screenshot_even_when_ocr_fails() {
        let (ocr_tx, ocr_rx) = mpsc::channel();
        let (img_tx, img_rx) = mpsc::channel();
        ocr_tx.send(Err("OCR unavailable".to_string())).unwrap();
        img_tx
            .send(Ok(EncodedScreenImage {
                mime_type: "image/jpeg".to_string(),
                data_base64: "AAAA".to_string(),
                bytes_len: 3,
                width: 1600,
                height: 900,
                source_width: 3840,
                source_height: 2160,
                elapsed_ms: 140,
            }))
            .unwrap();

        let pending = PendingScreenContext::new(
            Some(ScreenContextReceivers {
                ocr_rx,
                image_rx: Some(img_rx),
            }),
            500,
        );

        block_on(async {
            assert_eq!(pending.resolve().await, None);
            let post_edit = pending.resolve_for_post_edit().await;
            assert!(post_edit.ocr_text.is_none());
            let image = post_edit
                .image
                .expect("screenshot should still reach post-edit");
            assert_eq!(image.data_url(), "data:image/jpeg;base64,AAAA");
        });
    }

    #[test]
    fn encoded_screen_image_debug_redacts_base64_payload() {
        let image = EncodedScreenImage {
            mime_type: "image/png".to_string(),
            data_base64: "SECRET_BASE64_DATA".to_string(),
            bytes_len: 12,
            width: 800,
            height: 600,
            source_width: 800,
            source_height: 600,
            elapsed_ms: 42,
        };
        let debug_text = format!("{image:?}");
        assert!(debug_text.contains("<redacted>"));
        assert!(!debug_text.contains("SECRET_BASE64_DATA"));
    }

    #[test]
    fn base64_encoder_matches_rfc4648_vectors() {
        assert_eq!(encode_base64(b""), "");
        assert_eq!(encode_base64(b"f"), "Zg==");
        assert_eq!(encode_base64(b"fo"), "Zm8=");
        assert_eq!(encode_base64(b"foo"), "Zm9v");
        assert_eq!(encode_base64(b"foob"), "Zm9vYg==");
        assert_eq!(encode_base64(b"fooba"), "Zm9vYmE=");
        assert_eq!(encode_base64(b"foobar"), "Zm9vYmFy");
        assert_eq!(encode_base64(&[0xFF, 0xD8, 0xFF, 0xE0]), "/9j/4A==");
    }

    #[test]
    fn encodes_small_and_large_synthetic_screenshots_within_bounds() {
        // 小纯色图（alpha=0，模拟 GDI BitBlt）应编码成功并保持非透明。
        let small = CapturedImage {
            pixels: [40, 120, 220, 0].repeat(64 * 48),
            width: 64,
            height: 48,
        };
        let encoded_small = encode_screen_image(&small, Instant::now()).expect("encode small");
        assert!(matches!(
            encoded_small.mime_type.as_str(),
            "image/png" | "image/jpeg"
        ));
        assert_eq!((encoded_small.width, encoded_small.height), (64, 48));
        assert!(!encoded_small.data_base64.is_empty());
        assert!(encoded_small.bytes_len > 0);

        // 4K 合成纹理图应被缩小到长边 1600 以内且编码为有效 JPEG/PNG。
        let mut pixels_4k = vec![0u8; 3840 * 2160 * 4];
        for y in 0..2160usize {
            for x in 0..3840usize {
                let idx = (y * 3840 + x) * 4;
                pixels_4k[idx] = ((x * 7 + y * 3) % 256) as u8;
                pixels_4k[idx + 1] = ((x * 5 + y * 11) % 256) as u8;
                pixels_4k[idx + 2] = ((x * 13 + y * 2) % 256) as u8;
                pixels_4k[idx + 3] = 0;
            }
        }
        let large = CapturedImage {
            pixels: pixels_4k,
            width: 3840,
            height: 2160,
        };
        let encoded_large = encode_screen_image(&large, Instant::now()).expect("encode 4k");
        assert_eq!((encoded_large.width, encoded_large.height), (1600, 900));
        assert_eq!(encoded_large.source_width, 3840);
        assert_eq!(encoded_large.source_height, 2160);
        assert_eq!(encoded_large.mime_type, "image/jpeg");
        assert!(encoded_large.bytes_len <= SCREEN_IMAGE_MAX_BYTES);
    }

    #[test]
    fn treats_existing_com_apartment_as_usable() {
        let err = WindowsError::from_hresult(RPC_E_CHANGED_MODE);
        assert!(is_changed_thread_mode(&err));
    }

    #[test]
    fn screen_context_debug_outputs_redact_text_and_image_payloads() {
        let snapshot = ScreenContextSnapshot {
            text: "SENSITIVE_SCREEN_TEXT".to_string(),
            elapsed_ms: 12,
            selected_language: "zh-Hans-CN".to_string(),
            available_languages: vec!["zh-Hans-CN".to_string()],
            image_width: 1920,
            image_height: 1080,
        };
        assert!(!format!("{snapshot:?}").contains("SENSITIVE_SCREEN_TEXT"));

        let post_edit = PostEditScreenContext {
            ocr_text: Some("SENSITIVE_OCR_TEXT".to_string()),
            image: Some(EncodedScreenImage {
                mime_type: "image/jpeg".to_string(),
                data_base64: "SECRET_IMAGE_B64".to_string(),
                bytes_len: 12,
                width: 800,
                height: 600,
                source_width: 800,
                source_height: 600,
                elapsed_ms: 15,
            }),
        };
        let post_edit_debug = format!("{post_edit:?}");
        assert!(!post_edit_debug.contains("SENSITIVE_OCR_TEXT"));
        assert!(!post_edit_debug.contains("SECRET_IMAGE_B64"));

        let test_result = ScreenContextTestResult {
            available_languages: vec!["zh-Hans-CN".to_string()],
            selected_language: Some("zh-Hans-CN".to_string()),
            elapsed_ms: 20,
            text: "SENSITIVE_PREVIEW_TEXT".to_string(),
            text_chars: 22,
            image_width: 800,
            image_height: 600,
            warning: None,
            warning_code: None,
            llm_mode: "screenshot".to_string(),
            screenshot_data_url: Some("data:image/jpeg;base64,SECRET_PREVIEW_B64".to_string()),
            screenshot_mime: Some("image/jpeg".to_string()),
            screenshot_bytes: Some(1234),
            screenshot_width: Some(800),
            screenshot_height: Some(600),
            screenshot_elapsed_ms: Some(15),
            screenshot_warning: None,
            screenshot_warning_code: None,
        };
        let result_debug = format!("{test_result:?}");
        assert!(!result_debug.contains("SENSITIVE_PREVIEW_TEXT"));
        assert!(!result_debug.contains("SECRET_PREVIEW_B64"));
    }

    #[test]
    fn maps_screenshot_encode_errors_to_structured_warning_codes() {
        assert_eq!(
            super::screenshot_encode_warning_code("屏幕截图压缩后仍超过体积上限: 2000000 bytes"),
            super::WARNING_SCREEN_SCREENSHOT_TOO_LARGE
        );
        assert_eq!(
            super::screenshot_encode_warning_code("创建 JPEG 编码器失败"),
            super::WARNING_SCREEN_SCREENSHOT_ENCODE_FAILED
        );
    }
}
