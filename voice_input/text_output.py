import ctypes
import time

import pyperclip


user32 = ctypes.windll.user32
KEYEVENTF_KEYUP = 0x0002
KEYEVENTF_EXTENDEDKEY = 0x0001
MAPVK_VK_TO_VSC = 0
VK_CONTROL = 0x11
VK_SHIFT = 0x10
VK_V = 0x56
VK_INSERT = 0x2D
PASTE_METHOD_CTRL_V = "ctrl_v"
PASTE_METHOD_SHIFT_INSERT = "shift_insert"
PASTE_METHOD_CLIPBOARD_ONLY = "clipboard_only"


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
    _send_key_combo(VK_CONTROL, VK_V)


def _send_shift_insert() -> None:
    _send_key_combo(VK_SHIFT, VK_INSERT, key_extended=True)


def _send_key_event(virtual_key: int, *, key_up: bool = False, extended: bool = False) -> None:
    scan_code = user32.MapVirtualKeyW(virtual_key, MAPVK_VK_TO_VSC)
    flags = 0
    if extended:
        flags |= KEYEVENTF_EXTENDEDKEY
    if key_up:
        flags |= KEYEVENTF_KEYUP
    user32.keybd_event(virtual_key, scan_code, flags, 0)


def _send_key_combo(modifier_vk: int, key_vk: int, *, key_extended: bool = False) -> None:
    key_interval = 0.01
    try:
        _send_key_event(modifier_vk)
        time.sleep(key_interval)
        _send_key_event(key_vk, extended=key_extended)
        time.sleep(key_interval)
        _send_key_event(key_vk, key_up=True, extended=key_extended)
        time.sleep(key_interval)
    finally:
        _send_key_event(modifier_vk, key_up=True)


def _normalize_paste_method(paste_method: str | None) -> str:
    valid_methods = {
        PASTE_METHOD_CTRL_V,
        PASTE_METHOD_SHIFT_INSERT,
        PASTE_METHOD_CLIPBOARD_ONLY,
    }
    if paste_method in valid_methods:
        return paste_method
    return PASTE_METHOD_CTRL_V


def paste_text(
    text: str,
    paste_delay_ms: int,
    target_hwnd: int | None = None,
    paste_method: str = PASTE_METHOD_CTRL_V,
) -> None:
    """将文本写入剪贴板，并按配置决定是否自动粘贴。"""
    if not text.strip():
        return

    pyperclip.copy(text)
    normalized_method = _normalize_paste_method(paste_method)
    if normalized_method == PASTE_METHOD_CLIPBOARD_ONLY:
        return

    _activate_target_window(target_hwnd, paste_delay_ms)
    if normalized_method == PASTE_METHOD_SHIFT_INSERT:
        _send_shift_insert()
        return
    _send_ctrl_v()
