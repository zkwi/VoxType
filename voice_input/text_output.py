import ctypes
import time

import pyperclip


user32 = ctypes.windll.user32
KEYEVENTF_KEYUP = 0x0002
VK_CONTROL = 0x11
VK_V = 0x56


def get_foreground_window() -> int:
    return user32.GetForegroundWindow()


def _activate_target_window(target_hwnd: int | None, paste_delay_ms: int) -> None:
    if not target_hwnd or not user32.IsWindow(target_hwnd):
        time.sleep(max(0, paste_delay_ms) / 1000)
        return

    user32.SetForegroundWindow(target_hwnd)
    settle_seconds = max(0, paste_delay_ms) / 1000
    deadline = time.perf_counter() + settle_seconds
    while time.perf_counter() < deadline:
        if get_foreground_window() == target_hwnd:
            break
        time.sleep(0.01)
    remaining = deadline - time.perf_counter()
    if remaining > 0:
        time.sleep(remaining)


def _send_ctrl_v() -> None:
    key_interval = 0.01
    user32.keybd_event(VK_V, 0, KEYEVENTF_KEYUP, 0)
    user32.keybd_event(VK_CONTROL, 0, KEYEVENTF_KEYUP, 0)
    time.sleep(key_interval)
    try:
        user32.keybd_event(VK_CONTROL, 0, 0, 0)
        time.sleep(key_interval)
        user32.keybd_event(VK_V, 0, 0, 0)
        time.sleep(key_interval)
        user32.keybd_event(VK_V, 0, KEYEVENTF_KEYUP, 0)
        time.sleep(key_interval)
    finally:
        user32.keybd_event(VK_CONTROL, 0, KEYEVENTF_KEYUP, 0)


def paste_text(text: str, paste_delay_ms: int, target_hwnd: int | None = None) -> None:
    """将文本写入剪贴板，切换到目标窗口，再用 keybd_event 模拟 Ctrl+V。"""
    if not text.strip():
        return

    pyperclip.copy(text)
    _activate_target_window(target_hwnd, paste_delay_ms)
    _send_ctrl_v()
