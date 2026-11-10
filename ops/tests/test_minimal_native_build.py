"""Offline regression gates for the minimal native Linux delivery recipe."""
from pathlib import Path
import base64
import json
import os
import shlex
import shutil
import subprocess
import sys
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "ops/deploy/kind-local-images.ps1"


class MinimalNativeBuildTests(unittest.TestCase):
    def test_dockerfile_does_not_execute_python_artifact_collector(self):
        recipe = (ROOT / "Dockerfile.build").read_text(encoding="utf-8")
        self.assertNotIn("compile-artifacts.py", recipe)

    def test_full_rebuild_does_not_reuse_a_persistent_cargo_target(self):
        recipe = (ROOT / "Dockerfile.build").read_text(encoding="utf-8")
        self.assertNotRegex(recipe, r"--mount=type=cache[^\r\n]*target=/usr/src/target")
        self.assertNotIn("ARG TARGET_CACHE", recipe)

    def test_direct_context_excludes_private_key_material(self):
        ignored = set((ROOT / "Dockerfile.build.dockerignore").read_text(encoding="utf-8").splitlines())
        self.assertTrue({"**/*.key", "**/*.p12", "**/*.pfx", "**/id_ed25519", "**/id_rsa"} <= ignored)

    def test_bake_declares_cargo_arguments_from_the_inventory(self):
        docker = shutil.which("docker")
        if not docker:
            self.skipTest("Docker CLI unavailable; offline Bake plan NOT RUN")
        reserved = {"inventory", "image_targets", "cargo_profile", "source_revision", "sdk_revision", "artifacts_ref"}
        env = {key: value for key, value in os.environ.items() if key.casefold() not in reserved}
        env["DOCKER_HOST"] = "tcp://127.0.0.1:1"
        result = subprocess.run([
            docker, "buildx", "bake", "-f", "ops/deploy/build/targets.json",
            "-f", "ops/deploy/build/docker-bake.hcl", "--print", "workspace",
        ], cwd=ROOT, env=env, capture_output=True, text=True, timeout=60, check=False)
        self.assertEqual(result.returncode, 0, result.stderr)
        producer = json.loads(result.stdout)["target"]["artifacts"]
        self.assertIn("CARGO_BUILD_ARGS", producer["args"])
        argv = shlex.split(producer["args"]["CARGO_BUILD_ARGS"])
        inventory = json.loads((ROOT / "ops/deploy/build/targets.json").read_text())["variable"]["inventory"]["default"]
        selected = [inventory["targets"][name] for name in inventory["groups"]["workspace"]]
        expected = {(package, binary) for target in selected for package, binary in target["outputs"].items()}
        actual = {(argv[index + 1], argv[index + 3]) for index, value in enumerate(argv)
                  if value == "-p" and argv[index + 2] == "--bin"}
        self.assertEqual(actual, expected)
        features = {feature for target in selected for feature in target["features"]}
        self.assertEqual(set(argv[argv.index("--features") + 1].split(",")), features)


class NativeImageRecipeTests(unittest.TestCase):
    def run_action(self, action, **faults):
        self.assertTrue(SCRIPT.is_file(), "native build/import recipe is not implemented")
        powershell = shutil.which("powershell.exe")
        if not powershell:
            self.skipTest("native Windows PowerShell unavailable (NOT RUN)")
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            fake = root / "tool.py"
            fake.write_text("""import json, os, sys
from pathlib import Path
tool, args = sys.argv[1], sys.argv[2:]
with Path(os.environ['FAKE_CALLS']).open('a') as log:
    log.write(json.dumps({'tool': tool, 'args': args, 'tag': os.environ.get('BUILD_TAG')}) + chr(10))
if tool == 'docker' and '--print' in args:
    code = int(os.environ.get('FAKE_PLAN_EXIT', '0'))
    if code: sys.exit(code)
    tag = os.environ['BUILD_TAG']
    print(json.dumps({'group': {'kind-local-images': {'targets': ['alpha', 'beta']}},
        'target': {'alpha': {'tags': ['demo-alpha:' + tag]}, 'beta': {'tags': ['demo-beta:' + tag]}}}))
    sys.exit(0)
sys.exit(int(os.environ.get('FAKE_BUILD_EXIT' if tool == 'docker' else 'FAKE_KIND_EXIT', '0')))
""", encoding="utf-8")
            for tool in ("docker", "kind"):
                (root / (tool + ".cmd")).write_text(
                    '@echo off\n"' + sys.executable + '" "' + str(fake) + '" ' + tool + ' %*\nexit /b %errorlevel%\n',
                    encoding="utf-8")
            calls = root / "calls.jsonl"
            env = {key: value for key, value in os.environ.items() if not key.startswith("FAKE_")}
            env.update(PATH=str(root), PATHEXT=".COM;.EXE;.BAT;.CMD",
                       FAKE_BIN=str(root), FAKE_CALLS=str(calls))
            env.update(faults)
            # Functions deliberately intercept the Windows executable names.
            # Docker Desktop also installs an extensionless Unix shell wrapper;
            # PATH-only .cmd fixtures must never fall through to a real builder.
            quoted_script = str(SCRIPT).replace("'", "''")
            command = """$ErrorActionPreference = 'Stop'
function docker.exe {
    & (Join-Path $env:FAKE_BIN 'docker.cmd') @args
    $global:LASTEXITCODE = $LASTEXITCODE
}
function kind.exe {
    & (Join-Path $env:FAKE_BIN 'kind.cmd') @args
    $global:LASTEXITCODE = $LASTEXITCODE
}
& '""" + quoted_script + "' -Action " + action + " -BuildTag native-test; exit $LASTEXITCODE"
            result = subprocess.run([
                powershell, "-NoProfile", "-NonInteractive", "-EncodedCommand",
                base64.b64encode(command.encode("utf-16-le")).decode("ascii"),
            ], cwd=ROOT, env=env, capture_output=True, text=True, encoding="oem", timeout=30, check=False)
            records = [json.loads(line) for line in calls.read_text().splitlines()] if calls.exists() else []
            return result, records

    def test_build_is_one_native_bake_with_a_common_tag(self):
        result, calls = self.run_action("build")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(len(calls), 1)
        self.assertEqual(calls[0]["tool"], "docker")
        self.assertEqual(calls[0]["tag"], "native-test")
        self.assertIn("--no-cache", calls[0]["args"])
        self.assertIn("--load", calls[0]["args"])
        self.assertEqual(calls[0]["args"][-1], "kind-local-images")

    def test_import_reads_tags_from_the_native_plan(self):
        result, calls = self.run_action("import")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual([call["tool"] for call in calls], ["docker", "kind"])
        self.assertIn("--print", calls[0]["args"])
        self.assertEqual(calls[1]["args"], ["load", "docker-image", "demo-alpha:native-test",
                                             "demo-beta:native-test", "--name", "aiforall-local-full"])

    def test_failed_build_keeps_the_native_exit_code(self):
        result, calls = self.run_action("build", FAKE_BUILD_EXIT="23")
        self.assertEqual(result.returncode, 23, result.stderr)
        self.assertEqual(len(calls), 1)

    def test_failed_plan_never_calls_kind(self):
        result, calls = self.run_action("import", FAKE_PLAN_EXIT="19")
        self.assertEqual(result.returncode, 19, result.stderr)
        self.assertEqual([call["tool"] for call in calls], ["docker"])

    def test_failed_kind_load_keeps_the_native_exit_code(self):
        result, calls = self.run_action("import", FAKE_KIND_EXIT="22")
        self.assertEqual(result.returncode, 22, result.stderr)
        self.assertEqual(len(calls), 2)


if __name__ == "__main__":
    unittest.main()
