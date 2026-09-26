#Requires -Version 5.1
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
Set-Location $RepoRoot

$KindContext = 'kind-aiforall-local'
$ForbiddenContext = 'kind-apparatus-p4-it'
$ClusterName = 'aiforall-local'
$Image = 'aiforall-apparatus-controller:j2'
$Dockerfile = 'apparatus-operator/Dockerfile.controller'
# Pin Calico = même version que fixture IT (copie logique, pas d'import fixtures/kind).
$CalicoVersion = 'v3.29.7'
$CalicoManifestUrl = "https://raw.githubusercontent.com/projectcalico/calico/$CalicoVersion/manifests/calico.yaml"
# Kind + Calico IPPool : 10.244.0.0/16 (≠ Calico upstream 192.168.0.0/16 qui recouvre
# Docker Desktop host.docker.internal 192.168.65.254). ≠ fixture IT 192.168.0.0/16.
$DesiredPodSubnet = '10.244.0.0/16'

function Test-HasCommand {
    param([Parameter(Mandatory = $true)][string]$Name)
    return $null -ne (Get-Command $Name -ErrorAction SilentlyContinue)
}

function Invoke-Kubectl {
    param([Parameter(Mandatory = $true)][string[]]$KubectlArgs)
    & kubectl --context $KindContext @KubectlArgs
    $code = $LASTEXITCODE
    if ($code -ne 0) {
        Write-Host ("J2 echec : kubectl {0}" -f ($KubectlArgs -join ' ')) -ForegroundColor Red
        exit $code
    }
}

function Test-DaemonSetExists {
    param([Parameter(Mandatory = $true)][string]$Name)
    $prev = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    $out = & kubectl --context $KindContext -n kube-system get ds $Name -o name 2>$null
    $code = $LASTEXITCODE
    $ErrorActionPreference = $prev
    return ($code -eq 0 -and -not [string]::IsNullOrWhiteSpace([string]$out))
}

function Assert-CalicoPin {
    $prev = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    $img = & kubectl --context $KindContext -n kube-system get ds calico-node -o jsonpath='{.spec.template.spec.containers[0].image}' 2>$null
    $ErrorActionPreference = $prev
    if ([string]::IsNullOrWhiteSpace([string]$img) -or ([string]$img -notmatch [regex]::Escape($CalicoVersion))) {
        Write-Host ("J2 STOP : calico-node image '{0}' != pin {1} (recreate le cluster)" -f $img, $CalicoVersion) -ForegroundColor Red
        exit 1
    }
}

function Get-IpPoolCidr {
    $prev = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    $cidr = & kubectl --context $KindContext get ippool default-ipv4-ippool -o jsonpath='{.spec.cidr}' 2>$null
    $code = $LASTEXITCODE
    $ErrorActionPreference = $prev
    if ($code -ne 0) { return '' }
    return ([string]$cidr).Trim()
}

function Get-NodePodCidr {
    $prev = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    $cidr = & kubectl --context $KindContext get nodes -o jsonpath='{.items[0].spec.podCIDR}' 2>$null
    $code = $LASTEXITCODE
    $ErrorActionPreference = $prev
    if ($code -ne 0) { return '' }
    return ([string]$cidr).Trim()
}

function Test-WrongPodCidr {
    # Kind podSubnet / Calico IPPool = 10.244.0.0/16 ; node.spec.podCIDR = bloc /24|/26 ∈ pool.
    $ippool = Get-IpPoolCidr
    $node = Get-NodePodCidr
    if ($ippool -and $ippool -ne $DesiredPodSubnet) { return $true }
    if ($ippool -match '^192\.168\.') { return $true }
    if ($node -and $node -notmatch '^10\.244\.') { return $true }
    return $false
}

function Set-CalicoIpPoolCidr {
    # Après apply Calico : pin IPPool (souvent default-ipv4-ippool) AVANT wait Ready.
    # cidr est souvent immutable → delete + recreate si patch refuse.
    $deadline = (Get-Date).AddSeconds(120)
    $poolName = $null
    while ((Get-Date) -lt $deadline) {
        $prev = $ErrorActionPreference
        $ErrorActionPreference = 'Continue'
        $names = & kubectl --context $KindContext get ippool -o jsonpath='{.items[*].metadata.name}' 2>$null
        $ErrorActionPreference = $prev
        if ([string]$names -match 'default-ipv4-ippool') {
            $poolName = 'default-ipv4-ippool'
            break
        }
        $first = (@([string]$names -split '\s+' | Where-Object { $_ }) | Select-Object -First 1)
        if ($first) {
            $poolName = $first
            break
        }
        Start-Sleep -Seconds 2
    }
    if (-not $poolName) {
        Write-Host 'J2 echec : IPPool Calico introuvable apres apply' -ForegroundColor Red
        exit 1
    }
    $current = Get-IpPoolCidr
    if ($poolName -eq 'default-ipv4-ippool' -and $current -eq $DesiredPodSubnet) {
        Write-Host ("J2 : IPPool {0} deja {1}" -f $poolName, $DesiredPodSubnet)
        return
    }
    Write-Host ("J2 : pin IPPool {0} cidr -> {1} (was '{2}')" -f $poolName, $DesiredPodSubnet, $current) -ForegroundColor Cyan
    $patchJson = ('{{"spec":{{"cidr":"{0}"}}}}' -f $DesiredPodSubnet)
    $prev = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    & kubectl --context $KindContext patch ippool $poolName --type=merge -p $patchJson
    $patchCode = $LASTEXITCODE
    $ErrorActionPreference = $prev
    $after = Get-IpPoolCidr
    if ($patchCode -eq 0 -and $after -eq $DesiredPodSubnet) {
        return
    }
    Write-Host "J2 : IPPool cidr immutable ou patch refuse — delete + recreate $DesiredPodSubnet" -ForegroundColor Yellow
    $prev = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    & kubectl --context $KindContext delete ippool $poolName --wait=true --timeout=60s 2>$null | Out-Null
    $ErrorActionPreference = $prev
    $poolYaml = @"
apiVersion: crd.projectcalico.org/v1
kind: IPPool
metadata:
  name: default-ipv4-ippool
spec:
  blockSize: 26
  cidr: $DesiredPodSubnet
  ipipMode: Always
  natOutgoing: true
  nodeSelector: all()
  vxlanMode: Never
"@
    $poolYaml | & kubectl --context $KindContext apply -f -
    if ($LASTEXITCODE -ne 0) {
        Write-Host 'J2 echec : recreate IPPool' -ForegroundColor Red
        exit $LASTEXITCODE
    }
    $final = Get-IpPoolCidr
    if ($final -ne $DesiredPodSubnet) {
        Write-Host ("J2 echec : IPPool cidr='{0}' != {1}" -f $final, $DesiredPodSubnet) -ForegroundColor Red
        exit 1
    }
}

function Install-CalicoAndWait {
    if (Test-DaemonSetExists -Name 'kindnet') {
        Write-Host "J2 STOP : kindnet present apres create (interdit)" -ForegroundColor Red
        exit 1
    }
    if (-not (Test-DaemonSetExists -Name 'calico-node')) {
        Write-Host "J2 : apply Calico $CalicoVersion" -ForegroundColor Cyan
        & kubectl --context $KindContext apply -f $CalicoManifestUrl
        if ($LASTEXITCODE -ne 0) {
            Write-Host "J2 echec : kubectl apply Calico" -ForegroundColor Red
            exit $LASTEXITCODE
        }
        Set-CalicoIpPoolCidr
    }
    else {
        Assert-CalicoPin
        Write-Host "J2 : Calico $CalicoVersion deja present (reuse)."
        Set-CalicoIpPoolCidr
    }
    # Kind : FELIX_IGNORELOOSERPF (copie logique fixture IT, pas d'import).
    $deadline = (Get-Date).AddSeconds(90)
    $felixOk = $false
    while ((Get-Date) -lt $deadline) {
        if (Test-DaemonSetExists -Name 'calico-node') {
            & kubectl --context $KindContext -n kube-system set env daemonset/calico-node FELIX_IGNORELOOSERPF=true 2>$null
            if ($LASTEXITCODE -eq 0) {
                $felixOk = $true
                break
            }
        }
        Start-Sleep -Seconds 2
    }
    if (-not $felixOk) {
        Write-Host "J2 echec : FELIX_IGNORELOOSERPF sur calico-node" -ForegroundColor Red
        exit 1
    }
    Write-Host "J2 : wait Calico + CoreDNS Ready ($CalicoVersion)" -ForegroundColor Cyan
    Invoke-Kubectl @('rollout', 'status', 'daemonset/calico-node', '-n', 'kube-system', '--timeout=600s')
    Invoke-Kubectl @('rollout', 'status', 'deployment/calico-kube-controllers', '-n', 'kube-system', '--timeout=600s')
    Invoke-Kubectl @('rollout', 'status', 'deployment/coredns', '-n', 'kube-system', '--timeout=300s')
    if (Test-DaemonSetExists -Name 'kindnet') {
        Write-Host "J2 STOP : kindnet detecte apres Calico (interdit)" -ForegroundColor Red
        exit 1
    }
    $poolFinal = Get-IpPoolCidr
    if ($poolFinal -ne $DesiredPodSubnet) {
        Write-Host ("J2 STOP : IPPool cidr='{0}' != {1} apres Ready" -f $poolFinal, $DesiredPodSubnet) -ForegroundColor Red
        exit 1
    }
}

$missing = @()
if (-not (Test-HasCommand 'kind')) { $missing += 'kind' }
if (-not (Test-HasCommand 'docker')) { $missing += 'docker' }
if (-not (Test-HasCommand 'kubectl')) { $missing += 'kubectl' }

if ($missing.Count -gt 0) {
    Write-Host ("J2 STOP : outil manquant ({0})" -f ($missing -join ', ')) -ForegroundColor Red
    exit 1
}

Write-Host "J2 : contexte $KindContext (jamais $ForbiddenContext)" -ForegroundColor Cyan

$clusterOut = & kind get clusters 2>&1
$hasLocal = $false
$hasIt = $false
foreach ($line in @($clusterOut)) {
    $name = ([string]$line).Trim()
    if ($name -eq $ClusterName) { $hasLocal = $true }
    if ($name -eq 'apparatus-p4-it') { $hasIt = $true }
}

if ($hasLocal) {
    $kindnetPresent = $false
    $prev = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    $probe = & kubectl --context $KindContext -n kube-system get ds kindnet -o name 2>$null
    $probeCode = $LASTEXITCODE
    $ErrorActionPreference = $prev
    if ($probeCode -eq 0 -and -not [string]::IsNullOrWhiteSpace([string]$probe)) {
        $kindnetPresent = $true
    }
    $wrongCidr = $false
    if (-not $kindnetPresent) {
        $wrongCidr = Test-WrongPodCidr
    }
    if ($kindnetPresent -or $wrongCidr) {
        $why = if ($kindnetPresent) { 'kindnet' } else { ("CIDR Kind/IPPool != {0}" -f $DesiredPodSubnet) }
        Write-Host "J2 : $why detecte sur $ClusterName — delete + recreate (CIDR ne se remplace pas a chaud)" -ForegroundColor Yellow
        & kind delete cluster --name $ClusterName
        if ($LASTEXITCODE -ne 0) {
            Write-Host "J2 echec : kind delete cluster" -ForegroundColor Red
            exit $LASTEXITCODE
        }
        $hasLocal = $false
    }
}

if (-not $hasLocal) {
    Write-Host "Creation du cluster kind $ClusterName (disableDefaultCNI + Calico $CalicoVersion)..."
    & kind create cluster --config deploy/kind/cluster.yaml
    if ($LASTEXITCODE -ne 0) {
        Write-Host "J2 echec : kind create cluster" -ForegroundColor Red
        exit $LASTEXITCODE
    }
    Install-CalicoAndWait
}
else {
    Write-Host "Cluster $ClusterName deja present."
    if (-not (Test-DaemonSetExists -Name 'calico-node')) {
        Install-CalicoAndWait
    }
    else {
        Assert-CalicoPin
        Write-Host "J2 : Calico deja la — wait Ready avant manifests."
        Invoke-Kubectl @('rollout', 'status', 'daemonset/calico-node', '-n', 'kube-system', '--timeout=600s')
        Invoke-Kubectl @('rollout', 'status', 'deployment/calico-kube-controllers', '-n', 'kube-system', '--timeout=600s')
        Invoke-Kubectl @('rollout', 'status', 'deployment/coredns', '-n', 'kube-system', '--timeout=300s')
    }
}

Write-Host "J2 : docker build $Image" -ForegroundColor Cyan
& docker build --platform linux/amd64 --provenance=false --sbom=false -t $Image -f $Dockerfile .
if ($LASTEXITCODE -ne 0) {
    Write-Host "J2 echec : docker build" -ForegroundColor Red
    exit $LASTEXITCODE
}

Write-Host "J2 : kind load $Image --name $ClusterName" -ForegroundColor Cyan
& kind load docker-image $Image --name $ClusterName
if ($LASTEXITCODE -ne 0) {
    Write-Host "J2 echec : kind load" -ForegroundColor Red
    exit $LASTEXITCODE
}

$ZotImage = 'aiforall-zot:gold'
Write-Host "J2 : ensure zot image $ZotImage (single-platform kind load)" -ForegroundColor Cyan
$prev = $ErrorActionPreference
$ErrorActionPreference = 'Continue'
$null = docker image inspect $ZotImage 2>$null
$zotInspect = $LASTEXITCODE
$ErrorActionPreference = $prev
if ($zotInspect -ne 0) {
    & docker build --platform linux/amd64 --provenance=false --sbom=false -t $ZotImage -f deploy/p4/Dockerfile.zot deploy/p4
    if ($LASTEXITCODE -ne 0) {
        Write-Host "J2 echec : docker build zot" -ForegroundColor Red
        exit $LASTEXITCODE
    }
}
& kind load docker-image $ZotImage --name $ClusterName
if ($LASTEXITCODE -ne 0) {
    Write-Host "J2 echec : kind load zot" -ForegroundColor Red
    exit $LASTEXITCODE
}

# NP / manifests seulement apres Calico Ready.
Invoke-Kubectl @('apply', '-k', 'deploy/p4')
Invoke-Kubectl @('wait', '--for=condition=Established', 'crd/admissionrecords.apparatus.aiforall.dev', '--timeout=60s')
Invoke-Kubectl @('rollout', 'restart', 'deployment/apparatus-operator', '-n', 'aiforall-apparatus')
Invoke-Kubectl @('rollout', 'status', 'deployment/apparatus-operator', '-n', 'aiforall-apparatus', '--timeout=180s')
Invoke-Kubectl @('rollout', 'status', 'deployment/zot', '-n', 'aiforall-platform', '--timeout=120s')

Write-Host "J2 pod :" -ForegroundColor Cyan
Invoke-Kubectl @(
    'get', 'pod', '-n', 'aiforall-apparatus',
    '-l', 'app.kubernetes.io/name=apparatus-operator',
    '-o', 'custom-columns=NAME:.metadata.name,NS:.metadata.namespace,IMAGE:.spec.containers[0].image,PHASE:.status.phase,READY:.status.containerStatuses[0].ready'
)
Write-Host "J2 logs :" -ForegroundColor Cyan
Invoke-Kubectl @('logs', '-n', 'aiforall-apparatus', '-l', 'app.kubernetes.io/name=apparatus-operator', '--tail=20')

Write-Host "kind clusters :" -ForegroundColor Cyan
& kind get clusters
if ($hasIt) {
    Write-Host "Cluster IT apparatus-p4-it intact (non cible)."
}

Write-Host "J2 OK : apparatus-controller Ready sur $ClusterName (image $Image, Calico $CalicoVersion)." -ForegroundColor Green
exit 0
