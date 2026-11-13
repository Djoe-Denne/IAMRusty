"""Portable producer/consumer refusal tests; no Docker or compilation."""
import copy
from datetime import datetime, timezone
import hashlib
import io
import json
import os
from pathlib import Path
import struct
import sys
import tarfile
import tempfile
import unittest
from unittest.mock import patch
import zipfile

CI = Path(__file__).resolve().parents[1] / "ci"
sys.path.insert(0, str(CI))
import binaries as b
import artifacts as a
import package as p

SHA, SDK = "a" * 40, "b" * 40
ELF = {"architecture": "x86_64", "interpreter": "/lib64/ld-linux-x86-64.so.2",
       "needed": ["libc.so.6"], "required_versions": ["GLIBC_2.39"]}


def executable(marker=b""):
    header = bytearray(64)
    header[:6] = b"\x7fELF\x02\x01"
    struct.pack_into("<H", header, 16, 3)
    struct.pack_into("<H", header, 18, 62)
    return bytes(header) + marker


def inspect(args):
    if "--program-headers" in args:
        return "[Requesting program interpreter: /lib64/ld-linux-x86-64.so.2]"
    if "--dynamic" in args:
        return "(NEEDED) Shared library: [libc.so.6]"
    return "Name: GLIBC_2.39"


def manifest(data=None):
    data = executable() if data is None else data
    return dict(schema=b.SCHEMA, role="oauth", repository="owner/repo", repository_id=123,
                workflow_path=".github/workflows/ci.yml", workflow_id=456, source_sha=SHA, sdk_sha=SDK,
                run_id=789, run_attempt=1, job="build", job_name=b.ROLES["oauth"],
                cargo_profile="dev", output_directory="debug", target="x86_64-unknown-linux-gnu",
                producer_os="ubuntu-24.04", libc="glibc 2.39", cargo="cargo 1.99.1", rustc="rustc 1.99.1",
                instrumentation="none", flags={"environment": {name: "" for name in b.FLAG_NAMES}, "cargo_config_sha256": {}},
                build_args=["build", "--locked", "--all-targets", "--package", "oauth-connect-service"],
                cargo_lock_sha256="c" * 64, inventory_sha256="d" * 64,
                files=[dict(package="oauth-connect-service", name="oauth-connect-service", path="bin/oauth-connect-service",
                            size=len(data), sha256=hashlib.sha256(data).hexdigest(), elf=ELF)])


EXPECTED = {"role": "oauth", "repository": "owner/repo", "repository_id": 123,
            "workflow_id": 456, "source_sha": SHA, "sdk_sha": SDK, "run_id": 789, "run_attempt": 1,
            "cargo_lock_sha256": "c" * 64, "inventory_sha256": "d" * 64}
OUTPUTS = {"oauth-connect-service": "oauth-connect-service"}


def bundle(path, value=None, data=None, extra=None):
    data = executable() if data is None else data
    value = manifest(data) if value is None else value
    with tarfile.open(path, "w:gz") as archive:
        for name, raw in (("manifest.json", json.dumps(value).encode()), ("bin/oauth-connect-service", data)):
            member = tarfile.TarInfo(name)
            member.size = len(raw)
            archive.addfile(member, io.BytesIO(raw))
        if extra:
            archive.addfile(extra, io.BytesIO(b""))


class BinaryContractTests(unittest.TestCase):
    def test_inventory_includes_migration_and_exact_selected_packages(self):
        self.assertEqual(b.inventory(b.ROOT, "iam"), {"iam-service": "iam-service", "iam-migration": "iammigration"})
        self.assertEqual(b.inventory(b.ROOT, "oauth"), OUTPUTS)
        self.assertEqual(b.inventory(b.ROOT, "ext-authz"), {"ext-authz": "ext-authz"})
        with self.assertRaises(b.Refused):
            b.inventory(b.ROOT, "../other")

    def test_elf_platform_and_instrumentation_are_not_inferred_from_profile(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "binary"
            path.write_bytes(executable())
            self.assertEqual(b.elf(path, inspect), ELF)
            for data in (b"MZ" + b"a" * 64, executable(b"__llvm_profile_runtime"),
                         executable(b"__llvm_prf_cnts"), executable(b"__llvm_covmap")):
                path.write_bytes(data)
                with self.assertRaises(b.Refused):
                    b.elf(path, inspect)
            path.write_bytes(executable(b"a" * (1024 * 1024 - 70) + b"__llvm_profile_runtime"))
            with self.assertRaises(b.Refused):
                b.elf(path, inspect)

    def test_wrong_architecture_interpreter_or_absent_dynamic_metadata_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "binary"
            raw = bytearray(executable())
            struct.pack_into("<H", raw, 18, 183)
            path.write_bytes(raw)
            with self.assertRaises(b.Refused):
                b.elf(path, inspect)
            path.write_bytes(executable())
            with self.assertRaises(b.Refused):
                b.elf(path, lambda args: "")

    def test_flags_wrappers_and_cargo_config_cannot_hide_coverage(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            clean = {"CARGO_HOME": str(root / "cargo-home")}
            self.assertEqual(b.flags(clean, root)["environment"], {name: "" for name in b.FLAG_NAMES})
            for name in b.FLAG_NAMES:
                with self.subTest(name=name), self.assertRaises(b.Refused):
                    b.flags({**clean, name: "instrument"}, root)
            with self.assertRaises(b.Refused):
                b.flags({**clean, "CARGO_PROFILE_DEV_OPT_LEVEL": "1"}, root)
            (root / ".cargo").mkdir()
            (root / ".cargo/config.toml").write_text('[build]\nrustflags=["-Cinstrument-coverage"]\n')
            with self.assertRaises(b.Refused):
                b.flags(clean, root)

    def test_repository_clippy_aliases_do_not_change_or_instrument_builtin_builds(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / ".cargo").mkdir()
            config = root / ".cargo/config.toml"
            config.write_text((b.ROOT / ".cargo/config.toml").read_text(encoding="utf-8"), encoding="utf-8")
            accepted = b.flags({"CARGO_HOME": str(root / "cargo-home")}, root)
            self.assertEqual(accepted["cargo_config_sha256"], {"0": b.sha(config)})
            self.assertEqual(accepted["environment"], {name: "" for name in b.FLAG_NAMES})

    def test_alias_exception_cannot_admit_build_shell_target_or_environment_overrides(self):
        bad = ['[alias]\nbuild=["clippy"]\n', '[alias]\nclippy-ci-tests="!arbitrary-command"\n',
               '[alias]\nclippy-ci-production=["build"]\n', '[build]\nrustflags=["-Cinstrument-coverage"]\n',
               '[env]\nRUSTFLAGS="-Cinstrument-coverage"\n',
               '[target.x86_64-unknown-linux-gnu]\nrustflags=["-Cinstrument-coverage"]\n']
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / ".cargo").mkdir()
            config = root / ".cargo/config.toml"
            environment = {"CARGO_HOME": str(root / "cargo-home")}
            for value in bad:
                config.write_text(value, encoding="utf-8")
                with self.subTest(value=value), self.assertRaises(b.Refused):
                    b.flags(environment, root)
            config.write_text('[build]\njobs=24\n', encoding="utf-8")
            (root / "cargo-home").mkdir()
            (root / "cargo-home/config.toml").write_text('[alias]\nclippy-ci-tests=["clippy"]\n')
            with self.assertRaises(b.Refused):
                b.flags(environment, root)

    def test_each_manifest_provenance_field_is_bound_to_expected_source(self):
        for name, value in EXPECTED.items():
            changed = manifest()
            changed[name] = value + 1 if type(value) is int else value + "other"
            with self.subTest(field=name), self.assertRaises(b.Refused):
                b.validate_manifest(changed, EXPECTED, OUTPUTS)
        for field, value in (("schema", "future"), ("cargo_profile", "release"), ("instrumentation", "llvm"),
                             ("job_name", "Build (untrusted)"), ("producer_os", "ubuntu-22.04")):
            with self.assertRaises(b.Refused):
                b.validate_manifest({**manifest(), field: value}, EXPECTED, OUTPUTS)

    def test_existing_cargo_package_underscore_is_valid_but_flags_and_paths_are_not(self):
        # Real IAM matrix includes iam-http_server. The consumer never executes
        # these metadata arguments, but must recognize legal Cargo names.
        value = manifest()
        value["build_args"] += ["--package", "iam-http_server"]
        b.validate_manifest(value, EXPECTED, OUTPUTS)
        for invalid in ("../iam-http_server", "--release", "iam; command", "iam/http_server"):
            changed = copy.deepcopy(value)
            changed["build_args"][-1] = invalid
            with self.subTest(invalid=invalid), self.assertRaises(b.Refused):
                b.validate_manifest(changed, EXPECTED, OUTPUTS)

    def test_closed_archive_and_internal_hashes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / "bundle.tar.gz"
            bundle(path)
            value = b.unpack(path, root / "accepted", EXPECTED, OUTPUTS, lambda path: ELF)
            self.assertEqual(value["source_sha"], SHA)
            self.assertEqual((root / "accepted/oauth-connect-service").read_bytes(), executable())
            with self.assertRaises(b.Refused):
                b.unpack(path, root / "accepted", EXPECTED, OUTPUTS, lambda path: ELF)
            bundle(path, data=executable(b"corrupted"), value=manifest())
            with self.assertRaises(b.Refused):
                b.unpack(path, root / "bad-size", EXPECTED, OUTPUTS, lambda path: ELF)
            changed = manifest()
            changed["files"][0]["sha256"] = "0" * 64
            bundle(path, value=changed)
            with self.assertRaises(b.Refused):
                b.unpack(path, root / "bad-hash", EXPECTED, OUTPUTS, lambda path: ELF)

    def test_extra_traversal_duplicate_and_links_are_not_extracted(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for index, name in enumerate(("../../escape", "/absolute", "manifest.json", "foreign")):
                extra = tarfile.TarInfo(name)
                if index == 3:
                    extra.type, extra.linkname = tarfile.SYMTYPE, "../../escape"
                path = root / "bundle.tar.gz"
                bundle(path, extra=extra)
                with self.subTest(name=name), self.assertRaises(b.Refused):
                    b.unpack(path, root / str(index), EXPECTED, OUTPUTS, lambda path: ELF)
            self.assertFalse((root / "escape").exists())

    def test_missing_file_profile_override_unknown_field_and_malformed_tar_refused(self):
        for mutation in (lambda m: m["files"].clear(), lambda m: m.update(secret="bad"),
                         lambda m: m["files"][0].update(path="../../escape"),
                         lambda m: m["flags"]["environment"].update(RUSTFLAGS="-Cinstrument-coverage")):
            value = manifest()
            mutation(value)
            with self.assertRaises(b.Refused):
                b.validate_manifest(value, EXPECTED, OUTPUTS)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "bundle.tar.gz"
            path.write_bytes(b"invalid")
            with self.assertRaises(b.Refused):
                b.unpack(path, Path(directory) / "output", EXPECTED, OUTPUTS)


def run():
    return dict(id=789, run_attempt=1, workflow_id=456, path=".github/workflows/ci.yml", event="push",
                head_branch="master", head_sha=SHA, repository={"id": 123, "full_name": "owner/repo"},
                head_repository={"id": 123, "full_name": "owner/repo"})


def artifacts():
    return [dict(id=index + 1, name=f"k6-binaries-{role}-789-1", digest="sha256:" + "d" * 64,
                 expires_at="2030-01-01T00:00:00Z", expired=False, size_in_bytes=100,
                 workflow_run={"id": 789, "head_sha": SHA}) for index, role in enumerate(b.ROLES)]


class ArtifactTrustTests(unittest.TestCase):
    def test_exact_attempt_builds_succeed_even_when_overall_ci_is_not_complete(self):
        value = {**run(), "status": "in_progress", "conclusion": None}
        a.validate_run(value, "owner/repo", 123, 456, 789, 1, SHA)
        a.validate_jobs([dict(name=name, status="completed", conclusion="success") for name in b.ROLES.values()])
        self.assertEqual(set(a.select_artifacts(artifacts(), 789, 1, SHA)), set(b.ROLES))

    def test_foreign_pr_fork_attempt_workflow_and_sha_are_refused(self):
        variants = [dict(event="pull_request"), dict(head_branch="feature"), dict(workflow_id=999),
                    dict(run_attempt=2), dict(id=999), dict(head_sha="e" * 40), dict(path=".github/workflows/other.yml"),
                    dict(head_repository={"id": 999, "full_name": "foreign/fork"})]
        for changed in variants:
            with self.subTest(changed=changed), self.assertRaises(b.Refused):
                a.validate_run({**run(), **changed}, "owner/repo", 123, 456, 789, 1, SHA)

    def test_missing_failed_duplicate_and_running_build_jobs_are_refused(self):
        jobs = [dict(name=name, status="completed", conclusion="success") for name in b.ROLES.values()]
        for invalid in (jobs[:-1], jobs + [jobs[0]], [{**jobs[0], "conclusion": "failure"}, *jobs[1:]],
                        [{**jobs[0], "status": "in_progress"}, *jobs[1:]]):
            with self.assertRaises(b.Refused):
                a.validate_jobs(invalid)

    def test_expiration_corruption_attempt_mix_and_missing_digest_are_refused(self):
        changes = [dict(expired=True), dict(expires_at="2000-01-01T00:00:00Z"), dict(digest=None),
                   dict(digest="sha256:bad"), dict(name="k6-binaries-iam-789-2"),
                   dict(workflow_run={"id": 999, "head_sha": SHA}), dict(workflow_run={"id": 789, "head_sha": "f" * 40})]
        for change in changes:
            values = artifacts()
            values[0].update(change)
            with self.subTest(change=change), self.assertRaises(b.Refused):
                a.select_artifacts(values, 789, 1, SHA)
        for values in (artifacts()[:-1], artifacts() + [artifacts()[0]]):
            with self.assertRaises(b.Refused):
                a.select_artifacts(values, 789, 1, SHA)

    def test_closed_zip_is_not_a_general_archive_extractor(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for index, name in enumerate(("bundle.tar.gz", "../bundle.tar.gz", "foreign")):
                path = root / "archive.zip"
                with zipfile.ZipFile(path, "w") as archive:
                    archive.writestr(name, b"payload")
                if index == 0:
                    a.unwrap_zip(path, root / str(index))
                    self.assertEqual((root / str(index)).read_bytes(), b"payload")
                else:
                    with self.assertRaises(b.Refused):
                        a.unwrap_zip(path, root / str(index))

    def test_download_requires_api_archive_digest_before_unwrap(self):
        class Opener:
            def open(self, request, timeout):
                return io.BytesIO(b"actual archive")
        github = a.GitHub("owner/repo", "test-token-not-exported")
        github.opener = Opener()
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaises(b.Refused):
                github.download({"id": 1, "digest": "sha256:" + "0" * 64}, Path(directory) / "file")

    def test_signed_redirect_strips_api_authorization(self):
        import urllib.request
        request = urllib.request.Request("https://api.github.com/repos/owner/repo/actions/artifacts/1/zip",
                                         headers={"Authorization": "Bearer not-for-storage", "Cookie": "private"})
        redirected = a.SafeRedirect().redirect_request(request, None, 302, "", {},
                                                       "https://store.blob.core.windows.net/archive?signature=test")
        self.assertIsNone(redirected.get_header("Authorization"))
        self.assertIsNone(redirected.get_header("Cookie"))
        with self.assertRaises(b.Refused):
            a.SafeRedirect().redirect_request(request, None, 302, "", {}, "http://127.0.0.1/archive")


class PackagingTests(unittest.TestCase):
    def test_distinct_harness_and_target_keep_target_entrypoint_and_real_dev_profile(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            source, acquired = root / "source", root / "acquired"
            (source / "ops/deploy/build").mkdir(parents=True)
            (source / "services/IAMRusty").mkdir(parents=True)
            acquired.mkdir()
            (source / "Cargo.lock").write_text("target-lock")
            (source / "services/IAMRusty/docker-entrypoint.sh").write_text("#!/bin/sh\n# target revision\n")
            import shutil
            shutil.copyfile(b.ROOT / "ops/deploy/build/targets.json", source / "ops/deploy/build/targets.json")
            selected = dict(schema="aiforall.k6-selection/v1", **{key: EXPECTED[key] for key in
                            ("repository", "repository_id", "workflow_id", "run_id", "run_attempt", "source_sha")},
                            artifacts={role: {"id": index + 1} for index, role in enumerate(b.ROLES)})
            (acquired / "selection.json").write_text(json.dumps(selected))
            for role in b.ROLES:
                value = manifest()
                value.update(role=role, job_name=b.ROLES[role], cargo_lock_sha256=b.sha(source / "Cargo.lock"),
                             inventory_sha256=b.sha(source / "ops/deploy/build/targets.json"))
                value["files"] = []
                outputs = b.inventory(source, role)
                value["build_args"] = ["build", "--locked", "--all-targets"]
                with tarfile.open(acquired / (role + ".tar.gz"), "w:gz") as archive:
                    for package, name in outputs.items():
                        raw = executable()
                        value["files"].append(dict(package=package, name=name, path="bin/" + name,
                                                  size=len(raw), sha256=hashlib.sha256(raw).hexdigest(), elf=ELF))
                        value["build_args"] += ["--package", package]
                        member = tarfile.TarInfo("bin/" + name)
                        member.size = len(raw)
                        archive.addfile(member, io.BytesIO(raw))
                    raw = json.dumps(value).encode()
                    member = tarfile.TarInfo("manifest.json")
                    member.size = len(raw)
                    archive.addfile(member, io.BytesIO(raw))
            def unpack(*args):
                return b.unpack(*args, inspect=lambda path: ELF)
            with patch.object(p, "command", side_effect=[SHA, SDK, "e" * 40]), patch.object(p, "unpack", side_effect=unpack):
                result = p.prepare(acquired, source, root / "context")
            self.assertEqual(result["harness_sha"], "e" * 40)
            self.assertEqual(result["target_sha"], SHA)
            self.assertEqual(result["cargo_profile"], "dev")
            self.assertEqual((root / "context/services/IAMRusty/docker-entrypoint.sh").read_text(),
                             "#!/bin/sh\n# target revision\n")
            self.assertEqual((root / "context/release/source-revision").read_text().strip(), SHA)
            self.assertEqual(len(list((root / "context/release").iterdir())), 7)

    def test_runtime_and_packaging_are_compile_free(self):
        lock = p.runtime_lock()
        self.assertRegex(lock["runtime"], r"^ubuntu:24\.04@sha256:[a-f0-9]{64}$")
        dockerfile = (CI / "Dockerfile.artifacts").read_text()
        self.assertNotIn("RUN ", dockerfile)
        workflow = (b.ROOT / ".github/workflows/k6-images.yml").read_text()
        self.assertNotIn("cargo build", workflow)
        self.assertNotIn("cargo test", workflow)
        self.assertIn("workflow_dispatch:", workflow)
        self.assertNotIn("workflow_run:", workflow)
        self.assertNotIn("pull_request_target", workflow)

    def test_unresolved_runtime_dependencies_are_not_a_pass(self):
        packager = object.__new__(p.Packager)
        packager.owned_containers = set()
        packager.record = lambda *args, **kwargs: None
        packager.retire = lambda identity: packager.owned_containers.remove(identity)
        with patch.dict(os.environ, {"GITHUB_RUN_ID": "789", "GITHUB_RUN_ATTEMPT": "1"}), \
                patch.object(p, "command", side_effect=["a" * 64, "libssl.so.3 => not found"]):
            with self.assertRaises(b.Refused):
                packager.abi("local/image:run", ["/app/service"])
        self.assertFalse(packager.owned_containers)


if __name__ == "__main__":
    unittest.main()
