"""Retained opt-in native fresh-target oracle; legacy source/provenance code removed."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[2]


class ImageSourceTests(unittest.TestCase):
    def write(self, name, value="public source"):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(value, encoding="utf-8")

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)

    @unittest.skipUnless(os.environ.get("AIFORALL_IMAGE_REBUILD_PROOF") == "1", "explicit native Cargo fresh-target proof")
    def test_native_fresh_targets_follow_rollback_after_partial_cargo_failure(self):
        config = tomllib.loads((ROOT / ".cargo/config.toml").read_text(encoding="utf-8"))
        self.assertEqual(config["build"]["jobs"], 24)
        self.assertIn(os.environ.get("CARGO_BUILD_JOBS"), (None, "24"))
        self.write(".cargo/config.toml", (ROOT / ".cargo/config.toml").read_text(encoding="utf-8"))
        target = (Path(config["build"]["target-dir"]) / "image-rebuild-proof" / self.root.name
                  if os.name == "nt" else self.root / "target")
        if os.name == "nt":
            self.assertEqual(target.drive.upper(), "E:")
        env = {**os.environ, "CARGO_TARGET_DIR": str(target), "CARGO_TERM_COLOR": "never"}
        evidence = ROOT / ".tmp" / ("image-rebuild-" + self.root.name)
        evidence.mkdir(parents=True)
        self.write("Cargo.toml", '[workspace]\nmembers = ["input"]\n[package]\nname = "cache-image-oracle"\nversion = "0.0.0"\nedition = "2021"\n[dependencies]\ninput = { package = "cache-input-oracle", path = "input" }\n')
        self.write("input/Cargo.toml", '[package]\nname = "cache-input-oracle"\nversion = "0.0.0"\nedition = "2021"\n[lib]\nname = "cache_input"\n')
        self.write("build.rs", 'fn main() { println!("cargo:rerun-if-changed=asset.txt"); println!("cargo:rustc-env=ASSET={}", std::fs::read_to_string("asset.txt").unwrap()); }\n')
        main = 'fn main() { println!("{}:{}", input::value(), env!("ASSET")); }\n'
        self.write("src/main.rs", main)
        self.write("input/src/lib.rs", "pub fn value() -> char { 'A' }\n")
        self.write("asset.txt", "A")

        def cargo(label, arguments):
            env["CARGO_TARGET_DIR"] = str(target / label)
            self.assertFalse((target / label).exists(), "each rebuild needs a fresh target")
            result = subprocess.run(["cargo", *arguments], cwd=self.root, env=env, capture_output=True, text=True, timeout=120)
            (evidence / (label + ".stdout")).write_text(result.stdout, encoding="utf-8")
            (evidence / (label + ".stderr")).write_text(result.stderr, encoding="utf-8")
            return result

        self.assertEqual(cargo("lock", ["generate-lockfile", "--offline"]).returncode, 0)
        names = ["Cargo.toml", "Cargo.lock", "input/Cargo.toml", "input/src/lib.rs", "src/main.rs", "build.rs", "asset.txt", ".cargo/config.toml"]
        def build(label):
            result = cargo(label, ["build", "--locked", "--offline", "--message-format=json-render-diagnostics"])
            return result, [json.loads(line) for line in result.stdout.splitlines() if line]
        def output(records):
            executable = next(r["executable"] for r in records if r.get("reason") == "compiler-artifact" and r.get("executable"))
            return subprocess.run([executable], capture_output=True, text=True, check=True, timeout=10).stdout.strip()

        first, records = build("A")
        self.assertEqual(first.returncode, 0, "native fixture build failed; see evidence logs")
        self.assertEqual(output(records), "A:A")
        stamps = {name: (self.root / name).stat().st_mtime_ns for name in names}
        self.write("input/src/lib.rs", "pub fn value() -> char { 'B' }\n")
        self.write("asset.txt", "B")
        self.write("src/main.rs", 'compile_error!("INTENTIONAL_PARTIAL_FAILURE"); fn main() {}\n')
        for name in names:
            os.utime(self.root / name, ns=(stamps[name], stamps[name]))
        failed, partial = build("B-failed")
        self.assertNotEqual(failed.returncode, 0)
        self.assertTrue(any(r.get("reason") == "compiler-artifact" and r["target"]["name"] == "cache_input" and not r["fresh"] for r in partial))
        self.assertTrue(any(r.get("reason") == "build-script-executed" and ["ASSET", "B"] in r.get("env", []) for r in partial))
        self.write("input/src/lib.rs", "pub fn value() -> char { 'A' }\n")
        self.write("asset.txt", "A")
        self.write("src/main.rs", main)
        for name in names:
            os.utime(self.root / name, ns=(stamps[name], stamps[name]))
        final, records = build("A-rollback")
        self.assertEqual(final.returncode, 0, "native rollback build failed; see evidence logs")
        self.assertEqual(output(records), "A:A")
        proof = {"platform": sys.platform, "jobs": 24, "sequence": ["A:A", "partial B + asset B then failure", "A:A"],
                 "recipe_sha256": hashlib.sha256((ROOT / "Dockerfile.build").read_bytes()).hexdigest(),
                 "targets": [str(target / label) for label in ("A", "B-failed", "A-rollback")]}
        (evidence / "proof.json").write_text(json.dumps(proof, indent=2), encoding="utf-8")
        print("native fresh-target rollback proof: " + str(evidence.relative_to(ROOT)))


if __name__ == "__main__":
    unittest.main()
