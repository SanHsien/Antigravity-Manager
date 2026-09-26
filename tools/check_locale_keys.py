#!/usr/bin/env python3
"""Fail if src/locales/zh-TW.json is missing any key present in en.json.

en.json is treated as the source-of-truth key set (it is kept in sync with
the app's translation calls). zh-TW.json is the only Chinese locale this
fork ships (see FORK.md / AGENTS.md: Simplified Chinese was dropped in
favor of Traditional-only), so it must never silently fall back to a
missing-key placeholder for a Traditional Chinese user.

    python tools/check_locale_keys.py
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EN_PATH = ROOT / "src" / "locales" / "en.json"
ZHTW_PATH = ROOT / "src" / "locales" / "zh-TW.json"


def flatten(data: dict, prefix: str = "") -> set[str]:
    keys: set[str] = set()
    for key, value in data.items():
        full = f"{prefix}.{key}" if prefix else key
        if isinstance(value, dict):
            keys |= flatten(value, full)
        else:
            keys.add(full)
    return keys


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def main() -> int:
    if not EN_PATH.is_file() or not ZHTW_PATH.is_file():
        print(f"FAIL: missing locale file(s): {EN_PATH} / {ZHTW_PATH}")
        return 1

    en_keys = flatten(load(EN_PATH))
    zhtw_keys = flatten(load(ZHTW_PATH))
    missing = sorted(en_keys - zhtw_keys)

    if missing:
        print(f"FAIL: zh-TW.json is missing {len(missing)} key(s) present in en.json:")
        for key in missing:
            print(f"  {key}")
        return 1

    print(f"OK   zh-TW.json has all {len(en_keys)} keys present in en.json.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
