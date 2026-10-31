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

    def test_parent_diagnostic_catch_bounded_summary_retention_and_io_failure(self):
        path = self.directory / "parent-diagnostic"
        code = (f"$root={base.quote(path)}; "
                "foreach($size in @(-1,0,2,65536,65537)){ "
                "$o=[K6Windows.OwnedDirectory]::Create($root); try { "
                "$summary=Join-Path $root 'summary.json'; $saved=Join-Path $root 'parent-summary.json'; "
                "if($size-ge 0){[IO.File]::WriteAllBytes($summary,[byte[]]::new($size))}; "
                "try {$null=[int]::Parse('PRIVATE_SENTINEL https://private.invalid/')} catch { "
                "$original=$_; $originalMessage=$_.Exception.Message; Write-K6ParentDiagnostic $_ $root; "
                "Write-K6ParentDiagnostic $_ (Join-Path $root 'does-not-exist'); "
                "if($original.Exception.Message-cne $originalMessage){exit 5} }; "
                "$errorPath=Join-Path $root 'parent-error.json'; $text=[IO.File]::ReadAllText($errorPath); "
                "Assert-K6Privacy $text @('PRIVATE_SENTINEL'); if($text.Length-gt 2048){exit 6}; "
                "$v=ConvertFrom-K6Json $text; Assert-K6Fields $v @('exception_type','error_id','script_line','summary_present','summary_bytes','summary_copied'); "
                "if($v.exception_type-cne $original.Exception.GetType().FullName-or $v.error_id-cne $original.FullyQualifiedErrorId-or $v.error_id.Length-eq 0-or $v.script_line-le 0){exit 7}; "
                "if($v.summary_present-ne ($size-ge 0)-or $v.summary_copied-ne ($size-ge 0-and $size-le 65536)){exit 8}; "
                "if($size-ge 0){if($v.summary_bytes-ne $size){exit 9}}elseif($null-ne $v.summary_bytes){exit 10}; "
                "if($v.summary_copied){if([IO.FileInfo]::new($saved).Length-ne $size){exit 11}}elseif([IO.File]::Exists($saved)){exit 12}; "
                "[void][IO.Directory]::CreateDirectory((Join-Path $root 'profile')); "
                "[IO.File]::WriteAllText((Join-Path $root 'profile\\known.txt'),'known'); "
                "Remove-K6OwnedTree $o -KeepDiagnostics; "
                "if([IO.File]::Exists($summary)-or [IO.Directory]::Exists((Join-Path $root 'profile'))-or -not [IO.File]::Exists($errorPath)){exit 13}; "
                "if($v.summary_copied-and -not [IO.File]::Exists($saved)){exit 14}; "
                "Remove-K6OwnedTree $o } finally {$o.Dispose()} }; 'PRIVATE_DIAGNOSTICS_OK'")
        self.assertEqual(self.ps(code), "PRIVATE_DIAGNOSTICS_OK")


if __name__ == "__main__":
    unittest.main()
