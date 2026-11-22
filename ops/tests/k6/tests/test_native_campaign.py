"""Offline checks for native fixtures; none of these qualify scenario parity."""
import copy
import json
import os
import re
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[4]
NATIVE = ROOT / "ops/tests/k6/native"
HELM = Path(os.environ.get("AIFORALL_HELM") or shutil.which("helm") or
            ROOT / ".tmp/ops-iac-tools/helm-v4.3.1/windows-amd64/helm.exe")
K6 = Path("C:/Program Files/k6/k6.exe")


def render(values):
    with tempfile.TemporaryDirectory(prefix="k6-native-render-") as directory:
        path = Path(directory) / "values.json"
        path.write_text(json.dumps(values), encoding="utf-8")
        result = subprocess.run([str(HELM), "template", "native", str(NATIVE / "chart"),
                                 "-f", str(path)], capture_output=True, text=True, timeout=30)
        return result


@unittest.skipUnless(HELM.is_file(), "explicit existing Helm required")
class FixtureRenderTests(unittest.TestCase):
    def setUp(self):
        self.envoy = {
            "admin": {"address": {"socket_address": {"address": "127.0.0.1", "port_value": 9901}}},
            "static_resources": {"listeners": [{"name": "closed-listener", "filters": ["ext_authz"]}],
                                 "clusters": [{"name": "iam", "transport_socket": {"sni": "iam-authority"},
                                               "load_assignment": {"endpoints": [{"lb_endpoints": [{"endpoint": {
                                                   "address": {"socket_address": {
                                                       "address": "iam.aiforall-local-full.svc.cluster.local",
                                                       "port_value": 8443}}}}]}]}}]},
        }
        self.values = dict(name="k6-" + "a" * 32, runId="a" * 32,
                           images={role: "fixture@sha256:" + "b" * 64 for role in ("iam", "oauth", "envoy", "idp")},
                           envoySource=json.dumps(self.envoy), simulatorSource="# protocol fixture\n")

    def test_only_iam_network_destination_changes(self):
        result = render(self.values)
        self.assertEqual(result.returncode, 0, result.stderr)
        objects = json.loads(result.stdout[result.stdout.index("{"):])["items"]
        self.assertEqual(len(objects), 10)
        loaded = json.loads(objects[0]["data"]["envoy.json"])
        expected = copy.deepcopy(self.envoy)
        expected["static_resources"]["clusters"][0]["load_assignment"]["endpoints"][0]["lb_endpoints"][0]["endpoint"]["address"]["socket_address"]["address"] = self.values["name"] + "-iam.aiforall-local-full.svc.cluster.local"
        self.assertEqual(loaded, expected)
        deployments = [obj for obj in objects if obj["kind"] == "Deployment"]
        self.assertEqual(len(deployments), 4)
        for obj in deployments:
            pod = obj["spec"]["template"]["spec"]
            container = pod["containers"][0]
            self.assertFalse(pod["automountServiceAccountToken"])
            self.assertTrue(pod["securityContext"]["runAsNonRoot"])
            self.assertIn("tcpSocket", container["readinessProbe"])
            self.assertNotIn("httpGet", container["readinessProbe"])
            self.assertNotIn("hostPort", container["ports"][0])
            for volume in pod["volumes"]:
                self.assertNotIn("hostPath", volume)

    def test_foreign_destination_dynamic_config_and_exposed_admin_refused(self):
        variants = []
        exposed = copy.deepcopy(self.envoy)
        exposed["admin"]["address"]["socket_address"]["address"] = "0.0.0.0"
        variants.append(exposed)
        dynamic = copy.deepcopy(self.envoy)
        dynamic["dynamic_resources"] = {}
        variants.append(dynamic)
        missing = copy.deepcopy(self.envoy)
        missing["static_resources"]["clusters"] = []
        variants.append(missing)
        multiple = copy.deepcopy(self.envoy)
        multiple["static_resources"]["clusters"] *= 2
        variants.append(multiple)
        foreign = copy.deepcopy(self.envoy)
        foreign["static_resources"]["clusters"][0]["load_assignment"]["endpoints"][0]["lb_endpoints"][0]["endpoint"]["address"]["socket_address"]["address"] = "foreign.invalid"
        variants.append(foreign)
        for source in variants:
            with self.subTest(source=source):
                self.assertNotEqual(render({**self.values, "envoySource": json.dumps(source)}).returncode, 0)


@unittest.skipUnless(os.name == "nt", "Windows PowerShell semantics")
class NativePowerShellTests(unittest.TestCase):
    def ps(self, source):
        result = subprocess.run(["powershell.exe", "-NoProfile", "-NonInteractive", "-Command", source],
                                capture_output=True, text=True, timeout=30)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        return result.stdout

    def test_powershell_sources_parse_without_execution(self):
        for file in NATIVE.glob("*.ps1"):
            self.ps("$e=$null;$t=$null;$null=[Management.Automation.Language.Parser]::ParseFile('" +
                    str(file).replace("'", "''") + "',[ref]$t,[ref]$e);if($e.Count){throw 'AST errors'}")

    def test_link_summary_is_closed_and_phase_bound(self):
        self.ps("""
$ErrorActionPreference='Stop'
. 'WINDOWS'
. 'AUTH'
foreach($phase in @('setup','email','identity')){
  $case='auth-link-conflict-'+$phase
  $e=Get-K6AuthExpectation $case
  $requests=if($phase -ceq 'setup'){8}else{3}
  $checkCount=if($phase -ceq 'setup'){9}else{6}
  if($e.Requests -ne $requests -or $e.Checks.Count -ne $checkCount){throw 'Wrong phase contract'}
  $checks=@{};foreach($name in $e.Checks){$checks[$name]=@{passes=1;fails=0}}
  $v=[pscustomobject]@{version=1;run_id=('a'*32);phase='HTTP';cases=@($case);expected=1;started=1;completed=1;iterations=1;requests=$e.Requests;checks=[pscustomobject]$checks}
  $v=ConvertFrom-Json ($v|ConvertTo-Json -Depth 10)
  Assert-K6AuthSummary $v ('a'*32) $case 0
  $v.requests++
  $caught=$false;try{Assert-K6AuthSummary $v ('a'*32) $case 0}catch{$caught=$true}
  if(-not $caught){throw 'Extra traffic accepted'}
  $v.requests=$e.Requests
  $v.checks.($e.Checks[0]).fails=1
  $caught=$false;try{Assert-K6AuthSummary $v ('a'*32) $case 0}catch{$caught=$true}
  if(-not $caught){throw 'Failed named check accepted'}
}
""".replace("WINDOWS", str(ROOT / "ops/tests/k6/lib/windows.ps1"))
           .replace("AUTH", str(ROOT / "ops/tests/k6/lib/windows-auth.ps1")))

    def test_link_state_oracle_catches_changed_flags_methods_and_unconsumed_transaction(self):
        self.ps("""
$ErrorActionPreference='Stop'
. 'HELPERS'
$before=[pscustomobject]@{database='k6_run';oid=42;tables=[pscustomobject]@{};transactions=[pscustomobject]@{total=2;consumed=2;pending=0;secrets=0}}
foreach($table in @('users','user_emails','provider_tokens','identities','refresh_tokens','rustycog_outbox_events')){
  $before.tables | Add-Member NoteProperty $table ([pscustomobject]@{count=2;sha256=('a'*64)})
}
$after=ConvertFrom-Json ($before|ConvertTo-Json -Depth 10)
$after.transactions.total++;$after.transactions.consumed++
Assert-NativeLinkRefusalState $before $after
foreach($table in @('users','user_emails','provider_tokens','identities','refresh_tokens','rustycog_outbox_events')){
  $after.tables.$table.sha256='b'*64
  $caught=$false;try{Assert-NativeLinkRefusalState $before $after}catch{$caught=$true}
  if(-not $caught){throw 'Changed complete row fingerprint accepted'}
  $after.tables.$table.sha256='a'*64
}
$after.transactions.secrets=1
$caught=$false;try{Assert-NativeLinkRefusalState $before $after}catch{$caught=$true}
if(-not $caught){throw 'Retained transaction secrets accepted'}
""".replace("HELPERS", str(NATIVE / "link-conflicts.ps1")))

    def test_classification_red_does_not_accept_arrangement_or_process_failure(self):
        self.ps("""
$ErrorActionPreference='Stop'
. 'WINDOWS'
. 'AUTH'
. '__LINK_HELPER__'
$state=[pscustomobject]@{run_id=('a'*32)}
$case='auth-link-conflict-identity';$e=Get-K6AuthExpectation $case
$checks=@{};foreach($name in $e.Checks){$checks[$name]=@{passes=1;fails=0}}
foreach($name in @('conflict-http-status','conflict-error-code','conflict-envelope')){$checks[$name]=@{passes=0;fails=1}}
$v=ConvertFrom-Json (@{version=1;run_id=$state.run_id;phase='HTTP';cases=@($case);expected=1;started=1;completed=1;iterations=1;requests=3;checks=$checks}|ConvertTo-Json -Depth 10)
$p=[pscustomobject]@{exit=99;cleanup=$true;identities_complete=$true;reason='NONE';capture_complete=$true;overflow=$false}
Assert-NativeLinkClassificationRed $v $p $case
if($v.checks.'conflict-http-status'.fails -ne 1){throw 'Original red summary changed'}
$v.checks.'conflict-recognized-response'.passes=0;$v.checks.'conflict-recognized-response'.fails=1
$caught=$false;try{Assert-NativeLinkClassificationRed $v $p $case}catch{$caught=$true}
if(-not $caught){throw 'Unrecognized legacy wire envelope accepted as historical red'}
if($v.checks.'conflict-recognized-response'.fails -ne 1 -or $v.checks.'conflict-http-status'.fails -ne 1){throw 'Rejected original red summary changed'}
$v.checks.'conflict-recognized-response'.passes=1;$v.checks.'conflict-recognized-response'.fails=0
$v.checks.'link-start'.passes=0;$v.checks.'link-start'.fails=1
$caught=$false;try{Assert-NativeLinkClassificationRed $v $p $case}catch{$caught=$true}
if(-not $caught){throw 'Arrangement failure accepted as historical red'}
$v.checks.'link-start'.passes=1;$v.checks.'link-start'.fails=0
$p.capture_complete=$false
$caught=$false;try{Assert-NativeLinkClassificationRed $v $p $case}catch{$caught=$true}
if(-not $caught){throw 'Incomplete process accepted as historical red'}
$p.capture_complete=$true
$v.checks.'conflict-envelope'.passes=1;$v.checks.'conflict-envelope'.fails=0
$caught=$false;try{Assert-NativeLinkClassificationRed $v $p $case}catch{$caught=$true}
if(-not $caught){throw 'Partial identity classification red accepted'}
$v.checks.'conflict-envelope'.passes=0;$v.checks.'conflict-envelope'.fails=1
$case='auth-link-conflict-email';$v.cases=@($case)
$v.checks.'conflict-http-status'.passes=1;$v.checks.'conflict-http-status'.fails=0
Assert-NativeLinkClassificationRed $v $p $case
$v.checks.'conflict-recognized-response'.passes=0;$v.checks.'conflict-recognized-response'.fails=1
$caught=$false;try{Assert-NativeLinkClassificationRed $v $p $case}catch{$caught=$true}
if(-not $caught){throw 'Unrecognized email wire envelope accepted as historical red'}
$v.checks.'conflict-recognized-response'.passes=1;$v.checks.'conflict-recognized-response'.fails=0
$v.checks.'conflict-http-status'.passes=0;$v.checks.'conflict-http-status'.fails=1
$caught=$false;try{Assert-NativeLinkClassificationRed $v $p $case}catch{$caught=$true}
if(-not $caught){throw 'Wrong email HTTP status accepted as expected red'}
""".replace("WINDOWS", str(ROOT / "ops/tests/k6/lib/windows.ps1"))
           .replace("AUTH", str(ROOT / "ops/tests/k6/lib/windows-auth.ps1"))
           .replace("__LINK_HELPER__", str(NATIVE / "link-conflicts.ps1")))

    def test_both_refusals_are_observed_after_classification_only_red(self):
        self.ps("""
$ErrorActionPreference='Stop'
. 'WINDOWS'
. 'AUTH'
. '__LINK_HELPER__'
$state=[pscustomobject]@{run_id=('a'*32);database_oid=42}
$Directory=[IO.Path]::GetTempPath()
$script:cases=@();$script:saved=@();$script:total=0
function Save-NativeJson($Value,$Name){$script:saved+=,$Name}
function Get-NativeHttpJson($Url){[pscustomobject]@{checks=[pscustomobject]@{queue_publisher=[pscustomobject]@{status='disabled'}}}}
function Assert-NativeLinkReceipt {}
function Snapshot($Phase){[pscustomobject]@{}}
function Assert-NativeLinkTraffic($Before,$After,$PB,$PA,$Iam,$Provider,[switch]$ClassificationRed,$Phase){}
function Get-NativeLinkState($Phase){
  $script:saved+=,('link-state-'+$Phase+'.json')
  $tables=[pscustomobject]@{}
  foreach($name in @('users','user_emails','provider_tokens','identities','refresh_tokens','rustycog_outbox_events')){
    $tables|Add-Member NoteProperty $name ([pscustomobject]@{count=$(if($script:total){2}else{0});sha256=('a'*64)})
  }
  [pscustomobject]@{database='k6_run';oid=42;tables=$tables;transactions=[pscustomobject]@{total=$script:total;consumed=$script:total;pending=0;secrets=0}}
}
function Invoke-Scenario($Case){
  $script:cases+=,$Case
  if($Case -ceq 'auth-link-conflict-setup'){$script:total=2;return}
  $script:total++;throw 'SUMMARY_INVALID'
}
function Read-NativeJson($Name){
  if($Name.EndsWith('.process.json')){
    return [pscustomobject]@{exit=99;cleanup=$true;identities_complete=$true;capture_complete=$true;overflow=$false;reason='NONE'}
  }
  $case=$Name.Substring(0,$Name.Length-5);$e=Get-K6AuthExpectation $case
  $checks=@{};foreach($name in $e.Checks){$checks[$name]=@{passes=1;fails=0}}
  foreach($name in @('conflict-error-code','conflict-envelope')){$checks[$name]=@{passes=0;fails=1}}
  if($case -ceq 'auth-link-conflict-identity'){$checks['conflict-http-status']=@{passes=0;fails=1}}
  ConvertFrom-Json (@{version=1;run_id=$state.run_id;phase='HTTP';cases=@($case);expected=1;started=1;completed=1;iterations=1;requests=3;checks=$checks}|ConvertTo-Json -Depth 10)
}
$caught=$false;try{Invoke-NativeLinkConflictPhases}catch{
  if($_.Exception.Message -cne 'LINK_CONFLICT_CLASSIFICATION_FAILED'){throw}
  $caught=$true
}
if(-not $caught -or ($script:cases -join ',') -cne 'auth-link-conflict-setup,auth-link-conflict-email,auth-link-conflict-identity'){throw 'Both classification reds were not collected'}
foreach($name in @('link-state-email-after.json','link-state-identity-after.json','link-email-evidence.json','link-identity-evidence.json','evidence.json')){
  if($name -cnotin $script:saved){throw 'Missing post-refusal evidence'}
}
""".replace("WINDOWS", str(ROOT / "ops/tests/k6/lib/windows.ps1"))
           .replace("AUTH", str(ROOT / "ops/tests/k6/lib/windows-auth.ps1"))
           .replace("__LINK_HELPER__", str(NATIVE / "link-conflicts.ps1")))

    def test_link_red_5xx_is_exactly_phase_bound(self):
        self.ps("""
$ErrorActionPreference='Stop'
. '__LINK_HELPER__'
$state=[pscustomobject]@{run_id=('a'*32)}
$provider=[pscustomobject]@{run_id=$state.run_id;authorize=0;token=0;profile=0;pkce_validated=0}
$script:errors=0
function Get-K6AuthDelta($Before,$After,$Name){
  if($Name -ceq 'cluster.iam.upstream_rq_total'){return 2}
  if($Name -ceq 'cluster.iam.upstream_rq_5xx'){return $script:errors}
  return 0
}
foreach($phase in @('email','identity')){
  foreach($red in @($false,$true)){
    foreach($errors in @(0,1,2)){
      $script:errors=$errors
      $expected=if($red -and $phase -ceq 'identity'){1}else{0}
      $caught=$false
      try{Assert-NativeLinkTraffic $null $null $provider $provider 2 0 -ClassificationRed:$red -Phase $phase}catch{$caught=$true}
      if($caught -ne ($errors -ne $expected)){throw 'Incorrect phase-bound 5xx acceptance'}
    }
  }
}
""".replace("__LINK_HELPER__", str(NATIVE / "link-conflicts.ps1")))

    def test_delete_has_uid_precondition_and_failure_does_not_retire_receipt(self):
        with tempfile.TemporaryDirectory(prefix="k6-native-delete-") as directory:
            script = """
$ErrorActionPreference='Stop'
. 'HELPERS'
$Directory='DIRECTORY';$utf8=[Text.UTF8Encoding]::new($false)
$r=@{kind='Secret';name='k6-owned-iam-tls';uid='exact-uid';deleted=$false}
$script:state=@{resources=@($r)}
$script:calls=@()
function K($Arguments,$Timeout) { $script:calls+=,@($Arguments) }
Remove-NativeObject $r
$body=ConvertFrom-Json ([IO.File]::ReadAllText((Join-Path $Directory 'delete.json')))
if($body.preconditions.uid -cne 'exact-uid' -or $body.propagationPolicy -cne 'Background'){throw 'Missing API precondition'}
if($script:calls[0][1] -cne '--raw' -or $script:calls[0][2] -cne '/api/v1/namespaces/aiforall-local-full/secrets/k6-owned-iam-tls'){throw 'Wrong API scope'}
if(-not $r.deleted){throw 'Missing receipt'}
if($script:calls.Count -ne 1){throw 'Secret read through wait is forbidden'}
$r.deleted=$false
function K($Arguments,$Timeout) {throw 'UID_CONFLICT'}
$caught=$false;try {Remove-NativeObject $r}catch{$caught=$true}
if(-not $caught -or $r.deleted){throw 'Conflict was swallowed'}
""".replace("HELPERS", str(NATIVE / "helpers.ps1")).replace("DIRECTORY", directory)
            self.ps(script)


@unittest.skipUnless(os.name == "nt" and K6.is_file(), "existing Windows k6 required")
class CampaignContextTests(unittest.TestCase):
    def setUp(self):
        self.target = dict(version=1, profile="iam-native-parity", run_id="a" * 32,
                           origin="https://127.0.0.1:18443",
                           provider="https://localhost:18445/oauth/authorize",
                           issuer="https://127.0.0.1:18443/iam", audience="aiforall")

    def inspect(self, target, scenario="smoke/iam-health.js", phase="good", run_id="a" * 32):
        with tempfile.TemporaryDirectory(prefix="k6-native-context-") as directory:
            path = Path(directory) / "target.json"
            path.write_text(json.dumps(target), encoding="utf-8")
            config = Path(directory) / "options.json"
            config.write_text("{}", encoding="utf-8")
            environment = {name: value for name, value in os.environ.items()
                           if name.upper() in {"SYSTEMROOT", "WINDIR", "TEMP", "TMP"}}
            return subprocess.run([str(K6), "inspect", "--config", str(config),
                                   "-e", f"K6_CAMPAIGN_FILE={path}", "-e", f"K6_RUN_ID={run_id}",
                                   "-e", f"K6_AUTH_SAN_PHASE={phase}",
                                   str(ROOT / "ops/tests/k6/scenarios" / scenario)],
                                  env=environment, capture_output=True, text=True, timeout=20)

    def test_all_five_native_phases_have_one_bounded_case(self):
        for scenario, phase, case in [("smoke/iam-health.js", "", "iam-health"),
                                      ("auth/oauth-browser.js", "", "auth-oauth-browser"),
                                      *[("auth/envoy-san.js", phase, "auth-envoy-san-" + phase)
                                        for phase in ("good", "bad", "restored")]]:
            with self.subTest(case=case):
                result = self.inspect(self.target, scenario, phase)
                self.assertEqual(result.returncode, 0, result.stderr)
                options = json.loads(result.stdout)
                self.assertEqual(list(options["scenarios"]), [case])
                self.assertEqual(options["scenarios"][case]["vus"], 1)
                self.assertEqual(options["scenarios"][case]["iterations"], 1)

    def test_real_k6_projects_named_pass_and_fail_with_each_scenario_tag_set(self):
        # No network or browser: exercise the real summary projector, not a
        # fabricated handleSummary object. Omitting `group` loses named checks
        # in k6 2.3 even though the aggregate checks metric/exit can be green.
        for scenario in ("smoke/iam-health.js", "auth/oauth-browser.js", "auth/envoy-san.js"):
            text = (ROOT / "ops/tests/k6/scenarios" / scenario).read_text(encoding="utf-8")
            tags = re.search(r"systemTags:\s*(\[[^\]]+\])", text).group(1)
            with self.subTest(scenario=scenario), tempfile.TemporaryDirectory(prefix="k6-summary-project-") as directory:
                path = Path(directory)
                output = path / "summary.json"
                (path / "options.json").write_text("{}", encoding="utf-8")
                source = ("import { check } from 'k6';\nimport { started, completed, project } from " +
                          json.dumps((ROOT / "ops/tests/k6/lib/summary.js").as_uri()) + ";\n" +
                          "export const options={vus:1,iterations:1,systemTags:" + tags + "};\n" +
                          "export default function(){started.add(1);check(true,{'known-pass':v=>v});" +
                          "check(false,{'known-fail':v=>v});completed.add(1);}\n" +
                          "export function handleSummary(data){return project(data,'HTTP','probe');}\n")
                (path / "probe.js").write_text(source, encoding="utf-8")
                environment = {name: value for name, value in os.environ.items()
                               if name.upper() in {"SYSTEMROOT", "WINDIR", "TEMP", "TMP"}}
                result = subprocess.run([str(K6), "run", "--quiet", "--no-usage-report", "--config", str(path / "options.json"),
                                         "-e", "K6_RUN_ID=" + "a" * 32, "-e", f"K6_SUMMARY_PATH={output}", str(path / "probe.js")],
                                        env=environment, capture_output=True, text=True, timeout=20)
                self.assertEqual(result.returncode, 0, result.stderr)
                summary = json.loads(output.read_text(encoding="utf-8"))
                self.assertEqual(summary["checks"], {"known-pass": {"passes": 1, "fails": 0},
                                                      "known-fail": {"passes": 0, "fails": 1}})

    def test_native_metadata_rejects_http_foreign_origin_extra_fields_and_wrong_run(self):
        variants = [{**self.target, "origin": "http://127.0.0.1:18443"},
                    {**self.target, "provider": "https://127.0.0.1:18445/oauth/authorize"},
                    {**self.target, "issuer": "http://127.0.0.1:8080/iam"},
                    {**self.target, "audience": "foreign"},
                    {**self.target, "version": 2},
                    {**self.target, "profile": "legacy"},
                    {**self.target, "run_id": "b" * 32},
                    {**self.target, "insecure": True},
                    {key: value for key, value in self.target.items() if key != "issuer"}]
        for target in variants:
            with self.subTest(target=target):
                result = self.inspect(target)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("NATIVE_CAMPAIGN_INVALID", result.stderr)

@unittest.skipUnless(os.name == "nt", "Windows PowerShell semantics")
class NativeFailureTests(unittest.TestCase):
    ps = NativePowerShellTests.ps

    def test_forward_expiry_and_foreign_listener_are_rejected_before_stimulus(self):
        script = """
$ErrorActionPreference='Stop'
. 'HELPERS'
$script:forwards=@(18443,18445,19901 | ForEach-Object {@{port=$_;task=@{IsCompleted=$false};child=@{Identities=@(@{Pid=42})}}})
function Get-NetTCPConnection {param($LocalAddress,$LocalPort,$State,$ErrorAction) [pscustomobject]@{OwningProcess=42}}
Assert-NativeForwards
$script:forwards[0].task.IsCompleted=$true
$caught=$false;try{Assert-NativeForwards}catch{$caught=$_.Exception.Message -ceq 'FORWARD_EXITED_BEFORE_STIMULUS'}
if(-not $caught){throw 'Expired forward accepted'}
$script:forwards[0].task.IsCompleted=$false
function Get-NetTCPConnection {param($LocalAddress,$LocalPort,$State,$ErrorAction) [pscustomobject]@{OwningProcess=999}}
$caught=$false;try{Assert-NativeForwards}catch{$caught=$_.Exception.Message -ceq 'FORWARD_LISTENER_OWNERSHIP_LOST'}
if(-not $caught){throw 'Foreign listener accepted'}
""".replace("HELPERS", str(NATIVE / "helpers.ps1"))
        self.ps(script)

    def test_socket_invocation_drops_only_inherited_tcp_selectors(self):
        script = """
$ErrorActionPreference='Stop'
. 'HELPERS'
$script:state=@{postgres=@{pod='postgres-owned';uid='postgres-uid'}}
$script:seen=$null
function K($Arguments,$Timeout) {
    if($Arguments[0] -ceq 'get'){return '{"metadata":{"uid":"postgres-uid"}}'}
    $script:seen=$Arguments
    return '0'
}
$result=Invoke-NativeSql @('SELECT 1')
if($result -cne '0'){throw 'Result changed'}
$command=$script:seen -join '|'
if(-not $command.Contains('|env|-u|PGHOSTADDR|-u|PGHOST|psql|')){throw 'Inherited TCP selector retained'}
if(-not $command.Contains('|-h|/var/run/postgresql|')){throw 'Socket absent'}
if($command.Contains('sslmode=disable') -or $command.Contains('PGPASSWORD')){throw 'Unexpected TLS or credential change'}
""".replace("HELPERS", str(NATIVE / "helpers.ps1"))
        self.ps(script)

    def test_failed_restoration_blocks_deletion_and_does_not_report_cleanup_pass(self):
        with tempfile.TemporaryDirectory(prefix="k6-native-restore-") as directory:
            script = """
$ErrorActionPreference='Stop'
. 'HELPERS'
$Directory='DIRECTORY';$utf8=[Text.UTF8Encoding]::new($false)
$script:state=@{run_id='run';certificate_phase='bad';restoration='FAIL';ca_owned=$false;database_dropped=$false;cleanup='NOT_RUN';cleaned=$false}
$script:forwards=@();$script:deleted=$false
$e=$null;$t=$null
$ast=[Management.Automation.Language.Parser]::ParseFile('CAMPAIGN',[ref]$t,[ref]$e)
$function=$ast.Find({param($a) $a -is [Management.Automation.Language.FunctionDefinitionAst] -and $a.Name -ceq 'Cleanup-NativeFixtures'},$false)
Invoke-Expression $function.Extent.Text
function Restore-NativeIam {throw 'RESTORATION_FAILED'}
function Remove-NativeObject {$script:deleted=$true}
$caught=$false;try {Cleanup-NativeFixtures}catch{$caught=$true}
if(-not $caught -or $script:deleted -or $state.cleanup -cne 'FAIL' -or $state.cleaned){throw 'Unproven restoration was accepted'}
""".replace("HELPERS", str(NATIVE / "helpers.ps1")).replace("CAMPAIGN", str(NATIVE / "campaign.ps1")).replace("DIRECTORY", directory)
            self.ps(script)


if __name__ == "__main__":
    unittest.main()
