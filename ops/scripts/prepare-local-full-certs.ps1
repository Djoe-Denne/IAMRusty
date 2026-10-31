#Requires -Version 5.1
param(
    [string]$OutputDirectory = $env:LOCAL_FULL_CERT_DIR,
    [string]$OpenSslPath,
    [string]$PythonPath = 'python',
    [switch]$OAuthProof
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$python = (Get-Command -Name $PythonPath -CommandType Application -ErrorAction Stop | Select-Object -First 1).Source
$arguments = @('-B', (Join-Path $PSScriptRoot 'prepare-local-full-certs.py'))
if ($OutputDirectory) { $arguments += @('--output-dir', $OutputDirectory) }
if ($OpenSslPath) { $arguments += @('--openssl', $OpenSslPath) }
if ($OAuthProof) { $arguments += '--oauth-proof' }
# The Python entrypoint captures OpenSSL itself. Do not let Windows PowerShell
# turn its sanitized stderr into a terminating error before preserving the exit.
$ErrorActionPreference = 'Continue'
if (Test-Path variable:PSNativeCommandUseErrorActionPreference) {
    $PSNativeCommandUseErrorActionPreference = $false
}
& $python @arguments
exit $LASTEXITCODE
