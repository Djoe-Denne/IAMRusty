"""Owned run-data cleanup only; no executable locks, ACL or sandbox probes."""
import os
import unittest

import test_runner_windows as base


@unittest.skipUnless(os.name == "nt", "native Windows cleanup only")
class WindowsCleanupTests(unittest.TestCase):
    ps = base.WindowsTests.ps
    value = base.WindowsTests.value

    @classmethod
    def setUpClass(cls):
        base.WindowsTests.setUpClass.__func__(cls)

    @classmethod
    def tearDownClass(cls):
        base.WindowsTests.tearDownClass.__func__(cls)

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
                f"[void][IO.Directory]::CreateDirectory({base.quote(path)});[IO.File]::WriteAllText({base.quote(path / 'foreign.txt')},'winner'); "
                "try {Remove-K6OwnedTree $o;exit 5} catch {if($_.Exception.Message-notmatch 'CLEANUP_UNPROVEN'){throw}}; "
                f"if(-not [IO.File]::Exists({base.quote(path / 'foreign.txt')})){{exit 6}};'FOREIGN_PRESERVED'")
        self.assertEqual(self.ps(code), "FOREIGN_PRESERVED")

    def test_failure_keeps_only_private_diagnostics_not_profile_or_sources(self):
        path = self.directory / "failed-output"
        code = (f"$o=[K6Windows.OwnedDirectory]::Create({base.quote(path)}); try {{ "
                f"$root={base.quote(path)};[void][IO.Directory]::CreateDirectory((Join-Path $root 'profile')); "
                "foreach($n in @('summary.json','source.js','child-1.stdout.log','child-1.stderr.log','process-1234567890abcdef1234567890abcdef.json','profile\\child-2.stdout.log')){[IO.File]::WriteAllText((Join-Path $root $n),'known')}; "
                "Remove-K6OwnedTree $o -KeepDiagnostics; $files=@(Get-ChildItem -LiteralPath $root -Force); "
                "if($files.Count-ne 3-or @($files|Where-Object {$_.PSIsContainer}).Count-ne 0){exit 5}; "
                "if([IO.File]::Exists((Join-Path $root 'summary.json'))-or [IO.File]::Exists((Join-Path $root 'source.js'))){exit 6}; "
                "Remove-K6OwnedTree $o; if([IO.Directory]::Exists($root)){exit 7};'DIAGNOSTICS_ONLY' } finally {$o.Dispose()}")
        self.assertEqual(self.ps(code), "DIAGNOSTICS_ONLY")


if __name__ == "__main__":
    unittest.main()
