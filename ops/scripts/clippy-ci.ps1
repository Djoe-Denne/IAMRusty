# Native Windows report collection. Cargo aliases hold the lint/target policy.
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$root = Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
$reportDirectory = Join-Path $root 'target/sonar'
$null = New-Item -ItemType Directory -Force $reportDirectory
$cargo = (Get-Command cargo.exe -ErrorAction Stop).Source
$merged = Join-Path $reportDirectory 'clippy.json'
# Invalidate any previous combined report until BOTH passes finish successfully.
[System.IO.File]::WriteAllText($merged, '')

foreach ($lane in @('production', 'tests')) {
    $report = Join-Path $reportDirectory "clippy-$lane.jsonl"
    $stderr = Join-Path $reportDirectory "clippy-$lane.stderr.log"
    # Explicitly wait for the process tree; never rely on implicit shell waiting.
    $process = Start-Process -FilePath $cargo -ArgumentList "clippy-ci-$lane" `
        -WorkingDirectory $root -NoNewWindow -Wait -PassThru `
        -RedirectStandardOutput $report -RedirectStandardError $stderr
    try {
        if ($process.ExitCode -ne 0) {
            Get-Content $stderr -Tail 40 | Write-Host
            throw "Clippy $lane failed (exit $($process.ExitCode)); see $stderr"
        }
    }
    finally {
        $process.Dispose()
    }
    $lastLine = Get-Content -LiteralPath $report -Tail 1
    if (-not $lastLine) { throw "Clippy $lane produced an empty report" }
    $finished = $lastLine | ConvertFrom-Json
    if ($finished.reason -ne 'build-finished' -or $finished.success -ne $true) {
        throw "Clippy $lane did not attest a successful build-finished event"
    }
    Write-Host "Clippy $lane completed: $report"
}

$output = [System.IO.File]::Create($merged)
try {
    foreach ($lane in @('production', 'tests')) {
        $inputReport = [System.IO.File]::OpenRead((Join-Path $reportDirectory "clippy-$lane.jsonl"))
        try { $inputReport.CopyTo($output) }
        finally { $inputReport.Dispose() }
    }
}
finally { $output.Dispose() }
Write-Host "Combined report: $merged"
