import argparse
import re
import subprocess
import sys
from pathlib import Path


# 只做本地粗扫，目标是尽快发现明显的误提交风险。
ROOT = Path(__file__).resolve().parents[1]
DEFAULT_EXCLUDE_DIRS = {
    ".git",
    ".idea",
    ".pytest_cache",
    ".mypy_cache",
    ".ruff_cache",
    ".venv",
    "venv",
    "env",
    "__pycache__",
    "build",
    "dist",
}
DEFAULT_EXCLUDE_SUFFIXES = {
    ".png",
    ".jpg",
    ".jpeg",
    ".gif",
    ".webp",
    ".ico",
    ".pdf",
    ".zip",
    ".7z",
    ".rar",
    ".exe",
    ".dll",
    ".pyd",
    ".so",
    ".wav",
    ".mp3",
    ".mp4",
}
PLACEHOLDER_VALUES = {
    "",
    "null",
    "none",
    "changeme",
    "your-key",
    "your_api_key",
    "your-api-key",
    "your-access-key",
    "your-secret-key",
    "your token",
    "your access key",
    "your secret key",
    "your app key",
    "你的token",
    "你的密钥",
    "你的 api key",
    "你的 access key",
    "你的 app key",
    "你的阿里云百炼 api key",
    "你的豆包 access key",
    "你的豆包 app key",
}

SECRET_PATTERNS = [
    ("OpenAI-style key", re.compile(r"sk-[A-Za-z0-9_-]{16,}")),
    ("GitHub PAT", re.compile(r"github_pat_[A-Za-z0-9_]{20,}")),
    ("GitHub token", re.compile(r"ghp_[A-Za-z0-9]{20,}")),
    ("Slack token", re.compile(r"xox[baprs]-[A-Za-z0-9-]{10,}")),
    ("AWS access key", re.compile(r"AKIA[0-9A-Z]{16}")),
    (
        "Private key block",
        re.compile(r"-----BEGIN (?:RSA|DSA|EC|OPENSSH|PGP) PRIVATE KEY-----"),
    ),
]

ASSIGNMENT_PATTERN = re.compile(
    r"""(?ix)
    (?P<key>
        api[_-]?key|
        access[_-]?key|
        app[_-]?key|
        secret|
        secret[_-]?key|
        client[_-]?secret|
        token|
        password|
        passwd|
        pwd
    )
    \s*
    (?:
        [:=]
        |
        => 
    )
    \s*
    (?:
        (?P<q1>["'])(?P<quoted_value>.*?)(?P=q1)
        |
        (?P<bare_value>[A-Za-z0-9_./:+-]{8,})
    )
    \s*$
    """
)


def is_probably_binary(path: Path) -> bool:
    if path.suffix.lower() in DEFAULT_EXCLUDE_SUFFIXES:
        return True
    try:
        chunk = path.read_bytes()[:2048]
    except OSError:
        return True
    return b"\x00" in chunk


def should_skip(path: Path) -> bool:
    parts = set(path.parts)
    if parts & DEFAULT_EXCLUDE_DIRS:
        return True
    return is_probably_binary(path)


def looks_like_placeholder(value: str) -> bool:
    normalized = value.strip().strip('"').strip("'").lower()
    if not normalized:
        return True
    if normalized in PLACEHOLDER_VALUES:
        return True
    if normalized.isdigit():
        return True
    if normalized in {"true", "false"}:
        return True
    return any(word in normalized for word in ("example", "placeholder", "示例", "填写", "这里填"))


def scan_file(path: Path) -> list[tuple[int, str, str]]:
    findings: list[tuple[int, str, str]] = []
    try:
        text = path.read_text(encoding="utf-8")
    except UnicodeDecodeError:
        text = path.read_text(encoding="utf-8", errors="ignore")
    except OSError:
        return findings

    for line_no, line in enumerate(text.splitlines(), start=1):
        stripped = line.strip()
        if not stripped or stripped.startswith(("#", "//")):
            continue
        if stripped.count("|") >= 2:
            continue

        for name, pattern in SECRET_PATTERNS:
            match = pattern.search(line)
            if match:
                findings.append((line_no, name, mask_secret(match.group(0))))

        assignment = ASSIGNMENT_PATTERN.search(stripped)
        if assignment:
            value = assignment.group("quoted_value") or assignment.group("bare_value") or ""
            value = value.strip().strip('"').strip("'")
            if value and len(value) >= 8 and not looks_like_placeholder(value):
                if re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", value):
                    continue
                findings.append((line_no, f"Suspicious {assignment.group('key')}", mask_secret(value)))

    return findings


def mask_secret(value: str) -> str:
    if len(value) <= 8:
        return "*" * len(value)
    return f"{value[:4]}...{value[-4:]}"


def iter_files(root: Path):
    for path in root.rglob("*"):
        if not path.is_file():
            continue
        if should_skip(path.relative_to(root)):
            continue
        yield path


def get_git_root() -> Path | None:
    try:
        result = subprocess.run(
            ["git", "rev-parse", "--show-toplevel"],
            capture_output=True,
            text=True,
            check=True,
        )
    except (OSError, subprocess.CalledProcessError):
        return None
    return Path(result.stdout.strip())


def get_staged_files(repo_root: Path) -> list[Path]:
    try:
        result = subprocess.run(
            ["git", "diff", "--cached", "--name-only", "--diff-filter=ACMR"],
            cwd=repo_root,
            capture_output=True,
            text=True,
            check=True,
        )
    except (OSError, subprocess.CalledProcessError):
        return []

    files: list[Path] = []
    for line in result.stdout.splitlines():
        relative = line.strip()
        if not relative:
            continue
        path = repo_root / relative
        if path.is_file():
            files.append(path)
    return files


def main() -> int:
    parser = argparse.ArgumentParser(description="Scan repository for likely secrets.")
    parser.add_argument("path", nargs="?", default=".", help="Directory to scan")
    parser.add_argument(
        "--staged",
        action="store_true",
        help="Scan only files staged for commit",
    )
    args = parser.parse_args()

    if args.staged:
        repo_root = get_git_root()
        if repo_root is None:
            print("[scan-secrets] not inside a git repository")
            return 2
        target = repo_root
        files = get_staged_files(repo_root)
        if not files:
            print("[scan-secrets] no staged files to scan")
            return 0
    else:
        target = Path(args.path).resolve()
        if not target.exists():
            print(f"[scan-secrets] path not found: {target}")
            return 2
        files = [target] if target.is_file() else list(iter_files(target))

    findings_count = 0

    for file_path in files:
        if not args.staged and file_path.is_file() and target.is_file():
            rel = file_path
            if should_skip(Path(file_path.name)):
                continue
        else:
            rel = file_path.relative_to(target)

        findings = scan_file(file_path)
        for line_no, kind, masked in findings:
            findings_count += 1
            print(f"{rel}:{line_no}: {kind}: {masked}")

    if findings_count:
        print(f"[scan-secrets] found {findings_count} potential secret(s)")
        return 1

    print("[scan-secrets] no obvious secrets found")
    return 0


if __name__ == "__main__":
    sys.exit(main())
