from PyQt6.QtCore import Qt, QTimer
from PyQt6.QtGui import QFont, QFontMetrics
from PyQt6.QtWidgets import QApplication, QLabel, QVBoxLayout, QWidget


class FloatingCaption(QWidget):
    CENTER_ALIGN_MAX_CHARS = 18
    TWO_LINE_ALIGN_MAX_CHARS = 40
    SINGLE_LINE_MIN_HEIGHT = 60
    MULTI_LINE_MIN_HEIGHT = 72
    SINGLE_LINE_FONT_SIZE = 12
    DOUBLE_LINE_FONT_SIZE = 12
    MULTI_LINE_FONT_SIZE = 11

    def __init__(
        self,
        width: int,
        height: int,
        margin_bottom: int,
        opacity: float,
    ) -> None:
        super().__init__()
        self.box_width = width
        self.box_height = height
        self.margin_bottom = margin_bottom
        self._all_lines: list[str] = []
        self._scroll_offset = 0
        self._tail_hold_steps = 0
        self._last_display_text = ""
        self.setWindowFlags(
            Qt.WindowType.Tool
            | Qt.WindowType.FramelessWindowHint
            | Qt.WindowType.WindowStaysOnTopHint
        )
        self.setAttribute(Qt.WidgetAttribute.WA_ShowWithoutActivating, True)
        self.setAttribute(Qt.WidgetAttribute.WA_TranslucentBackground, True)
        self.setWindowOpacity(opacity)

        self.label = QLabel("正在录音...", self)
        self.label.setWordWrap(False)
        self.label.setAlignment(Qt.AlignmentFlag.AlignLeft | Qt.AlignmentFlag.AlignTop)
        self.label.setFont(QFont("Microsoft YaHei UI", self.DOUBLE_LINE_FONT_SIZE))
        self._apply_label_style(vertical_padding=12)

        layout = QVBoxLayout()
        layout.setContentsMargins(0, 0, 0, 0)
        layout.addWidget(self.label)
        self.setLayout(layout)
        self.resize(self.box_width, self.box_height)

        self.scroll_timer = QTimer(self)
        self.scroll_timer.timeout.connect(self._advance_scroll)
        self.scroll_interval_ms = 1200

    def show_for_recording(self) -> None:
        screen = QApplication.primaryScreen()
        if screen is None:
            self.show()
            return
        geometry = screen.availableGeometry()
        x = geometry.x() + (geometry.width() - self.box_width) // 2
        y = geometry.y() + geometry.height() - self.box_height - self.margin_bottom
        self.move(x, y)
        self.show()

    def update_text(self, text: str) -> None:
        display_text = self._normalize_text(text)
        if display_text == self._last_display_text:
            return
        self._last_display_text = display_text
        mode, font_size = self._resolve_layout_mode(display_text)
        self._apply_layout_style(mode, font_size)
        self._all_lines = self._wrap_text_to_lines(display_text, font_size)
        visible_count = self._visible_line_count()
        self._scroll_offset = max(0, len(self._all_lines) - visible_count)
        self._tail_hold_steps = 3 if len(self._all_lines) >= 3 else 1
        self._refresh_visible_lines()

    def set_scroll_interval(self, interval_ms: int) -> None:
        self.scroll_interval_ms = max(300, interval_ms)

    def _wrap_text_to_lines(self, text: str, font_size: int) -> list[str]:
        metrics = QFontMetrics(QFont("Microsoft YaHei UI", font_size))
        max_width = max(80, self.box_width - 32)
        wrapped_lines: list[str] = []

        for paragraph in text.splitlines() or [""]:
            if not paragraph:
                wrapped_lines.append("")
                continue

            current = ""
            for char in paragraph:
                candidate = current + char
                if current and metrics.horizontalAdvance(candidate) > max_width:
                    wrapped_lines.append(current)
                    current = char
                else:
                    current = candidate
            if current:
                wrapped_lines.append(current)

        return wrapped_lines or [text]

    @staticmethod
    def _normalize_text(text: str) -> str:
        raw = str(text or "").replace("\r\n", "\n").replace("\r", "\n").strip()
        if not raw:
            return "正在录音..."

        normalized_lines: list[str] = []
        blank_pending = False
        for line in raw.split("\n"):
            cleaned = line.strip()
            if not cleaned:
                if not blank_pending and normalized_lines:
                    normalized_lines.append("")
                blank_pending = True
                continue
            normalized_lines.append(cleaned)
            blank_pending = False

        return "\n".join(normalized_lines) or "正在录音..."

    def _resolve_layout_mode(self, text: str) -> tuple[str, int]:
        single_lines = self._wrap_text_to_lines(text, self.SINGLE_LINE_FONT_SIZE)
        if len(single_lines) <= 1 and len(text) <= self.CENTER_ALIGN_MAX_CHARS:
            return "single", self.SINGLE_LINE_FONT_SIZE

        double_lines = self._wrap_text_to_lines(text, self.DOUBLE_LINE_FONT_SIZE)
        if len(double_lines) <= 2 and len(text) <= self.TWO_LINE_ALIGN_MAX_CHARS:
            return "double", self.DOUBLE_LINE_FONT_SIZE

        return "multi", self.MULTI_LINE_FONT_SIZE

    def _apply_layout_style(self, mode: str, font_size: int) -> None:
        self.label.setFont(QFont("Microsoft YaHei UI", font_size))
        if mode == "single":
            self.label.setAlignment(Qt.AlignmentFlag.AlignHCenter | Qt.AlignmentFlag.AlignVCenter)
            self._apply_label_style(vertical_padding=10)
            self.setMinimumHeight(self.SINGLE_LINE_MIN_HEIGHT)
            return
        if mode == "double":
            self.label.setAlignment(Qt.AlignmentFlag.AlignLeft | Qt.AlignmentFlag.AlignTop)
            self._apply_label_style(vertical_padding=11)
            self.setMinimumHeight(self.MULTI_LINE_MIN_HEIGHT)
            return
        self.label.setAlignment(Qt.AlignmentFlag.AlignLeft | Qt.AlignmentFlag.AlignTop)
        self._apply_label_style(vertical_padding=12)
        self.setMinimumHeight(self.MULTI_LINE_MIN_HEIGHT)

    def _apply_label_style(self, vertical_padding: int) -> None:
        self.label.setStyleSheet(
            f"""
            QLabel {{
                color: #F3F5F7;
                background: rgba(32, 40, 48, 200);
                border: 1px solid rgba(255, 255, 255, 26);
                border-radius: 0px;
                padding: {vertical_padding}px 16px;
            }}
            """
        )

    def _visible_line_count(self) -> int:
        metrics = QFontMetrics(self.label.font())
        line_height = max(1, metrics.lineSpacing())
        available_height = max(1, self.box_height - 28)
        return max(1, available_height // line_height)

    def _refresh_visible_lines(self) -> None:
        visible_count = self._visible_line_count()
        if len(self._all_lines) <= visible_count:
            self.scroll_timer.stop()
            self.label.setText("\n".join(self._all_lines))
            return

        end = self._scroll_offset + visible_count
        visible_lines = self._all_lines[self._scroll_offset:end]
        self.label.setText("\n".join(visible_lines))
        if not self.scroll_timer.isActive():
            self.scroll_timer.start(self.scroll_interval_ms)

    def _advance_scroll(self) -> None:
        visible_count = self._visible_line_count()
        if len(self._all_lines) <= visible_count:
            self.scroll_timer.stop()
            return

        if self._tail_hold_steps > 0:
            self._tail_hold_steps -= 1
            return
        if self._scroll_offset <= 0:
            self.scroll_timer.stop()
            return

        self._scroll_offset -= 1
        self._refresh_visible_lines()
