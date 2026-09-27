#!/usr/bin/env python3
"""檢查本 fork overlay 文件之間的相對連結。

驗證 FORK.md、NOTICE.md、REVIEW.md、CLAUDE.md、GEMINI.md、SECURITY.md、
CONTRIBUTING.md、AGENTS.md、README.md、README_EN.md 與 docs/ 內文件的相對連結。

    python tools/check_links.py
"""

from __future__ import annotations

import re
import sys
from pathlib import Path
from urllib.parse import unquote

ROOT = Path(__file__).resolve().parent.parent
LINK_PATTERN = re.compile(r"\[[^\]]*\]\(([^)]+)\)")
IMAGE_PATTERN = re.compile(r"!\[[^\]]*\]\(([^)]+)\)")
HTML_SRC_PATTERN = re.compile(r"""(?is)<img[^>]+src=["']([^"']+)["']""")
SKIP_PREFIXES = ("http://", "https://", "mailto:", "tel:", "#")
SKIP_NAMES = {
    "upstream-review-report.md",
    "dependency-freshness-report.md",
}

FORK_DOCUMENTS = [
    ROOT / "FORK.md",
    ROOT / "NOTICE.md",
    ROOT / "REVIEW.md",
    ROOT / "CLAUDE.md",
    ROOT / "GEMINI.md",
    ROOT / "SECURITY.md",
    ROOT / "CONTRIBUTING.md",
    ROOT / "AGENTS.md",
]


def iter_documents() -> list[Path]:
    documents = [path for path in FORK_DOCUMENTS if path.is_file()]
    return [path for path in documents if path.name not in SKIP_NAMES]


def _missing_relative(path: Path, target: str) -> str | None:
    target = target.strip().strip("<>")
    if not target or target.startswith(SKIP_PREFIXES):
        return None
    file_part = unquote(target.split("#", 1)[0])
    if not file_part:
        return None
    resolved = (path.parent / file_part).resolve()
    if not resolved.is_relative_to(ROOT):
        return f"{target} → 逃出 repo 根目錄"
    if resolved.exists():
        return None
    try:
        shown = resolved.relative_to(ROOT)
    except ValueError:
        shown = resolved
    return f"{target} → 找不到 {shown}"


def check_document(path: Path) -> list[str]:
    problems: list[str] = []
    text = path.read_text(encoding="utf-8")
    for pattern in (LINK_PATTERN, IMAGE_PATTERN, HTML_SRC_PATTERN):
        for match in pattern.finditer(text):
            missing = _missing_relative(path, match.group(1))
            if missing:
                problems.append(missing)
    return problems


def main() -> int:
    documents = iter_documents()
    if not documents:
        print("找不到任何 fork overlay Markdown 檔")
        return 1

    failures = 0
    for path in documents:
        problems = check_document(path)
        rel = path.relative_to(ROOT)
        if problems:
            failures += 1
            for problem in problems:
                print(f"FAIL {rel}: {problem}")
        else:
            print(f"OK   {rel}")

    print(f"\n共 {len(documents)} 份 overlay 文件，{failures} 份有缺檔。")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
