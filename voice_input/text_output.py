import ctypes
import time

import pyperclip


user32 = ctypes.windll.user32
KEYEVENTF_KEYUP = 0x0002
VK_CONTROL = 0x11
VK_V = 0x56


def get_foreground_window() -> int:
    return user32.GetForegroundWindow()


def paste_text(text: str, paste_delay_ms: int, target_hwnd: int | None = None) -> None:
    if not text.strip():
        return

    pyperclip.copy(text)
    if target_hwnd and user32.IsWindow(target_hwnd):
        user32.SetForegroundWindow(target_hwnd)
    time.sleep(max(0, paste_delay_ms) / 1000)
    user32.keybd_event(VK_CONTROL, 0, 0, 0)
    user32.keybd_event(VK_V, 0, 0, 0)
    user32.keybd_event(VK_V, 0, KEYEVENTF_KEYUP, 0)
    user32.keybd_event(VK_CONTROL, 0, KEYEVENTF_KEYUP, 0)
