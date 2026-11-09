"""Native shell tests for the OpenBao 2.6 token startup oracle; no Docker."""
import importlib.util
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "ops/deploy"))
_spec = importlib.util.spec_from_file_location("probe_renderer", ROOT / "ops/deploy/render-local-full.py")
renderer = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(renderer)


class TokenProbeTests(unittest.TestCase):
    def setUp(self):
        self.shell = str(Path(os.environ.get("ProgramFiles", "C:/Program Files")) / "Git/bin/bash.exe") if os.name == "nt" else shutil.which("sh")
        if not self.shell or not Path(self.shell).is_file():
            self.skipTest("native POSIX shell required; Windows uses Git Bash, never WSL")
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.directory = Path(self.temp.name)
        self.bin = self.directory / "bin"
        self.bin.mkdir()
        self.log = self.directory / "calls.log"
        self.environment = {**os.environ, "PATH": str(self.bin) + os.pathsep + os.environ["PATH"],
            "PROBE_LOG": self.log.as_posix(), "PROBE_SINK": "/run/github-openbao/token",
            "FAKE_TTL": "900", "FAKE_POLICIES": '["local-full-oauth-connect"]', "BAO_TOKEN": "synthetic-unit-token"}
        scripts = {"bao": '''#!/bin/sh
printf '%s\n' "$*" >> "$PROBE_LOG"
[ "$BAO_TOKEN" = synthetic-unit-token ] || exit 67
if [ "${FAKE_LOOKUP_FAILURE:-0}" != 0 ]; then
  echo synthetic-sensitive-error >&2
  exit 68
fi
[ "$1" = read ] && [ "$3" = auth/token/lookup-self ] || { echo 'flag provided but not defined: -field' >&2; exit 64; }
case "$2" in
  -field=ttl) printf '%s' "$FAKE_TTL" ;;
  -field=policies) printf '%s' "$FAKE_POLICIES" ;;
  *) exit 69 ;;
esac
'''}
        for name, text in scripts.items():
            file = self.bin / name
            file.write_text(text, encoding="utf-8", newline="\n")
            file.chmod(0o755)
        self.scripts = {"github": (ROOT / "ops/deploy/apps/overlays/local-full/helpers/check-github-token.sh").read_text()}
        objects = renderer.render("nativeprobecheck", oauth_connect=True)
        self.scripts["oauth"] = next(o for o in objects if o["kind"] == "ConfigMap"
            and o["metadata"]["name"].startswith("local-full-bootstrap-scripts"))["data"]["check-oauth-token.sh"]

    def run_probe(self, profile="oauth", **environment):
        file = self.directory / "probe.sh"
        sink = "/run/oauth-openbao/token" if profile == "oauth" else "/run/github-openbao/token"
        self.assertIn("cat " + sink, self.scripts[profile])
        # Redirect only the public sink location to this test's actual file.
        file.write_text(self.scripts[profile].replace(sink, "sink-token"), encoding="utf-8", newline="\n")
        token_file = self.directory / "sink-token"
        if environment.get("FAKE_MISSING_SINK") == "1":
            if token_file.exists():
                token_file.unlink()
        else:
            token_file.write_text("synthetic-unit-token", encoding="utf-8")
        if self.log.exists():
            self.log.unlink()
        result = subprocess.run([self.shell, file.as_posix()], env={**self.environment, "PROBE_SINK": sink, **environment},
                                cwd=self.directory, capture_output=True, text=True, timeout=15)
        self.assertNotIn("synthetic-unit-token", result.stdout + result.stderr)
        self.assertNotIn("synthetic-sensitive-error", result.stdout + result.stderr)
        calls = self.log.read_text().splitlines() if self.log.exists() else []
        return result, calls

    def test_fixture_preflight_accepts_supported_read_and_refuses_token_lookup_field(self):
        result = subprocess.run([self.shell, "-c", "bao read -field=ttl auth/token/lookup-self"],
            env=self.environment, capture_output=True, text=True, timeout=15)
        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stdout, "900")
        result = subprocess.run([self.shell, "-c", "bao token lookup -field=ttl"],
            env=self.environment, capture_output=True, text=True, timeout=15)
        self.assertEqual(result.returncode, 64)

    def test_valid_token_checks_supported_lookup_self_fields_for_both_profiles(self):
        for profile in self.scripts:
            with self.subTest(profile=profile):
                result, calls = self.run_probe(profile)
                self.assertEqual(result.returncode, 0)
                self.assertEqual(result.stdout + result.stderr, "")
                self.assertEqual(calls, ["read -field=ttl auth/token/lookup-self", "read -field=policies auth/token/lookup-self"])

    def test_ttl_bounds_and_nonnumeric_values_remain_fail_closed(self):
        for ttl, valid in (("29", False), ("30", True), ("3600", True), ("3601", False),
                           ("0", False), ("-1", False), ("", False), ("15m", False)):
            with self.subTest(ttl=ttl):
                result, _ = self.run_probe(FAKE_TTL=ttl)
                self.assertEqual(result.returncode == 0, valid)

    def test_root_policy_still_refused(self):
        result, calls = self.run_probe(FAKE_POLICIES='["root"]')
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(len(calls), 2)

    def test_lookup_failure_is_refused_without_leaking_cli_error(self):
        result, calls = self.run_probe(FAKE_LOOKUP_FAILURE="1")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(len(calls), 1)

    def test_missing_sink_refuses_before_any_cli_call(self):
        result, calls = self.run_probe(FAKE_MISSING_SINK="1")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(calls, [])


if __name__ == "__main__":
    unittest.main()
