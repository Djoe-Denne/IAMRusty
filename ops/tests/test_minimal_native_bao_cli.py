"""Native process boundary of Bao init/unseal; generated CLI stub, no server."""
import base64
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
MODULE = ROOT / "ops/deploy/openbao-cli.psm1"
PUBLIC = ROOT / "ops/deploy/charts/local-full/files/openbao/bootstrap.json"
NEW_BUNDLE = {"unseal_keys_b64": ["ZmFrZS11bnNlYWwtc2VudGluZWw="],
              "unseal_shares": 1, "unseal_threshold": 1,
              "root_token": "fake-root-private-sentinel"}

STUB = r'''
using System;
using System.Collections.Generic;
using System.IO;
using System.Text;
using System.Web.Script.Serialization;
public class BaoStub {
    public static int Main(string[] args) {
        Console.OutputEncoding = new UTF8Encoding(false);
        string input = Console.In.ReadToEnd();
        var json = new JavaScriptSerializer();
        var row = new Dictionary<string, object> { {"args", args}, {"input", input} };
        foreach (string name in new [] {"BAO_TOKEN", "BAO_TOKEN_PATH", "BAO_CONFIG_PATH", "BAO_SKIP_VERIFY", "BAO_DISABLE_REDIRECTS", "BAO_CACERT", "BAO_TLS_SERVER_NAME", "BAO_MAX_RETRIES"})
            row[name] = Environment.GetEnvironmentVariable(name);
        File.AppendAllText(Environment.GetEnvironmentVariable("FAKE_CALLS"), json.Serialize(row) + "\n", new UTF8Encoding(false));
        string statePath = Environment.GetEnvironmentVariable("FAKE_STATE");
        string state = File.ReadAllText(statePath);
        if (args[0] == "version") { Console.WriteLine("OpenBao v2.6.2"); return 0; }
        if (args[0] == "status") {
            if (state == "error") { Console.Error.WriteLine("fake-root-private-sentinel"); return 1; }
            Console.WriteLine("{\"initialized\":" + (state == "new" ? "false" : "true") + ",\"sealed\":" + (state == "open" ? "false" : "true") + "}");
            return state == "open" ? 0 : 2;
        }
        if (args[0] == "operator" && args[1] == "init") {
            File.WriteAllText(statePath, "sealed");
            Console.WriteLine("{\"unseal_keys_b64\":[\"ZmFrZS11bnNlYWwtc2VudGluZWw=\"],\"unseal_shares\":1,\"unseal_threshold\":1,\"root_token\":\"fake-root-private-sentinel\"}");
            return 0;
        }
        if (args[0] == "write" && args[1] == "sys/unseal" && args[2] == "-") {
            var body = json.Deserialize<Dictionary<string, string>>(input);
            if (body["key"] != "ZmFrZS11bnNlYWwtc2VudGluZWw=") return 1;
            File.WriteAllText(statePath, "open");
            Console.WriteLine("{\"initialized\":true,\"sealed\":false}");
            return 0;
        }
        string failure = Environment.GetEnvironmentVariable("FAKE_FAILURE") ?? "";
        string token = Environment.GetEnvironmentVariable("BAO_TOKEN") ?? "";
        string revoked = statePath + ".revoked";
        string dbPath = statePath + ".db";
        var db = File.Exists(dbPath) ? json.Deserialize<Dictionary<string, object>>(File.ReadAllText(dbPath)) : new Dictionary<string, object>();
        if (token == "fake-root-private-sentinel" && File.Exists(revoked) && failure != "root-still-valid") {
            Console.Error.WriteLine(failure == "revocation-network" ? "network error fake-root-private-sentinel" : "Error making API request.\nCode: 403. Errors:\n* permission denied");
            return 2;
        }
        if (token == "") { Console.Error.WriteLine("missing explicit token"); return 2; }
        if (args[0] == "secrets" && args[1] == "list") {
            var mounts = new Dictionary<string, object>();
            foreach (var entry in db) if (entry.Key.StartsWith("sys/mounts/")) mounts[entry.Key.Substring(11) + "/"] = entry.Value;
            if (failure == "mount-conflict") mounts["secret/"] = new { type = "kv", options = new { version = "1" } };
            Console.WriteLine(json.Serialize(mounts)); return 0;
        }
        if (args[0] == "auth" && args[1] == "list") {
            if (failure == "auth-list-error") { Console.Error.WriteLine("Code: 403. Errors:"); return 2; }
            var auth = new Dictionary<string, object>();
            foreach (var entry in db) if (entry.Key.StartsWith("sys/auth/")) auth[entry.Key.Substring(9) + "/"] = entry.Value;
            Console.WriteLine(json.Serialize(auth)); return 0;
        }
        if (args[0] == "read") {
            string key = args[args.Length - 1];
            if (failure == "metadata-error" && key.StartsWith("secret/metadata/")) { Console.Error.WriteLine("Code: 403. Errors:"); return 2; }
            if (failure == "role-conflict" && key.StartsWith("auth/kubernetes/role/")) { Console.WriteLine("{\"data\":{\"bound_service_account_names\":[\"*\"]}}"); return 0; }
            if (failure == "auth-config-conflict" && key == "auth/kubernetes/config") { Console.WriteLine("{\"data\":{\"kubernetes_host\":\"https://wrong.example\"}}"); return 0; }
            if (failure == "policy-read-error" && key.StartsWith("sys/policies/acl/")) { Console.Error.WriteLine("Code: 403. Errors:\nfake-root-private-sentinel"); return 2; }
            if (!db.ContainsKey(key)) { Console.Error.WriteLine("No value found at " + key); return 2; }
            Console.WriteLine(json.Serialize(new { data = db[key] })); return 0;
        }
        if (args[0] == "write") {
            if (failure == "write-error") { Console.Error.WriteLine("fake-root-private-sentinel"); return 2; }
            string key = args[1];
            if (key.StartsWith("secret/data/")) {
                if (failure == "seed-cas-conflict") { Console.Error.WriteLine("Code: 400. Errors:\ncheck-and-set parameter did not match"); return 2; }
                db["secret/metadata/" + key.Substring(12)] = new { current_version = 1 };
            }
            db[key] = json.DeserializeObject(input);
            File.WriteAllText(dbPath, json.Serialize(db));
            return 0;
        }
        if (args[0] == "token" && args[1] == "create") {
            if (failure == "token-error") { Console.Write("{\"partial\":\"fake-product-private-sentinel\""); Console.Error.WriteLine("fake-root-private-sentinel"); return 2; }
            Console.WriteLine("{\"auth\":{\"client_token\":\"fake-product-private-sentinel\",\"policies\":[\"local-full-lazaret\"],\"lease_duration\":28800,\"renewable\":false,\"orphan\":true}}"); return 0;
        }
        if (args[0] == "token" && args[1] == "lookup") {
            if (token == "fake-product-private-sentinel") {
                if (failure == "product-invalid") { Console.Error.WriteLine("Code: 403. Errors:"); return 2; }
                object ttl = failure == "token-ttl-string" ? (object)"fake-product-private-sentinel" : 28790;
                Console.WriteLine(json.Serialize(new { data = new { policies = failure == "token-root-policy" ? new [] {"local-full-lazaret", "root"} : new [] {"local-full-lazaret"}, ttl = ttl, explicit_max_ttl = failure == "token-unbounded" ? 0 : 28800, renewable = failure == "token-renewable", orphan = failure != "token-not-orphan" } }));
            } else Console.WriteLine("{\"data\":{\"policies\":[\"root\"]}}");
            return 0;
        }
        if (args[0] == "token" && args[1] == "revoke") {
            if (failure == "revoke-error") { Console.Error.WriteLine("fake-root-private-sentinel"); return 2; }
            File.WriteAllText(revoked, "yes"); return 0;
        }
        return 99;
    }
}
'''


@unittest.skipUnless(os.name == "nt", "Windows PowerShell native process boundary")
class NativeBaoCliTests(unittest.TestCase):
    fixture = None

    @classmethod
    def tearDownClass(cls):
        if cls.fixture is not None:
            cls.fixture.cleanup()

    def tools(self):
        self.assertTrue(MODULE.is_file(), "native Bao primitives are not implemented")
        powershell = shutil.which("powershell.exe")
        if not powershell:
            self.skipTest("Windows PowerShell unavailable (NOT RUN)")
        if type(self).fixture is None:
            type(self).fixture = tempfile.TemporaryDirectory(dir=ROOT / ".tmp")
            path = Path(self.fixture.name)
            (path / "stub.cs").write_text(STUB, encoding="utf-8")
            command = "Add-Type -Path $env:FAKE_SOURCE -OutputAssembly $env:FAKE_EXE -OutputType ConsoleApplication -ReferencedAssemblies System.Web.Extensions.dll"
            result = subprocess.run([powershell, "-NoProfile", "-NonInteractive", "-Command", command],
                env=dict(os.environ, FAKE_SOURCE=str(path / "stub.cs"), FAKE_EXE=str(path / "bao.exe")),
                capture_output=True, encoding="oem", timeout=30, check=False)
            self.assertEqual(result.returncode, 0, result.stderr)
        return powershell, Path(self.fixture.name) / "bao.exe"

    def invoke(self, action, *, state="new", existing_output=None, bundle=None,
               failure="", existing_products=False, admin=False, twice=False,
               roles=(), seeds=None):
        powershell, executable = self.tools()
        with tempfile.TemporaryDirectory(dir=ROOT / ".tmp") as temporary:
            path = Path(temporary)
            (path / "state").write_text(state)
            (path / "ca.crt").write_text("public CA placeholder; native CLI stub only")
            output = path / "credentials/init.json"
            if existing_output is not None:
                output.parent.mkdir()
                output.write_bytes(existing_output)
            input_file = path / "unseal.json"
            if action == "bootstrap" and bundle is None:
                bundle = NEW_BUNDLE
            if bundle is not None:
                input_file.write_text(json.dumps(bundle))
            product_dir = path / "products"
            if existing_products:
                product_dir.mkdir()
                (product_dir / "sentinel").write_bytes(b"old-private-product")
            admin_file = path / "admin-token"
            if admin:
                admin_file.write_text("fake-admin-private-sentinel")
            seed_file = path / "seed.json"
            if seeds is not None:
                seed_file.write_text(json.dumps(seeds), encoding="utf-8")
            env = dict(os.environ, PATHEXT=".COM;.EXE;.BAT;.CMD", FAKE_EXE=str(executable),
                FAKE_MODULE=str(MODULE), FAKE_STATE=str(path / "state"), FAKE_CALLS=str(path / "calls.jsonl"),
                FAKE_CA=str(path / "ca.crt"), FAKE_OUTPUT=str(output), FAKE_INPUT=str(input_file),
                FAKE_FAILURE=failure, FAKE_PRODUCTS=str(product_dir), FAKE_PUBLIC=str(PUBLIC), FAKE_ADMIN=str(admin_file),
                FAKE_SEED=str(seed_file),
                BAO_TOKEN="ambient-token-must-not-be-used", VAULT_TOKEN="ambient-alias-must-not-be-used",
                BAO_SKIP_VERIFY="true", BAO_CONFIG_PATH="ambient-helper-must-not-be-used")
            init_call = """Initialize-KindOpenBao -Connection $connection -OutputFile $env:FAKE_OUTPUT
$acl = Get-Acl -LiteralPath $env:FAKE_OUTPUT
$sid = [Security.Principal.WindowsIdentity]::GetCurrent().User
$rules = @($acl.GetAccessRules($true,$true,[Security.Principal.SecurityIdentifier]))
[pscustomobject]@{protected=$acl.AreAccessRulesProtected; private=($rules.Count -eq 1 -and $rules[0].IdentityReference.Value -eq $sid.Value -and $rules[0].FileSystemRights -eq 'FullControl' -and -not $rules[0].IsInherited)} | ConvertTo-Json -Compress
"""
            bootstrap_call = "Invoke-KindOpenBaoBootstrap -Connection $connection -TokenConsumers lazaret -ProductOutputDirectory $env:FAKE_PRODUCTS " + (
                "-AdminTokenFile $env:FAKE_ADMIN" if admin else "-InitFile $env:FAKE_INPUT")
            if roles:
                bootstrap_call += " -KubernetesRoles " + ",".join(roles) + " -KubernetesCaFile $env:FAKE_CA"
            if seeds is not None:
                bootstrap_call += " -SeedFiles @{'ops/token'=$env:FAKE_SEED}"
            if twice:
                bootstrap_call += "; " + bootstrap_call.replace("$env:FAKE_PRODUCTS", "($env:FAKE_PRODUCTS+'-second')")
            bootstrap_call += """
$acl=Get-Acl -LiteralPath (Join-Path $env:FAKE_PRODUCTS 'lazaret.json')
$rules=@($acl.GetAccessRules($true,$true,[Security.Principal.SecurityIdentifier]))
[pscustomobject]@{protected=$acl.AreAccessRulesProtected;private=($rules.Count -eq 1 -and $rules[0].IdentityReference.Value -eq [Security.Principal.WindowsIdentity]::GetCurrent().User.Value -and -not $rules[0].IsInherited);ambient=$env:BAO_TOKEN} | ConvertTo-Json -Compress
"""
            call = {"status": "Get-KindOpenBaoStatus -Connection $connection | ConvertTo-Json -Compress",
                    "init": init_call,
                    "unseal": "Unlock-KindOpenBao -Connection $connection -InputFile $env:FAKE_INPUT | ConvertTo-Json -Compress",
                    "bootstrap": bootstrap_call}[action]
            command = "$ErrorActionPreference='Stop'; Import-Module $env:FAKE_MODULE -Force; $connection=@{Executable=$env:FAKE_EXE;Address='https://127.0.0.1:18444';CaFile=$env:FAKE_CA;ServerName='openbao.aiforall-local-full.svc.cluster.local'}; " + call
            result = subprocess.run([powershell, "-NoProfile", "-NonInteractive", "-EncodedCommand",
                base64.b64encode(command.encode("utf-16-le")).decode("ascii")],
                cwd=ROOT, env=env, capture_output=True, encoding="oem", timeout=60, check=False)
            calls = [json.loads(line) for line in (path / "calls.jsonl").read_text().splitlines()] if (path / "calls.jsonl").exists() else []
            contents = output.read_bytes() if output.exists() else None
            if action == "bootstrap":
                contents = {file.name: file.read_bytes() for file in product_dir.glob("*")}
            return result, calls, contents

    def test_init_uses_native_one_of_one_and_private_exclusive_output(self):
        result, calls, contents = self.invoke("init")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout), {"protected": True, "private": True})
        self.assertEqual(json.loads(contents)["unseal_threshold"], 1)
        init = next(row for row in calls if row["args"][:2] == ["operator", "init"])
        self.assertEqual(init["args"], ["operator", "init", "-format=json", "-key-shares=1", "-key-threshold=1"])
        self.assertNotIn("fake-root-private-sentinel", result.stdout + result.stderr)
        for row in calls:
            self.assertFalse(row["BAO_TOKEN"])
            self.assertEqual(row["BAO_TOKEN_PATH"], "NUL")
            self.assertEqual(row["BAO_CONFIG_PATH"], "NUL")
            self.assertEqual(row["BAO_SKIP_VERIFY"], "false")
            self.assertEqual(row["BAO_DISABLE_REDIRECTS"], "true")
            self.assertEqual(row["BAO_MAX_RETRIES"], "0")

    def test_existing_init_output_is_not_overwritten_or_reinitialized(self):
        result, calls, contents = self.invoke("init", existing_output=b"old-private-output")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(contents, b"old-private-output")
        self.assertFalse(any(row["args"][:2] == ["operator", "init"] for row in calls))

    def test_sealed_status_is_not_an_environment_failure(self):
        result, _, _ = self.invoke("status", state="sealed")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout), {"initialized": True, "sealed": True})

    def test_existing_backend_is_not_reinitialized(self):
        result, calls, contents = self.invoke("init", state="sealed")
        self.assertNotEqual(result.returncode, 0)
        self.assertIsNone(contents)
        self.assertFalse(any(row["args"][:2] == ["operator", "init"] for row in calls))

    def test_unseal_key_uses_json_stdin_never_argv(self):
        bundle = {"unseal_keys_b64": ["ZmFrZS11bnNlYWwtc2VudGluZWw="], "unseal_threshold": 1}
        result, calls, _ = self.invoke("unseal", state="sealed", bundle=bundle)
        self.assertEqual(result.returncode, 0, result.stderr)
        request = next(row for row in calls if row["args"][:2] == ["write", "sys/unseal"])
        self.assertEqual(request["args"], ["write", "sys/unseal", "-"])
        self.assertEqual(json.loads(request["input"]), {"key": bundle["unseal_keys_b64"][0]})
        self.assertNotIn(bundle["unseal_keys_b64"][0], str(request["args"]) + result.stdout + result.stderr)

    def test_unsealed_coffer_needs_no_key_file_and_is_not_mutated(self):
        result, calls, _ = self.invoke("unseal", state="open")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout), {"initialized": True, "sealed": False})
        self.assertFalse(any(row["args"][0] in {"write", "operator"} for row in calls))

    def test_invalid_private_bundle_is_redacted_before_unseal(self):
        bundle = {"unseal_keys_b64": ["ZmFrZS11bnNlYWwtc2VudGluZWw="],
                  "unseal_threshold": "fake-root-private-sentinel"}
        result, calls, _ = self.invoke("unseal", state="sealed", bundle=bundle)
        self.assertNotEqual(result.returncode, 0)
        self.assertNotIn("fake-root-private-sentinel", result.stdout + result.stderr)
        self.assertFalse(any(row["args"][0] == "write" for row in calls))

    def test_status_error_is_not_absence_or_init_and_diagnostic_is_redacted(self):
        result, calls, _ = self.invoke("init", state="error")
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(any(row["args"][:2] == ["operator", "init"] for row in calls))
        self.assertNotIn("fake-root-private-sentinel", result.stdout + result.stderr)

    def test_authenticated_bootstrap_creates_kv2_and_private_bounded_orphan_token_before_root_revoke(self):
        result, calls, products = self.invoke("bootstrap", state="open")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout), {"protected": True, "private": True, "ambient": "ambient-token-must-not-be-used"})
        self.assertEqual(set(products), {"lazaret.json"})
        self.assertEqual(json.loads(products["lazaret.json"])["auth"]["client_token"], "fake-product-private-sentinel")
        mount = next(row for row in calls if row["args"][:2] == ["write", "sys/mounts/secret"])
        self.assertEqual(json.loads(mount["input"]), {"type": "kv", "options": {"version": "2"}})
        issue = next(row for row in calls if row["args"][:2] == ["token", "create"])
        for flag in ("-orphan", "-no-default-policy", "-renewable=false", "-ttl=8h", "-explicit-max-ttl=8h", "-policy=local-full-lazaret"):
            self.assertIn(flag, issue["args"])
        revoke = next(i for i, row in enumerate(calls) if row["args"] == ["token", "revoke", "-self"])
        self.assertLess(calls.index(issue), revoke)
        self.assertTrue(any(row["args"][:2] == ["token", "lookup"] and row["BAO_TOKEN"] == NEW_BUNDLE["root_token"] for row in calls[revoke + 1:]))
        self.assertTrue(any(row["args"][:2] == ["token", "lookup"] and row["BAO_TOKEN"] == "fake-product-private-sentinel" for row in calls[revoke + 1:]))
        for row in calls:
            self.assertNotIn("private-sentinel", " ".join(row["args"]))
            self.assertNotIn("ambient-token-must-not-be-used", row["BAO_TOKEN"] or "")
        self.assertNotIn("private-sentinel", result.stdout + result.stderr)

    def test_existing_product_directory_refused_before_any_authenticated_mutation(self):
        result, calls, products = self.invoke("bootstrap", state="open", existing_products=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(products, {"sentinel": b"old-private-product"})
        self.assertFalse(any(row["args"][0] in {"write", "token"} for row in calls))

    def test_bootstrap_failure_preserves_partial_output_and_does_not_hide_active_root(self):
        for failure in ("write-error", "policy-read-error", "mount-conflict", "token-error", "product-invalid"):
            with self.subTest(failure=failure):
                result, calls, products = self.invoke("bootstrap", state="open", failure=failure)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("root may remain active", result.stderr)
                self.assertNotIn("private-sentinel", result.stdout + result.stderr)
                self.assertFalse(any(row["args"][:2] == ["token", "revoke"] for row in calls))
                if failure == "token-error":
                    self.assertIn(b"fake-product-private-sentinel", products["lazaret.json"])
                if failure in {"policy-read-error", "mount-conflict"}:
                    self.assertFalse(any(row["args"][:2] == ["write", "sys/policies/acl/local-full-lazaret"] for row in calls))

    def test_revocation_requires_explicit_denial_not_network_failure_or_live_root(self):
        for failure in ("revoke-error", "root-still-valid", "revocation-network"):
            with self.subTest(failure=failure):
                result, _, products = self.invoke("bootstrap", state="open", failure=failure)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("root may remain active", result.stderr)
                self.assertNotIn("private-sentinel", result.stdout + result.stderr)
                self.assertIn("lazaret.json", products)

    def test_explicit_admin_is_not_revoked_and_compatible_public_configuration_not_rewritten(self):
        result, calls, _ = self.invoke("bootstrap", state="open", admin=True, twice=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse(any(row["args"][:2] == ["token", "revoke"] for row in calls))
        self.assertEqual(sum(row["args"][:2] == ["write", "sys/mounts/secret"] for row in calls), 1)
        self.assertEqual(sum(row["args"][:2] == ["write", "sys/policies/acl/local-full-lazaret"] for row in calls), 1)
        self.assertEqual(sum(row["args"][:2] == ["token", "create"] for row in calls), 2)

    def test_selected_native_roles_transit_and_auth_are_created_once_not_all_roles(self):
        result, calls, _ = self.invoke("bootstrap", state="open", admin=True, twice=True, roles=("controller", "admit-sign"))
        self.assertEqual(result.returncode, 0, result.stderr)
        writes = [row for row in calls if row["args"][0] == "write"]
        for path in ("sys/mounts/transit", "transit/keys/apparatus-p4-cosign", "sys/auth/kubernetes", "auth/kubernetes/config", "auth/kubernetes/role/controller", "auth/kubernetes/role/admit-sign"):
            self.assertEqual(sum(row["args"][1] == path for row in writes), 1, path)
        self.assertFalse(any("role/github-connect" in row["args"] for row in calls))
        role = next(row for row in writes if row["args"][1] == "auth/kubernetes/role/controller")
        self.assertEqual(json.loads(role["input"]), {"bound_service_account_names": ["controller"], "bound_service_account_namespaces": ["aiforall-local-apparatus"], "token_policies": ["local-full-transit-verify"], "token_ttl": 900})
        config = next(row for row in writes if row["args"][1] == "auth/kubernetes/config")
        self.assertNotIn("/kube-ca/ca.crt", config["input"])
        self.assertFalse(json.loads(config["input"])["disable_local_ca_jwt"])

    def test_private_seed_uses_json_stdin_cas0_once_and_existing_value_is_not_reseeded(self):
        seed = {"token": "fake-seed-private-sentinel"}
        result, calls, _ = self.invoke("bootstrap", state="open", admin=True, twice=True, seeds=seed)
        self.assertEqual(result.returncode, 0, result.stderr)
        writes = [row for row in calls if row["args"][:2] == ["write", "secret/data/ops/token"]]
        self.assertEqual(len(writes), 1)
        self.assertEqual(json.loads(writes[0]["input"]), {"options": {"cas": 0}, "data": seed})
        self.assertNotIn(seed["token"], result.stdout + result.stderr + " ".join(writes[0]["args"]))

    def test_seed_errors_and_incompatible_auth_are_not_interpreted_as_missing(self):
        for failure in ("metadata-error", "seed-cas-conflict", "auth-list-error", "role-conflict", "auth-config-conflict"):
            with self.subTest(failure=failure):
                result, calls, _ = self.invoke("bootstrap", state="open", failure=failure, seeds={"token": "fake-seed-private-sentinel"}, roles=("controller",))
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("root may remain active", result.stderr)
                self.assertNotIn("private-sentinel", result.stdout + result.stderr)
                self.assertFalse(any(row["args"][:2] == ["token", "revoke"] for row in calls))
                if failure == "metadata-error":
                    self.assertFalse(any(row["args"][:2] == ["write", "secret/data/ops/token"] for row in calls))
                if failure == "role-conflict":
                    self.assertFalse(any(row["args"][:2] == ["write", "auth/kubernetes/role/controller"] for row in calls))

    def test_invalid_product_token_scope_or_lifetime_refuses_root_handback(self):
        for failure in ("token-root-policy", "token-unbounded", "token-renewable", "token-not-orphan", "token-ttl-string"):
            with self.subTest(failure=failure):
                result, calls, products = self.invoke("bootstrap", state="open", failure=failure)
                self.assertNotEqual(result.returncode, 0)
                self.assertNotIn("private-sentinel", result.stdout + result.stderr)
                self.assertFalse(any(row["args"][:2] == ["token", "revoke"] for row in calls))
                self.assertIn("lazaret.json", products)


if __name__ == "__main__":
    unittest.main()
