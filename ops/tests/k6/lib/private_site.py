"""Local, parent-supervised private site file. No reader, logger or authority manifest.

The trusted parent callback must revalidate its real host/ext4 home identity,
lease, nominal-run provenance/pins and reader appointment. It is not a JSON gate.
This module only binds that checked home to native descriptor-relative metadata.
"""
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import re
import stat
import sys
import time

from safety import BROWSER_DIAGNOSTICS, Rejected, closed_browser_observation, confidential, private_path

BROWSER_VERSION = "156.0.8078.73"
SITE_LIMIT = 1024
HOST_LABELS = {
    "unprivileged_userns_clone": {"ZERO", "ONE"}, "apparmor_restrict_unprivileged_userns": {"ZERO", "ONE"},
    "max_user_namespaces": {"ZERO", "ONE", "POSITIVE"}, "apparmor_enabled": {"ENABLED", "DISABLED"},
    "parent_seccomp": {"ZERO", "STRICT", "FILTER"}, "parent_no_new_privs": {"ZERO", "ONE"}}


def utc_now():
    return datetime.now(timezone.utc)


def deletion_deadline(value):
    if type(value) is not str or not re.fullmatch(r"[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z", value):
        raise Rejected("INPUT_INVALID")
    try:
        date = datetime.strptime(value, "%Y-%m-%dT%H:%M:%SZ").replace(tzinfo=timezone.utc)
    except ValueError:
        raise Rejected("INPUT_INVALID") from None
    if not 0 < (date - utc_now()).total_seconds() <= 3600:
        raise Rejected("INPUT_INVALID")
    return date


def source_pair(relative, line):
    if (type(relative) is not str or not 1 <= len(relative) <= 256
            or not re.fullmatch(r"[A-Za-z0-9_.-]+(?:/[A-Za-z0-9_.-]+)*", relative)
            or any(part in {".", ".."} for part in relative.split("/"))
            or not relative.endswith((".cc", ".h", ".c"))
            or type(line) is not int or not 1 <= line <= 1000000):
        raise Rejected("INPUT_INVALID")
    return relative, line


def site_document(value):
    if (type(value) is not dict or set(value) != {"version", "run_id", "browser_version", "source_relative", "source_line", "delete_by_utc"}
            or type(value["version"]) is not int or value["version"] != 1
            or type(value["run_id"]) is not str or not re.fullmatch(r"[a-f0-9]{32}", value["run_id"])
            or type(value["browser_version"]) is not str or value["browser_version"] != BROWSER_VERSION):
        raise Rejected("INPUT_INVALID")
    source_pair(value["source_relative"], value["source_line"])
    deletion_deadline(value["delete_by_utc"])
    raw = json.dumps(value, ensure_ascii=True, allow_nan=False, separators=(",", ":")).encode("ascii")
    if len(raw) > SITE_LIMIT:
        raise Rejected("OUTPUT_LIMIT")
    confidential(raw)
    return dict(value)


def nominal_refusal(value, expected):
    # The historical24 surface has no finished/versions/hashes or privacy PASS.
    # The parent checks that separate evidence; do not manufacture a22 receipt.
    fixed = {"version": 1, "HTTP": "INCONCLUSIVE", "Chromium": "INCONCLUSIVE", "cleanup": "PASS",
             "confidentiality": "NOT_QUALIFIED", "cleanup_scope": "LAUNCHER_CHILDREN_ONLY",
             "parent_runtime_cleanup": "INCONCLUSIVE", "browser_source": "K6", "diagnostic": "SANDBOX_UNPROVEN",
             "k6_exit": "SCRIPT_ABORTED"}
    fields = set(fixed) | {"run_id", "started_unix_ms", "browser_host", "browser_native_diagnostic", "browser_observation"}
    try:
        if (type(value) is not dict or set(value) != fields or type(value["version"]) is not int
                or any(value[key] != item for key, item in fixed.items())
                or type(expected) is not str or not re.fullmatch(r"[a-f0-9]{32}", expected) or value["run_id"] != expected
                or type(value["started_unix_ms"]) is not int or not 0 < value["started_unix_ms"] <= int(utc_now().timestamp() * 1000)
                or value["browser_native_diagnostic"] not in BROWSER_DIAGNOSTICS - {"K6_ZERO_EXIT_NO_SANDBOX_WITNESS"}):
            return False
        host = value["browser_host"]
        if (type(host) is not dict or set(host) != set(HOST_LABELS) or any(
                type(host[key]) is not str or host[key] not in labels | {"ABSENT", "DENIED", "UNREADABLE"}
                for key, labels in HOST_LABELS.items())):
            return False
        observation = closed_browser_observation(value["browser_observation"])
        return observation == {"native_exit": "NONZERO", "summary": "VALID",
            "counters": {"expected": "ONE", "started": "ZERO", "completed": "ZERO", "iterations": "ONE", "requests": "ZERO"},
            "renderer_observed": False, "namespace_reads": {"user": ["NOT_OBSERVED"], "pid": ["NOT_OBSERVED"]}}
    except (Rejected, KeyError, TypeError, ValueError):
        return False


def ci_present():
    markers = {"CI", "CONTINUOUS_INTEGRATION", "GITLAB_CI", "TF_BUILD", "JENKINS_URL", "BUILDKITE", "CIRCLECI", "TRAVIS", "APPVEYOR", "TEAMCITY_VERSION"}
    return any(key in markers or key.startswith(("GITHUB_", "RUNNER_")) for key in os.environ)


def opt_in(requested, mode, comparison, session):
    if (not requested or mode != "diagnose-chromium" or comparison or ci_present()
            or not isinstance(session, PrivateSiteSession) or not session._entered):
        raise Rejected("INPUT_INVALID")


def native_primitives():
    if (sys.platform != "linux" or sys.implementation.name != "cpython" or ci_present()
            or not callable(getattr(os, "getuid", None)) or not callable(getattr(os, "geteuid", None))
            or os.getuid() == 0 or os.geteuid() != os.getuid()
            or any(type(getattr(os, name, None)) is not int or getattr(os, name) == 0
                   for name in ("O_NOFOLLOW", "O_DIRECTORY", "O_CLOEXEC", "O_CREAT", "O_EXCL"))
            or not {os.open, os.mkdir, os.stat, os.unlink} <= getattr(os, "supports_dir_fd", set())
            or os.stat not in getattr(os, "supports_follow_symlinks", set())
            or any(not callable(getattr(os, name, None)) for name in ("fstat", "fchmod", "fsync", "write", "close"))):
        raise Rejected("PREREQUISITE_UNAVAILABLE", True)


class PrivateSiteSession:
    """One parent call and one file; entered context must outlive the read slot.

    parent_check is trusted parent code, never loaded from a file/environment. It
    checks real ext4/GUID/account/lease/provenance/appointment against the supplied
    opened-home identity. Descriptor device binding prevents descendant FS escape.
    No callback, manifest or CLI alone proves those external facts.
    """
    def __init__(self, refusal, expected_browser_run_id, *, parent_check, delete_by_utc):
        if not callable(parent_check) or not nominal_refusal(refusal, expected_browser_run_id):
            raise Rejected("INPUT_INVALID")
        self.deadline = deletion_deadline(delete_by_utc)
        # Clock rollback must not silently extend the agreed maximum retention.
        self._expires_monotonic = time.monotonic() + max(0, min(3600, (self.deadline - utc_now()).total_seconds()))
        self.delete_by_utc, self.browser_run_id = delete_by_utc, expected_browser_run_id
        self._parent_check = parent_check
        self._entered, self._active = False, False
        self._fds, self._bindings = [], []
        self._home_identity, self._file_identity = None, None
        self._sealed, self._attempted = False, False
        self.run_id, self.status = None, "NOT_STARTED"

    def __repr__(self):
        return "<PrivateSiteSession>"

    def __enter__(self):
        if self._entered or self.run_id is not None:
            raise Rejected("INPUT_INVALID")
        self._entered = True
        return self

    @staticmethod
    def _identity(item):
        return item.st_dev, item.st_ino, item.st_uid

    def _directory(self, item, private):
        mode = stat.S_IMODE(item.st_mode)
        if (not stat.S_ISDIR(item.st_mode) or item.st_uid not in {0, self.uid}
                or (mode & 0o022 and not item.st_mode & stat.S_ISVTX)
                or (private and (item.st_uid != self.uid or mode != 0o700
                                 or (self._home_identity is not None and item.st_dev != self._home_identity[0])))):
            raise Rejected("PATH_INVALID")

    def _open_directory(self, parent, name, *, private=False, create=False, exclusive=False):
        if create:
            try:
                os.mkdir(name, 0o700, dir_fd=parent)
            except FileExistsError:
                if exclusive:
                    raise Rejected("PATH_INVALID") from None
        fd = os.open(name, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC,
                     **({"dir_fd": parent} if parent is not None else {}))
        self._fds.append(fd)
        item = os.fstat(fd)
        self._directory(item, private)
        named = os.stat(name, follow_symlinks=False, **({"dir_fd": parent} if parent is not None else {}))
        if self._identity(named) != self._identity(item) or named.st_mode != item.st_mode:
            raise Rejected("PATH_INVALID")
        self._bindings.append((parent, name, fd, self._identity(item), item.st_mode))
        return fd

    def _verify(self):
        if os.getuid() != self.uid or os.geteuid() != self.uid:
            raise Rejected("PATH_INVALID")
        for parent, name, fd, identity, mode in self._bindings:
            item = os.stat(name, follow_symlinks=False, **({"dir_fd": parent} if parent is not None else {}))
            opened = os.fstat(fd)
            if self._identity(item) != identity or self._identity(opened) != identity or item.st_mode != mode or opened.st_mode != mode:
                raise Rejected("PATH_INVALID")

    def _parent_ready(self, phase):
        if self._expired() or self._parent_check({"phase": phase, "browser_run_id": self.browser_run_id,
                "run_id": self.run_id, "home_identity": self._home_identity, "delete_by_utc": self.delete_by_utc}) is not True:
            raise Rejected("INPUT_INVALID")

    def _expired(self):
        return utc_now() >= self.deadline or time.monotonic() >= self._expires_monotonic

    def authorize(self, run_id):
        try:
            native_primitives()
            if not self._entered or self.run_id is not None or type(run_id) is not str or not re.fullmatch(r"[a-f0-9]{32}", run_id):
                raise Rejected("INPUT_INVALID")
            self.run_id, self.uid = run_id, os.getuid()
            home = private_path(Path.home(), directory=True)
            if home.anchor != "/" or any(part in {".", ".."} for part in home.parts):
                raise Rejected("PATH_INVALID")
            fd = self._open_directory(None, "/")
            for index, part in enumerate(home.parts[1:]):
                fd = self._open_directory(fd, part, private=index == len(home.parts) - 2)
            self._home_identity = self._identity(os.fstat(fd))
            self._parent_ready("AUTHORIZE")
            for part in (".cache", "opencode", "k6-causal"):
                fd = self._open_directory(fd, part, private=True, create=True)
            self._site_dir = self._open_directory(fd, run_id, private=True, create=True, exclusive=True)
            self._directory_path = home / ".cache/opencode/k6-causal" / run_id
            # HOME outside Git is insufficient if a private cache descendant is
            # itself a checkout. Reuse the full leaf guard, without opening source.
            private_path(self._directory_path, directory=True)
            self._verify()
            self._active, self.status = True, "ADMITTED"
        except BaseException as error:
            self.status = "DECLINED"
            self._close_fds()
            if isinstance(error, Rejected):
                raise
            raise Rejected("PATH_INVALID") from None

    def record(self, relative, line):
        if not self._active or self._attempted:
            return False
        self._attempted = True
        fd, success = None, False
        try:
            document = site_document({"version": 1, "run_id": self.run_id, "browser_version": BROWSER_VERSION,
                "source_relative": relative, "source_line": line, "delete_by_utc": self.delete_by_utc})
            raw = json.dumps(document, ensure_ascii=True, separators=(",", ":")).encode("ascii")
            self._verify()
            self._parent_ready("WRITE")
            private_path(self._directory_path, directory=True)
            self._verify()
            fd = os.open("site.json", os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW | os.O_CLOEXEC,
                         0o600, dir_fd=self._site_dir)
            item = os.fstat(fd)
            if not stat.S_ISREG(item.st_mode) or item.st_uid != self.uid or item.st_nlink != 1 or item.st_dev != self._home_identity[0]:
                self.status = "RETENTION_UNPROVEN"
                return False
            self._file_identity = self._identity(item)
            if stat.S_IMODE(item.st_mode) != 0o600:
                raise Rejected("PATH_INVALID")
            done = 0
            while done < len(raw):
                written = os.write(fd, raw[done:])
                if type(written) is not int or not 0 < written <= len(raw) - done:
                    raise Rejected("PATH_INVALID")
                done += written
            os.fsync(fd)
            os.fchmod(fd, 0o400)  # only this exclusively created, identity-bound file
            self._sealed = True
            item = os.fstat(fd)
            named = os.stat("site.json", dir_fd=self._site_dir, follow_symlinks=False)
            if (self._identity(item) != self._file_identity or self._identity(named) != self._file_identity
                    or not stat.S_ISREG(named.st_mode) or named.st_nlink != 1 or item.st_nlink != 1
                    or stat.S_IMODE(item.st_mode) != 0o400 or stat.S_IMODE(named.st_mode) != 0o400
                    or item.st_size != len(raw) or named.st_size != len(raw)):
                raise Rejected("PATH_INVALID")
            self._verify()
            private_path(self._directory_path, directory=True)
            if self._expired():
                raise Rejected("INPUT_INVALID")
            success, self.status = True, "READY"
        except BaseException as error:
            self.status = "INTERRUPTED" if isinstance(error, KeyboardInterrupt) else "WRITE_FAILED"
            if not self._remove():
                self.status = "RETENTION_UNPROVEN"
        finally:
            if fd is not None:
                try:
                    os.close(fd)
                except OSError:
                    success, self.status = False, "RETENTION_UNPROVEN"
        return success

    def _remove(self):
        if self._file_identity is None:
            return self.status != "RETENTION_UNPROVEN"
        try:
            self._verify()
            try:
                item = os.stat("site.json", dir_fd=self._site_dir, follow_symlinks=False)
            except FileNotFoundError:
                self._file_identity = None
                if self.status == "READY":
                    self.status = "REMOVED"
                return True
            if (self._identity(item) != self._file_identity or not stat.S_ISREG(item.st_mode) or item.st_nlink != 1
                    or (self._sealed and stat.S_IMODE(item.st_mode) != 0o400)):
                raise Rejected("CLEANUP_FAILED")
            os.unlink("site.json", dir_fd=self._site_dir)
            try:
                os.stat("site.json", dir_fd=self._site_dir, follow_symlinks=False)
            except FileNotFoundError:
                self._file_identity = None
                if self.status == "READY":
                    self.status = "REMOVED"
                return True
        except (OSError, Rejected):
            pass
        self.status = "RETENTION_UNPROVEN"
        return False

    def retire(self, *, read_confirmed=False):
        if type(read_confirmed) is not bool:
            raise Rejected("INPUT_INVALID")
        if self._file_identity is not None and not read_confirmed and not self._expired():
            return False
        removed = self._remove()
        if removed:
            self.status = "REMOVED"
        return removed

    def _close_fds(self):
        for fd in reversed(self._fds):
            try:
                os.close(fd)
            except OSError:
                self.status = "RETENTION_UNPROVEN"
        self._fds.clear()
        self._active = False

    def __exit__(self, *_error):
        removed = self._remove()
        self._entered = False
        self._close_fds()
        if not removed or self.status == "RETENTION_UNPROVEN":
            raise Rejected("CLEANUP_FAILED") from None
