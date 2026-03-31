import json
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any


@dataclass
class AppConfig:
    """配置文件加载器。按优先级搜索 config.json：当前目录 > exe 同目录 > 项目根目录。"""
    raw: dict[str, Any]
    path: Path

    @classmethod
    def load(cls, path: str | Path = "config.json") -> "AppConfig":
        config_path = cls._resolve_config_path(path)
        if not config_path.exists():
            raise FileNotFoundError(
                f"未找到配置文件: {config_path}"
            )
        data = json.loads(config_path.read_text(encoding="utf-8"))
        return cls(raw=data, path=config_path)

    @staticmethod
    def _resolve_config_path(path: str | Path) -> Path:
        input_path = Path(path)
        if input_path.is_absolute():
            return input_path

        candidates = [
            Path.cwd() / input_path,
            Path(sys.executable).resolve().parent / input_path,
            Path(__file__).resolve().parent.parent / input_path,
        ]

        for candidate in candidates:
            if candidate.exists():
                return candidate
        return candidates[0]

    def get(self, *keys: str, default: Any = None) -> Any:
        current: Any = self.raw
        for key in keys:
            if not isinstance(current, dict) or key not in current:
                return default
            current = current[key]
        return current
