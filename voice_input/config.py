import sys
from copy import deepcopy
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import tomllib


DEFAULT_CONFIG: dict[str, Any] = {
    "typing": {
        "paste_delay_ms": 120,
        "paste_method": "ctrl_v",
    },
}


def _merge_defaults(data: dict[str, Any], defaults: dict[str, Any]) -> dict[str, Any]:
    if not isinstance(data, dict):
        return deepcopy(defaults)

    merged = deepcopy(defaults)
    for key, value in data.items():
        default_value = merged.get(key)
        if isinstance(value, dict) and isinstance(default_value, dict):
            merged[key] = _merge_defaults(value, default_value)
            continue
        merged[key] = value
    return merged


@dataclass
class AppConfig:
    """配置文件加载器。按优先级搜索 config.toml：当前目录 > exe 同目录 > 项目根目录。"""
    raw: dict[str, Any]
    path: Path

    @classmethod
    def load(cls, path: str | Path = "config.toml") -> "AppConfig":
        config_path = cls._resolve_config_path(path)
        if not config_path.exists():
            raise FileNotFoundError(
                f"未找到配置文件: {config_path}"
            )
        text = config_path.read_text(encoding="utf-8")
        data = tomllib.loads(text)
        return cls(raw=_merge_defaults(data, DEFAULT_CONFIG), path=config_path)

    @staticmethod
    def _resolve_config_path(path: str | Path) -> Path:
        input_path = Path(path)
        if input_path.is_absolute():
            return input_path

        base_dirs = [
            Path.cwd(),
            Path(sys.executable).resolve().parent,
            Path(__file__).resolve().parent.parent,
        ]

        for base_dir in base_dirs:
            candidate = base_dir / input_path
            if candidate.exists():
                return candidate
        return base_dirs[0] / input_path
