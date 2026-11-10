"""Native OpenSSL PKI gates; no Docker, Kind, service or historical private input."""
import base64
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "ops/deploy/kind-local-pki.ps1"


class NativePkiTests(unittest.TestCase):
    def tools(self):
        self.assertTrue(SCRIPT.is_file(), "native PKI recipe is not implemented")
        powershell = shutil.which("powershell.exe")
        openssl = shutil.which("openssl") or "C:/Program Files/Git/usr/bin/openssl.exe"
        if not powershell or not Path(openssl).is_file():
            self.skipTest("native PowerShell/OpenSSL unavailable (NOT RUN)")
        return powershell, openssl

    def prepare(self, directory):
        powershell, openssl = self.tools()
        env = dict(os.environ, PATHEXT=".COM;.EXE;.BAT;.CMD")
        return subprocess.run([
            powershell, "-NoProfile", "-NonInteractive", "-File", str(SCRIPT),
            "-Directory", str(directory), "-OpenSslPath", openssl,
        ], cwd=ROOT, env=env, capture_output=True, encoding="oem", timeout=90, check=False)

    def check_private_acls(self, directory):
        powershell, _ = self.tools()
        quoted = str(directory).replace("'", "''")
        command = """$ErrorActionPreference = 'Stop'
$root = '""" + quoted + """'
$sid = [Security.Principal.WindowsIdentity]::GetCurrent().User
$paths = @((Get-Item -LiteralPath $root)) + @(Get-ChildItem -LiteralPath $root -Directory -Recurse) + @(Get-ChildItem -LiteralPath $root -Filter '*.key' -File -Recurse)
@(foreach ($path in $paths) {
    $acl = Get-Acl -LiteralPath $path.FullName
    $rules = @($acl.GetAccessRules($true, $true, [Security.Principal.SecurityIdentifier]))
    [pscustomobject]@{
        key = -not $path.PSIsContainer
        owner = $acl.GetOwner([Security.Principal.SecurityIdentifier]).Value -eq $sid.Value
        protected = $acl.AreAccessRulesProtected
        private = $rules.Count -eq 1 -and $rules[0].IdentityReference.Value -eq $sid.Value -and $rules[0].AccessControlType -eq 'Allow' -and $rules[0].FileSystemRights -eq 'FullControl' -and -not $rules[0].IsInherited
    }
}) | ConvertTo-Json -Compress
"""
        result = subprocess.run([
            powershell, "-NoProfile", "-NonInteractive", "-EncodedCommand",
            base64.b64encode(command.encode("utf-16-le")).decode("ascii"),
        ], cwd=ROOT, capture_output=True, encoding="oem", timeout=30, check=False)
        self.assertEqual(result.returncode, 0, result.stderr)
        rows = json.loads(result.stdout)
        self.assertEqual(sum(not row["key"] for row in rows), 3)
        self.assertGreaterEqual(sum(row["key"] for row in rows), 15)
        self.assertTrue(all(row["owner"] and row["protected"] and row["private"] for row in rows))

    def test_initial_issue_reuse_and_real_dependency_sans(self):
        _, openssl = self.tools()
        with tempfile.TemporaryDirectory(dir=ROOT / ".tmp") as temporary:
            directory = Path(temporary) / "pki"
            first = self.prepare(directory)
            self.assertEqual(first.returncode, 0, first.stderr)
            self.check_private_acls(directory)
            files = [path for path in directory.rglob("*") if path.suffix in {".key", ".crt"}]
            before = {path: hashlib.sha256(path.read_bytes()).hexdigest() for path in files}
            self.assertGreaterEqual(len(files), 30)
            again = self.prepare(directory)
            self.assertEqual(again.returncode, 0, again.stderr)
            self.check_private_acls(directory)
            self.assertEqual(before, {path: hashlib.sha256(path.read_bytes()).hexdigest() for path in files})
            for name in ("postgres", "redis", "openfga", "localstack"):
                command = [openssl, "verify", "-CAfile", str(directory / "mesh/ca.crt"),
                           "-verify_hostname", name, "-purpose", "sslserver",
                           str(directory / ("mesh/" + name + ".crt"))]
                checked = subprocess.run(command, capture_output=True, text=True, check=False)
                self.assertEqual(checked.returncode, 0, checked.stderr)
                command[command.index(name)] = "unrelated.invalid"
                denied = subprocess.run(command, capture_output=True, text=True, check=False)
                self.assertNotEqual(denied.returncode, 0)
                self.assertIn("hostname mismatch", denied.stderr.lower())
            self.assertNotEqual((directory / "mesh/ca.key").read_bytes(),
                                (directory / "workload/ca.key").read_bytes())

    def test_partial_ca_is_refused_and_never_overwritten(self):
        self.tools()
        with tempfile.TemporaryDirectory(dir=ROOT / ".tmp") as temporary:
            directory = Path(temporary)
            (directory / "mesh").mkdir()
            sentinel = directory / "mesh/ca.crt"
            sentinel.write_bytes(b"existing-material-must-not-be-overwritten")
            result = self.prepare(directory)
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(sentinel.read_bytes(), b"existing-material-must-not-be-overwritten")
            self.assertFalse((directory / "mesh/ca.key").exists())


if __name__ == "__main__":
    unittest.main()
