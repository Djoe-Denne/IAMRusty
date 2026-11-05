"""Small real-Git oracle for dirty source identities; no Docker or Cargo."""
import importlib.util
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import tomllib
import unittest
from types import SimpleNamespace
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "ops/deploy"))
import image_sources as sources

spec = importlib.util.spec_from_file_location("image_builder", ROOT / "ops/deploy/build/images.py")
builder = importlib.util.module_from_spec(spec)
spec.loader.exec_module(builder)
spec = importlib.util.spec_from_file_location("artifact_compiler", ROOT / "ops/deploy/build/compile-artifacts.py")
compiler = importlib.util.module_from_spec(spec)
spec.loader.exec_module(compiler)


class ImageSourceTests(unittest.TestCase):
    def git(self, root, *args):
        return subprocess.run(["git", "-C", str(root), *args], check=True, capture_output=True).stdout.decode().strip()

    def write(self, name, value="public source"):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(value, encoding="utf-8")

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        for repository in (self.root, self.root / "rustycog"):
            repository.mkdir(exist_ok=True)
            self.git(repository, "init")
            self.git(repository, "config", "user.email", "synthetic@example.invalid")
            self.git(repository, "config", "user.name", "Offline oracle")
            self.git(repository, "config", "core.autocrlf", "false")
        for name in ("Cargo.toml", "Cargo.lock", "rustycog/Cargo.toml", "rustycog/src/lib.rs",
                     "services/Hive/src/main.rs", "services/Lazaret/src/main.rs"):
            self.write(name)
        self.write("Cargo.toml", '[patch.crates-io]\nrustycog-framework = { path = "rustycog" }\n')
        self.write("rustycog/Cargo.toml", '[package]\nname = "rustycog-framework"\nversion = "0.0.0"\n')
        self.git(self.root / "rustycog", "add", ".")
        self.git(self.root / "rustycog", "commit", "-m", "SDK fixture")
        self.git(self.root, "add", ".")
        self.git(self.root, "commit", "-m", "Source fixture")

    def test_dirty_untracked_and_deleted_sources_change_only_relevant_identity(self):
        first = sources.capture(self.root)
        self.write("services/Hive/src/new.rs")
        second = sources.capture(self.root)
        self.assertNotEqual(first["sha256"], second["sha256"])
        self.assertNotEqual(sources.target_identity(first, "hive"), sources.target_identity(second, "hive"))
        self.assertEqual(sources.target_identity(first, "lazaret"), sources.target_identity(second, "lazaret"))
        (self.root / "services/Hive/src/main.rs").unlink()
        self.assertNotEqual(second["sha256"], sources.capture(self.root)["sha256"])
        with self.assertRaises(ValueError):
            sources.require_unchanged(first, self.root)

    def test_dirty_consumed_sdk_invalidates_every_consumer(self):
        first = sources.capture(self.root)
        self.write("rustycog/src/lib.rs", "changed SDK bytes")
        second = sources.capture(self.root)
        for name in sources.TARGETS:
            self.assertNotEqual(sources.target_identity(first, name), sources.target_identity(second, name))
        self.assertEqual(first["sdk_revision"], second["sdk_revision"])

    def tracked_public_config_sources(self):
        public = {
            "rustycog/rustycog-config/src/secrets/mod.rs":
                'pub fn public_config_backend() -> &\'static str { "openbao" }\n',
            "rustycog/rustycog-config/src/secrets/tests.rs":
                "// Public rustycog-config test module source fixture.\n",
        }
        for name, contents in public.items():
            self.write(name, contents)
        sdk = self.root / "rustycog"
        relative = {name.removeprefix("rustycog/") for name in public}
        self.git(sdk, "add", *sorted(relative))
        self.git(sdk, "commit", "-m", "Public config source fixture")
        self.assertEqual(set(self.git(sdk, "ls-files", "--", "rustycog-config/src/secrets").splitlines()), relative)
        return public

    def test_tracked_public_config_sources_are_captured_and_copied(self):
        # G-1: the two approved public SDK paths must reach the image snapshot.
        public = self.tracked_public_config_sources()
        destination = self.root / "snapshot"
        captured = sources.capture(self.root, destination)
        entries = {entry["path"]: entry for entry in captured["files"]}
        for name, contents in public.items():
            expected = (self.root / name).read_bytes()
            with self.subTest(path=name, observable="manifest"):
                self.assertIn(name, entries, "tracked public SDK source missing from capture")
                self.assertEqual(entries[name]["sha256"], hashlib.sha256(expected).hexdigest())
            with self.subTest(path=name, observable="copy"):
                copied = destination / name
                self.assertTrue(copied.is_file(), "tracked public SDK source missing from image copy")
                self.assertEqual(copied.read_bytes(), expected)

    def test_public_config_exception_does_not_read_traverse_or_copy_private_neighbors(self):
        # G-2: opening this directory must not open its arbitrary/private content.
        self.tracked_public_config_sources()
        first = sources.capture(self.root)
        private = (
            "rustycog/rustycog-config/src/secrets/extra.rs",
            "rustycog/rustycog-config/src/secrets/.env",
            "rustycog/rustycog-config/src/secrets/.env.local",
            "rustycog/rustycog-config/src/secrets/credentials",
            "rustycog/rustycog-config/src/secrets/credentials.json",
            "rustycog/rustycog-config/src/secrets/credentials.toml",
            "rustycog/rustycog-config/src/secrets/openbao.key",
            "rustycog/rustycog-config/src/secrets/signing.pem",
            "rustycog/rustycog-config/src/secrets/nested/mod.rs",
            "rustycog/rustycog-config/src/secrets/nested/tests.rs",
            "rustycog/rustycog-config/src/secrets/private/mod.rs",
            "rustycog/rustycog-config/src/secrets/certs/leaf.pem",
            "rustycog/rustycog-config/src/secrets/keys/signing.key",
            "rustycog/rustycog-config/src/secrets/secrets/mod.rs",
            "rustycog/rustycog-config/src/other/secrets/mod.rs",
            "rustycog/rustycog-config/src/private/secrets/mod.rs",
            "rustycog/rustycog-core/src/secrets/mod.rs",
            "rustycog/rustycog-core/src/secrets/tests.rs",
            "rustycog/src/secrets/mod.rs",
            "rustycog/src/secrets/tests.rs",
            "services/GitHubConnect/src/secrets/mod.rs",
            "services/GitHubConnect/src/secrets/tests.rs",
            "services/GitHubConnect/rustycog-config/src/secrets/mod.rs",
        )
        for name in private:
            self.write(name, "SYNTHETIC_PRIVATE_DO_NOT_READ_OR_COPY")
        forbidden_files = {self.root / name for name in private}
        forbidden_directories = {self.root / name for name in (
            "rustycog/rustycog-config/src/secrets/nested",
            "rustycog/rustycog-config/src/secrets/private",
            "rustycog/rustycog-config/src/secrets/certs",
            "rustycog/rustycog-config/src/secrets/keys",
            "rustycog/rustycog-config/src/secrets/secrets",
            "rustycog/rustycog-config/src/other/secrets",
            "rustycog/rustycog-config/src/private",
            "rustycog/rustycog-core/src/secrets",
            "rustycog/src/secrets",
            "services/GitHubConnect/src/secrets",
            "services/GitHubConnect/rustycog-config/src/secrets",
        )}
        original_open, original_iterdir = Path.open, Path.iterdir

        def guarded_open(path, *args, **kwargs):
            self.assertNotIn(path, forbidden_files, "private fixture must never be opened")
            return original_open(path, *args, **kwargs)

        def guarded_iterdir(path):
            self.assertNotIn(path, forbidden_directories, "private directory must never be traversed")
            return original_iterdir(path)

        destination = self.root / "snapshot"
        with patch.object(Path, "open", guarded_open), patch.object(Path, "iterdir", guarded_iterdir):
            captured = sources.capture(self.root, destination)
        self.assertEqual(first["files"], captured["files"], "private neighbors must not enter the manifest")
        self.assertEqual(first["sha256"], captured["sha256"])
        copied = {path.relative_to(destination).as_posix() for path in destination.rglob("*") if path.is_file()}
        self.assertEqual(copied, {entry["path"] for entry in first["files"]})
        self.assertTrue(copied.isdisjoint(private), "private neighbors must not enter the image copy")

    def test_each_public_config_source_byte_change_invalidates_sdk_consumers(self):
        # G-3: dirty bytes, not merely the SDK commit, determine consumer identity.
        public = self.tracked_public_config_sources()
        self.assertIn("github", sources.TARGETS)
        for name, contents in public.items():
            first = sources.capture(self.root)
            self.write(name, contents + "// Changed public SDK bytes.\n")
            second = sources.capture(self.root)
            with self.subTest(path=name):
                self.assertEqual(first["sdk_revision"], second["sdk_revision"])
                self.assertEqual(first["revision"], second["revision"])
                unchanged = [target for target in sources.TARGETS
                             if sources.target_identity(first, target) == sources.target_identity(second, target)]
                self.assertEqual(unchanged, [], "public SDK byte change must invalidate every consumer, including github")

    def test_git_ignored_module_asset_and_effective_filter_are_captured(self):
        first = sources.capture(self.root)
        self.write(".gitignore", "services/Hive/src/ignored.rs\nservices/Hive/config/ignored.toml\nDockerfile.build.dockerignore\n")
        self.write("services/Hive/src/ignored.rs", 'pub fn value() -> u8 { 1 }')
        self.write("services/Hive/config/ignored.toml", 'value = "A"')
        self.write("Dockerfile.build.dockerignore", ".git\n**/.env\n")
        self.git(self.root, "check-ignore", "services/Hive/src/ignored.rs")
        second = sources.capture(self.root)
        self.assertNotEqual(first["sha256"], second["sha256"])
        self.assertTrue({"services/Hive/src/ignored.rs", "services/Hive/config/ignored.toml", "Dockerfile.build.dockerignore"}
                        <= {entry["path"] for entry in second["files"]})
        path = self.root / "services/Hive/src/ignored.rs"
        stamp = path.stat().st_mtime_ns
        self.write("services/Hive/src/ignored.rs", 'pub fn value() -> u8 { 2 }')
        os.utime(path, ns=(stamp, stamp))
        self.assertNotEqual(sources.target_identity(second, "hive"), sources.target_identity(sources.capture(self.root), "hive"))

    def test_cache_journal_precedes_failed_build_and_rollback_advances_time(self):
        target = self.root / "cargo-cache"
        source = self.root / "services/Hive/src/main.rs"
        manifest = self.root / "image-inputs.json"
        def sync(value):
            source.write_text(value)
            manifest.write_text(json.dumps({"files": [{"path": "services/Hive/src/main.rs",
                "sha256": hashlib.sha256(source.read_bytes()).hexdigest()}]}))
            return compiler.cached_source_times(self.root, target)
        first = sync("A")
        second = sync("B")
        self.assertNotEqual(first[source.relative_to(self.root).as_posix()]["sha256"], second[source.relative_to(self.root).as_posix()]["sha256"])
        journal = target / ".aiforall-source-times.json"
        self.assertEqual(json.loads(journal.read_text()), second)
        # A failed Cargo has already updated a retained fingerprint after sync B.
        fingerprint = target / "dev-image/.fingerprint/input/dep-lib-input"
        fingerprint.parent.mkdir(parents=True)
        fingerprint.write_bytes(b"partial B")
        retained = fingerprint.stat().st_mtime_ns
        third = sync("A")
        self.assertGreater(third["services/Hive/src/main.rs"]["mtime_ns"], retained)
        self.assertEqual(json.loads(journal.read_text()), third)
        with patch.object(compiler.time, "time_ns", return_value=0), self.assertRaises(ValueError):
            compiler.cached_source_times(self.root, target)
        self.assertTrue(fingerprint.exists(), "clock refusal must not purge the cache")

    @unittest.skipUnless(os.environ.get("AIFORALL_IMAGE_CACHE_PROOF") == "1", "explicit native Cargo cache proof")
    def test_native_binary_follows_rollback_after_partial_cargo_failure(self):
        config = tomllib.loads((ROOT / ".cargo/config.toml").read_text(encoding="utf-8"))
        self.assertEqual(config["build"]["jobs"], 24)
        self.assertIn(os.environ.get("CARGO_BUILD_JOBS"), (None, "24"))
        self.write(".cargo/config.toml", (ROOT / ".cargo/config.toml").read_text(encoding="utf-8"))
        target = (Path(config["build"]["target-dir"]) / "image-cache-proof" / self.root.name
                  if os.name == "nt" else self.root / "target")
        if os.name == "nt":
            self.assertEqual(target.drive.upper(), "E:")
        env = {**os.environ, "CARGO_TARGET_DIR": str(target), "CARGO_TERM_COLOR": "never"}
        evidence = ROOT / ".tmp" / ("image-cache-" + self.root.name)
        evidence.mkdir(parents=True)
        self.write("Cargo.toml", '[workspace]\nmembers = ["input"]\n[package]\nname = "cache-image-oracle"\nversion = "0.0.0"\nedition = "2021"\n[dependencies]\ninput = { package = "cache-input-oracle", path = "input" }\n')
        self.write("input/Cargo.toml", '[package]\nname = "cache-input-oracle"\nversion = "0.0.0"\nedition = "2021"\n[lib]\nname = "cache_input"\n')
        self.write("build.rs", 'fn main() { println!("cargo:rerun-if-changed=asset.txt"); println!("cargo:rustc-env=ASSET={}", std::fs::read_to_string("asset.txt").unwrap()); }\n')
        main = 'fn main() { println!("{}:{}", input::value(), env!("ASSET")); }\n'
        self.write("src/main.rs", main)
        self.write("input/src/lib.rs", "pub fn value() -> char { 'A' }\n")
        self.write("asset.txt", "A")

        def cargo(label, arguments):
            result = subprocess.run(["cargo", *arguments], cwd=self.root, env=env, capture_output=True, text=True, timeout=120)
            (evidence / (label + ".stdout")).write_text(result.stdout, encoding="utf-8")
            (evidence / (label + ".stderr")).write_text(result.stderr, encoding="utf-8")
            return result

        self.assertEqual(cargo("lock", ["generate-lockfile", "--offline"]).returncode, 0)
        names = ["Cargo.toml", "Cargo.lock", "input/Cargo.toml", "input/src/lib.rs", "src/main.rs", "build.rs", "asset.txt", ".cargo/config.toml"]
        states = []
        def sync():
            (self.root / "image-inputs.json").write_text(json.dumps({"files": [{"path": name,
                "sha256": hashlib.sha256((self.root / name).read_bytes()).hexdigest()} for name in names]}))
            states.append(compiler.cached_source_times(self.root, target))
        def build(label):
            result = cargo(label, ["build", "--locked", "--offline", "--message-format=json-render-diagnostics"])
            return result, [json.loads(line) for line in result.stdout.splitlines() if line]
        def output(records):
            executable = next(r["executable"] for r in records if r.get("reason") == "compiler-artifact" and r.get("executable"))
            return subprocess.run([executable], capture_output=True, text=True, check=True, timeout=10).stdout.strip()

        sync()
        first, records = build("A")
        self.assertEqual(first.returncode, 0, "native fixture build failed; see evidence logs")
        self.assertEqual(output(records), "A:A")
        stamps = {name: (self.root / name).stat().st_mtime_ns for name in names}
        self.write("input/src/lib.rs", "pub fn value() -> char { 'B' }\n")
        self.write("asset.txt", "B")
        self.write("src/main.rs", 'compile_error!("INTENTIONAL_PARTIAL_FAILURE"); fn main() {}\n')
        for name in names:
            os.utime(self.root / name, ns=(stamps[name], stamps[name]))
        sync()
        failed, partial = build("B-failed")
        self.assertNotEqual(failed.returncode, 0)
        self.assertTrue(any(r.get("reason") == "compiler-artifact" and r["target"]["name"] == "cache_input" and not r["fresh"] for r in partial))
        self.assertTrue(any(r.get("reason") == "build-script-executed" and ["ASSET", "B"] in r.get("env", []) for r in partial))
        self.write("input/src/lib.rs", "pub fn value() -> char { 'A' }\n")
        self.write("asset.txt", "A")
        self.write("src/main.rs", main)
        for name in names:
            os.utime(self.root / name, ns=(stamps[name], stamps[name]))
        sync()
        final, records = build("A-rollback")
        self.assertEqual(final.returncode, 0, "native rollback build failed; see evidence logs")
        self.assertEqual(output(records), "A:A")
        proof = {"platform": sys.platform, "jobs": 24, "sequence": ["A:A", "partial B + asset B then failure", "A:A"],
                 "compiler_sha256": hashlib.sha256((ROOT / "ops/deploy/build/compile-artifacts.py").read_bytes()).hexdigest(),
                 "journal_states": states}
        (evidence / "proof.json").write_text(json.dumps(proof, indent=2), encoding="utf-8")
        print("native cache rollback proof: " + str(evidence.relative_to(ROOT)) + "; target cache retained")

    def test_private_inputs_are_not_read_or_copied(self):
        first = sources.capture(self.root)
        for name in ("services/Hive/.env", "rustycog/rustycog-core/.env.private",
                     "services/Hive/certs/leaf.key", "services/Hive/credentials.json"):
            self.write(name, "DO_NOT_COPY")
        destination = self.root / "snapshot"
        captured = sources.capture(self.root, destination)
        self.assertEqual(first["sha256"], captured["sha256"])
        self.assertFalse(any(path.name.startswith(".env") or path.suffix == ".key" for path in destination.rglob("*")))
        for entry in captured["files"]:
            self.assertEqual((destination / entry["path"]).read_bytes(), (self.root / entry["path"]).read_bytes())

    def test_release_is_blocked_before_source_capture_or_docker(self):
        args = self.arguments()
        args.policy, args.revision = "release", "0" * 40
        with patch.object(builder, "capture") as capture, patch.object(builder, "command") as docker:
            with self.assertRaisesRegex(ValueError, "master export"):
                builder.run(args)
        capture.assert_not_called()
        docker.assert_not_called()

    def test_existing_matching_image_skips_cargo_and_packaging(self):
        snapshot = sources.capture(self.root)
        value = {"Id": "sha256:" + "d" * 64, "Os": "linux", "Architecture": "amd64", "Config": {"Labels": {
            "aiforall.dev.source-input": sources.target_identity(snapshot, "hive"), "aiforall.dev.source-policy": "local",
            "aiforall.dev.freshness-policy": builder.FRESHNESS_POLICY,
            "aiforall.dev.cargo-profile": "dev-image", "org.opencontainers.image.revision": snapshot["revision"],
            "aiforall.dev.build-targets": "hive", "aiforall.dev.build-context": builder.build_context(["hive"], "dev-image"),
            "aiforall.dev.sdk-revision": snapshot["sdk_revision"]}}}
        args = self.arguments()
        with self.builder_context(), patch.object(builder, "inspect", return_value=value), patch.object(builder, "build") as compile_image:
            builder.run(args)
        compile_image.assert_not_called()
        self.assertTrue(args.output.is_file())

    def test_source_change_during_compile_prevents_packaging_and_manifest(self):
        def artifact_build(*args, **kwargs):
            self.write("services/Hive/src/main.rs", "changed while compiling immutable snapshot")
            return {"Id": "sha256:" + "d" * 64}
        args = self.arguments()
        with self.builder_context(), patch.object(builder, "inspect", return_value=None), patch.object(builder, "build", side_effect=artifact_build) as compile_image:
            with self.assertRaises(ValueError):
                builder.run(args)
        compile_image.assert_called_once()
        self.assertFalse(args.output.exists())

    def arguments(self):
        return SimpleNamespace(action="build", targets="hive", github_openbao=False, oauth_proof=False, policy="local",
            revision=None, profile=None, cold=False, output=self.root / "result.json", owner="offline-test")

    def builder_context(self):
        from contextlib import ExitStack
        stack = ExitStack()
        stack.enter_context(patch.object(builder, "ROOT", self.root))
        stack.enter_context(patch.object(builder, "capture", side_effect=lambda **kw: sources.capture(self.root, **kw)))
        stack.enter_context(patch.object(builder, "require_unchanged", side_effect=lambda value: sources.require_unchanged(value, self.root)))
        stack.enter_context(patch.object(builder, "command", side_effect=lambda args, **kw: SimpleNamespace(
            stdout="linux\n" if "info" in args else "Name: offline\nDriver: docker\n")))
        return stack


if __name__ == "__main__":
    unittest.main()
