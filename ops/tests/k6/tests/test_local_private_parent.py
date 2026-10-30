"""Parent protocol/OS boundary fixtures only; never a private capture admission."""
import contextlib
from datetime import datetime, timedelta, timezone
import importlib.util
import io
import json
import os
from pathlib import Path
import subprocess
import sys
from types import SimpleNamespace
import unittest
from unittest import mock

from test_runner import ROOT, runner
from test_private_site import nominal
from test_private_site import FakePosix, FATAL
from test_direct_chromium import fake_child_reads, host
from test_boundaries import scratch

SPEC = importlib.util.spec_from_file_location("local_parent", ROOT / "local-private-parent.py")
parent = None
if Path(SPEC.origin).is_file():
    parent = importlib.util.module_from_spec(SPEC)
    SPEC.loader.exec_module(parent)


def initial(mode="preflight"):
    return dict(v=1, kind="INIT", seq=0, session="a" * 32, browser=nominal()["run_id"], run="",
                deadline=(datetime.now(timezone.utc) + timedelta(minutes=20)).strftime("%Y-%m-%dT%H:%M:%SZ"),
                home=[0, 0, 0], mode=mode, cleanup="NOT_STARTED", diag="NONE")


class ParentTests(unittest.TestCase):
    def setUp(self):
        self.assertIsNotNone(parent, "parent not implemented")

    def test_closed_frames_reject_types_keys_and_untrusted_text(self):
        frame = initial()
        self.assertEqual(parent.decode_frame(parent.encode_frame(frame)), frame)
        for change in ({"v": True}, {"seq": 1.0}, {"home": [1, True, 3]}, {"kind": "CONFIDENTIAL_SENTINEL"},
                       {"run": "foreign"}, {"extra": "site"}, {"deadline": "tomorrow"}):
            with self.subTest(field=next(iter(change))), self.assertRaises(parent.Refused):
                parent.encode_frame({**frame, **change})
        for raw in (b"{}", b"\xff", b"x" * 4097, b'{"v":1,"v":1}', b'{"v":NaN}'):
            with self.assertRaises(parent.Refused):
                parent.decode_frame(raw)

    def test_stream_reader_bounds_no_lf_truncation_and_bad_utf8(self):
        for raw in (b"x" * 4097, b'{"v":1}', b"\xff\n", b"{}\n{}\n"):
            wire = parent.Channel(10, 11)
            with mock.patch.object(parent.select, "select", return_value=([10], [], [])), \
                    mock.patch.object(parent.os, "read", side_effect=[raw, b""]):
                with self.assertRaises(parent.Refused):
                    wire.receive(1)

    def test_sequence_bindings_and_freshness_refuse_stale_or_foreign_reply(self):
        for alteration in ({"run": "f" * 32}, {"session": "f" * 32}, {"home": [4, 5, 6]},
                           {"seq": 2}, {"kind": "WAIT"}, {"deadline": "2020-01-01T00:00:00Z"}):
            transport = mock.Mock()
            peer = parent.Peer(transport, initial(), [1, 2, 3])
            def response(timeout):
                return {**transport.send.call_args.args[0], "kind": "ALLOW", **alteration}
            transport.receive.side_effect = response
            with self.subTest(field=next(iter(alteration))), self.assertRaises(parent.Refused):
                peer.ask("AUTHORIZE", "b" * 32)
        peer = parent.Peer(mock.Mock(), initial(), [1, 2, 3])
        with self.assertRaises(parent.Refused):
            peer.ask("WRITE", "b" * 32)

    def test_slow_callback_and_changed_home_never_authorize(self):
        peer = mock.Mock()
        facts = mock.Mock()
        facts.home_identity = [1, 2, 3]
        callback = parent.Admission(peer, facts, initial())
        evidence = dict(phase="AUTHORIZE", browser_run_id=initial()["browser"], run_id="b" * 32,
                        home_identity=(1, 2, 3), delete_by_utc=initial()["deadline"])
        with mock.patch.object(parent.time, "monotonic", side_effect=[0, 31, 32]):
            self.assertFalse(callback(evidence))
        callback = parent.Admission(peer, facts, initial())
        self.assertFalse(callback({**evidence, "home_identity": (7, 8, 9)}))

    def test_bounded_main_output_and_non_success_oracle(self):
        output = parent.BoundedText()
        with self.assertRaises(parent.Refused):
            output.write("x" * (runner.ARTIFACT_LIMIT + 1))
        result = dict(version=1, HTTP="INCONCLUSIVE", Chromium="INCONCLUSIVE", cleanup="PASS",
                      confidentiality="PASS", run_id="b" * 32, started_unix_ms=1,
                      cleanup_scope="LAUNCHER_CHILDREN_ONLY", parent_runtime_cleanup="INCONCLUSIVE",
                      diagnostic="SANDBOX_UNPROVEN")
        self.assertEqual(parent.result_check(json.dumps(result), 2)["run_id"], "b" * 32)
        for changed, code in ((result, 0), ({**result, "HTTP": "PASS"}, 2),
                              ({**result, "source_relative": "CONFIDENTIAL_SENTINEL"}, 2)):
            with self.assertRaises(parent.Refused):
                parent.result_check(json.dumps(changed), code)

    def test_preflight_is_nonarming_and_runs_both_real_observer_boundaries(self):
        facts = mock.Mock()
        facts.home_identity = [1, 2, 3]
        peer = mock.Mock()
        peer.init = initial()
        with mock.patch.object(parent.site_channel, "PrivateSiteSession") as session, \
                mock.patch.object(parent.runner, "main") as main:
            self.assertEqual(parent.preflight(peer, facts), 0)
        session.assert_not_called()
        main.assert_not_called()
        self.assertEqual(facts.verify.call_count, 4)
        self.assertEqual([call.args[0] for call in peer.ask.call_args_list], ["AUTHORIZE", "WRITE"])

    def test_retention_ack_absence_expiry_eof_and_removal_failure(self):
        for reply, expired, removed in (("ACK", False, True), ("WAIT", True, True), ("ACK", False, False)):
            session = mock.Mock()
            session._expired.side_effect = [False, expired, True]
            session.retire.return_value = removed
            session.status = "REMOVED" if removed else "RETENTION_UNPROVEN"
            session.deadline = datetime.now(timezone.utc) + timedelta(minutes=1)
            session._expires_monotonic = parent.time.monotonic() + 60
            peer, facts = mock.Mock(), mock.Mock()
            peer.ask.return_value = reply
            if removed:
                parent.await_read(session, peer, facts)
                session.retire.assert_called_with(read_confirmed=reply == "ACK")
            else:
                with self.assertRaises(parent.Refused):
                    parent.await_read(session, peer, facts)
        peer.ask.side_effect = parent.Refused()
        with self.assertRaises(parent.Refused):
            parent.await_read(session, peer, facts)

    def test_mount_and_provenance_changes_refuse(self):
        text = "25 1 8:1 / / rw - ext4 /dev/a rw\n"
        with mock.patch.object(parent.os, "major", return_value=8, create=True), \
                mock.patch.object(parent.os, "minor", side_effect=lambda value: value, create=True):
            self.assertTrue(parent.ext4_mount("mnt_id:\t25\n", text, 1))
            for fdinfo, mounts, dev in (("mnt_id:\t26\n", text, 1),
                                        ("mnt_id:\t25\n", text.replace("ext4", "9p"), 1),
                                        ("mnt_id:\t25\n", text, 2)):
                self.assertFalse(parent.ext4_mount(fdinfo, mounts, dev))

    def test_live_identity_recheck_is_not_a_cached_boolean(self):
        facts = object.__new__(parent.LinuxFacts)
        facts.fd, facts.profile = 42, {"uid": 789, "home": "/home/operator"}
        facts.home_identity, facts.home_chain = [7, 8, 789], ["home"]
        facts.account = ("aiforall", "/home/operator")
        facts.identities, facts.tools = {Path("/known/code.py"): ["code"]}, mock.Mock()
        item = mock.Mock(st_dev=7, st_ino=8, st_uid=789)
        pwd = SimpleNamespace(getpwuid=lambda uid: SimpleNamespace(pw_name="aiforall", pw_dir="/home/operator"))
        with mock.patch.dict(sys.modules, {"pwd": pwd}), mock.patch.object(parent.os, "fstat", return_value=item), \
                mock.patch.object(parent.os, "getuid", return_value=789, create=True), \
                mock.patch.object(parent.os, "geteuid", return_value=789, create=True), \
                mock.patch.object(parent, "name_chain", side_effect=[["home"], ["code"]]), \
                mock.patch.object(facts, "_mount"), mock.patch.object(parent.runner, "verify_tools"):
            facts.verify()
        for changed in (mock.Mock(st_dev=9, st_ino=8, st_uid=789), mock.Mock(st_dev=7, st_ino=9, st_uid=789),
                        mock.Mock(st_dev=7, st_ino=8, st_uid=0)):
            with mock.patch.dict(sys.modules, {"pwd": pwd}), mock.patch.object(parent.os, "fstat", return_value=changed), self.assertRaises(parent.Refused):
                facts.verify()
        with mock.patch.dict(sys.modules, {"pwd": pwd}), mock.patch.object(parent.os, "fstat", return_value=item), \
                mock.patch.object(parent.os, "getuid", return_value=789, create=True), \
                mock.patch.object(parent.os, "geteuid", return_value=789, create=True), \
                mock.patch.object(parent, "name_chain", side_effect=[["home"], ["changed"]]), \
                mock.patch.object(facts, "_mount"), self.assertRaises(parent.Refused):
            facts.verify()

    def test_full_known_capture_composition_keeps_context_until_real_ack(self):
        fs = FakePosix()
        with scratch() as root:
            (root / "private").mkdir()
            with fs.enabled(), fake_child_reads(FATAL, 1, True, native_exit=-6) as child, \
                    mock.patch.object(parent.runner, "linux_platform"), \
                    mock.patch.object(parent.runner, "new_output", return_value=root), \
                    mock.patch.object(parent.runner, "pinned_tools", return_value=({"chromium": Path("/pinned/chrome")}, {})), \
                    mock.patch.object(parent.runner, "version_checks", return_value={}), \
                    mock.patch.object(parent.runner, "host_metadata", return_value=host()), \
                    mock.patch.object(parent.runner, "verify_tools"), \
                    mock.patch.object(parent.runner, "Child", return_value=child), \
                    mock.patch.object(parent.runner, "publish") as publish:
                home = fs.nodes[str(fs.home)]
                facts = mock.Mock(profile={"output": str(root)}, refusal=nominal(),
                                  home_identity=[home.st_dev, home.st_ino, home.st_uid])
                transport = mock.Mock()
                peer = parent.Peer(transport, initial("capture"), facts.home_identity)
                kinds = []
                def receive(timeout):
                    request = transport.send.call_args.args[0]
                    kinds.append(request["kind"])
                    if request["kind"] == "READY":
                        self.assertTrue(any(path.endswith("/site.json") for path in fs.nodes))
                    return {**request, "kind": "ACK" if request["kind"] == "READY" else "ALLOW"}
                transport.receive.side_effect = receive
                self.assertEqual(parent.capture(peer, facts), 2)
                self.assertEqual(kinds, ["AUTHORIZE", "WRITE", "READY"])
                self.assertEqual(transport.send.call_args.args[0]["kind"], "CLOSED")
                for call in transport.send.call_args_list:
                    raw = parent.encode_frame(call.args[0])
                    self.assertNotIn(b"source_relative", raw)
                    self.assertNotIn(b"synthetic", raw)
                self.assertFalse(any(path.endswith("/site.json") for path in fs.nodes))
                publish.assert_not_called()
            self.assertEqual(fs.fds, {})

    def test_retention_failure_survives_full_main_after_ack_and_deadline(self):
        class ProtocolConsole(io.StringIO):
            def fileno(self):
                return 32

        for expiry in (False, True):
            with self.subTest(expiry=expiry), scratch() as root:
                fs, sessions, frames = FakePosix(), [], []
                (root / "private").mkdir()
                original_enter = parent.site_channel.PrivateSiteSession.__enter__
                def enter(session):
                    sessions.append(session)
                    return original_enter(session)
                with fs.enabled(), fake_child_reads(FATAL, 1, True, native_exit=-6) as child, \
                        mock.patch.object(parent.runner, "linux_platform"), \
                        mock.patch.object(parent.runner, "new_output", return_value=root), \
                        mock.patch.object(parent.runner, "pinned_tools", return_value=({"chromium": Path("/pinned/chrome")}, {})), \
                        mock.patch.object(parent.runner, "version_checks", return_value={}), \
                        mock.patch.object(parent.runner, "host_metadata", return_value=host()), \
                        mock.patch.object(parent.runner, "verify_tools"), \
                        mock.patch.object(parent.runner, "Child", return_value=child), \
                        mock.patch.object(parent.runner, "publish") as publish, \
                        mock.patch.object(parent.site_channel.PrivateSiteSession, "__enter__", autospec=True, side_effect=enter), \
                        mock.patch.object(parent.os, "set_blocking", create=True), \
                        mock.patch.object(sys, "stdin", ProtocolConsole()), \
                        contextlib.redirect_stdout(ProtocolConsole()) as output, contextlib.redirect_stderr(io.StringIO()) as errors:
                    home = fs.nodes[str(fs.home)]
                    facts = mock.Mock(profile={"output": str(root)}, refusal=nominal(),
                                      home_identity=[home.st_dev, home.st_ino, home.st_uid])
                    transport = mock.Mock()
                    transport.send.side_effect = lambda value, **kwargs: frames.append(value)
                    def receive(timeout):
                        if not frames:
                            return initial("capture")
                        request = frames[-1]
                        if request["kind"] == "READY":
                            self.assertTrue(any(path.endswith("/site.json") for path in fs.nodes))
                            fs.unlink_fault = True
                            if expiry:
                                sessions[0]._expires_monotonic = -1
                            return {**request, "kind": "WAIT" if expiry else "ACK"}
                        return {**request, "kind": "ALLOW"}
                    transport.receive.side_effect = receive
                    with contextlib.ExitStack() as stack:
                        stack.enter_context(mock.patch.object(parent, "Channel", return_value=transport))
                        stack.enter_context(mock.patch.object(parent, "LinuxFacts", return_value=facts))
                        stack.enter_context(mock.patch.object(parent, "private_read", return_value=json.dumps({"browser_run_id": nominal()["run_id"]}).encode()))
                        self.assertEqual(parent.main(["--mode", "capture", "--profile", "/private/profile.json"]), 2)
                    self.assertEqual(sessions[0].status, "RETENTION_UNPROVEN")
                    self.assertEqual(frames[-1]["kind"], "ABORT")
                    self.assertEqual(frames[-1]["diag"], "RETENTION_UNPROVEN")
                    self.assertEqual(frames[-1]["cleanup"], "FAIL")
                    self.assertTrue(any(path.endswith("/site.json") for path in fs.nodes))
                    self.assertFalse(any(frame["kind"] == "CLOSED" for frame in frames))
                    for frame in frames:
                        raw = parent.encode_frame(frame)
                        self.assertNotIn(b"source_relative", raw)
                        self.assertNotIn(b"synthetic", raw)
                        self.assertNotIn(b"k6-causal", raw)
                    self.assertEqual(output.getvalue() + errors.getvalue(), "")
                    publish.assert_not_called()
                self.assertEqual(fs.fds, {})

    def test_cannot_read_site_as_profile_or_provenance(self):
        with mock.patch.object(Path, "open") as opened:
            for path in ("/home/operator/site.json", "/home/operator/.cache/opencode/k6-causal/fixture/record"):
                with self.assertRaises(parent.Refused):
                    parent.bounded_read(path)
            opened.assert_not_called()

    def test_ci_refuses_before_channel_or_observers(self):
        with mock.patch.dict(os.environ, {"GITHUB_RUN_ID": ""}), \
                mock.patch.object(parent, "Channel") as channel, mock.patch.object(parent, "LinuxFacts") as facts, \
                contextlib.redirect_stdout(io.StringIO()) as output, contextlib.redirect_stderr(io.StringIO()) as errors:
            self.assertEqual(parent.main(["--mode", "capture", "--profile", "CONFIDENTIAL_SENTINEL"]), 2)
        channel.assert_not_called()
        facts.assert_not_called()
        self.assertEqual(output.getvalue() + errors.getvalue(), "")

    def test_peer_rejects_duplicate_phase_and_changed_code_after_windows_reply(self):
        peer, facts = mock.Mock(), mock.Mock(home_identity=[1, 2, 3])
        facts.verify.side_effect = [None, parent.Refused()]
        callback = parent.Admission(peer, facts, initial())
        evidence = dict(phase="AUTHORIZE", browser_run_id=initial()["browser"], run_id="b" * 32,
                        home_identity=(1, 2, 3), delete_by_utc=initial()["deadline"])
        self.assertFalse(callback(evidence))
        wire = mock.Mock()
        real_peer = parent.Peer(wire, initial(), [1, 2, 3])
        wire.receive.side_effect = lambda timeout: {**wire.send.call_args.args[0], "kind": "ALLOW"}
        real_peer.ask("AUTHORIZE", "b" * 32)
        with self.assertRaises(parent.Refused):
            real_peer.ask("AUTHORIZE", "b" * 32)


@unittest.skipUnless(os.name == "nt", "WindowsPowerShell5.1 function fixtures run natively on Windows")
class PowerShellParentTests(unittest.TestCase):
    def run_ps(self, code):
        script = ROOT / "local-private-parent.ps1"
        self.assertTrue(script.is_file(), "PowerShell parent not implemented")
        prefix = ". '" + str(script).replace("'", "''") + "' -LibraryOnly; "
        executable = Path(os.environ["SystemRoot"]) / "System32/WindowsPowerShell/v1.0/powershell.exe"
        run = subprocess.run([str(executable), "-NoLogo", "-NoProfile", "-NonInteractive", "-Command", prefix + code],
                             capture_output=True, timeout=40, env={**os.environ, "TEMP": os.environ["TEMP"], "TMP": os.environ["TMP"]})
        self.assertEqual(run.returncode, 0, "PowerShell fixture failed (output deliberately withheld)")
        self.assertTrue(run.stdout.strip() == b"OK", "PowerShell fixture output withheld")
        self.assertTrue(run.stderr == b"", "PowerShell fixture stderr withheld")

    def controller_fixture(self, case, retention=False):
        """Run the delivered controller; only OS observations/process construction
        are fixture seams. The intercepted Process.Start can NEVER launch WSL.
        """
        with scratch() as root:
            prefix = "$fixtureRoot='" + str(root).replace("'", "''") + "';$case='" + case + "';"
            prefix += "$retention=" + ("$true;" if retention else "$false;")
            code = r"""
Initialize-Native
$sid=[Security.Principal.WindowsIdentity]::GetCurrent().User.Value
$expected=@{sid=$sid;guid='{11111111-1111-1111-1111-111111111111}';name='AIForAll';
 base=(Join-Path $fixtureRoot 'public\storage');uid=1000;version=2;directory='leaf-dir';disk='leaf-disk';acl='leaf-acl'}
$config=@{v=1;expected=$expected;browser=('a'*32);deadline=([datetime]::UtcNow.AddMinutes(10).ToString("yyyy-MM-dd'T'HH:mm:ss'Z'"));
 linux_profile='/home/aiforall/profile.json';companion='/home/aiforall/parent.py';
 lease=(Join-Path $fixtureRoot 'lease');journal=(Join-Path $fixtureRoot 'journal');
 wsl_image=(Join-Path $env:SystemRoot 'System32\wsl.exe');
 controller_sha256=(Get-FileHash -LiteralPath $script:ControllerPath -Algorithm SHA256).Hash.ToLowerInvariant()}
$path=Join-Path $fixtureRoot 'profile.json'
[IO.File]::WriteAllText($path,(ConvertTo-Json -InputObject $config -Compress -Depth 5),(New-Object Text.UTF8Encoding($false)))
$script:starts=0;$script:ended=$false;$script:factCalls=@{};$script:incoming=0;$script:initial=$null;$script:allows=@()
function Get-PrivateAcl {param($Path,$Sid) return 'leaf-acl'}
function Get-WindowsSnapshot {param($Expected) return $Expected}
function Get-AuthenticodeSignature {param($LiteralPath) return @{Status='Valid'}}
function Get-WindowsPathFact {param($Path)
 if(!$script:factCalls.ContainsKey($Path)){$script:factCalls[$Path]=0};$script:factCalls[$Path]++
 $bad=([IO.Path]::GetFileName($Path.TrimEnd('\')) -ceq 'public')
 $rights=[long]0x1200a9;$inherit=$false;$reparse=$false;$id='id:'+ $Path;$owner='S-1-5-32-544'
 if($bad){
  if($case -ceq 'delete_child' -or ($case -ceq 'renewed' -and $script:starts -gt 0)){$rights=0x40}
  if(($case -ceq 'authorize_changed' -and $script:incoming -ge 2) -or ($case -ceq 'write_changed' -and $script:incoming -ge 3)){$rights=0x40}
  if($case -ceq 'delete'){$rights=0x10000}
  if($case -ceq 'write_dac'){$rights=0x40000}
  if($case -ceq 'write_owner'){$rights=0x80000}
  if($case -ceq 'write_data'){$rights=2}
  if($case -ceq 'create_subdirectory'){$rights=4}
  if($case -ceq 'reparse'){$reparse=$true}
  if($case -ceq 'changed' -and $script:factCalls[$Path] -gt 1){$id+='changed'}
  if($case -ceq 'inherit_only'){$rights=0x40;$inherit=$true}
  if($case -ceq 'owner_unknown'){$owner='S-1-5-21-999-999-999-999'}
  if($case -ceq 'trusted_installer'){$owner='S-1-5-80-956008885-3418522649-1831038044-1853292631-2271478464'}
  if($case -ceq 'unreadable'){throw 'PARENT_REFUSED'}
 }
 return @{identity=$id;owner=$owner;dacl='unchanged-leaves';reparse=$reparse;
  directory=($Path -notmatch '\.(json|exe|ps1|vhdx)$');
  rules=@(@{sid='S-1-1-0';allow=$true;rights=$rights;inherit_only=$inherit})}
}
function Get-ProcessObservation {param($Process)
 return [pscustomobject]@{Pid=77;Created=7;Image=$config.wsl_image;Sid=$sid;
  Wait=$(if($script:ended){0}else{258});Exited=$(if($script:ended){9}else{0})}
}
function New-Object {
 param([Parameter(Position=0)][string]$TypeName,[Parameter(Position=1)][object[]]$ArgumentList)
 if($TypeName -ceq 'Diagnostics.Process'){
  $inputStream=Microsoft.PowerShell.Utility\New-Object IO.MemoryStream
  $outputStream=Microsoft.PowerShell.Utility\New-Object IO.MemoryStream
  $errorStream=Microsoft.PowerShell.Utility\New-Object IO.MemoryStream
  $p=[pscustomobject]@{Id=77;StartInfo=$null;StandardInput=(Microsoft.PowerShell.Utility\New-Object IO.StreamWriter($inputStream));
   StandardOutput=(Microsoft.PowerShell.Utility\New-Object IO.StreamReader($outputStream));StandardError=(Microsoft.PowerShell.Utility\New-Object IO.StreamReader($errorStream));
   ExitCode=0;disposed=$false}
  $p|Add-Member ScriptMethod Start {$script:starts++;return $true}
  $p|Add-Member ScriptMethod WaitForExit {param($n) $script:ended=$true;return $true}
  $p|Add-Member ScriptMethod Dispose {$this.disposed=$true}
  return $p
 }
 return ,(Microsoft.PowerShell.Utility\New-Object @PSBoundParameters)
}
function Send-Frame {param($Process,$Frame,$Milliseconds)
 if($Frame.kind -ceq 'INIT'){$script:initial=@{}+$Frame};if($Frame.kind -ceq 'ALLOW'){$script:allows+=,$Frame.seq}
}
function Receive-Frame {param($Out,$Errors,$Process,$Milliseconds)
 $r=@{}+$script:initial;$r.home=@(7,8,1000)
 switch($script:incoming){
  0 {$r.kind='HELLO'}
  1 {$r.kind='AUTHORIZE';$r.seq=1;$r.run='b'*32}
  2 {$r.kind='WRITE';$r.seq=2;$r.run='b'*32}
  3 {$r.seq=3;$r.run='b'*32;if($retention){$r.kind='ABORT';$r.cleanup='FAIL';$r.diag='RETENTION_UNPROVEN'}
     else{$r.kind='PREFLIGHT';$r.diag='OBSERVERS_ONLY'}}
  default {throw 'fixture_frames_exhausted'}
 }
 $script:incoming++;return $r
}
$saved=[Console]::Out;$writer=Microsoft.PowerShell.Utility\New-Object IO.StringWriter
$exit=2
try{[Console]::SetOut($writer);try{$exit=Invoke-PrivateParent $path 'preflight'}catch{
 if($_.Exception.Message -cne 'PARENT_REFUSED'){throw 'fixture_controller_failed'}
}}finally{[Console]::SetOut($saved)}
$text=$writer.ToString()
if($retention){
 if($script:starts -ne 1 -or $exit -eq 0 -or !$text.Contains('RETENTION_UNPROVEN') -or $text.Contains('_CLOSED')){throw 'fixture_warning_missing'}
}elseif(@('safe','inherit_only','create_subdirectory','trusted_installer') -ccontains $case){
 if($script:starts -ne 1 -or $exit -ne 0){throw 'fixture_safe_chain_rejected'}
}else{
 $wanted=if(@('renewed','authorize_changed','write_changed') -ccontains $case){1}else{0}
 if($script:starts -ne $wanted -or $exit -eq 0 -or $text.Contains('_CLOSED')){throw 'fixture_unsafe_chain_started'}
 if($case -ceq 'authorize_changed' -and $script:allows.Count -ne 0){throw 'fixture_stale_authorize'}
 if($case -ceq 'write_changed' -and ($script:allows.Count -ne 1 -or $script:allows[0] -ne 1)){throw 'fixture_stale_write'}
}
if($text.Contains('CONFIDENTIAL_SENTINEL') -or $text.Contains($fixtureRoot)){throw 'fixture_output_leak'}
'OK'
"""
            self.run_ps(prefix + code)

    def test_ancestor_guards_before_process_start_and_on_renewal(self):
        for case in ("delete_child", "delete", "write_dac", "write_owner", "write_data", "reparse", "changed", "renewed",
                     "authorize_changed", "write_changed", "owner_unknown", "unreadable"):
            with self.subTest(case=case):
                self.controller_fixture(case)

    def test_safe_system_ancestors_can_have_public_readers(self):
        for case in ("safe", "inherit_only", "create_subdirectory", "trusted_installer"):
            with self.subTest(case=case):
                self.controller_fixture(case)

    def test_retention_warning_is_wired_through_delivered_controller(self):
        self.controller_fixture("safe", retention=True)

    def test_native_path_fact_only_on_owned_fixture(self):
        with scratch() as root:
            path = str(root).replace("'", "''")
            self.run_ps("$f=Get-WindowsPathFact '" + path + "';"
                        "if(!$f.directory -or $f.reparse -or $f.identity -cne [K6Parent.Native]::FileIdentity('" + path + "')){throw 'fixture'};"
                        "if($f.owner -cne (Get-Acl -LiteralPath '" + path + "').GetOwner([Security.Principal.SecurityIdentifier]).Value){throw 'fixture'};"
                        "foreach($r in $f.rules){if(!(Test-Integer $r.rights) -or $r.rights -lt 0){throw 'fixture'}};'OK'")

    def test_real_frame_functions_and_duplicate_keys(self):
        raw = json.dumps(initial(), separators=(",", ":"))
        self.run_ps("$f=Read-FrameText '" + raw + "'; [void](Write-FrameText $f); "
                    "$bad=$false;try{Read-FrameText '{\"v\":1,\"v\":1}'|Out-Null}catch{$bad=$true};"
                    "if(!$bad){throw 'fixture'}; 'OK'")

    def test_real_snapshot_comparator_on_changed_fixtures(self):
        self.run_ps("$a=@{sid='s';guid='g';name='AIForAll';base='b';uid=1000;version=2;directory='d';disk='v';acl='a'};"
                    "Assert-Snapshot $a $a; foreach($k in @('sid','guid','name','base','uid','version','directory','disk','acl')){"
                    "$b=$a.Clone();$b[$k]='changed';$bad=$false;try{Assert-Snapshot $a $b}catch{$bad=$true};"
                    "if(!$bad){throw 'fixture'}}; 'OK'")

    def test_real_argument_quoting_and_bounded_reader(self):
        self.run_ps("if((Quote-WindowsArgument 'a b') -cne '\"a b\"'){throw 'fixture'};"
                    "if((Quote-WindowsArgument 'a\"b') -cne '\"a\\\"b\"'){throw 'fixture'};"
                    "Initialize-Native; $s=New-Object IO.MemoryStream(,[Text.Encoding]::UTF8.GetBytes(('x'*4097)));"
                    "$r=New-Object K6Parent.FrameReader($s);$bad=$false;try{$r.Poll(1000)|Out-Null}catch{$bad=$true};"
                    "if(!$bad){throw 'fixture'};'OK'")

    def test_real_protocol_order_bindings_deadline_and_budget(self):
        raw = json.dumps({**initial(), "home": [1, 2, 3]}, separators=(",", ":"))
        self.run_ps("$i=Read-FrameText '" + raw + "';$s=@{seq=0;phase=0;run=''};"
                    "$r=@{}+$i;$r.kind='WRITE';$r.seq=1;$r.run='" + "b" * 32 + "';"
                    "$bad=$false;try{Assert-ProtocolStep $s $i $r}catch{$bad=$true};if(!$bad){throw 'fixture'};"
                    "$r.kind='AUTHORIZE';Assert-ProtocolStep $s $i $r;"
                    "$bad=$false;try{Assert-ProtocolStep $s $i $r}catch{$bad=$true};if(!$bad){throw 'fixture'};"
                    "$r=@{}+$r;$r.kind='WRITE';$r.seq=2;$r.home=@(4,5,6);"
                    "$bad=$false;try{Assert-ProtocolStep $s $i $r}catch{$bad=$true};if(!$bad){throw 'fixture'};"
                    "$bad=$false;try{Assert-Elapsed 30.01}catch{$bad=$true};if(!$bad){throw 'fixture'};"
                    "if((Read-UtcDeadline '2030-01-01T00:00:00Z').Kind -ne 'Utc'){throw 'fixture'};'OK'")

    def test_native_handle_observer_without_wsl_and_nonjoin_oracle(self):
        self.run_ps("Initialize-Native;$p=[Diagnostics.Process]::GetCurrentProcess();"
                    "try{$f=Get-ProcessObservation $p;if($f.Pid -ne $PID){throw 'fixture'};"
                    "Assert-Process $f $f;$bad=$false;try{Assert-Process $f $f $true}catch{$bad=$true};"
                    "if(!$bad){throw 'fixture'}}finally{$p.Dispose()};'OK'")

    def test_native_file_identity_on_owned_temporary_fixtures(self):
        with scratch() as root:
            first, second = root / "first.fixture", root / "second.fixture"
            first.write_bytes(b"known fixture")
            second.write_bytes(b"known fixture")
            a, b = str(first).replace("'", "''"), str(second).replace("'", "''")
            self.run_ps("Initialize-Native;$a=[K6Parent.Native]::FileIdentity('" + a + "');"
                        "$b=[K6Parent.Native]::FileIdentity('" + b + "');"
                        "if($a -ceq $b){throw 'fixture'};[IO.File]::WriteAllText('" + a + "','changed known fixture');"
                        "if($a -cne [K6Parent.Native]::FileIdentity('" + a + "')){throw 'fixture'};'OK'")

    def test_real_bounded_reader_utf8_truncation_and_frame_types(self):
        self.run_ps("Initialize-Native;$cases=New-Object 'Collections.Generic.List[byte[]]';"
                    "$cases.Add([byte[]](255,10));$cases.Add([Text.Encoding]::UTF8.GetBytes('{'));foreach($bytes in $cases){"
                    "$s=New-Object IO.MemoryStream(,$bytes);$r=New-Object K6Parent.FrameReader($s);"
                    "$bad=$false;try{$r.Poll(1000)|Out-Null}catch{$bad=$true};if(!$bad){throw 'fixture'}};"
                    "$bad=$false;try{Read-FrameText '{\"v\":true}'|Out-Null}catch{$bad=$true};"
                    "if(!$bad){throw 'fixture'};'OK'")

    def test_real_complete_and_unjoined_handle_paths_on_process_fixtures(self):
        self.run_ps("Initialize-Native;function Get-ProcessObservation($p){return $script:fact};"
                    "$script:fact=[pscustomobject]@{Pid=5;Created=7;Image='fixture';Sid='s';Wait=0;Exited=9};"
                    "foreach($complete in @($true,$false)){"
                    "$p=[pscustomobject]@{StandardInput=(New-Object IO.MemoryStream);StandardOutput=(New-Object IO.MemoryStream);"
                    "StandardError=(New-Object IO.MemoryStream);ExitCode=0;done=$complete;disposed=$false};"
                    "$p|Add-Member ScriptMethod WaitForExit {param($n) return $this.done};"
                    "$p|Add-Member ScriptMethod Dispose {$this.disposed=$true};"
                    "$o=New-Object K6Parent.FrameReader($p.StandardOutput);$e=New-Object K6Parent.FrameReader($p.StandardError);"
                    "$j=New-Object IO.MemoryStream;"
                    "$r=Complete-OwnedInvocation $p $script:fact $o $e $j 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa' $complete 100;"
                    "if($r.Joined -ne $complete -or $p.disposed -ne $complete){throw 'fixture'};"
                    "if($j.Length -eq 0){throw 'fixture'}};'OK'")

    def test_native_argv_roundtrip_empty_quotes_and_trailing_backslashes(self):
        self.run_ps("Initialize-Native;$values=@('','plain','a b','a\"b','tail\','with space\','a\\\"b','%PATH%');"
                    "$line=($values|ForEach-Object{Quote-WindowsArgument $_}) -join ' ';"
                    "$actual=[K6Parent.Native]::Arguments($line);"
                    "if($actual.Length -ne $values.Length){throw 'fixture'};"
                    "for($i=0;$i -lt $values.Length;$i++){if($values[$i] -cne $actual[$i]){throw 'fixture'}};'OK'")

    def test_ci_veto_runs_before_configuration_or_process(self):
        self.run_ps("$env:CI='1';$bad=$false;try{Invoke-PrivateParent 'CONFIDENTIAL_SENTINEL' 'capture'|Out-Null}"
                    "catch{$bad=$true;if($_.Exception.Message -cne 'PARENT_REFUSED'){throw 'fixture'}};"
                    "if(!$bad){throw 'fixture'};'OK'")

    def test_entry_failure_does_not_echo_parameter_sentinel(self):
        script = ROOT / "local-private-parent.ps1"
        self.assertTrue(script.is_file())
        executable = Path(os.environ["SystemRoot"]) / "System32/WindowsPowerShell/v1.0/powershell.exe"
        run = subprocess.run([str(executable), "-NoLogo", "-NoProfile", "-NonInteractive", "-File", str(script),
                              "-Mode", "CONFIDENTIAL_SENTINEL", "-Profile", "CONFIDENTIAL_SENTINEL"],
                             capture_output=True, timeout=20)
        self.assertEqual(run.returncode, 2)
        self.assertTrue(run.stdout.strip() == b"PARENT_REFUSED", "Entry output withheld")
        self.assertTrue(run.stderr == b"", "Entry stderr withheld")
