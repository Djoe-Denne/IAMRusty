#requires -Version 5.1
[CmdletBinding()]
param(
    [string] $Mode,
    [string] $Output,
    [string] $Config
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$WarningPreference = 'SilentlyContinue'
$InformationPreference = 'SilentlyContinue'

$result = [ordered]@{ version = 1; platform = 'windows-amd64'; mode = 'UNKNOWN'; tools = 'NOT_EXECUTED'; http = 'INCONCLUSIVE'; chromium = 'INCONCLUSIVE'; cleanup = 'NOT_EXECUTED'; confidentiality = 'NOT_EXECUTED'; wsl = 'INCONCLUSIVE'; linux = 'INCONCLUSIVE'; reason = 'PRECONDITION_REFUSED' }
$result.run_id = $null
$result.sources_sha256 = [ordered]@{ http = $null; chromium = $null; summary = $null }
$result.versions = [ordered]@{ k6 = $null; chromium = $null }
$result.browser_version_evidence = 'NOT_PROVEN'
$result.chrome_cli_version = [ordered]@{ execution = 'NOT_EXECUTED'; verdict = 'INCONCLUSIVE' }
$result.hashes = [ordered]@{ k6_archive = $null; k6_executable = $null; chromium_archive = $null; chromium_executable = $null }
$exitCode = 2; $outputCreated = $false; $outputPin = $null; $helpersLoaded = $false
$state = @{ Cleanup = $true; Privacy = $true; Executed = 0 }
try {
    . (Join-Path $PSScriptRoot 'lib\windows.ps1')
    $helpersLoaded = $true
    Initialize-K6Windows
    if ([string]::IsNullOrEmpty($Mode)) { throw 'INPUT_MISSING' }
    if ($Mode -cnotin @('tools', 'browser', 'validate', 'run', 'selftest')) { throw 'INPUT_INVALID' }
    $result.mode = $Mode
    if ($Mode -in @('validate', 'run', 'selftest')) { Assert-K6TargetInput $Config }
    if (-not [string]::IsNullOrEmpty($Config)) { throw 'INPUT_INVALID' }
    if ([string]::IsNullOrEmpty($Output)) { throw 'INPUT_MISSING' }
    Assert-K6OutsideGit $Output
    $lock = Read-K6WindowsLock (Join-Path $PSScriptRoot 'config\tools.windows.lock.json')
    $k6 = Open-K6PinnedTool $lock.k6 $env:K6_WINDOWS_EXE
    $chrome = Open-K6PinnedTool $lock.chromium $env:K6_BROWSER_EXECUTABLE_PATH
    $outputPin = [K6Windows.OwnedDirectory]::Create($Output)
    $outputCreated = $true
    $result.cleanup = 'INCONCLUSIVE'; $result.confidentiality = 'INCONCLUSIVE'; $result.tools = 'INCONCLUSIVE'
    $runId = [Guid]::NewGuid().ToString('N')
    $result.run_id = $runId
    $result.hashes.k6_archive = $k6.Pin.archive_sha256; $result.hashes.k6_executable = $k6.Pin.executable_sha256
    $result.hashes.chromium_archive = $chrome.Pin.archive_sha256; $result.hashes.chromium_executable = $chrome.Pin.executable_sha256
    [IO.File]::WriteAllText((Join-Path $Output 'lease.json'), (@{ version = 1; run_id = $runId; created_utc = [DateTime]::UtcNow.ToString('o'); expires_utc = [DateTime]::UtcNow.AddMinutes(2).ToString('o'); status = 'active-created-handles-only' } | ConvertTo-Json -Compress), [Text.UTF8Encoding]::new($false))
    $sentinels = @('PRIVATE_' + [Guid]::NewGuid().ToString('N'), 'TOKEN_' + [Guid]::NewGuid().ToString('N'))
    $environment = New-K6Environment $Output $runId $chrome.Exe $sentinels $chrome.Pin.version
    # Copies are exact, local and run-owned. The sole browser version check is
    # admitted by the Windows addendum; HTTP and summary sources are unchanged.
    foreach ($relative in @('scenarios\smoke\iam-health.js', 'scenarios\smoke\browser-prerequisite.js', 'lib\summary.js')) {
        $source = Join-Path $PSScriptRoot $relative
        $destination = Join-Path $Output $relative
        if ([IO.FileInfo]::new($source).Length -gt 65536) { throw 'INPUT_INVALID' }
        $key = if ($relative -ceq 'scenarios\smoke\iam-health.js') { 'http' } elseif ($relative -ceq 'scenarios\smoke\browser-prerequisite.js') { 'chromium' } else { 'summary' }
        $result.sources_sha256.$key = [K6Windows.Native]::Hash($source)
        $directory = [IO.Path]::GetDirectoryName($destination)
        if (-not [IO.Directory]::Exists($directory)) { [void][IO.Directory]::CreateDirectory($directory) }
        [IO.File]::WriteAllBytes($destination, [IO.File]::ReadAllBytes($source))
    }
    $r = Invoke-K6Child $k6 @('version') $environment $Output $sentinels 10000 $state
    Assert-K6Version $r.Stdout 'k6'; if ($r.ExitCode -ne 0) { throw 'VERSION_INVALID' }
    $result.versions.k6 = '2.3.0'
    foreach ($kind in @('HTTP', 'Chromium')) {
        $scenario = if ($kind -ceq 'HTTP') { 'iam-health.js' } else { 'browser-prerequisite.js' }
        $r = Invoke-K6Child $k6 @('inspect', (Join-Path $Output ('scenarios\smoke\' + $scenario))) $environment $Output $sentinels 10000 $state
        if ($r.ExitCode -ne 0) { throw 'OPTIONS_INVALID' }
        Assert-K6Inspect (ConvertFrom-K6Json $r.Stdout) $kind
    }
    if ($Mode -ceq 'tools') { Assert-K6ToolsOnly }
    if ($Mode -ceq 'browser') {
        Assert-K6BrowserEligibility $environment $chrome.Pin.version
        Assert-K6PinnedTool $chrome
        $summary = Join-Path $Output 'summary.json'
        if (Test-Path -LiteralPath $summary) { throw 'SUMMARY_INVALID' }
        $r = Invoke-K6Child $k6 @('run', '--quiet', '--address', '127.0.0.1:0', (Join-Path $Output 'scenarios\smoke\browser-prerequisite.js')) $environment $Output $sentinels 30000 $state $chrome.Exe
        $value = Read-K6PrivateJson $summary $outputPin
        Assert-K6BrowserEligibility $environment $chrome.Pin.version
        Set-K6BrowserEvidence $result $value $runId $r $state $chrome.Pin.version
    }
    $result.reason = 'NONE'; $exitCode = 0
} catch {
    # Never print a native exception or a tool stream, even on an early failure.
    $known = @('PLATFORM_REFUSED', 'ELEVATION_REFUSED', 'INPUT_MISSING', 'INPUT_INVALID', 'PATH_INVALID', 'OUTPUT_EXISTS', 'OUTPUT_CREATE_REFUSED', 'OUTPUT_BINDING_UNPROVEN', 'WINDOWS_LOCK_MISSING', 'PIN_INVALID', 'PE_INVALID', 'JSON_INVALID', 'SCHEMA_INVALID', 'COMPLETENESS_INVALID', 'SUMMARY_INVALID', 'OPTIONS_INVALID', 'VERSION_INVALID', 'CHROMIUM_VERSION_PENDING', 'BROWSER_EXPECTATION_INVALID', 'TARGET_REFUSED', 'TARGET_BINDING_UNQUALIFIED', 'PRIVACY_REFUSED', 'CLEANUP_UNPROVEN', 'CHILD_UNSUCCESSFUL', 'PROCESS_START_REFUSED')
    $message = $_.Exception.Message
    $label = @($known | Where-Object { $message -match ('(?:^|[\s"'':])' + [regex]::Escape($_) + '(?:$|[\s"''])') } | Select-Object -First 1)
    $result.reason = if ($label.Count -eq 1) { $label[0] } else { 'PRECONDITION_REFUSED' }
    if ($result.reason -ceq 'PRIVACY_REFUSED') { $result.confidentiality = 'FAIL' }
    if ($result.reason -ceq 'CLEANUP_UNPROVEN') { $result.cleanup = 'FAIL' }
    $exitCode = 2
} finally {
    # Only the freshly created run directory is disposable. Never remove
    # an output supplied by someone else, a reused path or a surviving child's data.
    if ($outputCreated -and $state.Cleanup) {
        try {
            # On failure retain only bounded private streams and process facts,
            # not profiles, source copies or summaries. Never publish those logs.
            Remove-K6OwnedTree $outputPin -KeepDiagnostics:($exitCode -ne 0)
            if ($exitCode -eq 0 -and (Test-Path -LiteralPath $Output)) { throw 'CLEANUP_UNPROVEN' }
            $result.cleanup = 'PASS'
        } catch { $result.cleanup = 'FAIL'; $result.reason = 'CLEANUP_UNPROVEN'; $exitCode = 2 }
    }
    if ($null -ne $outputPin) { $outputPin.Dispose() }
    if ($outputCreated -and -not $state.Cleanup) { $result.cleanup = 'FAIL'; $exitCode = 2 }
    if ($outputCreated -and $state.Executed -gt 0) { $result.confidentiality = if ($state.Privacy -and $result.confidentiality -cne 'FAIL') { 'PASS' } else { 'FAIL' } }
    if ($helpersLoaded) { Complete-K6WindowsVerdicts $result }
}
if ($exitCode -eq 0 -and ($result.cleanup -cne 'PASS' -or $result.confidentiality -cne 'PASS')) { $exitCode = 2 }
# This closed, bounded stdout projection is the only publishable output. No run
# captures, paths, PIDs/SIDs, raw metrics, arbitrary messages or secrets are exported.
[Console]::Out.WriteLine(($result | ConvertTo-Json -Compress))
exit $exitCode
