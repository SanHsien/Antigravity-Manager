"""Regression tests for tools/check_upstream_updates.py.

Run with:  python -m unittest discover -s tools -p "check_upstream_updates_test.py"

Each test name states the failure it prevents. They fake `gh` and the git helpers, so they need
neither network nor credentials.
"""

from __future__ import annotations

import io
import json
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parent))

import check_upstream_updates as checker  # noqa: E402

REPO_URL = "https://github.com/example/upstream.git"
WATERMARK = 100


def make_baseline(**overrides) -> dict:
    baseline = {
        "repo": REPO_URL,
        "branch": "main",
        "track": "release",
        "reviewed_release": "v1.0.0",
        "reviewed_through": "a" * 40,
        "reviewed_date": "2026-01-01",
        "reviewed_pr_through": WATERMARK,
        "reviewed_issue_through": WATERMARK,
        "decision_log": "REVIEW.md",
    }
    baseline.update(overrides)
    return baseline


class FakeGh:
    """Stands in for `gh pr|issue list --limit N`: newest first, truncated at N, like the real one."""

    def __init__(self, newest: int, title=lambda n: f"item {n}", fail: bool = False):
        self.newest = newest
        self.title = title
        self.fail = fail
        self.calls: list[list[str]] = []

    def __call__(self, cmd, **_kwargs):
        self.calls.append(list(cmd))
        if self.fail:
            return SimpleNamespace(returncode=1, stdout="", stderr="gh: not logged in")
        limit = int(cmd[cmd.index("--limit") + 1])
        rows = [
            {"number": n, "title": self.title(n)}
            for n in range(self.newest, 0, -1)
        ][:limit]
        return SimpleNamespace(returncode=0, stdout=json.dumps(rows), stderr="")


def numbers_of(scan) -> list[int]:
    return [item["number"] for item in scan.items]


class CollectNewTicketsTests(unittest.TestCase):
    def collect(self, gh: FakeGh, kind: str = "pr"):
        with mock.patch.object(checker.subprocess, "run", gh):
            return checker.collect_new_tickets(make_baseline(), kind)

    def test_few_new_items_are_listed_and_marked_complete(self):
        scan = self.collect(FakeGh(newest=WATERMARK + 3))
        self.assertTrue(scan.complete)
        self.assertEqual(numbers_of(scan), [101, 102, 103])

    def test_fast_upstream_is_not_silently_truncated_at_the_old_limit_of_50(self):
        # 180 new items is well past the old `--limit 50`; none may be dropped.
        scan = self.collect(FakeGh(newest=WATERMARK + 180))
        self.assertTrue(scan.complete)
        self.assertEqual(numbers_of(scan), list(range(WATERMARK + 1, WATERMARK + 181)))

    def test_exactly_a_full_page_of_new_items_is_still_complete(self):
        scan = self.collect(FakeGh(newest=WATERMARK + checker.TICKET_LIMIT_START))
        self.assertTrue(scan.complete)
        self.assertEqual(len(scan.items), checker.TICKET_LIMIT_START)

    def test_hitting_the_hard_cap_is_reported_incomplete_not_as_a_clean_list(self):
        scan = self.collect(FakeGh(newest=WATERMARK + checker.TICKET_LIMIT_CAP + 500))
        self.assertFalse(scan.complete)
        self.assertEqual(scan.limit, checker.TICKET_LIMIT_CAP)
        # What it did see is still reported, newest items only.
        self.assertEqual(len(scan.items), checker.TICKET_LIMIT_CAP)

    def test_gh_failure_is_unchecked_none_not_an_empty_scan(self):
        self.assertIsNone(self.collect(FakeGh(newest=WATERMARK + 5, fail=True)))

    def test_nothing_new_is_a_complete_empty_scan(self):
        scan = self.collect(FakeGh(newest=WATERMARK))
        self.assertTrue(scan.complete)
        self.assertEqual(scan.items, [])


class RenderTicketSectionTests(unittest.TestCase):
    def render(self, scan) -> str:
        return "\n".join(
            checker.render_ticket_section(
                "Upstream pull requests", WATERMARK, scan, "pr", "REVIEW.md"
            )
        )

    def test_incomplete_scan_never_says_there_is_nothing_new(self):
        scan = checker.TicketScan(
            items=[{"number": 5000, "title": "newest"}], complete=False, limit=1000
        )
        text = self.render(scan)
        self.assertIn("INCOMPLETE", text)
        self.assertIn("#5000", text)
        self.assertNotIn("No new items above that number", text)

    def test_complete_empty_scan_says_nothing_new(self):
        text = self.render(checker.TicketScan(items=[], complete=True, limit=100))
        self.assertIn("No new items above that number", text)
        self.assertNotIn("INCOMPLETE", text)

    def test_unchecked_is_still_distinguished_from_nothing_new(self):
        text = self.render(None)
        self.assertIn("Not checked", text)
        self.assertNotIn("No new items above that number", text)


class MainTests(unittest.TestCase):
    """Drive main() end to end with git and gh faked out."""

    def run_main(self, gh: FakeGh, stdout=None, strict: bool = False):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "report.md"
            argv = ["check_upstream_updates.py", "--output", str(out)]
            if strict:
                argv.append("--strict")
            patches = [
                mock.patch.object(checker, "load_baseline", lambda *a, **k: make_baseline()),
                mock.patch.object(
                    checker, "latest_upstream_release", lambda *a, **k: ("v1.0.0", "a" * 40)
                ),
                mock.patch.object(checker, "fetch_upstream", lambda *a, **k: "refs/x"),
                mock.patch.object(
                    checker,
                    "fork_status",
                    lambda *a, **k: {
                        "baseline": "aaaaaaa",
                        "fork_head": "bbbbbbb",
                        "upstream_tip": "ccccccc",
                        "ahead": 0,
                        "behind": 0,
                    },
                ),
                mock.patch.object(checker.subprocess, "run", gh),
                mock.patch.object(sys, "argv", argv),
                mock.patch.object(sys, "stdout", stdout if stdout is not None else io.StringIO()),
            ]
            for patch in patches:
                patch.start()
            try:
                code = checker.main()
            finally:
                for patch in reversed(patches):
                    patch.stop()
            return code, out.read_text(encoding="utf-8")

    def test_report_lists_every_new_item_when_upstream_outpaces_the_old_limit(self):
        code, report = self.run_main(FakeGh(newest=WATERMARK + 120), strict=True)
        self.assertEqual(code, 1, "new items under --strict must fail, not pass quietly")
        for number in range(WATERMARK + 1, WATERMARK + 121):
            self.assertIn(f"| #{number} |", report)

    def test_cap_reached_exits_2_and_says_incomplete(self):
        code, report = self.run_main(FakeGh(newest=WATERMARK + checker.TICKET_LIMIT_CAP + 1))
        self.assertEqual(code, 2, "an unverifiable listing must not exit 0 or look like 'no news'")
        self.assertIn("INCOMPLETE", report)

    def test_non_ascii_titles_do_not_crash_a_cp950_console(self):
        # A Traditional-Chinese Windows console encodes stdout as cp950, which has no emoji.
        # The checker used to die with UnicodeEncodeError at print(report).
        console = io.TextIOWrapper(io.BytesIO(), encoding="cp950", errors="strict", newline="\n")
        gh = FakeGh(newest=WATERMARK + 2, title=lambda n: f"fix \U0001F680 crash #{n}")
        code, report = self.run_main(gh, stdout=console)
        console.flush()
        self.assertEqual(code, 0)
        self.assertIn("\U0001F680", report, "the report FILE keeps the exact title (UTF-8)")
        shown = console.buffer.getvalue().decode("cp950")
        self.assertIn("crash #101", shown)
        self.assertIn("\\U0001f680", shown, "console shows an escape, never a crash or silent loss")

    def test_console_output_is_identical_to_the_file_when_the_console_can_encode_it(self):
        console = io.TextIOWrapper(io.BytesIO(), encoding="utf-8", errors="strict", newline="\n")
        code, report = self.run_main(
            FakeGh(newest=WATERMARK + 1, title=lambda n: "修復 \U0001F680"), stdout=console
        )
        console.flush()
        self.assertEqual(code, 0)
        shown = console.buffer.getvalue().decode("utf-8")
        self.assertIn("修復 \U0001F680", shown)
        self.assertEqual(shown.rstrip("\n"), report.rstrip("\n"))


class EmitTests(unittest.TestCase):
    def test_emit_never_raises_on_an_unencodable_character(self):
        stream = io.TextIOWrapper(io.BytesIO(), encoding="ascii", errors="strict", newline="\n")
        checker.emit("café \U0001F680", stream)
        stream.flush()
        self.assertEqual(stream.buffer.getvalue().decode("ascii").strip(), "caf\\xe9 \\U0001f680")

    def test_emit_works_on_a_stream_without_an_encoding_attribute(self):
        stream = io.StringIO()
        checker.emit("修復", stream)
        self.assertEqual(stream.getvalue(), "修復\n")


if __name__ == "__main__":
    unittest.main()
