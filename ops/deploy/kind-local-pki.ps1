# Native OpenSSL issuance/reuse for the local lab. No deployment state or rotation.
param(
    [Parameter(Mandatory = $true)][string]$Directory,
    [string]$OpenSslPath = $(if ($env:OPENSSL) { $env:OPENSSL } else { 'openssl.exe' })
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$OpenSslPath = (Get-Command $OpenSslPath -CommandType Application -ErrorAction Stop).Source
$Directory = [IO.Path]::GetFullPath($Directory)
$configuration = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../scripts/local-full-certificates.cnf'))
. (Join-Path $PSScriptRoot 'private-path.ps1')

function Invoke-OpenSsl {
    $previous = $ErrorActionPreference
    $ErrorActionPreference = 'Continue' # OpenSSL progress on stderr is not failure.
    try {
        $output = & $OpenSslPath @args 2>$null
        $code = $LASTEXITCODE
    } finally { $ErrorActionPreference = $previous }
    if ($code -ne 0) { throw "OpenSSL $($args[0]) failed (exit $code); existing keys are not rotated." }
    return $output
}

function Prepare-Pair([string]$Folder, [string]$Name, [string]$Section, [string]$Authority = '', [switch]$Client) {
    $base = Join-Path (Join-Path $Directory $Folder) $Name
    $key = "$base.key"
    $cert = "$base.crt"
    if ((Test-Path -LiteralPath $key) -xor (Test-Path -LiteralPath $cert)) {
        throw "Incomplete certificate/key pair: $Folder/$Name. Restore or explicitly dispose it; no overwrite."
    }
    if (-not (Test-Path -LiteralPath $cert)) {
        Invoke-OpenSsl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:2048 -out $key | Out-Null
        Set-KindPrivatePath $key
        if (-not $Authority) {
            Invoke-OpenSsl req -new -x509 -key $key -out $cert -days 3650 -subj "/CN=aiforall-local-$Folder-ca" -config $configuration -extensions authority | Out-Null
        } else {
            $ca = Join-Path (Join-Path $Directory $Authority) 'ca'
            Invoke-OpenSsl req -new -key $key -out "$base.csr" -subj "/CN=$Name" -config $configuration | Out-Null
            $serial = Invoke-OpenSsl rand -hex 16
            Invoke-OpenSsl x509 -req -in "$base.csr" -CA "$ca.crt" -CAkey "$ca.key" -set_serial "0x$serial" -out $cert -days 365 -extfile $configuration -extensions $Section | Out-Null
            Remove-Item -LiteralPath "$base.csr"
        }
    }
    Set-KindPrivatePath $key
    Invoke-OpenSsl pkey -in $key -check -noout | Out-Null
    Invoke-OpenSsl x509 -in $cert -noout -checkend 86400 | Out-Null
    $keyPublic = (Invoke-OpenSsl pkey -in $key -pubout) -join "`n"
    $certPublic = (Invoke-OpenSsl x509 -in $cert -noout -pubkey) -join "`n"
    if ($keyPublic -cne $certPublic) { throw "Certificate/key mismatch: $Folder/$Name. No rotation." }
    $caCert = if ($Authority) { Join-Path (Join-Path $Directory $Authority) 'ca.crt' } else { $cert }
    Invoke-OpenSsl verify -CAfile $caCert $cert | Out-Null
    if ($Authority) {
        $dns = if ($Folder -eq 'workload') { 'lazaret' } else { $Name }
        Invoke-OpenSsl verify -CAfile $caCert -verify_hostname $dns -purpose sslserver $cert | Out-Null
        if ($Client) { Invoke-OpenSsl verify -CAfile $caCert -purpose sslclient $cert | Out-Null }
    }
}

foreach ($folder in @($Directory, (Join-Path $Directory 'mesh'), (Join-Path $Directory 'workload'))) {
    $null = New-Item -ItemType Directory -Force -Path $folder
    Set-KindPrivatePath $folder -Folder
}
Prepare-Pair mesh ca authority
Prepare-Pair workload ca authority
$transportPublic = (Invoke-OpenSsl pkey -in (Join-Path $Directory 'mesh/ca.key') -pubout) -join "`n"
$workloadPublic = (Invoke-OpenSsl pkey -in (Join-Path $Directory 'workload/ca.key') -pubout) -join "`n"
if ($transportPublic -ceq $workloadPublic) { throw 'Transport and workload authorities must differ.' }
foreach ($name in @('iam-service', 'hive-service', 'telegraph-service', 'manifesto-service',
                    'envoy-mesh', 'ext-authz', 'mesh-client', 'gitlab-connect')) {
    Prepare-Pair mesh $name "mesh/$name" mesh -Client
}
foreach ($name in @('postgres', 'redis', 'openfga', 'localstack', 'zot')) {
    Prepare-Pair mesh $name "mesh/$name" mesh
}
foreach ($name in @('openbao', 'github-connect', 'oauth-connect')) {
    Prepare-Pair mesh $name $name mesh
}
Prepare-Pair workload server workload/server workload
$localRoots = [IO.File]::ReadAllText((Join-Path $Directory 'mesh/ca.crt')) + "`n" +
    [IO.File]::ReadAllText((Join-Path $Directory 'workload/ca.crt'))
[IO.File]::WriteAllText((Join-Path $Directory 'ca-bundle.crt'), $localRoots, [Text.UTF8Encoding]::new($false))
Write-Output 'Local PKI ready: existing CA/key pairs reused; no implicit rotation.'
