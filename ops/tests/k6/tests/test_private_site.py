"""Known synthetic sites only. Fake POSIX metadata is not host/WSL admission."""
import contextlib
from datetime import datetime, timedelta, timezone
import io
import json
import os
from pathlib import Path, PurePosixPath
import stat
import sys
from types import SimpleNamespace
import unittest
from unittest import mock

from test_boundaries import scratch
from test_direct_chromium import fake_child_reads, host
from test_direct_followup import refusal
from test_runner import runner
import safety


channel = getattr(runner, "site_channel", None)
RUN = "b" * 32
SOURCE = "base/synthetic_private_site.cc"
FATAL = ("[123:456:1030/121314.123456:FATAL:" + SOURCE + ":713] synthetic message\n").encode()


def nominal():
    value = refusal()
    for key in ("comparison", "tools", "finished_unix_ms"):
        del value[key]
    value.update(confidentiality="NOT_QUALIFIED", k6_exit="SCRIPT_ABORTED", started_unix_ms=1)
    return value


def deadline():
    return (datetime.now(timezone.utc) + timedelta(minutes=30)).strftime("%Y-%m-%dT%H:%M:%SZ")


class FakePosix:
    """Only filesystem syscall boundaries; production keeps its real dir_fd code."""
    def __init__(self, uid=789):
        self.uid, self.dev, self.serial = uid, 7, 10
        self.nodes, self.fds, self.calls = {}, {}, []
        self.kind, self.write_fault, self.unlink_fault = "ext4", None, False
        self.home = PurePosixPath("/home/operator")
        self.original = {key: getattr(os, key) for key in ("open", "mkdir", "stat", "fstat", "write", "close", "unlink")}
        self.add("/", stat.S_IFDIR | 0o755, 0, 1)
        self.add("/home", stat.S_IFDIR | 0o755, 0, 1)
        self.add(str(self.home), stat.S_IFDIR | 0o700, uid, self.dev)

    def add(self, path, mode, uid=None, dev=None):
        self.serial += 1
        self.nodes[path] = SimpleNamespace(st_mode=mode, st_uid=self.uid if uid is None else uid,
            st_dev=self.dev if dev is None else dev, st_ino=self.serial, st_nlink=2 if stat.S_ISDIR(mode) else 1,
            st_size=0, data=bytearray())
        return self.nodes[path]

    def path(self, name, dir_fd):
        return str(PurePosixPath(self.fds[dir_fd]) / str(name)) if dir_fd is not None else str(name)

    def fake_path(self, path):
        return path == "/" or path == "/home" or path.startswith("/home/operator")

    def open(self, name, flags, mode=0o777, *, dir_fd=None):
        path = self.path(name, dir_fd)
        if not self.fake_path(path):
            return self.original["open"](name, flags, mode, dir_fd=dir_fd)
        self.calls.append(("open", path, flags))
        if path in self.nodes and flags & os.O_EXCL:
            raise FileExistsError()
        if path not in self.nodes:
            if not flags & os.O_CREAT:
                raise FileNotFoundError()
            self.add(path, stat.S_IFREG | mode)
        node = self.nodes[path]
        if stat.S_ISLNK(node.st_mode) and flags & os.O_NOFOLLOW:
            raise OSError()
        if flags & os.O_DIRECTORY and not stat.S_ISDIR(node.st_mode):
            raise NotADirectoryError()
        fd = 1000 + len(self.fds)
        while fd in self.fds:
            fd += 1
        self.fds[fd] = path
        return fd

    def mkdir(self, name, mode=0o777, *, dir_fd=None):
        path = self.path(name, dir_fd)
        if not self.fake_path(path):
            return self.original["mkdir"](name, mode, dir_fd=dir_fd)
        self.calls.append(("mkdir", path, mode))
        if path in self.nodes:
            raise FileExistsError()
        self.add(path, stat.S_IFDIR | mode)

    def stat(self, name, *, dir_fd=None, follow_symlinks=True):
        path = self.path(name, dir_fd)
        if not self.fake_path(path):
            return self.original["stat"](name, dir_fd=dir_fd, follow_symlinks=follow_symlinks)
        if path not in self.nodes:
            raise FileNotFoundError()
        return self.nodes[path]

    def fstat(self, fd):
        return self.nodes[self.fds[fd]] if fd in self.fds else self.original["fstat"](fd)

    def close(self, fd):
        if fd in self.fds:
            del self.fds[fd]
        else:
            self.original["close"](fd)

    def write(self, fd, raw):
        node = self.nodes[self.fds[fd]]
        if self.write_fault:
            if node.data:
                raise self.write_fault
            raw = raw[:9]
        node.data.extend(raw)
        node.st_size = len(node.data)
        return len(raw)

    def unlink(self, name, *, dir_fd=None):
        path = self.path(name, dir_fd)
        self.calls.append(("unlink", path))
        if self.unlink_fault:
            raise PermissionError()
        del self.nodes[path]

    def parent_check(self, evidence):
        home = self.nodes[str(self.home)]
        return (self.kind == "ext4" and evidence["home_identity"] == (home.st_dev, home.st_ino, home.st_uid)
                and evidence["browser_run_id"] == nominal()["run_id"])

    @contextlib.contextmanager
    def enabled(self):
        with contextlib.ExitStack() as stack:
            for key, value in {"O_NOFOLLOW": 1 << 17, "O_DIRECTORY": 1 << 18, "O_CLOEXEC": 1 << 19}.items():
                stack.enter_context(mock.patch.object(os, key, value, create=True))
            for name in self.original:
                stack.enter_context(mock.patch.object(os, name, getattr(self, name)))
            stack.enter_context(mock.patch.object(os, "fchmod", lambda fd, mode: setattr(
                self.nodes[self.fds[fd]], "st_mode", stat.S_IFREG | mode), create=True))
            stack.enter_context(mock.patch.object(os, "fsync", lambda fd: None, create=True))
            stack.enter_context(mock.patch.object(os, "getuid", return_value=self.uid, create=True))
            stack.enter_context(mock.patch.object(os, "geteuid", return_value=self.uid, create=True))
            stack.enter_context(mock.patch.object(os, "supports_dir_fd", {os.open, os.mkdir, os.stat, os.unlink}))
            stack.enter_context(mock.patch.object(os, "supports_follow_symlinks", {os.stat}))
            stack.enter_context(mock.patch.object(sys, "platform", "linux"))
            stack.enter_context(mock.patch.object(Path, "home", return_value=self.home))
            stack.enter_context(mock.patch.object(channel, "private_path", side_effect=lambda value, **kw: value))
            stack.enter_context(mock.patch.dict(os.environ, {}, clear=True))
            yield self

    def site(self, run=RUN):
        return str(self.home / ".cache/opencode/k6-causal" / run / "site.json")


class PrivateSiteTests(unittest.TestCase):
    def setUp(self):
        self.assertIsNotNone(channel, "private consumer not implemented")

    def session(self, fs, **kwargs):
        return channel.PrivateSiteSession(nominal(), nominal()["run_id"], parent_check=kwargs.get("parent_check", fs.parent_check),
                                          delete_by_utc=kwargs.get("delete_by_utc", deadline()))

    def test_historical_nominal_refusal_is_not_a_fabricated22_receipt(self):
        fs = FakePosix()
        self.session(fs)
        for value in (refusal(), {}, {**nominal(), "cleanup": "FAIL"}, {**nominal(), "k6_exit": "ZERO"},
                      {**nominal(), "confidentiality": "PASS"}):
            with self.assertRaises(safety.Rejected):
                channel.PrivateSiteSession(value, nominal()["run_id"], parent_check=fs.parent_check, delete_by_utc=deadline())

    def test_source_and_document_grammar_never_normalize(self):
        good = {"version": 1, "run_id": RUN, "browser_version": "156.0.8078.73", "source_relative": SOURCE,
                "source_line": 713, "delete_by_utc": deadline()}
        self.assertEqual(channel.site_document(good), good)
        self.assertEqual(channel.site_document({**good, "source_relative": "x" * 253 + ".cc", "source_line": 1000000})["source_line"], 1000000)
        paths = ("/x.cc", "../x.cc", "a/./x.cc", "a//x.cc", "C:/x.cc", "x\\a.cc", "a%2fb.cc", "a+b.cc",
                 "a.hpp", "é.cc", "x" * 257 + ".cc", "", "a/../x.h")
        bad = [{**good, "source_relative": path} for path in paths]
        bad += [{**good, "source_line": value} for value in (True, 0, 1000001, "713", 713.0)]
        bad += [{**good, "extra": "private"}, {**good, "version": True}, {**good, "browser_version": "other"}]
        for value in bad:
            with self.assertRaises(safety.Rejected):
                channel.site_document(value)

    def test_exclusive_file_modes_read_confirmation_and_no_source_open(self):
        fs = FakePosix()
        with fs.enabled(), self.session(fs) as session:
            session.authorize(RUN)
            self.assertTrue(session.record(SOURCE, 713))
            node = fs.nodes[fs.site()]
            self.assertEqual(stat.S_IMODE(node.st_mode), 0o400)
            self.assertLessEqual(node.st_size, 1024)
            self.assertEqual(json.loads(node.data)["source_relative"], SOURCE)
            self.assertFalse(session.retire())
            self.assertTrue(session.retire(read_confirmed=True))
            self.assertNotIn(fs.site(), fs.nodes)
            self.assertFalse(any(call[0] == "open" and call[1] == SOURCE for call in fs.calls))
        self.assertEqual(session.status, "REMOVED")
        self.assertEqual(fs.fds, {})

    def test_parent_modes_links_uid_filesystem_and_collision_refuse_without_adoption(self):
        for change in ("parent", "mode", "link", "uid", "fs", "mount", "collision", "primitives"):
            fs = FakePosix()
            with fs.enabled():
                if change in {"mode", "link", "uid", "mount"}:
                    node = fs.add(str(fs.home / ".cache"), stat.S_IFDIR | 0o700)
                    if change == "mode": node.st_mode = stat.S_IFDIR | 0o755
                    if change == "link": node.st_mode = stat.S_IFLNK | 0o700
                    if change == "uid": node.st_uid += 1
                    if change == "mount": node.st_dev += 1
                if change == "fs": fs.kind = "drvfs"
                if change == "collision":
                    for part in (".cache", ".cache/opencode", ".cache/opencode/k6-causal", ".cache/opencode/k6-causal/" + RUN):
                        fs.add(str(fs.home / part), stat.S_IFDIR | 0o700)
                    fs.add(fs.site(), stat.S_IFREG | 0o400).data.extend(b"foreign")
                with contextlib.ExitStack() as stack:
                    if change == "primitives": stack.enter_context(mock.patch.object(os, "supports_dir_fd", set()))
                    with self.assertRaises(safety.Rejected), self.session(fs, parent_check=(lambda e: False) if change == "parent" else fs.parent_check) as session:
                        session.authorize(RUN)
                if change == "collision": self.assertEqual(fs.nodes[fs.site()].data, b"foreign")
                else: self.assertNotIn(fs.site(), fs.nodes)
                self.assertFalse(any(call[0] == "unlink" for call in fs.calls))
            self.assertEqual(fs.fds, {})

    def test_owned_partial_write_is_removed_on_error_or_interruption(self):
        for error in (OSError("CONFIDENTIAL_SENTINEL"), KeyboardInterrupt()):
            fs = FakePosix()
            with fs.enabled(), self.session(fs) as session:
                session.authorize(RUN)
                fs.write_fault = error
                self.assertFalse(session.record(SOURCE, 713))
                self.assertNotIn(fs.site(), fs.nodes)
                self.assertIn(session.status, {"WRITE_FAILED", "INTERRUPTED"})
            self.assertEqual(fs.fds, {})

    def test_expiry_removes_without_read_and_no_timer_is_claimed(self):
        fs = FakePosix()
        with fs.enabled(), self.session(fs) as session:
            session.authorize(RUN)
            self.assertTrue(session.record(SOURCE, 713))
            later = datetime.now(timezone.utc) + timedelta(hours=1)
            with mock.patch.object(channel, "utc_now", return_value=later):
                self.assertTrue(session.retire())
            self.assertNotIn(fs.site(), fs.nodes)

    def test_context_abort_removes_ready_file_without_claiming_it_still_ready(self):
        fs = FakePosix()
        with fs.enabled():
            with self.session(fs) as session:
                session.authorize(RUN)
                self.assertTrue(session.record(SOURCE, 713))
            self.assertNotIn(fs.site(), fs.nodes)
            self.assertEqual(session.status, "REMOVED")
            self.assertEqual(fs.fds, {})

    def test_retention_never_extends_on_clock_rollback(self):
        fs = FakePosix()
        with fs.enabled(), self.session(fs) as session:
            session.authorize(RUN)
            self.assertTrue(session.record(SOURCE, 713))
            with mock.patch.object(channel, "utc_now", return_value=datetime.now(timezone.utc) - timedelta(hours=2)), \
                    mock.patch.object(channel.time, "monotonic", return_value=session._expires_monotonic + 1):
                self.assertTrue(session.retire())
            self.assertNotIn(fs.site(), fs.nodes)

    def test_deadline_crossed_during_write_never_declares_ready(self):
        fs = FakePosix()
        clock = [channel.time.monotonic()]
        with fs.enabled(), mock.patch.object(channel.time, "monotonic", side_effect=lambda: clock[0]), self.session(fs) as session:
            session.authorize(RUN)
            def slow_flush(_fd):
                clock[0] += 3601
            with mock.patch.object(os, "fsync", side_effect=slow_flush):
                self.assertFalse(session.record(SOURCE, 713))
            self.assertNotIn(fs.site(), fs.nodes)
            self.assertNotEqual(session.status, "READY")

    def test_expired_slot_or_missing_callback_cannot_admit(self):
        fs = FakePosix()
        for when, callback in (((datetime.now(timezone.utc) - timedelta(seconds=1)).strftime("%Y-%m-%dT%H:%M:%SZ"), fs.parent_check),
                               ((datetime.now(timezone.utc) + timedelta(hours=2)).strftime("%Y-%m-%dT%H:%M:%SZ"), fs.parent_check),
                               (deadline(), None)):
            with self.assertRaises(safety.Rejected):
                self.session(fs, parent_check=callback, delete_by_utc=when)

    def test_sealing_error_removes_only_the_owned_partial(self):
        fs = FakePosix()
        with fs.enabled(), self.session(fs) as session:
            session.authorize(RUN)
            with mock.patch.object(os, "fchmod", side_effect=OSError("CONFIDENTIAL_SENTINEL")):
                self.assertFalse(session.record(SOURCE, 713))
            self.assertNotIn(fs.site(), fs.nodes)

    def test_file_inserted_after_authorize_is_neither_adopted_nor_deleted(self):
        fs = FakePosix()
        with fs.enabled(), self.session(fs) as session:
            session.authorize(RUN)
            node = fs.add(fs.site(), stat.S_IFREG | 0o400)
            node.data.extend(b"foreign")
            self.assertFalse(session.record(SOURCE, 713))
            self.assertEqual(fs.nodes[fs.site()].data, b"foreign")
            self.assertFalse(any(call[0] == "unlink" for call in fs.calls))

    def test_legacy_git_or_unsafe_home_guard_is_still_required(self):
        fs = FakePosix()
        with fs.enabled(), self.session(fs) as session, mock.patch.object(channel, "private_path", side_effect=safety.Rejected("PATH_INVALID")):
            with self.assertRaises(safety.Rejected):
                session.authorize(RUN)
            self.assertEqual(fs.calls, [])

    def test_nested_git_guard_is_checked_at_leaf_and_after_writing(self):
        fs = FakePosix()
        with fs.enabled(), self.session(fs) as session:
            def reject_leaf(value, **kwargs):
                if value != fs.home:
                    raise safety.Rejected("PATH_INVALID")
                return value
            with mock.patch.object(channel, "private_path", side_effect=reject_leaf), self.assertRaises(safety.Rejected):
                session.authorize(RUN)
            self.assertNotIn(fs.site(), fs.nodes)
        fs = FakePosix()
        with fs.enabled(), self.session(fs) as session:
            session.authorize(RUN)
            calls = [0]
            def git_appears_after_write(value, **kwargs):
                calls[0] += 1
                if calls[0] == 2:
                    raise safety.Rejected("PATH_INVALID")
                return value
            with mock.patch.object(channel, "private_path", side_effect=git_appears_after_write):
                self.assertFalse(session.record(SOURCE, 713))
            self.assertNotIn(fs.site(), fs.nodes)

    def test_changed_parent_admission_at_write_cannot_create_a_file(self):
        fs = FakePosix()
        with fs.enabled(), self.session(fs) as session:
            session.authorize(RUN)
            fs.kind = "drvfs"
            self.assertFalse(session.record(SOURCE, 713))
            self.assertNotIn(fs.site(), fs.nodes)

    def test_unremovable_or_replaced_file_is_not_a_cleanup_pass(self):
        for replace in (False, True):
            fs = FakePosix()
            with fs.enabled(), self.assertRaises(safety.Rejected):
                with self.session(fs) as session:
                    session.authorize(RUN)
                    self.assertTrue(session.record(SOURCE, 713))
                    if replace: fs.add(fs.site(), stat.S_IFLNK | 0o777, uid=999)
                    else: fs.unlink_fault = True
                    self.assertFalse(session.retire(read_confirmed=True))
                    self.assertEqual(session.status, "RETENTION_UNPROVEN")
                    self.assertIn(fs.site(), fs.nodes)
            self.assertEqual(fs.fds, {})

    def test_ci_and_wrong_modes_refuse_before_environment_scrub_or_child(self):
        for marker in ("CI", "GITHUB_ACTIONS", "GITHUB_RUN_ID", "RUNNER_NAME"):
            fs = FakePosix()
            with fs.enabled(), self.session(fs) as session, mock.patch.dict(os.environ, {marker: ""}), \
                    mock.patch.object(session, "authorize") as authorize, mock.patch.object(runner, "child_environment") as env, \
                    mock.patch.object(runner, "Child") as child, contextlib.redirect_stdout(io.StringIO()) as log:
                self.assertNotEqual(runner.main(["diagnose-chromium", "--private-site", "--output", "/private/new"], private_site=session), 0)
            authorize.assert_not_called()
            env.assert_not_called()
            child.assert_not_called()
            self.assertNotIn("/private/new", log.getvalue())
        for argv in (["browser", "--private-site", "--output", "/private/new"],
                     ["diagnose-chromium", "--private-site", "--compare-ubuntu-2204", "--browser-result", "/private/refusal", "--output", "/private/new"]):
            with mock.patch.object(runner, "Child") as child, contextlib.redirect_stdout(io.StringIO()):
                self.assertNotEqual(runner.main(argv), 0)
            child.assert_not_called()

    def test_flag_alone_or_manifest_is_not_parent_admission(self):
        for admission in (None, {"admitted": True}, True):
            with mock.patch.object(runner, "child_environment") as env, mock.patch.object(runner, "Child") as child, \
                    contextlib.redirect_stdout(io.StringIO()):
                self.assertNotEqual(runner.main(["diagnose-chromium", "--private-site", "--output", "/private/new"], private_site=admission), 0)
            env.assert_not_called()
            child.assert_not_called()

    def test_parent_session_without_cli_flag_is_not_ambient_activation(self):
        fs = FakePosix()
        with fs.enabled(), self.session(fs) as session, mock.patch.object(session, "authorize") as authorize, \
                mock.patch.object(runner, "Child") as child, contextlib.redirect_stdout(io.StringIO()):
            self.assertNotEqual(runner.main(["diagnose-chromium", "--output", "/private/new"], private_site=session), 0)
        authorize.assert_not_called()
        child.assert_not_called()

    def test_private_candidate_requires_complete_recorded_identities_and_both_private_streams(self):
        identity = {"pid": 123, "uid": 789, "start": 456}
        good = dict(cleanup=True, output_overflow=False, stop_path="NATIVE_OBSERVED_BEFORE_CLEANUP", code=-6,
                    owned={123: identity}, recorded={(123, 456, 789)}, stdout=FATAL, stderr=b"")
        with mock.patch.object(os, "getuid", return_value=789, create=True):
            self.assertEqual(runner._private_site_candidate(SimpleNamespace(**good), cleanup=True, diagnostic="SANDBOX_UNPROVEN"), (SOURCE, 713))
            for key, value in (("owned", {}), ("recorded", set()), ("code", True), ("cleanup", False),
                               ("output_overflow", True), ("stop_path", "INTERRUPTED"),
                               ("stderr", b"CONFIDENTIAL_SENTINEL"), ("stderr", b"\xff")):
                self.assertIsNone(runner._private_site_candidate(SimpleNamespace(**{**good, key: value}), cleanup=True,
                                                                diagnostic="SANDBOX_UNPROVEN"))

    def launch(self, payload=FATAL, *, cleanup=True, native_exit=-6):
        fs = FakePosix()
        with scratch() as root:
            (root / "private").mkdir()
            with fs.enabled(), self.session(fs) as session, fake_child_reads(payload, 1, cleanup, native_exit=native_exit) as child, \
                    mock.patch.object(runner, "linux_platform"), mock.patch.object(runner, "new_output", return_value=root), \
                    mock.patch.object(runner, "pinned_tools", return_value=({"k6": Path("/pinned/k6"), "chromium": Path("/pinned/chrome")}, {})), \
                    mock.patch.object(runner, "version_checks", return_value={}), mock.patch.object(runner, "host_metadata", return_value=host()), \
                    mock.patch.object(runner, "verify_tools"), mock.patch.object(runner, "Child", return_value=child), \
                    mock.patch.object(runner, "publish") as publish, contextlib.redirect_stdout(io.StringIO()) as log:
                code = runner.main(["diagnose-chromium", "--private-site", "--output", str(root)], private_site=session)
                result = json.loads(log.getvalue())
                data = [node.data for path, node in fs.nodes.items() if path.endswith("/site.json")]
                self.assertNotEqual(code, 0)
                self.assertEqual(result["HTTP"], "INCONCLUSIVE")
                self.assertEqual(result["Chromium"], "INCONCLUSIVE")
                self.assertEqual(result["parent_runtime_cleanup"], "INCONCLUSIVE")
                self.assertNotIn(SOURCE, log.getvalue())
                self.assertNotIn("source_relative", log.getvalue())
                self.assertNotIn("k6-causal", log.getvalue())
                self.assertNotIn("CONFIDENTIAL_SENTINEL", log.getvalue())
                publish.assert_not_called()
                if data:
                    value = json.loads(data[0])
                    self.assertEqual(value["run_id"], result["run_id"])
                    self.assertEqual(value["source_relative"], SOURCE)
                    self.assertEqual(value["source_line"], 713)
                    self.assertTrue(session.retire(read_confirmed=True))
            self.assertEqual(fs.fds, {})
            return data, result

    def test_real_child_private_positive_native_zero_and_failures_never_publish(self):
        for native in (-6, 0):
            data, result = self.launch(native_exit=native)
            self.assertEqual(len(data), 1)
            self.assertEqual(result["diagnostic"], "SANDBOX_UNPROVEN")
        for raw in (FATAL + FATAL, FATAL + b"other.c:1: Assertion `x' failed.\n", FATAL.rstrip(b"\n"), b"noise\n",
                    FATAL.replace(SOURCE.encode(), b"../bad.cc"), FATAL.replace(b":713]", b":0713]"),
                    FATAL.replace(b":713]", b":1000001]"),
                    FATAL + b"CONFIDENTIAL_SENTINEL", FATAL + b"https://example.invalid/private", FATAL + b"\xff"):
            self.assertEqual(self.launch(raw)[0], [])
        self.assertEqual(self.launch(FATAL, cleanup=False)[0], [])
        self.assertEqual(self.launch(FATAL + b"x" * safety.LIMIT, cleanup=False, native_exit=None)[0], [])
        self.assertEqual(self.launch(FATAL, native_exit=None)[0], [])


@unittest.skipUnless(sys.platform == "linux" and getattr(os, "getuid", lambda: 0)() != 0,
                     "real POSIX dir_fd/mode/symlink primitives require native non-root Linux")
class NativePrimitiveTests(unittest.TestCase):
    def test_known_fixture_file_permissions_and_retirement_only(self):
        # No real host/reader admission or browser: callback and HOME are fixture
        # boundaries. This exercises actual CPython POSIX primitives, not ext4 proof.
        with scratch() as root:
            home = root / "home"
            home.mkdir(mode=0o700)
            with mock.patch.object(Path, "home", return_value=home), mock.patch.dict(os.environ, {}, clear=True):
                session = channel.PrivateSiteSession(nominal(), nominal()["run_id"], parent_check=lambda e: True,
                                                     delete_by_utc=deadline())
                with session:
                    session.authorize(RUN)
                    self.assertTrue(session.record(SOURCE, 713))
                    path = home / ".cache/opencode/k6-causal" / RUN / "site.json"
                    self.assertEqual(stat.S_IMODE(path.stat().st_mode), 0o400)
                    self.assertEqual(json.loads(path.read_bytes())["source_relative"], SOURCE)
                    self.assertTrue(session.retire(read_confirmed=True))
                    self.assertFalse(path.exists())
