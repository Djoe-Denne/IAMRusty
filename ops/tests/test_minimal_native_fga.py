"""Native FGA Jobs and Windows host boundary, never a real cluster/server."""
import base64
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

import yaml

from ops.tests import test_minimal_native_openbao as bao

ROOT = Path(__file__).resolve().parents[2]
CALLER = ROOT / "ops/deploy/kind-local-fga.ps1"
STORE = "01H0H015178Y2V4CX10C2KGHF4"
MODEL = "01GXSA8YR785C4FYS3C0RTG7B1"
IMAGE = "openfga/cli:v0.8.2@sha256:31a565d14d1c5ca8170d3a2225eb66517d9f490539967bcb3421613c5e2538a4"


class NativeFgaDeclarationsTests(unittest.TestCase):
    render = bao.NativeOpenBaoDeclarationsTests.render

    def job(self, flag):
        objects = self.render(flag)
        self.assertEqual(len(objects), 1)
        self.assertEqual(objects[0]["kind"], "Job")
        job = objects[0]
        self.assertEqual(job["spec"]["backoffLimit"], 0)
        self.assertNotIn("ttlSecondsAfterFinished", job["spec"], "retain ambiguous native results for explicit operator recovery")
        pod = job["spec"]["template"]["spec"]
        self.assertEqual(pod["restartPolicy"], "Never")
        self.assertFalse(pod["automountServiceAccountToken"])
        self.assertNotIn("serviceAccountName", pod)
        self.assertEqual(len(pod["containers"]), 1)
        container = pod["containers"][0]
        self.assertEqual(container["command"], ["/fga"])
        self.assertEqual(container["image"], IMAGE)
        env = {item["name"]: item["value"] for item in container["env"]}
        self.assertEqual(env["FGA_API_URL"], "https://openfga.aiforall-local-full.svc.cluster.local:8080")
        self.assertEqual(env["SSL_CERT_FILE"], "/ca/ca.crt")
        self.assertFalse(any("TOKEN" in name or "SECRET" in name for name in env))
        self.assertTrue(container["securityContext"]["readOnlyRootFilesystem"])
        return job, pod, container

    def test_readonly_inspection_lists_without_name_selection_or_model_mutation(self):
        job, pod, container = self.job("openfga.inspect=true")
        self.assertEqual(job["metadata"]["name"], "kind-local-fga-inspect")
        self.assertEqual(container["args"], ["store", "list", "--max-pages=20"])
        self.assertFalse(any(volume["name"] == "model" for volume in pod["volumes"]))

    def test_explicit_creation_uses_native_dsl_and_real_nested_output_not_a_publisher_sidecar(self):
        job, pod, container = self.job("openfga.initialize=true")
        self.assertEqual(job["metadata"]["name"], "kind-local-fga-initialize")
        self.assertEqual(container["args"], ["store", "create", "--name", "aiforall-local-full", "--model", "/model/model.fga", "--format", "fga"])
        model = next(volume for volume in pod["volumes"] if volume["name"] == "model")
        self.assertEqual(model["configMap"]["name"], "kind-local-fga-model")
        mount = next(mount for mount in container["volumeMounts"] if mount["name"] == "model")
        self.assertTrue(mount["readOnly"])


STUB = r'''
using System;
using System.Collections.Generic;
using System.IO;
using System.Text;
using System.Web.Script.Serialization;
public class KubectlStub {
    public static int Main(string[] args) {
        Console.OutputEncoding = new UTF8Encoding(false);
        string input = Console.In.ReadToEnd();
        var json = new JavaScriptSerializer();
        File.AppendAllText(Environment.GetEnvironmentVariable("FAKE_CALLS"), json.Serialize(new { args = args, input = input }) + "\n", new UTF8Encoding(false));
        string mode = Environment.GetEnvironmentVariable("FAKE_MODE");
        var command = new List<string>(args).GetRange(4, args.Length - 4);
        if (command[0] == "get" && command[1] == "configmap") {
            if (mode == "read-error") { Console.Error.WriteLine("fake-credential-private-sentinel"); return 1; }
            if (mode == "existing") { Console.WriteLine("{\"data\":{\"store-id\":\"01H0H015178Y2V4CX10C2KGHF4\",\"model-id\":\"01GXSA8YR785C4FYS3C0RTG7B1\",\"extra\":\"keep\"}}"); }
            if (mode == "incomplete") Console.WriteLine("{\"data\":{\"store-id\":\"old\"}}");
            return 0;
        }
        if (command[0] == "get" && command[1] == "job") {
            if (mode == "existing-mutator") Console.WriteLine("{\"metadata\":{\"name\":\"kind-local-fga-initialize\"}}");
            return 0;
        }
        if (command[0] == "delete") {
            if (command[1] != "job" || command[2] != "kind-local-fga-inspect") return 97;
            return 0;
        }
        if (command[0] == "create") {
            if (command[1] == "configmap" && command[2] == "local-full-fga" && mode == "collision") { Console.Error.WriteLine("AlreadyExists"); return 1; }
            return 0;
        }
        if (command[0] == "wait") {
            if (mode == "inspect-failed" && command.Contains("job/kind-local-fga-inspect")) return 1;
            if (mode == "create-failed" && command.Contains("job/kind-local-fga-initialize")) return 1;
            return 0;
        }
        if (command[0] == "logs") {
            if (command[1] == "job/kind-local-fga-inspect") {
                if (mode == "stores-present") Console.WriteLine("{\"stores\":[{\"id\":\"old-store\"}]}");
                else if (mode == "bad-inspect") Console.WriteLine("{\"stores\":null}");
                else Console.WriteLine("{\"stores\":[]}");
                return 0;
            }
            if (mode == "mixed-create") Console.WriteLine("CLI diagnostic before JSON");
            if (mode == "incomplete-result") Console.WriteLine("{\"store\":{\"id\":\"01H0H015178Y2V4CX10C2KGHF4\"}}");
            else Console.WriteLine("{\"store\":{\"id\":\"01H0H015178Y2V4CX10C2KGHF4\"},\"model\":{\"authorization_model_id\":\"01GXSA8YR785C4FYS3C0RTG7B1\"}}");
            if (mode == "trailing-create") Console.WriteLine("CLI diagnostic after JSON");
            if (mode == "two-json") Console.WriteLine("{\"store\":{\"id\":\"wrong\"},\"model\":{\"authorization_model_id\":\"wrong\"}}");
            return 0;
        }
        return 98;
    }
}
'''


@unittest.skipUnless(os.name == "nt", "Windows PowerShell native process boundary")
class NativeFgaCallerTests(unittest.TestCase):
    fixture = None

    @classmethod
    def tearDownClass(cls):
        if cls.fixture is not None:
            cls.fixture.cleanup()

    def tools(self):
        self.assertTrue(CALLER.is_file(), "native FGA caller is not implemented")
        ps = shutil.which("powershell.exe")
        helm = os.environ.get("AIFORALL_HELM") or shutil.which("helm")
        if not ps or not helm:
            self.skipTest("qualified native PowerShell/Helm unavailable (NOT RUN)")
        if type(self).fixture is None:
            type(self).fixture = tempfile.TemporaryDirectory(dir=ROOT / ".tmp")
            path = Path(self.fixture.name)
            (path / "stub.cs").write_text(STUB, encoding="utf-8")
            compile_command = "Add-Type -Path $env:FAKE_SOURCE -OutputAssembly $env:FAKE_EXE -OutputType ConsoleApplication -ReferencedAssemblies System.Web.Extensions.dll"
            result = subprocess.run([ps, "-NoProfile", "-NonInteractive", "-Command", compile_command],
                env=dict(os.environ, FAKE_SOURCE=str(path / "stub.cs"), FAKE_EXE=str(path / "kubectl.exe")),
                capture_output=True, encoding="oem", timeout=30, check=False)
            self.assertEqual(result.returncode, 0, result.stderr)
        return ps, helm, Path(self.fixture.name) / "kubectl.exe"

    def invoke(self, mode, *, initialize=True, missing_helm=False):
        ps, helm, kubectl = self.tools()
        with tempfile.TemporaryDirectory(dir=ROOT / ".tmp") as directory:
            calls_path = Path(directory) / "calls.jsonl"
            command = "$ErrorActionPreference='Stop'; & $env:FAKE_CALLER -Kubectl $env:FAKE_EXE -Helm $env:FAKE_HELM " + ("-InitializeEmpty " if initialize else "") + "| ConvertTo-Json -Compress"
            result = subprocess.run([ps, "-NoProfile", "-NonInteractive", "-EncodedCommand", base64.b64encode(command.encode("utf-16-le")).decode("ascii")],
                cwd=ROOT, env=dict(os.environ, PATHEXT=".COM;.EXE;.BAT;.CMD", FAKE_CALLER=str(CALLER), FAKE_EXE=str(kubectl),
                    FAKE_HELM="not-an-installed-helm.exe" if missing_helm else helm, FAKE_MODE=mode, FAKE_CALLS=str(calls_path)),
                capture_output=True, encoding="oem", timeout=90, check=False)
            calls = [json.loads(line) for line in calls_path.read_text().splitlines()] if calls_path.exists() else []
            for row in calls:
                self.assertEqual(row["args"][:4], ["--context", "kind-aiforall-local-full", "--namespace", "aiforall-local-full"])
            return result, calls

    def test_existing_complete_references_reuse_without_helm_job_or_any_write(self):
        result, calls = self.invoke("existing", initialize=False, missing_helm=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout), {"store-id": STORE, "model-id": MODEL})
        self.assertEqual(len(calls), 1)

    def test_absence_needs_explicit_first_install_and_read_errors_or_partial_refs_do_not_initialize(self):
        for mode, initialize in (("empty", False), ("read-error", True), ("incomplete", True)):
            with self.subTest(mode=mode):
                result, calls = self.invoke(mode, initialize=initialize)
                self.assertNotEqual(result.returncode, 0)
                self.assertFalse(any(row["args"][4] in {"create", "delete", "wait", "logs"} for row in calls))
                self.assertNotIn("fake-credential-private-sentinel", result.stdout + result.stderr)

    def test_new_empty_install_runs_readonly_then_native_create_and_publishes_both_real_ids(self):
        result, calls = self.invoke("empty")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout), {"store-id": STORE, "model-id": MODEL})
        jobs = [yaml.safe_load(row["input"]) for row in calls if row["args"][4:6] == ["create", "-f"]]
        self.assertEqual([job["metadata"]["name"] for job in jobs], ["kind-local-fga-inspect", "kind-local-fga-initialize"])
        publication = next(row for row in calls if row["args"][4:7] == ["create", "configmap", "local-full-fga"])
        self.assertIn("--from-literal=store-id=" + STORE, publication["args"])
        self.assertIn("--from-literal=model-id=" + MODEL, publication["args"])
        model = next(row for row in calls if row["args"][4:7] == ["create", "configmap", "kind-local-fga-model"])
        self.assertTrue(any(arg.startswith("--from-file=model.fga=") and arg.replace("\\", "/").endswith("/ops/openfga/model.fga") for arg in model["args"]))
        self.assertFalse(any(row["args"][4:7] == ["delete", "job", "kind-local-fga-initialize"] for row in calls))

    def test_nonempty_or_failed_inspection_and_existing_mutator_never_create_another_store(self):
        for mode in ("stores-present", "inspect-failed", "bad-inspect", "existing-mutator"):
            with self.subTest(mode=mode):
                result, calls = self.invoke(mode)
                self.assertNotEqual(result.returncode, 0)
                self.assertFalse(any("kind-local-fga-initialize" in row["input"] for row in calls))
                self.assertFalse(any(row["args"][4:7] == ["create", "configmap", "local-full-fga"] for row in calls))

    def test_creation_failure_mixed_or_incomplete_results_and_publication_collision_are_partial_errors_not_retries(self):
        for mode in ("create-failed", "mixed-create", "trailing-create", "two-json", "incomplete-result", "collision"):
            with self.subTest(mode=mode):
                result, calls = self.invoke(mode)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("explicit operator recovery", result.stderr)
                self.assertEqual(sum("kind-local-fga-initialize" in row["input"] for row in calls), 1)
                publications = [row for row in calls if row["args"][4:7] == ["create", "configmap", "local-full-fga"]]
                self.assertEqual(len(publications), 1 if mode == "collision" else 0)


if __name__ == "__main__":
    unittest.main()
