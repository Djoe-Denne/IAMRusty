# One explicit, persistent USER-profile trust operation, outside campaign timers.
# Public certificate only; no elevation, machine store or private-key import.
param(
    [ValidateSet('verify', 'install-profile')][string]$Action = 'verify',
    [Parameter(Mandatory = $true)][string]$CertificateFile,
    [Parameter(Mandatory = $true)][ValidatePattern('^[a-fA-F0-9]{64}$')][string]$ExpectedSha256
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
if (-not [IO.Path]::IsPathRooted($CertificateFile)) { throw 'EXPLICIT_PUBLIC_CA_REQUIRED' }
$text = [IO.File]::ReadAllText($CertificateFile)
if ($text -cnotmatch '(?s)^\s*-----BEGIN CERTIFICATE-----\s*([A-Za-z0-9+/=\s]+)-----END CERTIFICATE-----\s*$') { throw 'ONE_PUBLIC_CERTIFICATE_ONLY' }
$der = [Convert]::FromBase64String($Matches[1])
$hash = [Security.Cryptography.SHA256]::Create()
try { $fingerprint = ([BitConverter]::ToString($hash.ComputeHash($der))).Replace('-', '').ToLowerInvariant() }
finally { $hash.Dispose() }
if ($fingerprint -cne $ExpectedSha256.ToLowerInvariant()) { throw 'CA_FINGERPRINT_MISMATCH' }
$ca = [Security.Cryptography.X509Certificates.X509Certificate2]::new($der)
try {
    $constraints = @($ca.Extensions | Where-Object { $_.Oid.Value -ceq '2.5.29.19' })
    if ($ca.HasPrivateKey -or $constraints.Count -ne 1 -or -not $constraints[0].CertificateAuthority -or
        $ca.NotBefore.ToUniversalTime() -gt [DateTime]::UtcNow -or $ca.NotAfter.ToUniversalTime() -le [DateTime]::UtcNow) { throw 'VALID_PUBLIC_CA_REQUIRED' }
    $store = [Security.Cryptography.X509Certificates.X509Store]::new('Root', 'CurrentUser')
    try {
        $flags = if ($Action -ceq 'install-profile') { [Security.Cryptography.X509Certificates.OpenFlags]::ReadWrite } else { [Security.Cryptography.X509Certificates.OpenFlags]::ReadOnly }
        $store.Open($flags)
        $present = $store.Certificates.Find([Security.Cryptography.X509Certificates.X509FindType]::FindByThumbprint, $ca.Thumbprint, $false).Count -gt 0
        if (-not $present) {
            if ($Action -ceq 'verify') { throw 'PROFILE_CA_NOT_TRUSTED' }
            # Windows may ask its own confirmation. No forward or test is alive
            # here, and nothing suppresses or accepts that confirmation for you.
            $store.Add($ca)
            $store.Close(); $store.Open([Security.Cryptography.X509Certificates.OpenFlags]::ReadOnly)
            if ($store.Certificates.Find([Security.Cryptography.X509Certificates.X509FindType]::FindByThumbprint, $ca.Thumbprint, $false).Count -ne 1) { throw 'PROFILE_CA_INSTALL_UNPROVEN' }
        }
        @{store = 'CurrentUser\Root'; sha256 = $fingerprint; persistent = $true; previously_present = $present; trusted = $true} | ConvertTo-Json -Compress
    } finally { $store.Close() }
} finally { $ca.Dispose() }
