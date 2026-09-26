#Requires -Version 5.1
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
Set-Location $RepoRoot

$KindContext = 'kind-aiforall-local'
$ForbiddenContext = 'kind-apparatus-p4-it'
$Image = 'aiforall-oodhive-monolith:j3'
$Dockerfile = 'monolith/Dockerfile'
$Overlay = 'deploy/apps/overlays/kind-demo-monolith'
$DefaultHostGw = '192.168.65.254'

function Test-HasCommand {
    param([Parameter(Mandatory = $true)][string]$Name)
    return $null -ne (Get-Command $Name -ErrorAction SilentlyContinue)
}

$missing = @()
if (-not (Test-HasCommand 'kind')) { $missing += 'kind' }
if (-not (Test-HasCommand 'docker')) { $missing += 'docker' }
if (-not (Test-HasCommand 'kubectl')) { $missing += 'kubectl' }

if ($missing.Count -gt 0) {
    Write-Host ("J3 STOP : outil manquant ({0})" -f ($missing -join ', ')) -ForegroundColor Red
    exit 1
}

Write-Host "J3 : contexte $KindContext (jamais $ForbiddenContext)" -ForegroundColor Cyan
Write-Host 'J3 : Compose hote (just up-infra) via host.docker.internal - pas de Postgres in-kind.' -ForegroundColor Cyan

$pgOk = $false
try {
    $tcp = New-Object System.Net.Sockets.TcpClient
    $tcp.ReceiveTimeout = 1000
    $tcp.SendTimeout = 1000
    $iar = $tcp.BeginConnect('127.0.0.1', 5432, $null, $null)
    $ok = $iar.AsyncWaitHandle.WaitOne(1000, $false)
    if ($ok -and $tcp.Connected) { $pgOk = $true }
    $tcp.Close()
} catch {
    $pgOk = $false
}
if (-not $pgOk) {
    Write-Host 'J3 STOP : Postgres hote :5432 injoignable. Lance just up-infra.' -ForegroundColor Red
    exit 1
}

$clusterOut = & kind get clusters 2>&1
$hasLocal = $false
$hasIt = $false
foreach ($line in @($clusterOut)) {
    $name = ([string]$line).Trim()
    if ($name -eq 'aiforall-local') { $hasLocal = $true }
    if ($name -eq 'apparatus-p4-it') { $hasIt = $true }
}
if (-not $hasLocal) {
    Write-Host 'J3 STOP : cluster kind aiforall-local absent. Utiliser just deploy-m2 pour le creer.' -ForegroundColor Red
    exit 1
}
else {
    Write-Host 'Cluster aiforall-local deja present.'
}

function Invoke-Kubectl {
    param([Parameter(Mandatory = $true)][string[]]$KubectlArgs)
    & kubectl --context $KindContext @KubectlArgs
    $code = $LASTEXITCODE
    if ($code -ne 0) {
        Write-Host ("J3 echec : kubectl {0}" -f ($KubectlArgs -join ' ')) -ForegroundColor Red
        exit $code
    }
}

function Get-HostDockerInternalIp {
    $node = 'aiforall-local-control-plane'
    $raw = & docker exec $node getent hosts host.docker.internal 2>$null
    if ($LASTEXITCODE -eq 0 -and $raw) {
        foreach ($token in ([string]$raw).Split(@(' ', "`t"), [System.StringSplitOptions]::RemoveEmptyEntries)) {
            if ($token -match '^\d+\.\d+\.\d+\.\d+$') { return $token }
        }
    }
    $hostsLine = & docker exec $node sh -c "grep -E 'host.docker.internal' /etc/hosts | head -n 1" 2>$null
    if ($hostsLine) {
        foreach ($token in ([string]$hostsLine).Split(@(' ', "`t"), [System.StringSplitOptions]::RemoveEmptyEntries)) {
            if ($token -match '^\d+\.\d+\.\d+\.\d+$') { return $token }
        }
    }
    return $DefaultHostGw
}

Write-Host "J3 : docker build $Image" -ForegroundColor Cyan
& docker build -t $Image -f $Dockerfile .
if ($LASTEXITCODE -ne 0) {
    Write-Host 'J3 STOP : docker build monolithe echoue (pas de nginx deguise).' -ForegroundColor Red
    exit $LASTEXITCODE
}

Write-Host "J3 : kind load $Image --name aiforall-local" -ForegroundColor Cyan
& kind load docker-image $Image --name aiforall-local
if ($LASTEXITCODE -ne 0) {
    Write-Host 'J3 echec : kind load monolith' -ForegroundColor Red
    exit $LASTEXITCODE
}

Write-Host 'J3 : docker build + kind load j3-curl:local' -ForegroundColor Cyan
& docker build --platform linux/amd64 -t j3-curl:local -f deploy/apps/overlays/kind-demo-monolith/Dockerfile.curl deploy/apps/overlays/kind-demo-monolith
if ($LASTEXITCODE -ne 0) {
    Write-Host 'J3 echec : docker build j3-curl' -ForegroundColor Red
    exit $LASTEXITCODE
}
& kind load docker-image j3-curl:local --name aiforall-local
if ($LASTEXITCODE -ne 0) {
    Write-Host 'J3 echec : kind load j3-curl' -ForegroundColor Red
    exit $LASTEXITCODE
}

$hostIp = Get-HostDockerInternalIp
Write-Host "J3 : host.docker.internal -> $hostIp (noeud kind / Docker Desktop)" -ForegroundColor Cyan

function Set-MonolithHostGateway {
    param([Parameter(Mandatory = $true)][string]$Ip)
    $aliasPatchFile = Join-Path $env:TEMP 'j3-hostalias.json'
    @"
[{"op":"replace","path":"/spec/template/spec/hostAliases/0/ip","value":"$Ip"},{"op":"replace","path":"/spec/template/spec/hostAliases/0/hostnames","value":["host.docker.internal","localstack"]}]
"@ | Set-Content -LiteralPath $aliasPatchFile -Encoding ascii
    Invoke-Kubectl @('patch', 'deploy', 'oodhive-monolith', '-n', 'aiforall-gateway', '--type=json', '--patch-file', $aliasPatchFile)
    # sqlx/tokio may ignore /etc/hosts (ndots search). Pin the Docker Desktop gateway IP.
    # rustycog SQS: host MUST be localhost|localstack (path-style URL). An IP host
    # talks to Scaleway SQS and starves sentinel-sync (~2 min / 403 on POST components).
    Invoke-Kubectl @(
        'set', 'env', 'deploy/oodhive-monolith', '-n', 'aiforall-gateway',
        "IAM_DATABASE__HOST=$Ip",
        "TELEGRAPH_DATABASE__HOST=$Ip",
        "HIVE_DATABASE__HOST=$Ip",
        "MANIFESTO_DATABASE__HOST=$Ip",
        "LAZARET_DATABASE__HOST=$Ip",
        "IAM_OPENFGA__HOST=$Ip",
        "TELEGRAPH_OPENFGA__HOST=$Ip",
        "HIVE_OPENFGA__HOST=$Ip",
        "MANIFESTO_OPENFGA__HOST=$Ip",
        "LAZARET_OPENFGA__HOST=$Ip",
        "MANIFESTO_QUEUE__HOST=localstack",
        "MANIFESTO_QUEUE__PORT=4566",
        "MANIFESTO_QUEUE__ENDPOINT_URL=http://${Ip}:4566",
        "MANIFESTO_QUEUE__ENABLED=true",
        "IAM_QUEUE__ENABLED=false",
        "TELEGRAPH_QUEUE__ENABLED=false",
        "HIVE_QUEUE__ENABLED=false",
        "LAZARET_QUEUE__ENABLED=false",
        "MANIFESTO_SERVICE__COMPONENT_SERVICE__BASE_URL=http://${Ip}:9000",
        "MANIFESTO_SERVICE__COMPONENT_SERVICE__API_KEY=aiforall-gold-catalog",
        "LAZARET_PLUGIN_HOP__USE_DNS_FORMULA=true",
        "LAZARET_PLUGIN_HOP__ENDPOINT_URL-"
    )
    $envFile = Join-Path $RepoRoot '.env.openfga'
    if (-not (Test-Path -LiteralPath $envFile)) {
        Write-Host 'J3 : .env.openfga absent — openfga/ensure-host-store.ps1'
        . (Join-Path $RepoRoot 'openfga\ensure-host-store.ps1')
    }
    $storeId = $null
    $modelId = $null
    Get-Content -LiteralPath $envFile | ForEach-Object {
        if ($_ -match '^MANIFESTO_OPENFGA__STORE_ID=(.+)$') { $storeId = $Matches[1].Trim() }
        if ($_ -match '^MANIFESTO_OPENFGA__AUTHORIZATION_MODEL_ID=(.+)$') { $modelId = $Matches[1].Trim() }
    }
    if ($storeId -and $modelId) {
        Invoke-Kubectl @(
            'set', 'env', 'deploy/oodhive-monolith', '-n', 'aiforall-gateway',
            "MANIFESTO_OPENFGA__STORE_ID=$storeId",
            "MANIFESTO_OPENFGA__AUTHORIZATION_MODEL_ID=$modelId"
        )
    } else {
        Write-Host 'J3 WARN : store OpenFGA manquant dans .env.openfga (POST components restera 403).'
    }
}

Invoke-Kubectl @('delete', 'job', 'invoke-probe-j3', '-n', 'aiforall-plugins', '--ignore-not-found=true')
Invoke-Kubectl @('apply', '-k', $Overlay)
Invoke-Kubectl @('delete', 'job', 'invoke-probe-j3', '-n', 'aiforall-plugins', '--ignore-not-found=true')
Set-MonolithHostGateway -Ip $hostIp

# NP OpenBao :8200 = meme IP que hostAliases (pas 0.0.0.0/0).
$prevNp = $ErrorActionPreference
$ErrorActionPreference = 'Continue'
$npJson = & kubectl --context $KindContext -n aiforall-apparatus get networkpolicy allow-apparatus-admit-egress -o json 2>$null
$npCode = $LASTEXITCODE
$ErrorActionPreference = $prevNp
if ($npCode -eq 0 -and $npJson) {
    $npObj = $npJson | ConvertFrom-Json
    $npIdx = 0
    $npFound = $false
    foreach ($rule in $npObj.spec.egress) {
        $hit = $false
        foreach ($p in @($rule.ports)) {
            if ([string]$p.port -eq '8200') { $hit = $true }
        }
        if ($hit) { $npFound = $true; break }
        $npIdx++
    }
    if ($npFound) {
        $npPatch = Join-Path $env:TEMP 'j3-np-8200.json'
        @"
[{"op":"replace","path":"/spec/egress/$npIdx/to/0/ipBlock/cidr","value":"$hostIp/32"}]
"@ | Set-Content -LiteralPath $npPatch -Encoding ascii
        Write-Host "J3 : NP OpenBao :8200 cidr=$hostIp/32" -ForegroundColor Cyan
        Invoke-Kubectl @('patch', 'networkpolicy', 'allow-apparatus-admit-egress', '-n', 'aiforall-apparatus', '--type=json', '--patch-file', $npPatch)
    }
}

$enrollUrl = 'http://oodhive-monolith.aiforall-gateway.svc.cluster.local:8080/lazaret/enroll'
Write-Host "J3 : operator LAZARET_ENROLL_URL via APPARATUS_PLUGIN_ENROLL_URL=$enrollUrl (si J2 present)" -ForegroundColor Cyan
& kubectl --context $KindContext set env deploy/apparatus-operator -n aiforall-apparatus "APPARATUS_PLUGIN_ENROLL_URL=$enrollUrl"
if ($LASTEXITCODE -ne 0) {
    Write-Host 'J3 : operator absent (deploy-j2 pas encore applique) — enroll env sera pose au prove-gold / j2.'
}

Write-Host 'J3 : wait Ready oodhive-monolith' -ForegroundColor Cyan
& kubectl --context $KindContext rollout status deployment/oodhive-monolith -n aiforall-gateway --timeout=240s
if ($LASTEXITCODE -ne 0) {
    Write-Host 'J3 STOP : oodhive-monolith pas Ready. Logs :' -ForegroundColor Red
    & kubectl --context $KindContext logs -n aiforall-gateway -l app.kubernetes.io/name=oodhive-monolith --tail=80
    & kubectl --context $KindContext describe deploy oodhive-monolith -n aiforall-gateway
    exit 1
}

Invoke-Kubectl @('delete', 'job', 'invoke-probe-j3', '-n', 'aiforall-plugins', '--ignore-not-found=true')
Invoke-Kubectl @('apply', '-f', 'deploy/apps/overlays/kind-demo-monolith/invoke-probe-j3-job.yaml')
Invoke-Kubectl @('wait', '--for=condition=complete', 'job/invoke-probe-j3', '-n', 'aiforall-plugins', '--timeout=180s')

Write-Host 'J3 preuve HTTP (logs job) :' -ForegroundColor Cyan
& kubectl --context $KindContext logs -n aiforall-plugins job/invoke-probe-j3 --tail=40
$probeLogs = & kubectl --context $KindContext logs -n aiforall-plugins job/invoke-probe-j3
$probeText = [string]$probeLogs
if ($probeText -match '(?m)^BODY ok\s*$' -or $probeText -match 'nginx stub still in path') {
    Write-Host 'J3 FAIL : corps ok (nginx) encore en chemin.' -ForegroundColor Red
    exit 1
}
if ($probeText -notmatch 'HTTP 401' -or $probeText -notmatch 'unauthorized') {
    Write-Host 'J3 FAIL : preuve 401 JSON unauthorized absente des logs.' -ForegroundColor Red
    exit 1
}

Write-Host 'Deployments :' -ForegroundColor Cyan
Invoke-Kubectl @('get', 'deploy', '-A')
Write-Host 'J3 cible :' -ForegroundColor Cyan
Invoke-Kubectl @(
    'get', 'pod', '-n', 'aiforall-gateway',
    '-l', 'app.kubernetes.io/name=oodhive-monolith',
    '-o', 'custom-columns=NAME:.metadata.name,NS:.metadata.namespace,IMAGE:.spec.containers[0].image,PHASE:.status.phase,READY:.status.containerStatuses[0].ready'
)

Write-Host 'kind clusters :' -ForegroundColor Cyan
& kind get clusters
if ($hasIt) {
    Write-Host 'Cluster IT apparatus-p4-it intact (non cible).'
}

Write-Host "J3 OK : POST in-cluster /lazaret/invoke = 401 JSON (image $Image)." -ForegroundColor Green
exit 0
