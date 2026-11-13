"""Offline OpenSSL proof of fresh same-CA good/wrong TLS identities."""
import importlib.util
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[4]
SPEC = importlib.util.spec_from_file_location("ci_pki", ROOT / "ops/tests/k6/ci/pki.py")
pki = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(pki)
OPENSSL = shutil.which("openssl")
RUN = "a" * 32


class PkiInputsTests(unittest.TestCase):
    def test_identities_are_closed_and_negative_has_no_iam_name(self):
        names = pki.identities(RUN)
        self.assertEqual(len(names), 9)
        self.assertIn("DNS:iam.aiforall-local-full.svc.cluster.local", names["iam-service"])
        self.assertTrue(all("iam" not in value for value in names["hive-service"]))
        self.assertIn("DNS:localhost", names["idp"])
        self.assertIn("IP:127.0.0.1", names["envoy-mesh"])
        for invalid in ("", None, "A" * 32, "a" * 32 + "\n", "../../key"):
            with self.assertRaises(ValueError):
                pki.identities(invalid)

    def test_existing_key_is_not_overwritten_or_executed(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            (root / "ca.key").write_text("retained", encoding="ascii")
            calls = []
            with self.assertRaises(ValueError):
                pki.generate(root, RUN, calls.append)
            self.assertEqual(calls, [])
            self.assertEqual((root / "ca.key").read_text(), "retained")

    def test_missing_outputs_cannot_be_claimed_as_success(self):
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaises(ValueError):
                pki.generate(Path(directory).resolve(), RUN, lambda args: None)


@unittest.skipUnless(OPENSSL, "existing OpenSSL required; no trust or runtime installed")
class RealPkiTests(unittest.TestCase):
    def test_fresh_ca_verified_iam_and_same_ca_wrong_san(self):
        with tempfile.TemporaryDirectory(prefix="k6-ci-pki-") as directory:
            root = Path(directory).resolve()

            def execute(args):
                result = subprocess.run([OPENSSL, *args], capture_output=True, timeout=30)
                self.assertEqual(result.returncode, 0, "OpenSSL generation failed (private output omitted)")

            previous = os.umask(0o077)
            try:
                pki.generate(root, RUN, execute)
            finally:
                os.umask(previous)
            for name in pki.identities(RUN):
                execute(["verify", "-CAfile", str(root / "ca.crt"), str(root / (name + ".crt"))])
            execute(["verify", "-CAfile", str(root / "ca.crt"), "-verify_hostname",
                     "iam.aiforall-local-full.svc.cluster.local", str(root / "iam-service.crt")])
            wrong = subprocess.run([OPENSSL, "verify", "-CAfile", str(root / "ca.crt"),
                                    "-verify_hostname", "iam.aiforall-local-full.svc.cluster.local",
                                    str(root / "hive-service.crt")], capture_output=True, timeout=10)
            self.assertNotEqual(wrong.returncode, 0)
            self.assertIn(b"hostname mismatch", wrong.stdout + wrong.stderr)
            execute(["verify", "-CAfile", str(root / "ca.crt"), "-verify_ip", "127.0.0.1",
                     str(root / "envoy-mesh.crt")])
            execute(["verify", "-CAfile", str(root / "ca.crt"), "-verify_hostname", "localhost",
                     str(root / "idp.crt")])
            with self.assertRaises(ValueError):
                pki.generate(root, RUN, execute)


if __name__ == "__main__":
    unittest.main()
