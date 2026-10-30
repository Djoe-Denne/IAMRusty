# Private implementation helpers; run-windows.ps1 is the only campaign entry point.
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Initialize-K6Windows {
    if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT -or [IntPtr]::Size -ne 8 -or
        $PSVersionTable.PSVersion.Major -ne 5 -or $PSVersionTable.PSVersion.Minor -ne 1) { throw 'PLATFORM_REFUSED' }
    if (-not ('K6Windows.Native' -as [type])) { Add-Type -Path (Join-Path $PSScriptRoot 'windows-native.cs') -ErrorAction Stop }
    if (-not [K6Windows.Native]::NativeX64) { throw 'PLATFORM_REFUSED' }
    if ([K6Windows.Native]::Elevated) { throw 'ELEVATION_REFUSED' }
}

function ConvertFrom-K6Json([string] $Text) {
    [K6Windows.JsonGuard]::Check($Text)
    return ConvertFrom-Json -InputObject $Text -ErrorAction Stop
}

function Assert-K6Fields($Value, [string[]] $Names) {
    if ($null -eq $Value -or $Value -isnot [pscustomobject]) { throw 'SCHEMA_INVALID' }
    $actual = @($Value.PSObject.Properties.Name | Sort-Object)
    $expected = @($Names | Sort-Object)
    if (($actual -join '|') -cne ($expected -join '|')) { throw 'SCHEMA_INVALID' }
}

function Assert-K6Integer($Value, [long] $Expected) {
    if (($Value -isnot [int] -and $Value -isnot [long]) -or $Value -ne $Expected) { throw 'COMPLETENESS_INVALID' }
}

function Assert-K6Privacy([string] $Text, [string[]] $Sentinels) {
    if ($Text.Length -gt 65536) { throw 'PRIVACY_REFUSED' }
    foreach ($sentinel in $Sentinels) {
        if ([string]::IsNullOrEmpty($sentinel) -or $Text.IndexOf($sentinel, [StringComparison]::OrdinalIgnoreCase) -ge 0) { throw 'PRIVACY_REFUSED' }
    }
    # These data never belong in a public functional projection. Native streams
    # remain private even when this bounded sentinel/lexical check succeeds.
    if ($Text -match '(?i)https?://|[A-Z]:[\\/]|bearer\s|eyJ[A-Za-z0-9_-]{8,}\.|(?:password|authorization|cookie|access_token|refresh_token|id_token|client_secret|code_verifier)\s*[:=]|(?:code|state|verifier)=[^\s&]+|S-1-5-21-') { throw 'PRIVACY_REFUSED' }
}

function Read-K6PrivateJson([string] $Path) {
    $pin = [K6Windows.PathLease]::new($Path, $false, $true)
    try {
        if ([IO.FileInfo]::new($pin.Path).Length -gt 65536) { throw 'JSON_INVALID' }
        $bytes = [IO.File]::ReadAllBytes($pin.Path)
        if ($bytes.Length -gt 65536) { throw 'JSON_INVALID' }
        $text = [Text.UTF8Encoding]::new($false, $true).GetString($bytes)
        return ConvertFrom-K6Json $text
    } finally { $pin.Dispose() }
}

function Assert-K6OutsideGit([string] $Path) {
    $current = [IO.Path]::GetDirectoryName([K6Windows.PathLease]::Absolute($Path))
    while ($current) {
        if (Test-Path -LiteralPath (Join-Path $current '.git')) { throw 'PATH_INVALID' }
        $current = [IO.Path]::GetDirectoryName($current)
    }
}

function Read-K6WindowsLock([string] $Path) {
    if (-not [IO.File]::Exists($Path)) { throw 'WINDOWS_LOCK_MISSING' }
    $pin = [K6Windows.PathLease]::new($Path, $false, $false)
    try {
        if ([IO.FileInfo]::new($pin.Path).Length -gt 65536) { throw 'JSON_INVALID' }
        $lock = ConvertFrom-K6Json ([IO.File]::ReadAllText($Path, [Text.UTF8Encoding]::new($false, $true)))
    }
    finally { $pin.Dispose() }
    Assert-K6Fields $lock @('version', 'platform', 'k6', 'chromium')
    Assert-K6Integer $lock.version 1
    if ($lock.platform -isnot [string] -or $lock.platform -cne 'windows-amd64') { throw 'PIN_INVALID' }
    Assert-K6Fields $lock.k6 @('version', 'url', 'checksum_url', 'archive', 'archive_sha256', 'executable', 'executable_sha256')
    Assert-K6Fields $lock.chromium @('version', 'revision', 'url', 'metadata_url', 'archive', 'archive_sha256', 'executable', 'executable_sha256')
    foreach ($tool in @($lock.k6, $lock.chromium)) {
        foreach ($property in $tool.PSObject.Properties) { if ($property.Value -isnot [string]) { throw 'PIN_INVALID' } }
    }
    if ($lock.k6.version -cne '2.3.0' -or $lock.k6.url -cne 'https://github.com/grafana/k6/releases/download/v2.3.0/k6-v2.3.0-windows-amd64.zip' -or
        $lock.k6.checksum_url -cne 'https://github.com/grafana/k6/releases/download/v2.3.0/k6-v2.3.0-checksums.txt' -or
        $lock.k6.archive -cne 'k6-v2.3.0-windows-amd64.zip' -or $lock.k6.executable -cne 'k6-v2.3.0-windows-amd64\k6.exe' -or
        $lock.k6.archive_sha256 -cne '112276d495e5741c968e2bc09ea6196099c1275bd6db9ee0875d173c7148ce43' -or
        $lock.chromium.version -cne '156.0.8078.73' -or $lock.chromium.revision -cne '1706539' -or
        $lock.chromium.url -cne 'https://storage.googleapis.com/chrome-for-testing-public/156.0.8078.73/win64/chrome-win64.zip' -or
        $lock.chromium.metadata_url -cne 'https://googlechromelabs.github.io/chrome-for-testing/156.0.8078.73.json' -or
        $lock.chromium.archive -cne 'chrome-win64.zip' -or $lock.chromium.executable -cne 'chrome-win64\chrome.exe') { throw 'PIN_INVALID' }
    foreach ($tool in @($lock.k6, $lock.chromium)) {
        foreach ($hash in @($tool.archive_sha256, $tool.executable_sha256)) {
            if ($hash -isnot [string] -or $hash -cnotmatch '^[a-f0-9]{64}$' -or $hash -match '^0{64}$') { throw 'PIN_INVALID' }
        }
    }
    return $lock
}

function Open-K6PinnedTool($Pin, [string] $Executable) {
    $path = [K6Windows.PathLease]::Absolute($Executable)
    $suffix = '\' + $Pin.executable
    if (-not $path.EndsWith($suffix, [StringComparison]::OrdinalIgnoreCase)) { throw 'PIN_INVALID' }
    $root = $path.Substring(0, $path.Length - $suffix.Length)
    $exePin = $null; $zipPin = $null
    try {
        $exePin = [K6Windows.PathLease]::new($path, $false, $true)
        $zipPin = [K6Windows.PathLease]::new((Join-Path $root $Pin.archive), $false, $true)
        if ($exePin.Hash() -cne $Pin.executable_sha256 -or $zipPin.Hash() -cne $Pin.archive_sha256) { throw 'PIN_INVALID' }
        $stream = [IO.File]::Open($path, 'Open', 'Read', 'Read')
        try {
            $reader = [IO.BinaryReader]::new($stream)
            if ($reader.ReadUInt16() -ne 0x5a4d) { throw 'PE_INVALID' }
            $stream.Position = 0x3c; $offset = $reader.ReadUInt32()
            if ($offset -lt 64 -or $offset -gt ($stream.Length - 26)) { throw 'PE_INVALID' }
            $stream.Position = $offset
            if ($reader.ReadUInt32() -ne 0x4550 -or $reader.ReadUInt16() -ne 0x8664) { throw 'PE_INVALID' }
            $stream.Position = $offset + 24
            if ($reader.ReadUInt16() -ne 0x20b) { throw 'PE_INVALID' }
        } finally { $stream.Dispose() }
        return @{ Exe = $exePin; Archive = $zipPin; Pin = $Pin }
    } catch {
        if ($null -ne $exePin) { $exePin.Dispose() }; if ($null -ne $zipPin) { $zipPin.Dispose() }; throw
    }
}

function New-K6Environment([string] $Directory, [string] $RunId, [string] $Chrome, [string[]] $Sentinels) {
    # A fresh block, never a copy of the parent environment. No PATH lookup,
    # proxy, K6_* override, loader, remote browser or user browser profile adopted.
    $e = [Collections.Generic.Dictionary[string,string]]::new([StringComparer]::OrdinalIgnoreCase)
    $system = [Environment]::GetFolderPath('Windows')
    $e.Add('SystemRoot', $system); $e.Add('WINDIR', $system)
    $e.Add('PATH', (Join-Path $system 'System32'))
    foreach ($key in @('TEMP', 'TMP', 'USERPROFILE', 'LOCALAPPDATA', 'APPDATA', 'HOME')) { $e.Add($key, $Directory) }
    $e.Add('K6_NO_USAGE_REPORT', 'true'); $e.Add('K6_NO_COLOR', 'true')
    $e.Add('K6_RUN_ID', $RunId); $e.Add('K6_SUMMARY_PATH', (Join-Path $Directory 'summary.json'))
    $e.Add('K6_BROWSER_EXECUTABLE_PATH', $Chrome); $e.Add('K6_BROWSER_HEADLESS', 'true')
    $e.Add('K6_BROWSER_TIMEOUT', '10s')
    # A private sentinel is deliberately present at the real child boundary.
    $e.Add('K6_PRIVACY_SENTINEL', $Sentinels[0])
    return ,$e
}

function Invoke-K6Child($Tool, [string[]] $Arguments, $Environment, [string] $Directory, [string[]] $Sentinels, [int] $TimeoutMs, $State, [string] $Chrome = $null) {
    # Rehash under the retained identity/name-chain handles at every actual callsite.
    if ($Tool.Exe.Hash() -cne $Tool.Pin.executable_sha256 -or $Tool.Archive.Hash() -cne $Tool.Pin.archive_sha256) { throw 'PIN_INVALID' }
    $State.Cleanup = $false; $State.Privacy = $false
    $child = [K6Windows.Child]::new($Tool.Exe.Path, $Arguments, $Environment, $Directory)
    $State.Executed = $State.Executed + 1
    $leasePath = Join-Path $Directory ('process-' + [Guid]::NewGuid().ToString('N') + '.json')
    try {
        # Persist the live observations before the bounded operation. A lease is
        # evidence, never authority to adopt a PID during recovery.
        $records = @($child.Identities | ForEach-Object { @{ pid = $_.Pid; created = $_.Created; sid = $_.Sid; image = $_.Image; ended = $false } })
        [IO.File]::WriteAllText($leasePath, (@{ version = 1; status = 'created-suspended'; observed_utc = [DateTime]::UtcNow.ToString('o'); identities = $records } | ConvertTo-Json -Depth 6 -Compress), [Text.UTF8Encoding]::new($false))
        $r = $child.Finish($TimeoutMs, 3000, $Chrome)
        $State.Cleanup = $r.Cleanup
        # Raw identities stay only in the private run lease; never on stdout.
        $records = @($r.Identities | ForEach-Object { @{ pid = $_.Pid; created = $_.Created; sid = $_.Sid; image = $_.Image; ended = $_.Ended } })
        $lease = @{ version = 1; observed_utc = [DateTime]::UtcNow.ToString('o'); identities = $records; total = $r.Total; active = $r.Active; cleanup = $r.Cleanup }
        [IO.File]::WriteAllText($leasePath, ($lease | ConvertTo-Json -Depth 6 -Compress), [Text.UTF8Encoding]::new($false))
        if ($r.Overflow -or $r.Reason -eq 'CAPTURE_INVALID' -or -not $r.CaptureComplete) { throw 'PRIVACY_REFUSED' }
        Assert-K6Privacy $r.Stdout $Sentinels; Assert-K6Privacy $r.Stderr $Sentinels
        $State.Privacy = $true
        if (-not $r.Cleanup) { throw 'CLEANUP_UNPROVEN' }
        if ($r.Reason -cne 'NONE') { throw 'CHILD_UNSUCCESSFUL' }
        return $r
    } finally {
        if (-not $State.Cleanup) {
            # A failed private ledger write/PowerShell stop must still join the
            # created child. No force, even if a final proof remains unavailable.
            $child.Cancel(); [void]$child.Finish(1, 3000, $Chrome)
        }
        $child.Dispose()
    }
}

function Assert-K6ChromeVersionCapability {
    # Pinned ChromeMainDelegate handles --version only under IS_POSIX (1256).
    # Windows chrome.exe --version could start a browser UI. No unsafe probe or
    # file-version substitute: this is a distinct unresolved executable proof.
    throw 'CHROMIUM_VERSION_UNAVAILABLE'
}

function Remove-K6OwnedTree([string] $Path) {
    # Do not traverse or remove a reparse entry created inside a profile.
    if (([IO.File]::GetAttributes($Path) -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw 'CLEANUP_UNPROVEN' }
    $entries = [Collections.Generic.List[object]]::new()
    $pending = [Collections.Generic.Queue[string]]::new(); $pending.Enqueue($Path)
    while ($pending.Count -gt 0) {
        foreach ($entry in @(Get-ChildItem -LiteralPath $pending.Dequeue() -Force -ErrorAction Stop)) {
            if (($entry.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0 -or $entries.Count -ge 4096) { throw 'CLEANUP_UNPROVEN' }
            $entries.Add($entry)
            if ($entry.PSIsContainer) { $pending.Enqueue($entry.FullName) }
        }
    }
    foreach ($entry in @($entries.ToArray() | Sort-Object { $_.FullName.Length } -Descending)) {
        if ($entry.PSIsContainer) { [IO.Directory]::Delete($entry.FullName, $false) } else { [IO.File]::Delete($entry.FullName) }
    }
    [IO.Directory]::Delete($Path, $false)
}

function Assert-K6Version([string] $Text, [string] $Kind) {
    $value = $Text.Trim()
    if ($Kind -ceq 'k6') {
        if ($value -cnotmatch '^k6 v2\.3\.0 \(commit/[a-f0-9]+, go[0-9.]+, windows/amd64\)$') { throw 'VERSION_INVALID' }
    } elseif ($Kind -ceq 'chromium') {
        if ($value -cnotmatch '^Google Chrome for Testing 156\.0\.8078\.73$') { throw 'VERSION_INVALID' }
    } else { throw 'VERSION_INVALID' }
}

function Assert-K6Inspect($Value, [string] $Kind) {
    # inspect emits resolved options (not just the exported source object).
    if ($Kind -cnotin @('HTTP', 'Chromium') -or $null -eq $Value -or $null -eq $Value.scenarios) { throw 'OPTIONS_INVALID' }
    $case = if ($Kind -ceq 'HTTP') { 'iam-health' } else { 'browser-prerequisite' }
    if (@($Value.scenarios.PSObject.Properties.Name).Count -ne 1 -or @($Value.scenarios.PSObject.Properties.Name)[0] -cne $case) { throw 'OPTIONS_INVALID' }
    $scenario = $Value.scenarios.$case
    Assert-K6Integer $scenario.vus 1; Assert-K6Integer $scenario.iterations 1
    if ($scenario.executor -isnot [string] -or $scenario.maxDuration -isnot [string] -or $scenario.gracefulStop -isnot [string] -or
        $scenario.executor -cne 'per-vu-iterations' -or $scenario.maxDuration -cne $(if ($Kind -ceq 'HTTP') { '15s' } else { '20s' }) -or
        $scenario.gracefulStop -cne $(if ($Kind -ceq 'HTTP') { '2s' } else { '3s' })) { throw 'OPTIONS_INVALID' }
    Assert-K6Fields $Value.thresholds @('checks', 'case_started', 'case_completed')
    foreach ($name in @('checks', 'case_started', 'case_completed')) {
        $expected = if ($name -ceq 'checks') { 'rate==1' } else { 'count==1' }
        if ($Value.thresholds.$name -isnot [array] -or @($Value.thresholds.$name).Count -ne 1 -or
            @($Value.thresholds.$name)[0] -isnot [string] -or @($Value.thresholds.$name)[0] -cne $expected) { throw 'OPTIONS_INVALID' }
    }
    if ($Kind -ceq 'HTTP') { Assert-K6Integer $Value.maxRedirects 0; if ($Value.discardResponseBodies -isnot [bool] -or $Value.discardResponseBodies) { throw 'OPTIONS_INVALID' } }
    else { if ($scenario.options.browser.type -cne 'chromium') { throw 'OPTIONS_INVALID' } }
}

function Assert-K6Summary($Value, [string] $RunId, [string] $Kind, [int] $ExitCode) {
    Assert-K6Fields $Value @('version', 'run_id', 'phase', 'cases', 'expected', 'started', 'completed', 'iterations', 'requests', 'checks')
    Assert-K6Integer $Value.version 1
    if ($Kind -cnotin @('HTTP', 'Chromium') -or $ExitCode -ne 0 -or $Value.run_id -isnot [string] -or $Value.run_id -cne $RunId -or
        $RunId -cnotmatch '^[a-f0-9]{32}$' -or $Value.phase -isnot [string] -or $Value.phase -cne $Kind) { throw 'SUMMARY_INVALID' }
    $case = if ($Kind -ceq 'HTTP') { 'iam-health' } else { 'browser-prerequisite' }
    if ($Value.cases -isnot [array] -or $Value.cases.Count -ne 1 -or $Value.cases[0] -isnot [string] -or $Value.cases[0] -cne $case) { throw 'COMPLETENESS_INVALID' }
    foreach ($n in @('expected', 'started', 'completed', 'iterations')) { Assert-K6Integer $Value.$n 1 }
    Assert-K6Integer $Value.requests $(if ($Kind -ceq 'HTTP') { 1 } else { 0 })
    $checks = if ($Kind -ceq 'HTTP') { @('iam-status', 'iam-body') } else { @('browser-evaluation', 'browser-blank') }
    Assert-K6Fields $Value.checks $checks
    foreach ($name in $checks) { Assert-K6Fields $Value.checks.$name @('passes', 'fails'); Assert-K6Integer $Value.checks.$name.passes 1; Assert-K6Integer $Value.checks.$name.fails 0 }
}

function Assert-K6TargetInput([string] $Config) {
    if ([string]::IsNullOrEmpty($Config)) { throw 'INPUT_MISSING' }
    Assert-K6OutsideGit $Config
    $value = Read-K6PrivateJson $Config
    Assert-K6Fields $value @('version', 'profile', 'cluster', 'context', 'task_id', 'target_id', 'origin', 'lease', 'observations')
    Assert-K6Integer $value.version 1
    if ($value.origin -cne 'http://127.0.0.1:18080') { throw 'TARGET_REFUSED' }
    # The only existing authority is the guarded Linux anchor + origin binding,
    # which is itself unqualified. No Windows boolean/manifest can replace it.
    throw 'TARGET_BINDING_UNQUALIFIED'
}
