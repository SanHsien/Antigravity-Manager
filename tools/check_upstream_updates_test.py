"""Properties of the canonical fork upstream checker.

CANONICAL COPY (SanHsien/repo-fleet-ops, ``tools/canonical/``), shipped to every fork next to
``check_upstream_updates.py`` as ``tools/check_upstream_updates_test.py``. Standard library only,
so a fork needs no test framework: ``python -m unittest discover -s tools -p
"check_upstream_updates_test.py"``.

Every test name states what it guards against. They pin behaviour -- not the implementation -- so
a copy of the checker that passes them is safe to run unattended in a workflow.
"""

from __future__ import annotations

import importlib.util
import io
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

CHECKER = Path(__file__).resolve().with_name("check_upstream_updates.py")


def _load():
    spec = importlib.util.spec_from_file_location("fleet_upstream_check_under_test", CHECKER)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


check = _load()


def completed(stdout: str = "", returncode: int = 0, stderr: str = ""):
    return subprocess.CompletedProcess([], returncode, stdout=stdout, stderr=stderr)


def fake_gh(prs=(), issues=(), titles=None):
    """A ``run_gh`` replacement serving numbered corpora, newest first, honouring ``--limit``."""
    corpora = {"pr": sorted(prs, reverse=True), "issue": sorted(issues, reverse=True)}
    calls: list[list[str]] = []

    def run(args: list[str]):
        calls.append(args)
        limit = int(args[args.index("--limit") + 1])
        rows = [
            {"number": n, "title": (titles or {}).get(n, f"item {n}")}
            for n in corpora[args[0]][:limit]
        ]
        return completed(json.dumps(rows))

    run.calls = calls
    return run


def baseline_for(sha: str, **extra) -> dict:
    return {
        "repo": "r",
        "branch": "main",
        "reviewed_through": sha,
        "reviewed_date": "2026-01-01",
        **extra,
    }


def git(cwd: Path, *args: str) -> str:
    env = {
        "GIT_AUTHOR_NAME": "t",
        "GIT_AUTHOR_EMAIL": "t@example.invalid",
        "GIT_COMMITTER_NAME": "t",
        "GIT_COMMITTER_EMAIL": "t@example.invalid",
        "PATH": os.environ.get("PATH", ""),
        "SYSTEMROOT": os.environ.get("SYSTEMROOT", ""),
        "HOME": str(cwd),
    }
    done = subprocess.run(
        ["git", *args], cwd=cwd, env=env, capture_output=True, text=True, encoding="utf-8"
    )
    assert done.returncode == 0, done.stderr
    return done.stdout.strip()


def commit(repo: Path, name: str, message: str) -> str:
    (repo / name).write_text(message, encoding="utf-8")
    git(repo, "add", name)
    git(repo, "commit", "-q", "-m", message)
    return git(repo, "rev-parse", "HEAD")


class PatchedCase(unittest.TestCase):
    """Slug resolution and ``gh`` are stubbed; a test overrides ``self.gh`` as needed."""

    def setUp(self):
        self.addCleanup(mock.patch.stopall)
        mock.patch.object(check, "upstream_slug", lambda repo, repo_dir=None: "owner/name").start()
        self.set_gh(fake_gh())

    def set_gh(self, fn):
        mock.patch.object(check, "run_gh", fn).start()


class TicketTruncation(PatchedCase):
    def test_listing_grows_until_it_reaches_the_watermark(self):
        """Upstream moved 2300 numbers past a watermark a 100-item listing cannot see."""
        gh = fake_gh(prs=range(1, 2301))
        self.set_gh(gh)
        scan = check.collect_new_tickets({"repo": "r", "reviewed_pr_through": 1500}, "pr", cap=5000)
        self.assertTrue(scan.complete)
        self.assertEqual([i["number"] for i in scan.items], list(range(1501, 2301)))
        self.assertGreater(len(gh.calls), 1)

    def test_listing_that_hits_the_cap_is_reported_incomplete_never_clean(self):
        """The rea reproduction: ~2300 unreviewed numbers behind a 1000-item gh cap."""
        self.set_gh(fake_gh(prs=range(1, 2301)))
        scan = check.collect_new_tickets({"repo": "r", "reviewed_pr_through": 0}, "pr")
        self.assertFalse(scan.complete)
        self.assertEqual(len(scan.items), check.TICKET_LIMIT_CAP)
        text = "\n".join(check.render_ticket_section("PRs", 0, scan, "pr", "docs/DECISIONS.md"))
        self.assertIn("INCOMPLETE", text)
        self.assertIn("NOT a full list", text)

    def test_incomplete_scan_exits_2_even_under_strict(self):
        """An unfinished scan must not collapse into the 'review needed' exit code."""
        self.set_gh(fake_gh(prs=range(1, 2301)))
        scan = check.collect_new_tickets({"repo": "r", "reviewed_pr_through": 0}, "pr")
        findings = check.Findings(
            {"repo": "r"}, [], prs=scan, issues=check.TicketScan([], True, 100)
        )
        code, message = check.exit_code(findings, strict=True)
        self.assertEqual(code, 2)
        self.assertIn("cap", message)

    def test_default_cap_still_finds_a_watermark_near_the_top_of_a_huge_history(self):
        """3000 PRs, 50 unreviewed: reaching the watermark ends the scan (not INCOMPLETE)."""
        self.set_gh(fake_gh(prs=range(1, 3001)))
        scan = check.collect_new_tickets({"repo": "r", "reviewed_pr_through": 2950}, "pr")
        self.assertTrue(scan.complete)
        self.assertEqual(len(scan.items), 50)

    def test_prs_and_issues_are_listed_with_state_all(self):
        """An item opened and closed between two runs is still one this fork never triaged."""
        gh = fake_gh(prs=[1], issues=[1])
        self.set_gh(gh)
        for kind in ("pr", "issue"):
            check.collect_new_tickets({"repo": "r"}, kind)
        self.assertEqual({call[0] for call in gh.calls}, {"pr", "issue"})
        for call in gh.calls:
            self.assertEqual(call[call.index("--state") + 1], "all")

    def test_exhausted_listing_with_nothing_new_is_complete_and_empty(self):
        self.set_gh(fake_gh(prs=range(1, 30)))
        scan = check.collect_new_tickets({"repo": "r", "reviewed_pr_through": 29}, "pr")
        self.assertTrue(scan.complete)
        self.assertEqual(scan.items, [])

    def test_cap_is_configurable(self):
        self.set_gh(fake_gh(prs=range(1, 500)))
        scan = check.collect_new_tickets({"repo": "r"}, "pr", cap=200)
        self.assertFalse(scan.complete)
        self.assertEqual(scan.limit, 200)


class TicketFailureIsNotEmpty(PatchedCase):
    def assert_unchecked(self, result):
        self.set_gh(lambda args: result)
        self.assertIsNone(check.collect_new_tickets({"repo": "r"}, "pr"))

    def test_nonzero_exit(self):
        self.assert_unchecked(completed("", 1, "HTTP 502"))

    def test_invalid_json(self):
        self.assert_unchecked(completed("not json"))

    def test_json_that_is_not_a_list(self):
        self.assert_unchecked(completed('{"number": 1}'))

    def test_rows_without_a_number(self):
        self.assert_unchecked(completed('[{"title": "no number"}]'))

    def test_gh_that_could_not_run(self):
        self.assert_unchecked(None)

    def test_baseline_without_a_github_url_is_unchecked(self):
        mock.patch.stopall()
        self.assertIsNone(check.collect_new_tickets({"repo": "/some/local/path"}, "pr"))

    def test_disabled_ticket_type_is_reported_not_checked_and_not_a_failure(self):
        err = "the 'o/n' repository has disabled issues"
        self.set_gh(lambda args: completed("", 1, err))
        scan = check.collect_new_tickets({"repo": "r"}, "issue")
        self.assertTrue(scan.disabled)
        text = "\n".join(check.render_ticket_section("Issues", 0, scan, "issue", "d.md"))
        self.assertIn("Not checked", text)
        self.assertNotIn("No new items", text)
        findings = check.Findings({"repo": "r"}, [], prs=check.TicketScan([], True, 1), issues=scan)
        self.assertEqual(check.exit_code(findings, strict=True), (0, None))


class RunnerFailures(unittest.TestCase):
    def test_missing_gh_executable_is_unchecked(self):
        with mock.patch.object(check.subprocess, "run", side_effect=FileNotFoundError("gh")):
            self.assertIsNone(check.run_gh(["pr", "list"]))

    def test_hung_gh_is_unchecked(self):
        with mock.patch.object(
            check.subprocess, "run", side_effect=subprocess.TimeoutExpired("gh", 1)
        ):
            self.assertIsNone(check.run_gh(["pr", "list"]))

    def test_missing_git_executable_is_a_check_error(self):
        missing = mock.patch.object(check.subprocess, "run", side_effect=FileNotFoundError("git"))
        with missing, self.assertRaisesRegex(check.UpstreamCheckError, "could not run"):
            check.run_git(["status"], Path("."))

    def test_hung_git_is_a_check_error(self):
        with (
            mock.patch.object(
                check.subprocess, "run", side_effect=subprocess.TimeoutExpired("git", 1)
            ),
            self.assertRaisesRegex(check.UpstreamCheckError, "timed out"),
        ):
            check.run_git(["fetch"], Path("."))


class SupportedPythons(unittest.TestCase):
    def test_script_parses_as_python_3_9_and_avoids_runtime_union_types(self):
        """Forks still test on 3.9: `isinstance(x, int | str)` is a TypeError there (3.10+)."""
        import ast

        source = CHECKER.read_text(encoding="utf-8")
        ast.parse(source, feature_version=(3, 9))
        self.assertNotRegex(source, r"isinstance\([^\n]*\|")


class TagParsing(unittest.TestCase):
    def test_two_part_release_tags_are_tracked(self):
        """jev-chat-jarvis upstream tags `v1.7`; dropping it exits 2 every week."""
        raw = "a" * 40 + "\trefs/tags/v1.7\n" + "b" * 40 + "\trefs/tags/global-v0.1.0\n"
        self.assertEqual(check.parse_tag_refs(raw), [((1, 7, 0), "v1.7", "a" * 40)])

    def test_highest_version_wins_and_peeled_commit_is_used(self):
        raw = (
            f"{'1' * 40}\trefs/tags/v1.9.0\n{'2' * 40}\trefs/tags/v1.10.0\n"
            f"{'3' * 40}\trefs/tags/v1.10.0^{{}}\n"
        )
        self.assertEqual(check.parse_tag_refs(raw)[-1][1:], ("v1.10.0", "3" * 40))


class RunnerEdges(unittest.TestCase):
    def test_undecodable_output_does_not_kill_the_check(self):
        """One non-UTF-8 byte in a title or subject must not raise."""
        script = "import sys; sys.stdout.buffer.write(b'ok \\xff\\xfe')"
        done = check._run([sys.executable, "-c", script], None, 30)
        self.assertIn("ok", done.stdout)

    def test_default_track_is_commit(self):
        report = check.render_markdown(baseline_for("a" * 40), [])
        self.assertIn("Tracking: commit", report)


class SlugResolution(unittest.TestCase):
    def test_trailing_slash_and_dot_git_are_tolerated(self):
        """A repo URL written with a trailing slash must not turn every ticket axis 'unchecked'."""
        for url in (
            "https://github.com/o/n/",
            "https://github.com/o/n.git",
            "https://github.com/o/n.git/",
            "git@github.com:o/n.git",
        ):
            self.assertEqual(check.upstream_slug(url, None), "o/n", url)

    def test_remote_name_in_baseline_resolves_through_git(self):
        with mock.patch.object(check, "run_git", return_value="https://github.com/up/stream.git\n"):
            self.assertEqual(check.upstream_slug("upstream", Path(".")), "up/stream")
        self.assertEqual(check.upstream_slug("https://github.com/a/b", None), "a/b")
        self.assertIsNone(check.upstream_slug("upstream", None))


class ConsoleEncoding(PatchedCase):
    def assert_survives(self, encoding):
        stream = io.TextIOWrapper(io.BytesIO(), encoding=encoding)
        check.emit("| #1 | 🚀 修復 ✓ |", stream)
        stream.flush()
        self.assertIn("#1", stream.buffer.getvalue().decode(encoding))

    def test_emit_survives_cp950(self):
        """cp950 has no emoji: a crash after the report is written exits 1 = 'review needed'."""
        self.assert_survives("cp950")

    def test_emit_survives_cp1252(self):
        """cp1252 has no CJK."""
        self.assert_survives("cp1252")

    def test_emit_survives_ascii(self):
        self.assert_survives("ascii")

    def test_emit_keeps_encodable_text_exact(self):
        stream = io.TextIOWrapper(io.BytesIO(), encoding="utf-8")
        check.emit("修復 🚀", stream)
        stream.flush()
        self.assertEqual(stream.buffer.getvalue().decode("utf-8").strip(), "修復 🚀")

    def test_report_file_stays_exact_utf8(self):
        self.set_gh(fake_gh(prs=[7], titles={7: "🚀 emoji 與 中文"}))
        scan = check.collect_new_tickets({"repo": "r"}, "pr")
        findings = check.Findings(
            baseline_for("a" * 40), [], prs=scan, issues=check.TicketScan([], True, 1)
        )
        self.assertIn("🚀 emoji 與 中文", check.build_report(findings))

    def test_table_cells_cannot_break_the_table(self):
        self.assertEqual(check._cell("a|b\nc\r\nd"), "a\\|b c d")


class EndToEnd(PatchedCase):
    """Real git repositories; ``gh`` answers from fake corpora."""

    def setUp(self):
        super().setUp()
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        root = Path(tmp.name)
        self.upstream = root / "upstream"
        self.upstream.mkdir()
        git(self.upstream, "init", "-q", "-b", "main")
        self.first = commit(self.upstream, "a.txt", "first")
        git(root, "clone", "-q", str(self.upstream), "fork")
        self.fork = root / "fork"
        self.report = root / "report.md"
        self.baseline_file = root / "baseline.json"

    def write(self, sha=None, **extra):
        data = baseline_for(sha or self.first, **extra)
        data["repo"] = str(self.upstream)
        self.baseline_file.write_text(json.dumps(data), encoding="utf-8")

    def run_check(self, *flags):
        return check.main(
            [
                "--repo-dir",
                str(self.fork),
                "--baseline",
                str(self.baseline_file),
                "--output",
                str(self.report),
                *flags,
            ]
        )

    def report_text(self):
        return self.report.read_text(encoding="utf-8")

    # -- exit codes: 0 clean, 1 strict+pending, 2 could not complete

    def test_exit_0_when_nothing_is_pending(self):
        self.write()
        self.assertEqual(self.run_check("--strict"), 0)

    def test_exit_0_without_strict_even_when_commits_are_pending(self):
        self.write()
        commit(self.upstream, "b.txt", "second")
        self.assertEqual(self.run_check(), 0)
        self.assertIn("1 upstream commit(s) require review", self.report_text())

    def test_exit_1_only_for_strict_with_pending_commits(self):
        self.write()
        commit(self.upstream, "b.txt", "second")
        self.assertEqual(self.run_check("--strict"), 1)

    def test_exit_2_when_baseline_is_missing(self):
        self.assertEqual(self.run_check("--strict"), 2)

    def test_exit_2_when_baseline_is_not_json(self):
        self.baseline_file.write_text("{nope", encoding="utf-8")
        self.assertEqual(self.run_check(), 2)

    def test_exit_2_when_gh_cannot_answer_even_if_nothing_else_is_pending(self):
        self.write()
        self.set_gh(lambda args: None)
        self.assertEqual(self.run_check(), 2)
        self.assertEqual(self.run_check("--strict"), 2)

    def test_exit_2_when_upstream_is_unreachable(self):
        self.write()
        data = json.loads(self.baseline_file.read_text(encoding="utf-8"))
        data["repo"] = str(self.upstream) + "-does-not-exist"
        self.baseline_file.write_text(json.dumps(data), encoding="utf-8")
        self.assertEqual(self.run_check("--strict"), 2)
        self.assertIn("Check failed", self.report_text())

    def test_exit_2_when_an_unexpected_exception_escapes(self):
        """An uncaught traceback exits 1 -- indistinguishable from 'review needed'."""
        self.write()
        with mock.patch.object(check, "gather", side_effect=ValueError("surprise")):
            self.assertEqual(self.run_check("--strict"), 2)

    def test_exit_2_when_upstream_rewrote_its_history(self):
        self.write()
        git(self.upstream, "commit", "-q", "--amend", "-m", "rewritten")
        self.assertEqual(self.run_check("--strict"), 2)
        self.assertIn("rewrote its history", self.report_text())

    def test_malformed_baselines_are_clean_check_errors(self):
        """A bad baseline is exit 2 with a message, never a traceback or a silent pass."""
        sha = "a" * 40
        cases = [
            "null",
            "[]",
            json.dumps(baseline_for(None)),
            json.dumps(baseline_for("z" * 40)),  # 40 characters, not hex
            json.dumps(baseline_for(sha, reviewed_pr_through="abc")),
            json.dumps(baseline_for(sha, reviewed_issue_through=-1)),
            json.dumps(baseline_for(sha, reviewed_pr_through=True)),
            json.dumps(baseline_for(sha, reviewed_pr_through=[1])),
        ]
        for content in cases:
            self.baseline_file.write_text(content, encoding="utf-8")
            self.assertEqual(self.run_check(), 2, content)
            self.assertIn("Check failed", self.report_text(), content)

    def test_numeric_string_watermarks_still_load(self):
        self.write(reviewed_pr_through="12")
        self.assertEqual(self.run_check(), 0)

    def test_garbled_git_log_output_is_a_check_error_not_a_crash(self):
        self.write()
        commit(self.upstream, "b.txt", "second")
        real = check.run_git

        def garbled(args, repo_dir):
            return "no separators here\n" if args[:1] == ["log"] else real(args, repo_dir)

        with mock.patch.object(check, "run_git", garbled):
            self.assertEqual(self.run_check("--strict"), 2)
        self.assertIn("unexpected `git log` line", self.report_text())

    def test_checking_upstream_does_not_import_its_tags_into_the_fork(self):
        """A local run must not pollute the maintainer's clone with the upstream's tags."""
        git(self.upstream, "tag", "v9.9.9", self.first)
        self.write()
        self.assertEqual(self.run_check("--strict"), 0)
        self.assertEqual(git(self.fork, "tag", "--list", "v9.9.9"), "")

    def test_unknown_track_is_rejected(self):
        self.write(track="weekly")
        self.assertEqual(self.run_check(), 2)

    def test_pending_tickets_make_strict_exit_1(self):
        self.write(reviewed_pr_through=5)
        self.set_gh(fake_gh(prs=[4, 5, 6]))
        self.assertEqual(self.run_check("--strict"), 1)
        self.assertIn("| #6 |", self.report_text())

    def test_console_that_cannot_encode_the_report_still_exits_by_contract(self):
        self.write(reviewed_pr_through=0)
        self.set_gh(fake_gh(prs=[1], titles={1: "🚀 修復"}))
        console = io.TextIOWrapper(io.BytesIO(), encoding="cp950")
        with mock.patch.object(sys, "stdout", console):
            self.assertEqual(self.run_check("--strict"), 1)  # pending, not UnicodeEncodeError

    # -- tracking modes

    def test_release_track_reports_only_tagged_work(self):
        git(self.upstream, "tag", "v1.0.0", self.first)
        self.write(track="release")
        commit(self.upstream, "b.txt", "untagged work")
        self.assertEqual(self.run_check("--strict"), 0)
        tagged = commit(self.upstream, "c.txt", "tagged work")
        git(self.upstream, "tag", "v1.2.3", tagged)
        self.assertEqual(self.run_check("--strict"), 1)
        self.assertIn("v1.2.3", self.report_text())

    def test_release_track_with_no_tags_is_a_failure_not_a_pass(self):
        self.write(track="release")
        self.assertEqual(self.run_check("--strict"), 2)

    def test_unrelated_history_does_not_invent_an_ahead_count(self):
        """After a squash the fork shares no ancestor with upstream; 'ahead' means nothing."""
        self.write()
        git(self.fork, "checkout", "-q", "--orphan", "squashed")
        commit(self.fork, "z.txt", "squashed root")
        self.assertEqual(self.run_check("--strict"), 0)
        self.assertIn("無共同祖先", self.report_text())

    # -- optional axes

    def test_unregistered_upstream_branch_is_reported(self):
        git(self.upstream, "branch", "release-1")
        self.write(branches=["main"], check_branches=True)
        self.assertEqual(self.run_check("--strict"), 1)
        self.assertIn("release-1", self.report_text())

    def test_registered_branch_head_that_moved_is_reported(self):
        git(self.upstream, "branch", "side")
        side = git(self.upstream, "rev-parse", "side")
        self.write(reviewed_branch_heads={"side": side})
        self.assertEqual(self.run_check("--strict"), 0)
        git(self.upstream, "checkout", "-q", "side")
        commit(self.upstream, "s.txt", "moved")
        self.assertEqual(self.run_check("--strict"), 1)

    def test_branches_list_alone_does_not_switch_the_branch_axis_on(self):
        """AIHOT and jev-chat-jarvis keep `branches` as a note; it must not start failing them."""
        git(self.upstream, "branch", "release-1")
        self.write(branches=["main"])
        self.assertEqual(self.run_check("--strict"), 0)
        self.assertNotIn("Upstream branches", self.report_text())

    def _view(self, head, state="OPEN"):
        def view(args):
            if args[:2] == ["pr", "view"]:
                body = {"state": state, "isDraft": False, "headRefOid": head}
                return completed(json.dumps(body))
            return fake_gh()(args)

        return view

    def test_monitored_pr_that_moved_is_reported(self):
        self.write(monitored=[{"item": "pr#9", "head": "a" * 40, "state": "open"}])
        self.set_gh(self._view("b" * 40))
        self.assertEqual(self.run_check("--strict"), 1)
        self.assertIn("pr#9", self.report_text())

    def test_monitored_pr_that_closed_is_reported(self):
        self.write(monitored=[{"item": "pr#9", "head": "a" * 40, "state": "open"}])
        self.set_gh(self._view("a" * 40, state="MERGED"))
        self.assertEqual(self.run_check("--strict"), 1)

    def test_monitored_pr_unchanged_is_quiet_and_unreadable_is_a_failure(self):
        self.write(monitored=[{"item": "pr#9", "head": "a" * 7, "state": "open"}])
        self.set_gh(self._view("a" * 40))
        self.assertEqual(self.run_check("--strict"), 0)
        self.set_gh(lambda args: None)
        self.assertEqual(self.run_check("--strict"), 2)

    def test_monitored_pr_that_vanished_is_reported_not_ignored(self):
        self.write(monitored=[{"item": "pr#9", "head": "a" * 40, "state": "open"}])

        def gone(args):
            if args[:2] == ["pr", "view"]:
                return completed("", 1, "Could not resolve to a PullRequest")
            return fake_gh()(args)

        self.set_gh(gone)
        self.assertEqual(self.run_check("--strict"), 1)
        self.assertIn("not found", self.report_text())

    def test_monitored_pin_too_short_to_compare_counts_as_drift(self):
        self.write(monitored=[{"item": "pr#9", "head": "abc", "state": "open"}])
        self.set_gh(self._view("abc" + "d" * 37))
        self.assertEqual(self.run_check("--strict"), 1)

    def test_empty_branch_listing_is_a_failure_not_no_branches(self):
        self.write(check_branches=True, branches=[])
        real = check.run_git

        def fake(args, repo_dir):
            return "" if args[:2] == ["ls-remote", "--heads"] else real(args, repo_dir)

        with mock.patch.object(check, "run_git", fake):
            self.assertEqual(self.run_check("--strict"), 2)

    def test_report_file_written_by_main_is_utf8_on_disk(self):
        self.write(reviewed_pr_through=0)
        self.set_gh(fake_gh(prs=[1], titles={1: "🚀 修復"}))
        self.run_check()
        self.assertIn("🚀 修復", self.report.read_bytes().decode("utf-8"))

    def test_failed_monitored_lookup_exits_2_even_when_listings_succeed(self):
        """gh answering the ticket lists but failing `pr view` must not read as 'nothing moved'."""
        self.write(monitored=[{"item": "pr#9", "head": "a" * 40, "state": "open"}])

        def flaky(args):
            if args[:2] == ["pr", "view"]:
                return completed("", 1, "HTTP 502")
            return fake_gh()(args)

        self.set_gh(flaky)
        self.assertEqual(self.run_check("--strict"), 2)

    def test_draft_pr_is_a_different_state_from_open(self):
        self.write(monitored=[{"item": "pr#9", "head": "a" * 40, "state": "open"}])

        def draft(args):
            if args[:2] == ["pr", "view"]:
                body = {"state": "OPEN", "isDraft": True, "headRefOid": "a" * 40}
                return completed(json.dumps(body))
            return fake_gh()(args)

        self.set_gh(draft)
        self.assertEqual(self.run_check("--strict"), 1)

    def test_decision_log_from_baseline_is_what_the_report_names(self):
        self.write(decision_log="docs/fork/DECISIONS.md", reviewed_pr_through=0)
        self.set_gh(fake_gh(prs=[1]))
        self.run_check()
        text = self.report_text()
        self.assertIn("docs/fork/DECISIONS.md", text)
        self.assertNotIn("`docs/DECISIONS.md`", text)

    def test_baseline_with_a_short_sha_is_rejected(self):
        self.write(sha="abc1234")
        self.assertEqual(self.run_check(), 2)

    def test_malformed_monitored_entry_is_rejected(self):
        self.write(monitored=[{"item": "issue#1"}])
        self.assertEqual(self.run_check(), 2)


class _Response:
    def __init__(self, body: bytes, status: int = 200):
        self.status, self._body = status, body

    def read(self):
        return self._body

    def __enter__(self):
        return self

    def __exit__(self, *exc):
        return False


class Website(unittest.TestCase):
    base = baseline_for("a" * 40, website="https://example.invalid", reviewed_website_version="1")

    def test_version_change_is_pending(self):
        reply = _Response(b'{"version": "2"}')
        with mock.patch.object(check.urllib.request, "urlopen", return_value=reply):
            info = check.check_upstream_website(self.base)
        self.assertTrue(check.pending(check.Findings(self.base, [], website=info)))

    def test_unchanged_version_is_not_pending(self):
        reply = _Response(b'{"version": "1"}')
        with mock.patch.object(check.urllib.request, "urlopen", return_value=reply):
            info = check.check_upstream_website(self.base)
        self.assertFalse(check.pending(check.Findings(self.base, [], website=info)))

    def test_unreachable_site_is_reported_not_checked(self):
        with mock.patch.object(check.urllib.request, "urlopen", side_effect=OSError("down")):
            info = check.check_upstream_website(self.base)
        self.assertIn("error", info)
        self.assertIn("Not checked", "\n".join(check.render_website_section(info)))

    def test_unreachable_site_exits_2_not_0(self):
        findings = check.Findings(
            self.base,
            [],
            prs=check.TicketScan([], True, 1),
            issues=check.TicketScan([], True, 1),
            website={"url": "u", "error": "down"},
        )
        self.assertEqual(check.exit_code(findings, strict=True)[0], 2)

    def test_no_website_in_baseline_means_no_axis(self):
        self.assertIsNone(check.check_upstream_website({}))


if __name__ == "__main__":
    unittest.main()
