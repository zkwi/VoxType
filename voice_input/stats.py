import json
from dataclasses import dataclass
from datetime import datetime, timedelta
from pathlib import Path


@dataclass
class UsageStats:
    session_count: int = 0
    total_seconds: float = 0.0
    total_chars: int = 0

    @property
    def total_minutes_int(self) -> int:
        return int(self.total_seconds // 60)

    @property
    def avg_chars_per_minute(self) -> float:
        if self.total_seconds <= 0:
            return 0.0
        return self.total_chars / (self.total_seconds / 60)


def count_text_chars(text: str) -> int:
    # 统计时忽略空白，更接近“实际输入了多少文字”
    return len("".join(str(text).split()))


class StatsStore:
    def __init__(self, path: str | Path) -> None:
        self.path = Path(path)

    def append_event(self, *, text: str, duration_seconds: float, created_at: datetime | None = None) -> None:
        event = {
            "created_at": (created_at or datetime.now()).isoformat(timespec="seconds"),
            "duration_seconds": round(max(0.0, float(duration_seconds)), 2),
            "text_chars": count_text_chars(text),
        }
        self.path.parent.mkdir(parents=True, exist_ok=True)
        with self.path.open("a", encoding="utf-8") as file:
            file.write(json.dumps(event, ensure_ascii=False) + "\n")

    def summarize_since(self, start_time: datetime) -> UsageStats:
        stats = UsageStats()
        for event in self._iter_events():
            created_at = event.get("created_at")
            if created_at is None or created_at < start_time:
                continue
            stats.session_count += 1
            stats.total_seconds += float(event.get("duration_seconds", 0.0) or 0.0)
            stats.total_chars += int(event.get("text_chars", 0) or 0)
        return stats

    def summarize_recent_days(self, days: int) -> UsageStats:
        return self.summarize_since(datetime.now() - timedelta(days=days))

    def summarize_recent_hours(self, hours: int) -> UsageStats:
        return self.summarize_since(datetime.now() - timedelta(hours=hours))

    def summarize_by_day(self, days: int = 7) -> list[tuple[str, UsageStats]]:
        today = datetime.now().date()
        stats_by_day = {
            (today - timedelta(days=offset)).isoformat(): UsageStats()
            for offset in range(days)
        }
        for event in self._iter_events():
            created_at = event.get("created_at")
            if created_at is None:
                continue
            day_key = created_at.date().isoformat()
            day_stats = stats_by_day.get(day_key)
            if day_stats is None:
                continue
            day_stats.session_count += 1
            day_stats.total_seconds += float(event.get("duration_seconds", 0.0) or 0.0)
            day_stats.total_chars += int(event.get("text_chars", 0) or 0)
        return sorted(stats_by_day.items())

    def _iter_events(self):
        if not self.path.exists():
            return
        with self.path.open("r", encoding="utf-8") as file:
            for line in file:
                line = line.strip()
                if not line:
                    continue
                try:
                    raw = json.loads(line)
                    created_at = datetime.fromisoformat(str(raw["created_at"]))
                    yield {
                        "created_at": created_at,
                        "duration_seconds": raw.get("duration_seconds", 0.0),
                        "text_chars": raw.get("text_chars", 0),
                    }
                except Exception:
                    continue
