"""S-1/R-1/R-2 regressions: owned disposable directories and inert ZIP bytes only."""
import hashlib
import json
import os
from pathlib import Path
import unittest
import zipfile

import test_runner_windows as base


@unittest.skipUnless(os.name == "nt", "native Windows ownership and sharing only")
class WindowsSecurityTests(unittest.TestCase):
    ps = base.WindowsTests.ps
    value = base.WindowsTests.value

    @classmethod
    def setUpClass(cls):
        base.WindowsTests.setUpClass.__func__(cls)

    @classmethod
    def tearDownClass(cls):
        base.WindowsTests.tearDownClass.__func__(cls)

    def test_two_synchronized_creators_only_one_owns_witness(self):
        fixture = r"""
public static class DirectoryRaceFixture {
    public static int[] Run(System.Type factory, string path) {
        int successes=0, refusals=0;
        var gate=new System.Threading.Barrier(2);
        System.Threading.Thread[] threads=new System.Threading.Thread[2];
        for(int i=0;i<2;i++) {
            int actor=i;
            threads[i]=new System.Threading.Thread(()=>{
                try {
                    if(!gate.SignalAndWait(5000)) throw new System.Exception("BARRIER");
                    factory.GetMethod("CreatePrivateDirectory").Invoke(null,new object[]{path});
                    System.IO.File.WriteAllText(System.IO.Path.Combine(path,"owner-"+actor+".txt"),"owned witness");
                    System.Threading.Interlocked.Increment(ref successes);
                } catch { System.Threading.Interlocked.Increment(ref refusals); }
            });
            threads[i].Start();
        }
        foreach(var t in threads) if(!t.Join(6000)) throw new System.Exception("THREAD_NOT_JOINED");
        gate.Dispose();
        return new int[]{successes,refusals,System.IO.Directory.GetFiles(path).Length};
    }
}
"""
        path = self.directory / "two-creators"
        code = (f"Add-Type -TypeDefinition {base.quote(fixture)}; "
                f"$r=[DirectoryRaceFixture]::Run([K6Windows.PathLease],{base.quote(path)}); "
                "if($r[0]-ne 1-or $r[1]-ne 1-or $r[2]-ne 1){throw ('EXCLUSIVE_REQUIRED:'+($r-join ','))};'ONE_OWNER'")
        self.assertEqual(self.ps(code), "ONE_OWNER")

    def test_owned_cleanup_nominal_and_reuse_refused(self):
        path = self.directory / "owned-output"
        code = (f"$o=[K6Windows.OwnedDirectory]::Create({base.quote(path)}); "
                f"try {{[IO.File]::WriteAllText({base.quote(path / 'known.txt')},'known'); "
                f"[IO.File]::WriteAllText({base.quote(path / 'known.json')},'{{\"version\":1}}'); "
                f"$v=Read-K6PrivateJson {base.quote(path / 'known.json')} $o;if($v.version-ne 1){{exit 8}}; "
                f"try {{[K6Windows.OwnedDirectory]::Create({base.quote(path)});exit 5}} catch {{}}; "
                "Remove-K6OwnedTree $o; if([IO.Directory]::Exists($o.Path)){exit 6};'REMOVED_OWNED' } finally {$o.Dispose()}")
        self.assertEqual(self.ps(code), "REMOVED_OWNED")

    def test_binding_failure_cannot_authorize_cleanup(self):
        path = self.directory / "unbound-output"
        # The private constructor performs the actual native create. Bind is the
        # real factory step, exercised with an unavailable observation handle.
        # Restore the retained OS handle before disposal; no foreign handle closes.
        code = (f"$o=[Activator]::CreateInstance([K6Windows.OwnedDirectory],[Reflection.BindingFlags]'NonPublic,Instance',$null,@({base.quote(path)}),$null); "
                "$flags=[Reflection.BindingFlags]'NonPublic,Instance'; $f=$o.GetType().GetField('handle',$flags);$h=$f.GetValue($o); "
                f"[IO.File]::WriteAllText({base.quote(path / 'preserve.txt')},'known'); "
                "try {$f.SetValue($o,[IntPtr]::Zero); try {$o.GetType().GetMethod('Bind',$flags).Invoke($o,@());exit 5} catch {}; "
                "$f.SetValue($o,$h);try {Remove-K6OwnedTree $o;exit 6} catch {if($_.Exception.Message-notmatch 'CLEANUP_UNPROVEN'){throw}}; "
                f"if(-not [IO.File]::Exists({base.quote(path / 'preserve.txt')})){{exit 7}};'PRESERVED' "
                "} finally {$f.SetValue($o,$h);$o.Dispose()}")
        self.assertEqual(self.ps(code), "PRESERVED")

    def test_disposed_identity_does_not_adopt_replacement(self):
        path = self.directory / "replaced-output"; moved = self.directory / "original-output"
        code = (f"$o=[K6Windows.OwnedDirectory]::Create({base.quote(path)});$o.Dispose(); "
                f"[IO.Directory]::Move({base.quote(path)},{base.quote(moved)}); "
                f"[K6Windows.PathLease]::CreatePrivateDirectory({base.quote(path)});[IO.File]::WriteAllText({base.quote(path / 'foreign.txt')},'winner'); "
                "try {Remove-K6OwnedTree $o;exit 5} catch {if($_.Exception.Message-notmatch 'CLEANUP_UNPROVEN'){throw}}; "
                f"if(-not [IO.File]::Exists({base.quote(path / 'foreign.txt')})){{exit 6}};'FOREIGN_PRESERVED'")
        self.assertEqual(self.ps(code), "FOREIGN_PRESERVED")

    def bundle(self, name, entries=None):
        root = self.directory / name
        folder = root / "chrome-win64"; folder.mkdir(parents=True)
        pe = bytearray(128); pe[:2] = b"MZ"; pe[60:64] = (64).to_bytes(4, "little")
        pe[64:68] = b"PE\0\0"; pe[68:70] = (0x8664).to_bytes(2, "little"); pe[88:90] = (0x20B).to_bytes(2, "little")
        members = [("chrome-win64/chrome.exe", bytes(pe)), ("chrome-win64/chrome.dll", b"INERT DATA, never loaded")]
        if entries:
            members.extend(entries)
        archive = root / "chrome-win64.zip"
        with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_STORED) as z:
            for member, data in members:
                z.writestr(member, data)
        (folder / "chrome.exe").write_bytes(pe); (folder / "chrome.dll").write_bytes(members[1][1])
        pin = base.lock()["chromium"]
        pin["executable_sha256"] = hashlib.sha256(pe).hexdigest()
        pin["archive_sha256"] = hashlib.sha256(archive.read_bytes()).hexdigest()
        return folder, pin

    def refusal(self, folder, pin):
        expression = f"$t=Open-K6PinnedTool {self.value(pin)} {base.quote(folder / 'chrome.exe')}; "
        code = "$t=$null;try {try {" + expression + "throw 'UNEXPECTED_PREFLIGHT_SUCCESS'} catch {if($_.Exception.Message-notmatch 'BUNDLE_INVALID'){throw}};'REFUSED_BEFORE_PROCESS'} finally {if($null-ne $t){if(Get-Command Close-K6PinnedTool -ErrorAction SilentlyContinue){Close-K6PinnedTool $t}else{$t.Exe.Dispose();$t.Archive.Dispose()}}}"
        self.assertEqual(self.ps(code), "REFUSED_BEFORE_PROCESS")

    def test_real_preflight_rejects_changed_dll_with_exe_and_archive_intact(self):
        folder, pin = self.bundle("changed-dll")
        (folder / "chrome.dll").write_bytes(b"different INERT bytes, never loaded")
        self.refusal(folder, pin)

    def test_closed_bundle_refuses_extra_missing_and_ambiguous_entries(self):
        folder, pin = self.bundle("extra-dll"); (folder / "extra.dll").write_bytes(b"inert extra")
        self.refusal(folder, pin)
        folder, pin = self.bundle("missing-dll"); (folder / "chrome.dll").unlink()
        self.refusal(folder, pin)
        for name, member in (("collision", "chrome-win64/CHROME.dll"), ("traversal", "chrome-win64/../escape.dll"),
                             ("ambiguous-directory", "chrome-win64//")):
            folder, pin = self.bundle(name, [(member, b"" if member.endswith("/") else b"never extracted")])
            self.refusal(folder, pin)

    def test_real_bundle_pins_prevent_dependency_rewrite_until_release(self):
        folder, pin = self.bundle("locked-dll")
        code = (f"$t=Open-K6PinnedTool {self.value(pin)} {base.quote(folder / 'chrome.exe')}; "
                "try {Assert-K6PinnedTool $t; "
                f"try {{[IO.File]::WriteAllText({base.quote(folder / 'chrome.dll')},'not executable');exit 5}} catch {{}}; "
                "Assert-K6PinnedTool $t;'PINNED' } finally {Close-K6PinnedTool $t}")
        self.assertEqual(self.ps(code), "PINNED")

    def test_primary_sandbox_boolean_never_qualifies_chromium(self):
        from test_browser_version_windows import current_summary
        state = dict(Cleanup=True, Privacy=True, Executed=1)
        child = dict(ExitCode=0, Cleanup=True, IdentityComplete=True, CaptureComplete=True, Overflow=False,
                     Sandbox=True, BrowserArguments=True)
        code = ("$p=[ordered]@{tools='INCONCLUSIVE';chromium='INCONCLUSIVE';versions=@{chromium=$null};browser_version_evidence='NOT_PROVEN'}; "
                f"Set-K6BrowserEvidence $p {self.value(current_summary())} '{base.RUN_ID}' {self.value(child)} {self.value(state)} '156.0.8078.73'; "
                "if($p.tools-cne 'PASS'-or $p.chromium-cne 'INCONCLUSIVE'){exit 5};'PRIMARY_ONLY_INCONCLUSIVE'")
        self.assertEqual(self.ps(code), "PRIMARY_ONLY_INCONCLUSIVE")


if __name__ == "__main__":
    unittest.main()
