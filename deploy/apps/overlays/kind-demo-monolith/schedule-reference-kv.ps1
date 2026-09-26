#Requires -Version 5.1
# HORS GOLD 0605 / 0008 — dette 0604 (schedule manuel + Job enroll).
# Nominal = just prove-gold. Ne plus utiliser comme étape nominale.
<#
.SYNOPSIS
  Schedule apparatus-reference-kv via l'operator J2 actuel + enroll Lazaret (HTTP).

.DESCRIPTION
  - Staging M5 + Dockerfile.http → apparatus-reference-kv:e2e
  - kind load aiforall-local
  - Seed DiskStore JSON (kubectl cp / exec) + AdmissionRecord dans apparatus-system
  - Attend Pod plugin-{32hex} dans apparatus-plugins
  - Job enroll (openssl CSR + curl) vers oodhive-monolith:/lazaret/enroll

  Jamais apparatus-p4-it. Pas de GKE/Factory. Ne touche pas le binaire operator.
#>
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..')
Set-Location $RepoRoot

$KindContext = 'kind-aiforall-local'
$KindCluster = 'aiforall-local'
$ForbiddenContext = 'kind-apparatus-p4-it'
$CriName = 'apparatus-reference-kv'
$CriTag = 'apparatus-reference-kv:e2e'
$EnrollImage = 'j3-enroll:local'
$PolicyId = 'apparatus-p4-conformance/v1'
$PluginsNs = 'apparatus-plugins'
$SystemNs = 'apparatus-system'
$OperatorNs = 'aiforall-apparatus'
$Release = '0.1.0'

# Évite la conversion Git bash de /var/lib/... → C:/Program Files/Git/var/...
$env:MSYS_NO_PATHCONV = '1'

function Invoke-Native {
    param(
        [Parameter(Mandatory = $true)][string]$File,
        [Parameter(Mandatory = $true)][string[]]$NativeArgs,
        [Parameter(Mandatory = $true)][string]$FailMessage
    )
    # Docker/kind écrivent la progression sur stderr → ErrorRecord sous Stop.
    $prev = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    & $File @NativeArgs
    $code = $LASTEXITCODE
    $ErrorActionPreference = $prev
    if ($code -ne 0) { throw ("{0} (exit {1})" -f $FailMessage, $code) }
}

function Test-HasCommand {
    param([Parameter(Mandatory = $true)][string]$Name)
    return $null -ne (Get-Command $Name -ErrorAction SilentlyContinue)
}

function Invoke-Kubectl {
    param([Parameter(Mandatory = $true)][string[]]$KubectlArgs)
    & kubectl --context $KindContext @KubectlArgs
    if ($LASTEXITCODE -ne 0) {
        throw ("kubectl failed ({0}): {1}" -f $LASTEXITCODE, ($KubectlArgs -join ' '))
    }
}

function Get-ReferenceKvDigests {
    $tmp = Join-Path $env:TEMP ("aiforall-print-desc-" + [guid]::NewGuid().ToString('N').Substring(0, 8))
    New-Item -ItemType Directory -Path $tmp | Out-Null
    try {
        $rootUnix = ($RepoRoot.Path -replace '\\', '/')
        @"
[package]
name = "print_desc"
version = "0.0.0"
edition = "2021"
[dependencies]
apparatus-operator = { path = "$rootUnix/apparatus-operator", default-features = false }
"@ | Set-Content -Encoding utf8 (Join-Path $tmp 'Cargo.toml')
        New-Item -ItemType Directory -Path (Join-Path $tmp 'src') | Out-Null
        @"
use apparatus_operator::{digest_manifest, evaluate_conformance, parse_manifest};
fn main() {
    let path = std::env::args().nth(1).expect("toml path");
    let raw = std::fs::read_to_string(&path).expect("read");
    let v = parse_manifest(&raw).expect("parse");
    let d = digest_manifest(&v.manifest).expect("digest");
    let r = evaluate_conformance(&raw);
    assert!(r.passed, "T5 must pass for reference-kv");
    println!("DESCRIPTOR={}", d.as_str());
    println!("REPORT={}", r.report_digest.as_str());
}
"@ | Set-Content -Encoding utf8 (Join-Path $tmp 'src\main.rs')
        $toml = Join-Path $RepoRoot 'apparatus-reference-kv\apparatus.toml'
        Push-Location $tmp
        $out = & cargo run -q -- $toml 2>&1 | Out-String
        Pop-Location
        if ($LASTEXITCODE -ne 0) { throw "digest oneshot failed: $out" }
        $descriptor = ($out | Select-String -Pattern 'DESCRIPTOR=(.+)').Matches[0].Groups[1].Value.Trim()
        $report = ($out | Select-String -Pattern 'REPORT=(.+)').Matches[0].Groups[1].Value.Trim()
        if (-not $descriptor -or -not $report) { throw "digest oneshot parse failed: $out" }
        return @{ Descriptor = $descriptor; Report = $report }
    }
    finally {
        Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
    }
}

function New-M5HttpStaging {
    $staging = Join-Path $env:TEMP ("apparatus-m5-cri-" + [guid]::NewGuid().ToString('N').Substring(0, 8))
    if (Test-Path $staging) { Remove-Item -Recurse -Force $staging }
    New-Item -ItemType Directory -Path $staging | Out-Null
    @"
[workspace]
resolver = "2"
members = ["apparatus-contracts", "apparatus-reference-kv"]

[workspace.package]
version = "0.1.0"
edition = "2021"
authors = ["Djoé Denne <djoe.denne@gmail.com>"]
license = "MIT OR Apache-2.0"

[workspace.dependencies]
serde = { version = "1.0.197", features = ["derive"] }
serde_json = "1.0.120"
toml = "0.8.23"
thiserror = "2.0.11"
uuid = { version = "1.4.1", features = ["v4", "serde"] }
sha2 = "0.10.8"
semver = "1.0.23"
axum = { version = "0.8.4", features = ["macros", "json"] }
tokio = { version = "1.44", features = ["rt-multi-thread", "macros", "net", "signal"] }

[workspace.lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"

[workspace.lints.clippy]
all = { level = "warn", priority = -1 }
pedantic = { level = "warn", priority = -1 }
nursery = { level = "warn", priority = -1 }
cargo = { level = "warn", priority = -1 }
cargo_common_metadata = "allow"
multiple_crate_versions = "allow"
module_name_repetitions = "allow"
must_use_candidate = "allow"
"@ | Set-Content -Encoding utf8 (Join-Path $staging 'Cargo.toml')
    Copy-Item -Recurse (Join-Path $RepoRoot 'apparatus-contracts') (Join-Path $staging 'apparatus-contracts')
    Copy-Item -Recurse (Join-Path $RepoRoot 'apparatus-reference-kv') (Join-Path $staging 'apparatus-reference-kv')
    # M5 : Dockerfile.http → Dockerfile à la racine du staging (contexte docker = staging).
    Copy-Item (Join-Path $RepoRoot 'apparatus-reference-kv\Dockerfile.http') (Join-Path $staging 'Dockerfile')
    return $staging
}

function Get-CriPin {
    param([Parameter(Mandatory = $true)][string]$Tag)
    $prev = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    $id = (& docker inspect --format '{{.Id}}' $Tag).Trim()
    $code = $LASTEXITCODE
    $ErrorActionPreference = $prev
    if ($code -ne 0 -or -not $id) { throw "docker inspect $Tag failed" }
    if (-not $id.StartsWith('sha256:')) { throw "docker Id must be sha256:<hex>, got: $id" }
    $hex = $id.Substring(7)
    if ($hex.Length -ne 64) { throw "docker Id hex length: $id" }
    return ("{0}@sha256:{1}" -f $CriName, $hex)
}

function Pin-CriOnKindNode {
    param(
        [Parameter(Mandatory = $true)][string]$Tag,
        [Parameter(Mandatory = $true)][string]$CriImage
    )
    # Aligné fixtures kind : tagger le pin @sha256 dans le store ctr du nœud
    # (forme courte + docker.io/library/… — sinon kubelet tente un pull registry).
    $node = "$KindCluster-control-plane"
    $prev = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    $ls = & docker exec $node ctr -n k8s.io images ls 2>&1 | Out-String
    $ErrorActionPreference = $prev
    $source = $null
    foreach ($line in ($ls -split "`n")) {
        if ($line -match [regex]::Escape($Tag) -and $line -notmatch '@sha256:') {
            $source = ($line -split '\s+')[0]
            break
        }
    }
    if (-not $source) {
        Write-Host "WARN: no ctr source for $Tag (pin may still resolve via kind load)" -ForegroundColor Yellow
        return
    }
    $libraryPin = "docker.io/library/$CriImage"
    foreach ($pin in @($CriImage, $libraryPin)) {
        $ErrorActionPreference = 'Continue'
        & docker exec $node ctr -n k8s.io images tag $source $pin | Out-Null
        $code = $LASTEXITCODE
        $ErrorActionPreference = $prev
        if ($code -ne 0) {
            Write-Host "WARN: ctr tag $pin failed (exit $code)" -ForegroundColor Yellow
        }
        else {
            Write-Host "  ctr pin on node: $pin" -ForegroundColor Green
        }
    }
}


function Get-ManifestoBinding {
    $sql = @"
SELECT c.project_id::text, c.id::text
FROM project_components c
WHERE c.component_type = 'io.aiforall.reference-kv'
ORDER BY c.added_at DESC
LIMIT 1;
"@
    $row = & docker exec aiforall-postgres-1 psql -U postgres -d manifesto_dev -At -F '|' -c $sql
    if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($row)) {
        throw "no project_components row for io.aiforall.reference-kv in manifesto_dev"
    }
    $parts = ($row.Trim() -split '\|')
    return @{ ProjectId = $parts[0]; Binding = $parts[1] }
}

function Write-AdmissionStoreJson {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Descriptor,
        [Parameter(Mandatory = $true)][string]$Report,
        [Parameter(Mandatory = $true)][string]$CriImage
    )
    $obj = @{
        records = @(
            @{
                descriptorDigest = $Descriptor
                policyVersion    = $PolicyId
                reportDigest     = $Report
                status           = @{ phase = 'VALID' }
                criImage         = $CriImage
            }
        )
    }
    [System.IO.File]::WriteAllText($Path, ($obj | ConvertTo-Json -Depth 6 -Compress))
}

function Seed-AdmissionStore {
    param([Parameter(Mandatory = $true)][string]$LocalJson)
    $pod = (& kubectl --context $KindContext get pod -n $OperatorNs -l app.kubernetes.io/name=apparatus-operator -o jsonpath='{.items[0].metadata.name}').Trim()
    if (-not $pod) { throw "operator pod not found in $OperatorNs" }
    # Windows : `pod:/abs` est ambigu (drive letter). Prefer exec+tee ; MSYS_NO_PATHCONV pour Git.
    $prev = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    Get-Content -Raw -Path $LocalJson | & kubectl --context $KindContext exec -i -n $OperatorNs $pod -- `
        /bin/sh -c 'cat > /var/lib/apparatus/admission.json'
    $code = $LASTEXITCODE
    $ErrorActionPreference = $prev
    if ($code -ne 0) { throw "seed admission.json via exec failed (exit $code)" }
    $check = & kubectl --context $KindContext exec -n $OperatorNs $pod -- cat /var/lib/apparatus/admission.json
    if ($LASTEXITCODE -ne 0 -or ($check -notmatch 'VALID')) {
        throw "admission.json not readable/VALID after seed"
    }
    Write-Host "Seeded admission store on $pod" -ForegroundColor Green
}

function Apply-AdmissionRecord {
    param(
        [Parameter(Mandatory = $true)][string]$Descriptor,
        [Parameter(Mandatory = $true)][string]$Report,
        [Parameter(Mandatory = $true)][string]$CriImage,
        [Parameter(Mandatory = $true)][string]$ProjectId,
        [Parameter(Mandatory = $true)][string]$Binding
    )
    $hex = $Descriptor.Substring('sha256:'.Length)
    $name = "sha256-$hex"
    $yaml = @"
apiVersion: apparatus.aiforall.dev/v1alpha1
kind: AdmissionRecord
metadata:
  name: $name
  namespace: $SystemNs
  labels:
    apparatus.aiforall.dev/project: "$ProjectId"
    apparatus.aiforall.dev/binding: "$Binding"
spec:
  descriptorDigest: $Descriptor
  policyVersion: $PolicyId
  reportDigest: $Report
  criImage: $CriImage
"@
    $path = Join-Path $env:TEMP ("admissionrecord-$hex.yaml")
    [System.IO.File]::WriteAllText($path, $yaml)
    Invoke-Kubectl @('apply', '-f', $path)
    return $name
}

function Wait-PluginPod {
    param(
        [Parameter(Mandatory = $true)][string]$PodName,
        [Parameter(Mandatory = $true)][string]$ExpectedImage,
        [int]$TimeoutSec = 180
    )
    $deadline = (Get-Date).AddSeconds($TimeoutSec)
    while ((Get-Date) -lt $deadline) {
        $phase = & kubectl --context $KindContext get pod $PodName -n $PluginsNs -o jsonpath='{.status.phase}' 2>$null
        $image = & kubectl --context $KindContext get pod $PodName -n $PluginsNs -o jsonpath='{.spec.containers[0].image}' 2>$null
        if ($phase -eq 'Running' -and $image -eq $ExpectedImage) {
            Write-Host "Pod $PluginsNs/$PodName Running image=$image" -ForegroundColor Green
            return
        }
        if ($phase -eq 'Failed') {
            & kubectl --context $KindContext describe pod $PodName -n $PluginsNs
            throw "plugin pod Failed"
        }
        Start-Sleep -Seconds 3
    }
    & kubectl --context $KindContext get pod -n $PluginsNs -o wide
    & kubectl --context $KindContext logs -n $OperatorNs -l app.kubernetes.io/name=apparatus-operator --tail=80
    throw "timeout waiting for $PluginsNs/$PodName Running with $ExpectedImage"
}

function New-EnrollIdentityConfigMap {
    param(
        [Parameter(Mandatory = $true)][string]$ProjectId,
        [Parameter(Mandatory = $true)][string]$Binding
    )
    & kubectl --context $KindContext delete job lazaret-enroll-reference-kv -n $PluginsNs --ignore-not-found
    & kubectl --context $KindContext delete configmap lazaret-enroll-identity -n $PluginsNs --ignore-not-found
    Invoke-Kubectl @(
        'create', 'configmap', 'lazaret-enroll-identity', '-n', $PluginsNs,
        '--from-literal=project_id=' + $ProjectId,
        '--from-literal=binding=' + $Binding,
        '--from-literal=release=' + $Release
    )
}

# --- main ---
$missing = @()
foreach ($c in @('kind', 'docker', 'kubectl', 'cargo')) {
    if (-not (Test-HasCommand $c)) { $missing += $c }
}
if ($missing.Count -gt 0) {
    Write-Host ("STOP : outil manquant ({0})" -f ($missing -join ', ')) -ForegroundColor Red
    exit 1
}

$ctx = (& kubectl config current-context 2>$null)
Write-Host "Schedule+enroll : contexte kubectl=$KindContext (jamais $ForbiddenContext; current=$ctx)" -ForegroundColor Cyan

$clusters = @(& kind get clusters 2>&1 | ForEach-Object { ([string]$_).Trim() })
if ($clusters -notcontains $KindCluster) {
    Write-Host "STOP : cluster kind $KindCluster absent" -ForegroundColor Red
    exit 1
}

Write-Host "1/7 apply deploy/p4 (ns plugins + RBAC pods)" -ForegroundColor Cyan
Invoke-Kubectl @('apply', '-k', 'deploy/p4')

Write-Host "2/7 digests 0002 (cargo oneshot parse_manifest+digest_manifest)" -ForegroundColor Cyan
$digests = Get-ReferenceKvDigests
$descriptor = $digests.Descriptor
$report = $digests.Report
$hex = $descriptor.Substring('sha256:'.Length)
$podName = 'plugin-' + $hex.Substring(0, [Math]::Min(32, $hex.Length))
Write-Host "  descriptor=$descriptor"
Write-Host "  report=$report"
Write-Host "  pod=$podName"

Write-Host "3/7 staging M5 + docker build $CriTag" -ForegroundColor Cyan
$staging = New-M5HttpStaging
try {
    # Sans attestations BuildKit : {{.Id}} = digest image unique (pas manifest list),
    # sinon kind load tag mais le pin @sha256 du index n'est pas résolu localement → ErrImagePull.
    Invoke-Native -File 'docker' -NativeArgs @(
        'build', '--provenance=false', '--sbom=false', '-t', $CriTag, $staging
    ) -FailMessage "docker build $CriTag failed"
}
finally {
    Remove-Item -Recurse -Force $staging -ErrorAction SilentlyContinue
}
$criImage = Get-CriPin -Tag $CriTag
Write-Host "  criImage=$criImage"

Write-Host "4/7 kind load + enroll image" -ForegroundColor Cyan
Invoke-Native -File 'kind' -NativeArgs @('load', 'docker-image', $CriTag, '--name', $KindCluster) `
    -FailMessage "kind load $CriTag failed"
Pin-CriOnKindNode -Tag $CriTag -CriImage $criImage
Invoke-Native -File 'docker' -NativeArgs @(
    'build', '--provenance=false', '--sbom=false', '-t', $EnrollImage,
    '-f', 'deploy/apps/overlays/kind-demo-monolith/Dockerfile.enroll',
    'deploy/apps/overlays/kind-demo-monolith'
) -FailMessage "docker build $EnrollImage failed"
Invoke-Native -File 'kind' -NativeArgs @('load', 'docker-image', $EnrollImage, '--name', $KindCluster) `
    -FailMessage "kind load $EnrollImage failed"

Write-Host "5/7 seed DiskStore + AdmissionRecord" -ForegroundColor Cyan
$binding = Get-ManifestoBinding
Write-Host "  project_id=$($binding.ProjectId) binding=$($binding.Binding)"
# Reset prior schedule so watch voit un ADDED frais après seed JSON.
& kubectl --context $KindContext delete pod $podName -n $PluginsNs --ignore-not-found 2>$null | Out-Null
$crName = "sha256-$hex"
& kubectl --context $KindContext delete admissionrecord $crName -n $SystemNs --ignore-not-found 2>$null | Out-Null
$storePath = Join-Path $env:TEMP 'apparatus-admission-seed.json'
Write-AdmissionStoreJson -Path $storePath -Descriptor $descriptor -Report $report -CriImage $criImage
Seed-AdmissionStore -LocalJson $storePath
Apply-AdmissionRecord -Descriptor $descriptor -Report $report -CriImage $criImage `
    -ProjectId $binding.ProjectId -Binding $binding.Binding | Out-Null

Write-Host "6/7 wait plugin pod" -ForegroundColor Cyan
Wait-PluginPod -PodName $podName -ExpectedImage $criImage

Write-Host "7/7 enroll Job" -ForegroundColor Cyan
# Gateway default-deny n'autorise que aiforall-plugins — autoriser apparatus-plugins.
Invoke-Kubectl @('apply', '-f', 'deploy/apps/overlays/kind-demo-monolith/allow-apparatus-plugins-np.yaml')
$gwPatch = Join-Path $env:TEMP 'gw-np-apparatus-plugins.json'
[System.IO.File]::WriteAllText($gwPatch, '[{"op":"add","path":"/spec/ingress/0/from/-","value":{"namespaceSelector":{"matchLabels":{"kubernetes.io/metadata.name":"apparatus-plugins"}}}}]')
$prev = $ErrorActionPreference
$ErrorActionPreference = 'Continue'
& kubectl --context $KindContext patch networkpolicy default-deny -n aiforall-gateway --type=json --patch-file $gwPatch 2>$null | Out-Null
$ErrorActionPreference = $prev
New-EnrollIdentityConfigMap -ProjectId $binding.ProjectId -Binding $binding.Binding
Invoke-Kubectl @('apply', '-f', 'deploy/apps/overlays/kind-demo-monolith/enroll-job.yaml')
& kubectl --context $KindContext wait --for=condition=complete job/lazaret-enroll-reference-kv -n $PluginsNs --timeout=120s
$waitCode = $LASTEXITCODE
$jobPod = (& kubectl --context $KindContext get pods -n $PluginsNs -l app.kubernetes.io/name=lazaret-enroll-reference-kv -o jsonpath='{.items[0].metadata.name}').Trim()
$logs = ''
if ($jobPod) {
    $logs = & kubectl --context $KindContext logs -n $PluginsNs $jobPod 2>&1 | Out-String
}
Write-Host "--- enroll logs ---"
Write-Host $logs
if ($waitCode -ne 0) {
    Write-Host "ENROLL BLOCKED (job not complete)" -ForegroundColor Red
    exit $waitCode
}
if ($logs -notmatch 'HTTP 20[01]') {
    Write-Host "ENROLL BLOCKED : HTTP non-2xx" -ForegroundColor Red
    exit 1
}
if ($logs -match 'BEGIN CERTIFICATE') {
    Write-Host "ENROLL OK : cert PEM émis" -ForegroundColor Green
}
else {
    Write-Host "ENROLL HTTP 2xx (vérifier body pour certificate_pem)" -ForegroundColor Green
}

Write-Host ""
Write-Host "VALIDATION" -ForegroundColor Cyan
Invoke-Kubectl @('get', 'pod', $podName, '-n', $PluginsNs, '-o', 'wide')
Invoke-Kubectl @('get', 'pod', $podName, '-n', $PluginsNs, '-o', 'jsonpath={.spec.containers[0].image}{"\n"}')
