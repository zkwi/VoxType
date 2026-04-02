import asyncio
import ctypes
from ctypes import wintypes
import logging
import os
import sys
import threading
from collections import deque
from pathlib import Path

from PyQt6.QtCore import QObject, QTimer, pyqtSignal
from PyQt6.QtGui import QAction, QCursor
from PyQt6.QtWidgets import QApplication, QMenu, QStyle, QSystemTrayIcon

from voice_input.asr_client import DoubaoAsrClient
from voice_input.audio_capture import MicrophoneRecorder
from voice_input.config import AppConfig
from voice_input.input_hooks import InputHookThread
from voice_input.llm_post_edit import AliyunLlmPostEditor
from voice_input.overlay import FloatingCaption
from voice_input.system_audio import SystemVolumeController
from voice_input.text_output import get_foreground_window, paste_text


LOG_PATH = Path("voice_input.log")

# Windows API 相关
user32 = ctypes.windll.user32
user32.VkKeyScanW.argtypes = [wintypes.WCHAR]
user32.VkKeyScanW.restype = ctypes.c_short
WM_HOTKEY = 0x0312
MOD_ALT = 0x0001
MOD_CONTROL = 0x0002
MOD_SHIFT = 0x0004
MOD_WIN = 0x0008
HOTKEY_ID = 1


def parse_hotkey(hotkey: str) -> tuple[int, int]:
    """将 "ctrl+q" 格式的热键字符串解析为 (modifiers, vk_code)，用于 RegisterHotKey。"""
    modifiers = 0
    parts = [part.strip().lower() for part in hotkey.split("+") if part.strip()]
    if not parts:
        raise ValueError("热键不能为空")

    key_name = parts[-1]
    modifier_parts = parts[:-1]
    modifier_map = {
        "alt": MOD_ALT,
        "ctrl": MOD_CONTROL,
        "control": MOD_CONTROL,
        "shift": MOD_SHIFT,
        "win": MOD_WIN,
    }
    for part in modifier_parts:
        if part not in modifier_map:
            raise ValueError(f"不支持的热键修饰键: {part}")
        modifiers |= modifier_map[part]

    if len(key_name) != 1:
        raise ValueError(f"暂不支持的热键按键: {key_name}")

    vk = user32.VkKeyScanW(key_name.upper()[0])
    if vk == -1:
        raise ValueError(f"不支持的热键按键: {key_name}")
    return modifiers, vk & 0xFF


class UiBridge(QObject):
    toggle_requested = pyqtSignal()
    show_overlay = pyqtSignal()
    hide_overlay = pyqtSignal()
    update_text = pyqtSignal(str)
    session_finished = pyqtSignal()


class GlobalHotkeyThread(threading.Thread):
    """在独立线程里注册全局热键并直接消费 WM_HOTKEY。"""

    def __init__(self, event: threading.Event, hotkey: str):
        super().__init__(daemon=True)
        self.event = event
        self.hotkey = hotkey
        self._thread_id: int | None = None
        self._ready = threading.Event()
        self.register_error: str | None = None

    def run(self) -> None:
        self._thread_id = ctypes.windll.kernel32.GetCurrentThreadId()
        try:
            modifiers, vk = parse_hotkey(self.hotkey)
            if not user32.RegisterHotKey(None, HOTKEY_ID, modifiers, vk):
                error_code = ctypes.GetLastError()
                self.register_error = f"注册全局热键失败: {self.hotkey} (WinError={error_code})"
                return
            self._ready.set()

            msg = wintypes.MSG()
            while user32.GetMessageW(ctypes.byref(msg), None, 0, 0) > 0:
                if msg.message == WM_HOTKEY and msg.wParam == HOTKEY_ID:
                    self.event.set()
                    continue
                user32.TranslateMessage(ctypes.byref(msg))
                user32.DispatchMessageW(ctypes.byref(msg))
        except Exception as exc:
            self.register_error = str(exc)
        finally:
            if self._thread_id is not None:
                user32.UnregisterHotKey(None, HOTKEY_ID)
            self._ready.set()

    def wait_until_ready(self, timeout: float = 1.0) -> None:
        self._ready.wait(timeout)
        if self.register_error:
            raise RuntimeError(self.register_error)
        if not self._ready.is_set():
            raise RuntimeError(f"注册全局热键超时: {self.hotkey}")

    def stop(self) -> None:
        if self._thread_id is not None:
            user32.PostThreadMessageW(self._thread_id, 0x0012, 0, 0)  # WM_QUIT


class VoiceInputApp:
    """主应用类，管理录音会话的完整生命周期。

    工作流程：
    1. 用户按下热键 -> toggle_recording()
    2. start_recording(): 记住目标窗口、静音系统、启动麦克风、开启识别线程
    3. 识别线程 (_run_session): ASR 识别 -> 可选 LLM 润色 -> 粘贴到目标窗口
    4. 再按热键 -> stop_recording(): 停止麦克风、等待最终结果
    """

    def __init__(self) -> None:
        self.app_config = AppConfig.load()
        self.config = self.app_config.raw
        self.qt_app = QApplication(sys.argv)
        self.qt_app.setQuitOnLastWindowClosed(False)
        self.bridge = UiBridge()
        self.bridge.toggle_requested.connect(self.toggle_recording)
        self.overlay = FloatingCaption(
            width=self.config["ui"].get("width", 460),
            height=self.config["ui"].get("height", 88),
            margin_bottom=self.config["ui"].get("margin_bottom", 64),
            opacity=self.config["ui"].get("opacity", 0.96),
        )
        self.overlay.set_scroll_interval(self.config["ui"].get("scroll_interval_ms", 1200))
        self.bridge.show_overlay.connect(self.overlay.show_for_recording)
        self.bridge.hide_overlay.connect(self.overlay.hide)
        self.bridge.update_text.connect(self.overlay.update_text)
        self.bridge.session_finished.connect(self._on_session_finished)

        audio_config = self.config["audio"]
        self.recorder = MicrophoneRecorder(
            sample_rate=audio_config.get("sample_rate", 16000),
            channels=audio_config.get("channels", 1),
            max_record_seconds=audio_config.get("max_record_seconds", 300),
            input_device=audio_config.get("input_device"),
            block_ms=audio_config.get("segment_ms", 200),
        )
        self.asr_client = DoubaoAsrClient(self.config)
        self.llm_post_editor = AliyunLlmPostEditor(self.config)
        self.system_volume = SystemVolumeController()
        self.recording = False
        self.worker_thread: threading.Thread | None = None
        self.stop_event = threading.Event()
        self.target_hwnd: int | None = None
        self.latest_text = ""
        self.pending_stop = False
        self.muted_for_recording = False
        recent_rounds = int(self.config["context"].get("recent_context_rounds", 5) or 0)
        self.recent_context_history: deque[str] = deque(maxlen=max(0, recent_rounds))
        self.auto_stop_timer = QTimer()
        self.auto_stop_timer.setSingleShot(True)
        self.auto_stop_timer.timeout.connect(self._force_stop_recording)
        self.stop_grace_timer = QTimer()
        self.stop_grace_timer.setSingleShot(True)
        self.stop_grace_timer.timeout.connect(self._force_stop_recording)
        self.tray_menu: QMenu | None = None
        self.input_hook_event = threading.Event()
        self.hotkey_thread: GlobalHotkeyThread | None = None
        self.input_hook_thread: InputHookThread | None = None
        self.input_poll_timer = QTimer()
        self.input_poll_timer.setInterval(50)
        self.input_poll_timer.timeout.connect(self._poll_input_hook)
        self.tray_icon = self._create_tray_icon()

    @staticmethod
    def _should_run_llm_post_edit(config: dict) -> bool:
        settings = config.get("llm_post_edit", {})
        if not settings.get("enabled", False):
            return False
        min_chars = int(settings.get("min_chars", 100) or 0)
        return min_chars > 0

    def toggle_recording(self) -> None:
        if self.recording:
            self.stop_recording()
        else:
            self.start_recording()

    def start_recording(self) -> None:
        if self.recording:
            return
        if self.worker_thread is not None and self.worker_thread.is_alive():
            logging.warning("上一次识别线程仍未退出，本次热键触发被忽略")
            return

        self.recording = True
        self.pending_stop = False
        self.stop_event.clear()
        self.latest_text = ""
        self._refresh_recent_context_config()
        self.target_hwnd = get_foreground_window()
        self.bridge.update_text.emit("正在录音...")
        self.bridge.show_overlay.emit()
        if self.config["audio"].get("mute_system_volume_while_recording", True):
            self.muted_for_recording = self.system_volume.safe_mute_and_save()
        self.recorder.start()

        max_seconds = self.config["audio"].get("max_record_seconds", 300)
        self.auto_stop_timer.start(max_seconds * 1000)
        self.worker_thread = threading.Thread(target=self._run_session, daemon=True)
        self.worker_thread.start()

    def stop_recording(self) -> None:
        if not self.recording:
            return
        if self.pending_stop:
            return

        self.pending_stop = True
        grace_ms = int(self.config["audio"].get("stop_grace_ms", 500) or 0)
        if grace_ms <= 0:
            self._force_stop_recording()
            return
        self.stop_grace_timer.start(grace_ms)

    def _force_stop_recording(self) -> None:
        if not self.recording:
            return

        self.recording = False
        self.pending_stop = False
        self.auto_stop_timer.stop()
        self.stop_grace_timer.stop()
        self.stop_event.set()
        self.recorder.stop()

    def _run_session(self) -> None:
        try:
            final_text = asyncio.run(self._recognize_and_post_edit())
            self.latest_text = final_text
            if final_text:
                self._remember_recent_context(final_text)
                paste_text(
                    final_text,
                    paste_delay_ms=self.config["typing"].get("paste_delay_ms", 120),
                    target_hwnd=self.target_hwnd,
                )
        except Exception as exc:
            logging.exception("识别流程失败: %s", exc)
            self.bridge.update_text.emit(f"识别失败: {exc}")
        finally:
            self.recorder.stop()
            self.bridge.session_finished.emit()

    async def _recognize(self) -> str:
        def on_partial(text: str) -> None:
            self.latest_text = text
            self.bridge.update_text.emit(text)

        return await self.asr_client.run(
            audio_chunks=iter(self.recorder.iter_chunks()),
            on_partial=on_partial,
            stop_event=self.stop_event,
        )

    async def _recognize_and_post_edit(self) -> str:
        final_text = await self._recognize()
        if not final_text:
            return ""

        if self._should_run_llm_post_edit(self.config):
            min_chars = int(self.config.get("llm_post_edit", {}).get("min_chars", 100) or 0)
            if len(final_text.strip()) >= min_chars:
                self.bridge.update_text.emit("识别完成，正在调用大模型润色...")

        hotwords = list(self.config.get("context", {}).get("hotwords", []))
        context_config = self.config.get("context", {})
        prompt_contexts = [
            str(item.get("text", "")).strip()
            for item in context_config.get("recent_context", [])
            if str(item.get("text", "")).strip()
        ]
        prompt_contexts.extend(
            [
                str(item.get("text", "")).strip()
                for item in context_config.get("prompt_context", [])
                if str(item.get("text", "")).strip()
            ]
        )
        polished = await self.llm_post_editor.polish(
            final_text,
            hotwords=hotwords,
            prompt_contexts=prompt_contexts,
        )
        return self._normalize_final_text(polished)

    @staticmethod
    def _normalize_final_text(text: str) -> str:
        result = str(text).strip()
        if result.endswith("。") or result.endswith("."):
            result = result[:-1].rstrip()
        return result

    def _refresh_recent_context_config(self) -> None:
        context_config = self.config.setdefault("context", {})
        if not context_config.get("enable_recent_context", True):
            context_config["recent_context"] = []
            return
        context_config["recent_context"] = [{"text": text} for text in self.recent_context_history]

    def _remember_recent_context(self, text: str) -> None:
        if not self.config["context"].get("enable_recent_context", True):
            return
        cleaned = text.strip()
        if not cleaned:
            return
        self.recent_context_history.appendleft(cleaned)
        self._refresh_recent_context_config()

    def _create_tray_icon(self) -> QSystemTrayIcon:
        if not QSystemTrayIcon.isSystemTrayAvailable():
            logging.warning("系统托盘不可用，托盘菜单和气泡提示将不可用")
        tray_icon = QSystemTrayIcon(self.qt_app.style().standardIcon(QStyle.StandardPixmap.SP_MediaVolume))
        tray_icon.setToolTip("ASR语音输入")

        menu = QMenu()
        open_config_action = QAction("打开配置文件", self.qt_app)
        open_config_action.triggered.connect(self.open_config_file)
        menu.addAction(open_config_action)

        exit_action = QAction("退出", self.qt_app)
        exit_action.triggered.connect(self.quit_app)
        menu.addAction(exit_action)

        self.tray_menu = menu
        tray_icon.setContextMenu(menu)
        tray_icon.activated.connect(self.on_tray_activated)
        return tray_icon

    def open_config_file(self) -> None:
        try:
            os.startfile(str(self.app_config.path))
        except Exception as exc:
            logging.warning("打开配置文件失败: %s", exc)

    def on_tray_activated(self, reason: QSystemTrayIcon.ActivationReason) -> None:
        if reason == QSystemTrayIcon.ActivationReason.DoubleClick:
            self.open_config_file()

    def quit_app(self) -> None:
        if self.recording or self.pending_stop:
            self._force_stop_recording()
        worker = self.worker_thread
        if worker is not None and worker.is_alive():
            worker.join(timeout=2.0)
            if worker.is_alive():
                logging.warning("识别线程退出超时，程序将继续退出")
        self.input_poll_timer.stop()
        if self.hotkey_thread is not None:
            self.hotkey_thread.stop()
            self.hotkey_thread = None
        if self.input_hook_thread is not None:
            self.input_hook_thread.stop()
            self.input_hook_thread = None
        self.tray_icon.hide()
        self.qt_app.quit()

    def _on_session_finished(self) -> None:
        self.auto_stop_timer.stop()
        self.stop_grace_timer.stop()
        if self.muted_for_recording:
            self.system_volume.safe_restore()
            self.muted_for_recording = False
        self.bridge.hide_overlay.emit()
        self.recording = False
        self.pending_stop = False
        self.worker_thread = None

    def _start_input_hooks(self) -> None:
        self.input_hook_thread = InputHookThread(
            self.input_hook_event, enable_ralt=True, enable_middle=True,
        )
        self.input_hook_thread.start()
        self.input_poll_timer.start()

    def _poll_input_hook(self) -> None:
        if self.input_hook_event.is_set():
            self.input_hook_event.clear()
            self.toggle_recording()

    def run(self) -> None:
        hotkey = self.config.get("hotkey", "ctrl+q")
        self.hotkey_thread = GlobalHotkeyThread(self.input_hook_event, hotkey)
        self.hotkey_thread.start()
        self.hotkey_thread.wait_until_ready()
        self._start_input_hooks()
        self.tray_icon.show()
        self._show_startup_tray_message()
        self.qt_app.exec()

    def _show_startup_tray_message(self) -> None:
        tray_config = self.config.get("tray", {})
        if not tray_config.get("show_startup_message", True):
            return
        hotkey = self.config.get("hotkey", "ctrl+q")
        timeout_ms = int(tray_config.get("startup_message_timeout_ms", 6000) or 6000)
        self.tray_icon.showMessage(
            "ASR语音输入已启动",
            f"按 {hotkey.upper()} / 右Alt / 鼠标中键 开始/停止语音输入",
            QSystemTrayIcon.MessageIcon.Information,
            timeout_ms,
        )


def main() -> None:
    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s %(levelname)s %(message)s",
        handlers=[
            logging.FileHandler(LOG_PATH, encoding="utf-8"),
            logging.StreamHandler(),
        ],
    )
    app = VoiceInputApp()
    app.run()
