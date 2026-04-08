"""
专属线程 + 专属消息泵的低级输入钩子。
回调只做 threading.Event.set()，避免 GIL 竞争导致系统卡死。
"""

import ctypes
import ctypes.wintypes as wt
import logging
import threading

user32 = ctypes.windll.user32
ULONG_PTR = ctypes.c_ulonglong if ctypes.sizeof(ctypes.c_void_p) == 8 else ctypes.c_ulong

# 常量
WH_KEYBOARD_LL = 13
WH_MOUSE_LL = 14
WM_KEYUP = 0x0101
WM_SYSKEYUP = 0x0105
WM_MBUTTONDOWN = 0x0207
WM_MBUTTONUP = 0x0208
VK_RMENU = 0xA5  # 右侧 Alt
LLKHF_INJECTED = 0x10
LLKHF_LOWER_IL_INJECTED = 0x02
LLMHF_INJECTED = 0x00000001

# 正确的签名: LRESULT CALLBACK proc(int nCode, WPARAM wParam, LPARAM lParam)
HOOKPROC = ctypes.WINFUNCTYPE(
    ctypes.c_long,   # LRESULT 返回值
    ctypes.c_int,    # nCode
    ctypes.c_ulonglong if ctypes.sizeof(ctypes.c_void_p) == 8 else ctypes.c_ulong,  # WPARAM
    ctypes.c_longlong if ctypes.sizeof(ctypes.c_void_p) == 8 else ctypes.c_long,    # LPARAM
)

# 声明 CallNextHookExW 的参数/返回类型，确保 64 位下 LPARAM 不被截断
user32.CallNextHookEx.argtypes = [wt.HHOOK, ctypes.c_int,
                                   ctypes.c_ulonglong if ctypes.sizeof(ctypes.c_void_p) == 8 else ctypes.c_ulong,
                                   ctypes.c_longlong if ctypes.sizeof(ctypes.c_void_p) == 8 else ctypes.c_long]
user32.CallNextHookEx.restype = ctypes.c_long


class KBDLLHOOKSTRUCT(ctypes.Structure):
    _fields_ = [
        ("vkCode", wt.DWORD),
        ("scanCode", wt.DWORD),
        ("flags", wt.DWORD),
        ("time", wt.DWORD),
        ("dwExtraInfo", ULONG_PTR),
    ]


class MSLLHOOKSTRUCT(ctypes.Structure):
    _fields_ = [
        ("pt", wt.POINT),
        ("mouseData", wt.DWORD),
        ("flags", wt.DWORD),
        ("time", wt.DWORD),
        ("dwExtraInfo", ULONG_PTR),
    ]


def _is_injected_keyboard_event(flags: int) -> bool:
    return bool(flags & (LLKHF_INJECTED | LLKHF_LOWER_IL_INJECTED))


def _is_injected_mouse_event(flags: int) -> bool:
    return bool(flags & LLMHF_INJECTED)


class InputHookThread(threading.Thread):
    """在独立线程中运行低级键盘+鼠标钩子，带专属消息泵。"""

    def __init__(self, event: threading.Event, enable_ralt: bool = True, enable_middle: bool = True):
        super().__init__(daemon=True)
        self.event = event
        self.enable_ralt = enable_ralt
        self.enable_middle = enable_middle
        self._thread_id: int | None = None
        # 必须持有回调引用，防止被 GC 回收导致崩溃
        self._kb_proc = None
        self._mouse_proc = None

    def run(self) -> None:
        self._thread_id = ctypes.windll.kernel32.GetCurrentThreadId()
        kb_hook = None
        mouse_hook = None

        try:
            if self.enable_ralt:
                self._kb_proc = HOOKPROC(self._keyboard_callback)
                kb_hook = user32.SetWindowsHookExW(WH_KEYBOARD_LL, self._kb_proc, None, 0)
                if not kb_hook:
                    logging.error("安装键盘钩子失败: %s", ctypes.GetLastError())

            if self.enable_middle:
                self._mouse_proc = HOOKPROC(self._mouse_callback)
                mouse_hook = user32.SetWindowsHookExW(WH_MOUSE_LL, self._mouse_proc, None, 0)
                if not mouse_hook:
                    logging.error("安装鼠标钩子失败: %s", ctypes.GetLastError())

            # 专属消息泵
            msg = wt.MSG()
            while user32.GetMessageW(ctypes.byref(msg), None, 0, 0) > 0:
                user32.TranslateMessage(ctypes.byref(msg))
                user32.DispatchMessageW(ctypes.byref(msg))
        finally:
            if kb_hook:
                user32.UnhookWindowsHookEx(kb_hook)
            if mouse_hook:
                user32.UnhookWindowsHookEx(mouse_hook)

    def stop(self) -> None:
        """向钩子线程的消息泵发送 WM_QUIT，让 GetMessage 返回 0 退出循环。"""
        if self._thread_id is not None:
            user32.PostThreadMessageW(self._thread_id, 0x0012, 0, 0)  # WM_QUIT

    def _keyboard_callback(self, nCode, wParam, lParam):
        # wParam 是整数: WM_KEYUP / WM_SYSKEYUP 等
        # lParam 是指向 KBDLLHOOKSTRUCT 的指针（整数形式）
        if nCode >= 0 and wParam in (WM_KEYUP, WM_SYSKEYUP):
            kb = KBDLLHOOKSTRUCT.from_address(lParam)
            if kb.vkCode == VK_RMENU:
                if _is_injected_keyboard_event(kb.flags):
                    logging.info("忽略注入的右 Alt 事件")
                    return user32.CallNextHookEx(None, nCode, wParam, lParam)
                logging.info("输入触发来源: 右 Alt")
                self.event.set()
        return user32.CallNextHookEx(None, nCode, wParam, lParam)

    def _mouse_callback(self, nCode, wParam, lParam):
        # wParam 是整数: WM_MBUTTONDOWN / WM_MBUTTONUP 等
        if nCode >= 0:
            mouse = MSLLHOOKSTRUCT.from_address(lParam)
            if _is_injected_mouse_event(mouse.flags):
                if wParam in (WM_MBUTTONDOWN, WM_MBUTTONUP):
                    logging.info("忽略注入的鼠标中键事件")
                return user32.CallNextHookEx(None, nCode, wParam, lParam)
            if wParam == WM_MBUTTONDOWN:
                logging.info("输入触发来源: 鼠标中键")
                self.event.set()
                return 1  # 吞掉中键按下
            if wParam == WM_MBUTTONUP:
                return 1  # 吞掉中键抬起，保持配对
        return user32.CallNextHookEx(None, nCode, wParam, lParam)
