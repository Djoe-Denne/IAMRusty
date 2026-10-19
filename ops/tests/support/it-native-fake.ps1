#Requires -Version 5.1
# Reusable hermetic shim for the IT launcher tests (mission 20261019-it-fixtures-lifecycle).
# Ported from the proven ephemeral smoke .tmp/f1-hermetic.ps1 and extended:
# per-ID stop/inspect overrides, prune detection, RUST_TEST_THREADS recording,
# opaque-start-failure ledger events, Engine --since/--until time filtering.
# Zero real Docker daemon, zero real Cargo: a file-driven fake docker.exe/cargo.exe
# compiled once per session stands in for both. Dot-sourcing performs no operation.
Set-StrictMode -Version Latest

$script:ItFakeBinDir = $null

$script:ItFakeCSharp = @'
using System;
using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Threading;

// File-driven fake for docker.exe/cargo.exe. The state directory comes from the
// ITFAKE_STATE environment variable so one binary serves every sandbox.
class ItFakeRuntime
{
    static string StateDir { get { return Environment.GetEnvironmentVariable("ITFAKE_STATE") ?? ""; } }
    static string P(string name) { return Path.Combine(StateDir, name); }

    static string IdForPackage(string pkg)
    {
        // The launcher passes Cargo package names (iam-service, ...); map them
        // through the suite name. Letters must stay in [a-f0-9] because the
        // launcher validates full IDs against ^[a-f0-9]{64}$.
        switch (pkg.Replace("-service", ""))
        {
            case "iam": return new string('a', 64);
            case "hive": return new string('b', 64);
            case "manifesto": return new string('c', 64);
            case "telegraph": return new string('d', 64);
            case "lazaret": return new string('e', 64);
            default: return new string('9', 64);
        }
    }

    static void AppendLine(string path, string text)
    {
        using (var w = new StreamWriter(new FileStream(path, FileMode.Append, FileAccess.Write, FileShare.ReadWrite)))
        {
            w.Write(text + "\n");
        }
    }

    static string[] ReadLines(string path)
    {
        if (!File.Exists(path)) return new string[0];
        using (var r = new StreamReader(new FileStream(path, FileMode.Open, FileAccess.Read, FileShare.ReadWrite)))
        {
            return r.ReadToEnd().Split(new[] { '\n' }, StringSplitOptions.RemoveEmptyEntries);
        }
    }

    static void EmitEvent(string id, string action)
    {
        AppendLine(P("events"), "{\"Actor\":{\"ID\":\"" + id + "\"},\"Action\":\"" + action + "\",\"timeNano\":" + DateTime.UtcNow.Ticks + "}");
    }

    static string Override(string name)
    {
        string path = P(name);
        return File.Exists(path) ? File.ReadAllText(path).Trim() : null;
    }

    static long TimeNanoOf(string line)
    {
        int i = line.IndexOf("\"timeNano\":");
        if (i < 0) return 0;
        long v;
        long.TryParse(line.Substring(i + 11).Trim().TrimEnd('}', ','), out v);
        return v;
    }

    static long ArgTime(string value)
    {
        return DateTime.Parse(value, CultureInfo.InvariantCulture, DateTimeStyles.RoundtripKind).ToUniversalTime().Ticks;
    }

    static int Main(string[] args)
    {
        if (Path.GetFileNameWithoutExtension(Assembly.GetEntryAssembly().Location) == "cargo")
        {
            string pkg = args[2];
            string id = IdForPackage(pkg);
            string run = Environment.GetEnvironmentVariable("RUSTYCOG_TEST_RUN_ID") ?? "";
            string ledgerDir = Environment.GetEnvironmentVariable("RUSTYCOG_TEST_LEDGER_DIR") ?? "";
            string threads = Environment.GetEnvironmentVariable("RUST_TEST_THREADS") ?? "";
            string fail = Environment.GetEnvironmentVariable("ITFAKE_FAIL");
            string opaque = Environment.GetEnvironmentVariable("ITFAKE_OPAQUE");
            AppendLine(P("calls"), "cargo " + pkg + " threads=" + threads + (fail == "1" ? " fail=1" : "") + (opaque == "1" ? " opaque=1" : ""));
            string ledger = Path.Combine(ledgerDir, "fixture-" + run + "-" + Process.GetCurrentProcess().Id + ".jsonl");
            if (opaque == "1")
            {
                AppendLine(ledger, "{\"run_id\":\"" + run + "\",\"event\":\"unknown_opaque_start_failure\",\"container_id\":null}");
                return 7;
            }
            File.WriteAllText(P(id), "running");
            EmitEvent(id, "create");
            EmitEvent(id, "start");
            AppendLine(ledger, "{\"run_id\":\"" + run + "\",\"event\":\"created\",\"container_id\":\"" + id + "\"}");
            Thread.Sleep(500);
            return fail == "1" ? 7 : 0;
        }
        AppendLine(P("calls"), "docker " + string.Join(" ", args));
        if (args.Any(a => a == "prune"))
        {
            File.WriteAllText(P("PRUNE-DETECTED"), string.Join(" ", args));
            Console.Error.WriteLine("destructive prune invoked");
            return 42;
        }
        if (args[0] == "info") { Console.WriteLine(DateTime.UtcNow.ToString("o")); return 0; }
        if (args[0] == "ps")
        {
            foreach (string f in Directory.GetFiles(StateDir))
            {
                string name = Path.GetFileName(f);
                if (name.Length != 64) continue;
                string state = File.ReadAllText(f).Trim();
                Console.WriteLine(name + "|" + state + "|" + (state == "running" ? "Up 2 minutes" : "Exited (0)"));
            }
            return 0;
        }
        if (args[0] == "events")
        {
            long since = long.MinValue;
            long until = long.MaxValue;
            for (int i = 0; i < args.Length - 1; i++)
            {
                if (args[i] == "--since") since = ArgTime(args[i + 1]);
                if (args[i] == "--until") until = ArgTime(args[i + 1]);
            }
            if (args.Contains("--until"))
            {
                foreach (string line in ReadLines(P("events")))
                {
                    long t = TimeNanoOf(line);
                    if (t >= since && t <= until) Console.WriteLine(line);
                }
                return 0;
            }
            int count = 0;
            while (true)
            {
                string[] lines = ReadLines(P("events"));
                for (; count < lines.Length; count++)
                {
                    long t = TimeNanoOf(lines[count]);
                    if (t >= since && t <= until) Console.WriteLine(lines[count]);
                }
                Console.Out.Flush();
                Thread.Sleep(20);
            }
        }
        string id2 = args[args.Length - 1];
        if (args[0] == "stop")
        {
            string mode = Override("stop-" + id2 + ".txt");
            if (mode == "refuse") { Console.Error.WriteLine("fake stop refused"); return 1; }
            if (mode == "stuck") return 0;
            File.WriteAllText(P(id2), "exited");
            EmitEvent(id2, "die");
            return 0;
        }
        if (args[0] == "inspect")
        {
            Console.WriteLine(Override("inspect-" + id2 + ".txt") ?? "exited|0");
            return 0;
        }
        if (args[0] == "rm")
        {
            string path = P(id2);
            if (!File.Exists(path) || File.ReadAllText(path).Trim() != "exited") return 8;
            File.Delete(path);
            EmitEvent(id2, "destroy");
            return 0;
        }
        return 9;
    }
}
'@

function Get-ItFakeBinaries {
    # Compile once per session, then copy the same file-driven binaries into
    # every sandbox (the behavior lives in ITFAKE_STATE files, not in the binary).
    if ($null -ne $script:ItFakeBinDir -and (Test-Path -LiteralPath (Join-Path $script:ItFakeBinDir 'docker.exe'))) {
        return $script:ItFakeBinDir
    }
    $dir = Join-Path $env:LOCALAPPDATA ('Temp\opencode\itfake-bin-' + [guid]::NewGuid().ToString('N').Substring(0, 8))
    $null = New-Item -ItemType Directory -Path $dir -Force
    Add-Type -TypeDefinition $script:ItFakeCSharp -Language CSharp -OutputAssembly (Join-Path $dir 'docker.exe') -OutputType ConsoleApplication
    Copy-Item (Join-Path $dir 'docker.exe') (Join-Path $dir 'cargo.exe')
    $script:ItFakeBinDir = $dir
    return $dir
}

function New-ItFakeSandbox {
    param([switch]$NoTmp)
    $binDir = Get-ItFakeBinaries
    $root = Join-Path $env:LOCALAPPDATA ('Temp\opencode\itfake-' + [guid]::NewGuid().ToString('N').Substring(0, 8))
    $null = New-Item -ItemType Directory -Path (Join-Path $root 'ops/tests'), (Join-Path $root 'ops/deploy'), (Join-Path $root 'bin'), (Join-Path $root 'state') -Force
    if (-not $NoTmp) { $null = New-Item -ItemType Directory -Path (Join-Path $root '.tmp') -Force }
    Copy-Item (Join-Path $PSScriptRoot '..\it-native.ps1') (Join-Path $root 'ops/tests/it-native.ps1')
    Set-Content (Join-Path $root 'ops/deploy/common.ps1') 'function Get-AIForAllRoot { param($Path) (Resolve-Path (Join-Path $Path "../..")).Path }'
    Copy-Item (Join-Path $binDir 'docker.exe') (Join-Path $root 'bin/docker.exe')
    Copy-Item (Join-Path $binDir 'cargo.exe') (Join-Path $root 'bin/cargo.exe')
    return @{
        Root = $root
        Bin = (Join-Path $root 'bin')
        State = (Join-Path $root 'state')
        Tmp = (Join-Path $root '.tmp')
        Launcher = (Join-Path $root 'ops/tests/it-native.ps1')
        CallsPath = (Join-Path $root 'state/calls')
    }
}

function Get-ItFakePackageId {
    param([string]$Package, [switch]$Foreign)
    if ($Foreign) { return ('f' * 64) }
    $letter = switch ($Package) {
        'iam' { 'a' }
        'hive' { 'b' }
        'manifesto' { 'c' }
        'telegraph' { 'd' }
        'lazaret' { 'e' }
        default { throw "Unknown fake package: $Package" }
    }
    return ($letter * 64)
}

function Add-ItFakeForeignContainer {
    # Pre-existing foreign running container canary (e.g. a Kind control-plane
    # node or another campaign's container): must survive every campaign.
    param($Sandbox)
    [IO.File]::WriteAllText((Join-Path $Sandbox.State (Get-ItFakePackageId -Foreign)), 'running')
}

function New-ItFakeStopOverride {
    param($Sandbox, [string]$Package, [ValidateSet('stuck', 'refuse')][string]$Mode)
    [IO.File]::WriteAllText((Join-Path $Sandbox.State ('stop-' + (Get-ItFakePackageId $Package) + '.txt')), $Mode)
}

function New-ItFakeInspectOverride {
    param($Sandbox, [string]$Package, [string]$Response)
    [IO.File]::WriteAllText((Join-Path $Sandbox.State ('inspect-' + (Get-ItFakePackageId $Package) + '.txt')), $Response)
}

function Get-ItFakeCalls {
    param($Sandbox)
    if (Test-Path -LiteralPath $Sandbox.CallsPath) { return ,@(Get-Content -LiteralPath $Sandbox.CallsPath) }
    return ,@()
}

function Assert-ItDockerTargets {
    # Every docker stop/rm must target a full 64-hex ID, exactly once per expected
    # ID, and never a forbidden (foreign/retained) ID. Scoped from StartIndex so
    # multi-invocation sandboxes only judge their own calls. Catches stop/rm-by-
    # name and truncated-ID regressions (QA plan RQ1/RQ2).
    param($Sandbox, [string]$Action, [string[]]$RequiredIds = @(), [string[]]$ForbiddenIds = @(), [int]$StartIndex = 0)
    $calls = Get-ItFakeCalls $Sandbox
    $targets = @()
    for ($i = $StartIndex; $i -lt $calls.Count; $i++) {
        $call = $calls[$i]
        if ($call -like "docker $Action *") {
            $target = ($call -split ' ')[-1]
            if ($target -notmatch '^[a-f0-9]{64}$') { throw "docker $Action invoked without a full 64-hex ID: $call" }
            $targets += $target
        }
    }
    foreach ($id in $RequiredIds) {
        if ($targets -notcontains $id) { throw "Missing docker $Action for owned ID $id" }
    }
    foreach ($id in $ForbiddenIds) {
        if ($targets -contains $id) { throw "Forbidden docker $Action on retained/foreign ID $id" }
    }
}

function Assert-ItStopAfterCargo {
    # G-4 ordering oracle, scoped to one invocation: the finally-cleanup stop
    # runs after every cargo run of THAT invocation (the batch is fail-fast, so
    # a failing suite ends the cargo line but cleanup still runs).
    param($Calls, [int]$StartIndex = 0)
    $lastCargo = -1
    $firstStop = -1
    for ($i = $StartIndex; $i -lt $Calls.Count; $i++) {
        if ($Calls[$i] -like 'cargo *') { $lastCargo = $i }
        if ($firstStop -lt 0 -and $Calls[$i] -like 'docker stop *') { $firstStop = $i }
    }
    if ($lastCargo -lt 0 -or $firstStop -lt 0 -or $firstStop -lt $lastCargo) {
        throw "Stop was not executed after all cargo runs (last cargo: $lastCargo, first stop: $firstStop)"
    }
}

function Assert-ItNoPrune {
    # T-3d (dynamic): the shim records any prune invocation in a marker file.
    param($Sandbox)
    if (Test-Path -LiteralPath (Join-Path $Sandbox.State 'PRUNE-DETECTED')) {
        throw "Destructive prune reached the Docker shim: $((Get-Content (Join-Path $Sandbox.State 'PRUNE-DETECTED')))"
    }
}

function Assert-ItForeignUntouched {
    param($Sandbox)
    $foreign = Get-ItFakePackageId -Foreign
    $path = Join-Path $Sandbox.State $foreign
    if (-not (Test-Path -LiteralPath $path)) { throw "Foreign container state file vanished: $foreign" }
    if ((Get-Content -LiteralPath $path) -ne 'running') { throw "Foreign container $foreign was stopped or removed" }
}

function Assert-ItLauncherHasNoDestructiveCall {
    # T-3d (static): no command AST in the launcher may even spell a prune
    # subcommand, whatever the docker/cargo target or the indirection used.
    param([string]$LauncherPath)
    $tokens = $null
    $errors = $null
    $ast = [Management.Automation.Language.Parser]::ParseFile((Resolve-Path -LiteralPath $LauncherPath).Path, [ref]$tokens, [ref]$errors)
    if ($errors.Count -gt 0) { throw "Launcher parse failed: $($errors | ForEach-Object Message)" }
    $commands = $ast.FindAll({ param($node) $node -is [Management.Automation.Language.CommandAst] }, $true)
    foreach ($command in $commands) {
        if ($command.Extent.Text -match '(?i)prune') {
            throw "Destructive prune invocation present in launcher: $($command.Extent.Text)"
        }
    }
}

function Get-ItFakeRegistryPath {
    # Today's registry is the newest docker-resources-*.log in the sandbox.
    param($Sandbox)
    $files = @(Get-ChildItem -LiteralPath $Sandbox.Tmp -File -Filter 'docker-resources-*.log' | Sort-Object LastWriteTime)
    if ($files.Count -eq 0) { throw 'No docker-resources registry was created by the launcher' }
    return $files[-1].FullName
}

function Get-ItFakeRunDirs {
    # The comma operator keeps the array intact through the pipeline unroll.
    param($Sandbox)
    return ,@(Get-ChildItem -LiteralPath $Sandbox.Tmp -Directory -Filter 'itfixtures-*' | Sort-Object CreationTime)
}

function Get-ItFakeReport {
    # Parsed phase='after' inventory record of the Nth campaign invocation.
    param($Sandbox, [int]$RunIndex)
    $runs = Get-ItFakeRunDirs $Sandbox
    if ($runs.Count -lt $RunIndex) { throw "Expected at least $RunIndex campaign run directories, got $($runs.Count)" }
    $inventory = Join-Path $runs[$RunIndex - 1].FullName 'inventory.jsonl'
    $after = $null
    foreach ($line in (Get-Content -LiteralPath $inventory)) {
        $record = $line | ConvertFrom-Json
        if ($record.phase -eq 'after') { $after = $record }
    }
    if ($null -eq $after) { throw "No phase=after inventory record in $inventory" }
    return $after
}

function Invoke-ItFakeScenario {
    # Runs one hermetic launcher scenario against a fresh sandbox. The fake
    # docker/cargo shim is wired on PATH for the body and restored afterwards.
    # On success the sandbox is removed; on failure it is kept for triage and
    # the error message carries its path.
    param([string]$Name, [scriptblock]$Body)
    $sandbox = New-ItFakeSandbox
    $oldPath = $env:PATH
    $oldState = $env:ITFAKE_STATE
    $ok = $false
    try {
        $env:PATH = "$($sandbox.Bin);$oldPath"
        $env:ITFAKE_STATE = $sandbox.State
        & $Body $sandbox
        $ok = $true
        Write-Host "PASS scenario: $Name"
    }
    catch { throw "Scenario '$Name' failed (sandbox kept for triage: $($sandbox.Root)): $_" }
    finally {
        $env:PATH = $oldPath
        $env:ITFAKE_STATE = $oldState
        if ($ok) { Remove-Item -LiteralPath $sandbox.Root -Recurse -Force }
        else { Write-Warning "Sandbox kept for triage: $($sandbox.Root)" }
    }
}
