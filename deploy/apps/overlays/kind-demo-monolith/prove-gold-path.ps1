#Requires -Version 5.1
<#
.SYNOPSIS
  Gold path Kind (ADR-0605 G1–G5) : HTTP 200 in-cluster sur oodhive-monolith.

.DESCRIPTION
  Signup → projet → POST components (sans seed SQL) → T5 consents → VALID Adm-A
  → Pod+Service plugin-{32hex} → enroll workload → session mTLS → kv.get 200
  dont hostname = plugin-{digest}.

  Preuve = kubectl + HTTP ClusterIP. Curl hôte :8080 n'est pas la preuve.
  Jamais apparatus-p4-it. Pas de schedule-reference-kv comme étape nominale.
#>
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..')
Set-Location $RepoRoot

$KindContext = 'kind-aiforall-local'
$ForbiddenContext = 'kind-apparatus-p4-it'
$CurlImage = 'j3-curl:local'
$MonolithHttp = 'http://oodhive-monolith.aiforall-gateway.svc.cluster.local:8080'
$MonolithHttps = 'https://oodhive-monolith.aiforall-gateway.svc.cluster.local:8443'
$EnrollUrl = "$MonolithHttp/lazaret/enroll"
$Digest = 'sha256:98ba747fc572de29d76dfd08f92537782bf0353b652adcace10200020ee560cf'
$PluginHost = 'plugin-98ba747fc572de29d76dfd08f9253778'
$Stamp = Get-Date -Format 'yyyyMMddHHmmss'
$Email = "gold-$Stamp@local.test"
$Password = 'GoldPath1234'
$Username = "gold$Stamp"
$ProjectName = "gold-$Stamp"
$CatalogToken = 'aiforall-gold-catalog'
$Observed = [System.Collections.Generic.List[string]]::new()

function Invoke-Kubectl {
    param([Parameter(Mandatory = $true)][string[]]$KubectlArgs)
    & kubectl --context $KindContext @KubectlArgs
    if ($LASTEXITCODE -ne 0) {
        throw ("kubectl failed ({0}): {1}" -f $LASTEXITCODE, ($KubectlArgs -join ' '))
    }
}

function Invoke-InClusterCurl {
    param(
        [Parameter(Mandatory = $true)][string]$Method,
        [Parameter(Mandatory = $true)][string]$Url,
        [string]$Body = '',
        [string[]]$Headers = @(),
        [string[]]$Extra = @()
    )
    $hdr = @()
    foreach ($h in $Headers) { $hdr += @('-H', $h) }
    $data = @()
    if ($Body) {
        # PowerShell pipe → kubectl is often UTF-16; IAM rejects as invalid_json_syntax.
        $b64 = [Convert]::ToBase64String([System.Text.Encoding]::UTF8.GetBytes($Body))
        & kubectl --context $KindContext exec -n aiforall-plugins gold-curl -- sh -c "echo $b64 | base64 -d > /tmp/gold.req.json"
        if ($LASTEXITCODE -ne 0) { throw 'kubectl exec write request body failed' }
        $data = @('-H', 'Content-Type: application/json', '--data-binary', '@/tmp/gold.req.json')
    }
    $out = Invoke-Kubectl -KubectlArgs (@(
            'exec', '-n', 'aiforall-plugins', 'gold-curl', '--',
            'curl', '-sS', '-k', '-o', '/tmp/gold.body', '-w', '%{http_code}',
            '-X', $Method, $Url
        ) + $hdr + $data + $Extra)
    $code = [string]$out
    $body = ''
    try {
        $body = (Invoke-Kubectl -KubectlArgs @('exec', '-n', 'aiforall-plugins', 'gold-curl', '--', 'cat', '/tmp/gold.body') | Out-String).Trim()
    } catch {
        $body = ''
    }
    Write-Host ("IN-CLUSTER {0} {1} -> {2}" -f $Method, $Url, $code)
    if ($body) { Write-Host "BODY $body" }
    $Observed.Add(("{0} {1} -> {2}" -f $Method, $Url, $code)) | Out-Null
    return [pscustomobject]@{ Code = [int]$code; Body = $body }
}

function Write-Observed {
    Write-Host 'HTTP codes observes :'
    foreach ($line in $Observed) { Write-Host "  $line" }
}

function Get-DbVerificationToken {
    param([string]$TargetEmail)
    $sql = "SELECT verification_token FROM user_email_verification WHERE email = '$TargetEmail' ORDER BY created_at DESC LIMIT 1;"
    $token = docker compose exec -T postgres psql -U postgres -d iam_dev -t -A -c $sql
    if ($LASTEXITCODE -ne 0) { return $null }
    $token = ($token | Out-String).Trim()
    if ($token -and $token -notmatch 'ERROR') { return $token }
    return $null
}

Write-Host "Gold path Kind : contexte $KindContext (jamais $ForbiddenContext)" -ForegroundColor Cyan
$cur = (& kubectl config current-context 2>$null)
if ("$cur" -eq $ForbiddenContext) {
    throw "STOP: contexte $ForbiddenContext interdit"
}

Write-Host "ensure curl image $CurlImage (alpine curl+jq)"
$imgOk = docker image inspect $CurlImage 2>$null
if (-not $imgOk) {
    & docker build --platform linux/amd64 -t $CurlImage -f deploy/apps/overlays/kind-demo-monolith/Dockerfile.curl deploy/apps/overlays/kind-demo-monolith
    if ($LASTEXITCODE -ne 0) { throw "docker build $CurlImage failed" }
}
& kind load docker-image $CurlImage --name aiforall-local
if ($LASTEXITCODE -ne 0) { throw "kind load $CurlImage failed" }

Write-Host 'operator enroll env (HTTP 8080 T14b, nest in-cluster)'
& kubectl --context $KindContext set env deploy/apparatus-operator -n aiforall-apparatus "APPARATUS_PLUGIN_ENROLL_URL=$EnrollUrl" 2>$null | Out-Null

$gwIp = (& kubectl --context $KindContext -n aiforall-gateway get deploy oodhive-monolith -o jsonpath="{.spec.template.spec.hostAliases[0].ip}").Trim()
if (-not $gwIp) { $gwIp = '192.168.65.254' }
Write-Host "compose egress NP + LocalStack SQS (host=localstack endpoint=$gwIp)"
& kubectl --context $KindContext apply -f deploy/apps/overlays/kind-demo-monolith/allow-oodhive-compose-egress-np.yaml | Out-Null
$aliasPatchFile = Join-Path $env:TEMP 'gold-hostalias.json'
@"
[{"op":"replace","path":"/spec/template/spec/hostAliases/0/ip","value":"$gwIp"},{"op":"replace","path":"/spec/template/spec/hostAliases/0/hostnames","value":["host.docker.internal","localstack"]}]
"@ | Set-Content -LiteralPath $aliasPatchFile -Encoding ascii
& kubectl --context $KindContext patch deploy oodhive-monolith -n aiforall-gateway --type=json --patch-file $aliasPatchFile | Out-Null

$curQueueHost = (& kubectl --context $KindContext -n aiforall-gateway get deploy oodhive-monolith -o jsonpath="{.spec.template.spec.containers[0].env[?(@.name=='MANIFESTO_QUEUE__HOST')].value}").Trim()
$needRollout = $false
if ($curQueueHost -ne 'localstack') {
    & kubectl --context $KindContext set env deploy/oodhive-monolith -n aiforall-gateway `
        "MANIFESTO_QUEUE__HOST=localstack" `
        "MANIFESTO_QUEUE__PORT=4566" `
        "MANIFESTO_QUEUE__ENDPOINT_URL=http://${gwIp}:4566" `
        "MANIFESTO_QUEUE__ENABLED=true" `
        "MANIFESTO_SERVICE__COMPONENT_SERVICE__BASE_URL=http://${gwIp}:9000" `
        "MANIFESTO_SERVICE__COMPONENT_SERVICE__API_KEY=$CatalogToken" | Out-Null
    $needRollout = $true
}
$curApiKey = ''
$rawApiKey = & kubectl --context $KindContext -n aiforall-gateway get deploy oodhive-monolith -o jsonpath="{.spec.template.spec.containers[0].env[?(@.name=='MANIFESTO_SERVICE__COMPONENT_SERVICE__API_KEY')].value}" 2>$null
if ($null -ne $rawApiKey) { $curApiKey = "$rawApiKey".Trim() }
if ($curApiKey -ne $CatalogToken) {
    & kubectl --context $KindContext set env deploy/oodhive-monolith -n aiforall-gateway `
        "MANIFESTO_SERVICE__COMPONENT_SERVICE__API_KEY=$CatalogToken" | Out-Null
    $needRollout = $true
}

Write-Host 'hop DNS 0605 : USE_DNS_FORMULA=true, unset ENDPOINT_URL'
$curDns = ''
$rawDns = & kubectl --context $KindContext -n aiforall-gateway get deploy oodhive-monolith -o jsonpath="{.spec.template.spec.containers[0].env[?(@.name=='LAZARET_PLUGIN_HOP__USE_DNS_FORMULA')].value}" 2>$null
if ($null -ne $rawDns) { $curDns = "$rawDns".Trim() }
$curHopUrl = ''
$rawHop = & kubectl --context $KindContext -n aiforall-gateway get deploy oodhive-monolith -o jsonpath="{.spec.template.spec.containers[0].env[?(@.name=='LAZARET_PLUGIN_HOP__ENDPOINT_URL')].value}" 2>$null
if ($null -ne $rawHop) { $curHopUrl = "$rawHop".Trim() }
& kubectl --context $KindContext set env deploy/oodhive-monolith -n aiforall-gateway `
    "LAZARET_PLUGIN_HOP__USE_DNS_FORMULA=true" `
    "LAZARET_PLUGIN_HOP__ENDPOINT_URL-" | Out-Null
if ($curDns -ne 'true' -or $curHopUrl) {
    $needRollout = $true
}

$envFile = Join-Path $RepoRoot '.env.openfga'
if (Test-Path -LiteralPath $envFile) {
    $storeId = $null
    $modelId = $null
    Get-Content -LiteralPath $envFile | ForEach-Object {
        if ($_ -match '^MANIFESTO_OPENFGA__STORE_ID=(.+)$') { $storeId = $Matches[1].Trim() }
        if ($_ -match '^MANIFESTO_OPENFGA__AUTHORIZATION_MODEL_ID=(.+)$') { $modelId = $Matches[1].Trim() }
    }
    $curStore = (& kubectl --context $KindContext -n aiforall-gateway get deploy oodhive-monolith -o jsonpath="{.spec.template.spec.containers[0].env[?(@.name=='MANIFESTO_OPENFGA__STORE_ID')].value}").Trim()
    if ($storeId -and $modelId -and $curStore -ne $storeId) {
        Write-Host "OpenFGA store pin $storeId"
        & kubectl --context $KindContext set env deploy/oodhive-monolith -n aiforall-gateway "MANIFESTO_OPENFGA__STORE_ID=$storeId" "MANIFESTO_OPENFGA__AUTHORIZATION_MODEL_ID=$modelId" | Out-Null
        $needRollout = $true
    }
}
if ($needRollout) {
    & kubectl --context $KindContext -n aiforall-gateway rollout status deploy/oodhive-monolith --timeout=180s | Out-Null
}

if (-not (Get-Process -Name sentinel-sync -ErrorAction SilentlyContinue)) {
    Write-Host 'sentinel-sync hote absent — just sentinel-sync'
    Start-Process -FilePath 'just' -ArgumentList 'sentinel-sync' -WorkingDirectory $RepoRoot -WindowStyle Hidden
    Start-Sleep -Seconds 8
}

function Get-CatalogHttpCode {
    param([switch]$WithBearer)
    $hdr = @()
    if ($WithBearer) { $hdr = @('-H', "Authorization: Bearer $CatalogToken") }
    return [string](& curl.exe -sS -o NUL -w '%{http_code}' --max-time 2 @hdr 'http://127.0.0.1:9000/api/components')
}

$anonCat = Get-CatalogHttpCode
if ("$anonCat" -eq '200') {
    Write-Host 'catalogue :9000 encore anonyme — docker rm aiforall-component-catalog + just component-catalog'
    cmd /c "docker rm -f aiforall-component-catalog >NUL 2>&1"
    Start-Process -FilePath 'just' -ArgumentList 'component-catalog' -WorkingDirectory $RepoRoot -WindowStyle Hidden
} elseif ("$anonCat" -ne '401') {
    Write-Host 'catalogue hote :9000 absent — just component-catalog'
    Start-Process -FilePath 'just' -ArgumentList 'component-catalog' -WorkingDirectory $RepoRoot -WindowStyle Hidden
}

$catDeadline = (Get-Date).AddSeconds(60)
do {
    $anonCat = Get-CatalogHttpCode
    $authCat = Get-CatalogHttpCode -WithBearer
    if ("$anonCat" -eq '401' -and "$authCat" -eq '200') { break }
    if ((Get-Date) -ge $catDeadline) {
        throw "STOP: catalogue :9000 attendu anonyme 401 + Bearer 200 (obtenu anon=$anonCat auth=$authCat)"
    }
    Start-Sleep -Seconds 1
} while ($true)
Write-Host "L3 catalogue anonyme HTTP $anonCat ; Bearer HTTP $authCat"

$CriImage = 'apparatus-reference-kv@sha256:07b9e8aa1f1ee8a78d50cc142798778a1ee47c2da95d5de94575bd4ed20c70ab'
$CriTag = 'apparatus-reference-kv:e2e'
$ReportDigest = 'sha256:715b2e4c77accec63f2f5376cbaf95e3ac138168af2066aabf46626862e2e43d'
$AdmitImage = 'aiforall-apparatus-admit:gold'
$ZotSvc = 'zot.aiforall-platform.svc.cluster.local:5000'

function Ensure-TransitCosign {
    Write-Host 'Transit Cosign : seed OpenBao compose (D-TRANSIT-TCB = meme -dev que KV)'
    $composeNames = (& docker compose ps --format '{{.Names}}' 2>$null)
    if ("$composeNames" -notmatch 'openbao') {
        throw 'STOP: OpenBao compose absent (just up-infra)'
    }
    & docker compose run --rm --no-deps openbao-seed
    if ($LASTEXITCODE -ne 0) { throw "STOP: openbao-seed exit $LASTEXITCODE" }
    $mounts = & docker compose exec -T openbao bao secrets list 2>$null
    if ("$mounts" -notmatch 'transit') {
        throw 'STOP: Transit mount absent apres openbao-seed'
    }
}

function Connect-OpenBaoToKindNetwork {
    $id = (& docker compose ps -q openbao | Out-String).Trim()
    if (-not $id) { throw 'STOP: container openbao introuvable' }
    $prev = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    $out = & docker network connect kind $id 2>&1 | Out-String
    $netCode = $LASTEXITCODE
    $ErrorActionPreference = $prev
    if ($netCode -ne 0 -and "$out" -notmatch 'already exists|already connected') {
        throw "STOP: docker network connect kind openbao : $out"
    }
}

function Get-KindControlPlaneIp {
    $info = & docker inspect aiforall-local-control-plane | ConvertFrom-Json
    $ip = $null
    try { $ip = [string]$info[0].NetworkSettings.Networks.kind.IPAddress } catch { $ip = $null }
    if (-not $ip) {
        try { $ip = [string]$info.NetworkSettings.Networks.kind.IPAddress } catch { $ip = $null }
    }
    $ip = "$ip".Trim()
    if (-not $ip) { throw 'STOP: IP kind control-plane absente (reseau kind)' }
    return $ip
}

function Patch-OpenBaoEgressCidr {
    param([Parameter(Mandatory = $true)][string]$Ip)
    $cidr = "$Ip/32"
    Write-Host "NP OpenBao :8200 cidr=$cidr"
    $raw = & kubectl --context $KindContext -n aiforall-apparatus get networkpolicy allow-apparatus-admit-egress -o json
    if ($LASTEXITCODE -ne 0) { throw 'STOP: NP allow-apparatus-admit-egress absente' }
    $obj = $raw | ConvertFrom-Json
    $idx = 0
    $found = $false
    foreach ($rule in $obj.spec.egress) {
        $hit = $false
        foreach ($p in @($rule.ports)) {
            if ([string]$p.port -eq '8200') { $hit = $true }
        }
        if ($hit) { $found = $true; break }
        $idx++
    }
    if (-not $found) { throw 'STOP: NP allow-apparatus-admit-egress sans egress :8200' }
    $patchFile = Join-Path $env:TEMP 'np-8200.json'
    @"
[{"op":"replace","path":"/spec/egress/$idx/to/0/ipBlock/cidr","value":"$cidr"}]
"@ | Set-Content -LiteralPath $patchFile -Encoding ascii
    Invoke-Kubectl -KubectlArgs @('patch', 'networkpolicy', 'allow-apparatus-admit-egress', '-n', 'aiforall-apparatus', '--type=json', '--patch-file', $patchFile)
}

function Ensure-OpenBaoKubernetes {
    Write-Host 'OpenBao kubernetes auth : kind network + TokenReview + roles'
    Connect-OpenBaoToKindNetwork
    $nodeIp = Get-KindControlPlaneIp
    $caB64 = (& kubectl --context $KindContext config view --raw --minify -o jsonpath="{.clusters[0].cluster.certificate-authority-data}" | Out-String).Trim()
    if (-not $caB64) { throw 'STOP: CA kubeconfig absente' }
    $caFile = Join-Path $env:TEMP 'aiforall-kind-ca.crt'
    [IO.File]::WriteAllBytes($caFile, [Convert]::FromBase64String($caB64))
    $reviewerYaml = @"
apiVersion: v1
kind: ServiceAccount
metadata:
  name: openbao-tokenreview
  namespace: kube-system
---
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRoleBinding
metadata:
  name: openbao-tokenreview
roleRef:
  apiGroup: rbac.authorization.k8s.io
  kind: ClusterRole
  name: system:auth-delegator
subjects:
  - kind: ServiceAccount
    name: openbao-tokenreview
    namespace: kube-system
"@
    $reviewerFile = Join-Path $env:TEMP 'openbao-tokenreview.yaml'
    Set-Content -LiteralPath $reviewerFile -Value $reviewerYaml -Encoding ascii
    Invoke-Kubectl -KubectlArgs @('apply', '-f', $reviewerFile)
    Write-Host "probe TokenReview login admit-sign (kubernetes_host=https://${nodeIp}:6443)"
    $saJwt = (& kubectl --context $KindContext create token admit-sign -n aiforall-apparatus --duration=10m | Out-String).Trim()
    if ($saJwt) {
        $loginJson = '{"role":"admit-sign","jwt":"' + $saJwt + '"}'
        $loginFile = Join-Path $env:TEMP 'bao-k8s-login.json'
        Set-Content -LiteralPath $loginFile -Value $loginJson -Encoding ascii
        $loginOut = Join-Path $env:TEMP 'bao-k8s-login.out'
        $prevFast = $ErrorActionPreference
        $ErrorActionPreference = 'Continue'
        $fastCode = & curl.exe -sS -o $loginOut -w '%{http_code}' --max-time 8 -X POST 'http://127.0.0.1:8200/v1/auth/kubernetes/login' -H 'Content-Type: application/json' --data-binary "@$loginFile"
        $ErrorActionPreference = $prevFast
        if ("$fastCode" -eq '200') {
            Write-Host 'OpenBao kubernetes login admit-sign HTTP 200 (seed k8s deja en place)'
            return
        }
    }
    $reviewerJwt = (& kubectl --context $KindContext create token openbao-tokenreview -n kube-system --duration=12h | Out-String).Trim()
    if (-not $reviewerJwt) { throw 'STOP: kubectl create token reviewer vide' }
    & docker compose run --rm --no-deps `
        -e "KUBERNETES_HOST=https://${nodeIp}:6443" `
        -e 'KUBERNETES_CA_CERT_FILE=/k8s-ca.crt' `
        -e "TOKEN_REVIEWER_JWT=$reviewerJwt" `
        -v "${caFile}:/k8s-ca.crt:ro" `
        openbao-seed
    if ($LASTEXITCODE -ne 0) { throw "STOP: openbao-seed kubernetes auth exit $LASTEXITCODE" }
    $saJwt = (& kubectl --context $KindContext create token admit-sign -n aiforall-apparatus --duration=10m | Out-String).Trim()
    if (-not $saJwt) { throw 'STOP: kubectl create token admit-sign vide' }
    $loginJson = '{"role":"admit-sign","jwt":"' + $saJwt + '"}'
    $loginFile = Join-Path $env:TEMP 'bao-k8s-login.json'
    Set-Content -LiteralPath $loginFile -Value $loginJson -Encoding ascii
    $loginOut = Join-Path $env:TEMP 'bao-k8s-login.out'
    $prev = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    $code = & curl.exe -sS -o $loginOut -w '%{http_code}' --max-time 15 -X POST 'http://127.0.0.1:8200/v1/auth/kubernetes/login' -H 'Content-Type: application/json' --data-binary "@$loginFile"
    $curlExit = $LASTEXITCODE
    $ErrorActionPreference = $prev
    if ($curlExit -ne 0 -or "$code" -ne '200') {
        $body = ''
        if (Test-Path -LiteralPath $loginOut) { $body = Get-Content -LiteralPath $loginOut -Raw }
        throw ("STOP: OpenBao kubernetes login TokenReview impossible (curl_exit={0} http={1}). Ne pas retomber sur le token root." -f $curlExit, $code)
    }
    Write-Host 'OpenBao kubernetes login admit-sign HTTP 200'
}

function Ensure-ControllerImage {
    $controllerImage = 'aiforall-apparatus-controller:j2'
    Write-Host "ensure controller image $controllerImage"
    $prev = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    $null = docker image inspect $controllerImage 2>$null
    $inspectCode = $LASTEXITCODE
    $ErrorActionPreference = $prev
    if ($inspectCode -ne 0) {
        & docker build --platform linux/amd64 --provenance=false --sbom=false -t $controllerImage -f apparatus-operator/Dockerfile.controller .
        if ($LASTEXITCODE -ne 0) { throw "STOP: docker build $controllerImage failed" }
    }
    & kind load docker-image $controllerImage --name aiforall-local
    if ($LASTEXITCODE -ne 0) { throw "STOP: kind load $controllerImage failed" }
}

function Ensure-ZotInCluster {
    param([string]$HostGw)
    Write-Host "zot in-cluster aiforall-platform (registry $ZotSvc)"
    $zotLocal = 'aiforall-zot:gold'
    $imgOk = docker image inspect $zotLocal 2>$null
    if (-not $imgOk) {
        & docker build --platform linux/amd64 --provenance=false --sbom=false -t $zotLocal -f deploy/p4/Dockerfile.zot deploy/p4
        if ($LASTEXITCODE -ne 0) { throw "STOP: docker build $zotLocal failed" }
    }
    & kind load docker-image $zotLocal --name aiforall-local
    if ($LASTEXITCODE -ne 0) { throw "STOP: kind load zot failed" }
    Invoke-Kubectl -KubectlArgs @('apply', '-k', 'deploy/p4')
    Invoke-Kubectl -KubectlArgs @('rollout', 'status', 'deploy/zot', '-n', 'aiforall-platform', '--timeout=120s')
    return $HostGw
}

function Assert-ZotAuthNegatives {
    Write-Host 'L1 negatif : zot GET /v2/ anonyme -> 401 ; push reader refuse'
    $pod = "zot-probe-$Stamp"
    $prev = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    & kubectl --context $KindContext -n aiforall-apparatus delete pod $pod --ignore-not-found --wait=true --timeout=60s 2>$null | Out-Null
    $ErrorActionPreference = $prev
    Invoke-Kubectl -KubectlArgs @(
        'run', $pod, '-n', 'aiforall-apparatus',
        '--restart=Never', "--image=$CurlImage", '--image-pull-policy=Never',
        '--command', '--', 'sleep', '90'
    )
    Invoke-Kubectl -KubectlArgs @('wait', '-n', 'aiforall-apparatus', "pod/$pod", '--for=condition=Ready', '--timeout=60s')
    $anon = Invoke-Kubectl -KubectlArgs @(
        'exec', '-n', 'aiforall-apparatus', $pod, '--',
        'curl', '-sS', '-o', '/dev/null', '-w', '%{http_code}', '--max-time', '8',
        "http://$ZotSvc/v2/"
    )
    if ("$anon" -ne '401') {
        $ErrorActionPreference = 'Continue'
        & kubectl --context $KindContext -n aiforall-apparatus delete pod $pod --ignore-not-found 2>$null | Out-Null
        $ErrorActionPreference = $prev
        throw "STOP: zot anonyme GET /v2/ attendu 401, obtenu $anon"
    }
    $readerUserB64 = & kubectl --context $KindContext -n aiforall-apparatus get secret zot-registry-accounts -o jsonpath="{.data.reader-user}"
    $readerPassB64 = & kubectl --context $KindContext -n aiforall-apparatus get secret zot-registry-accounts -o jsonpath="{.data.reader-password}"
    $readerUser = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String("$readerUserB64"))
    $readerPass = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String("$readerPassB64"))
    $push = Invoke-Kubectl -KubectlArgs @(
        'exec', '-n', 'aiforall-apparatus', $pod, '--',
        'curl', '-sS', '-o', '/dev/null', '-w', '%{http_code}', '--max-time', '8',
        '-u', "${readerUser}:${readerPass}",
        '-X', 'POST',
        "http://$ZotSvc/v2/apparatus/envelope/blobs/uploads/"
    )
    $ErrorActionPreference = 'Continue'
    & kubectl --context $KindContext -n aiforall-apparatus delete pod $pod --ignore-not-found 2>$null | Out-Null
    $ErrorActionPreference = $prev
    if ("$push" -match '^(200|201|202)$') {
        throw "STOP: zot push reader attendu refuse, obtenu $push"
    }
    Write-Host "L1 zot anonyme HTTP $anon ; reader push HTTP $push"
}

function Assert-L2Negatives {
    param([Parameter(Mandatory = $true)][string]$HostGw)
    Write-Host 'L2 negatif : pas de lazaret-dev-root sur operator / admit job'
    $opYaml = & kubectl --context $KindContext -n aiforall-apparatus get deploy apparatus-operator -o yaml | Out-String
    if ($opYaml -match 'lazaret-dev-root') { throw 'STOP: lazaret-dev-root encore dans operator' }
    $jobYaml = & kubectl --context $KindContext -n aiforall-apparatus get job "apparatus-admit-gold-$Stamp" -o yaml | Out-String
    if ($jobYaml -match 'lazaret-dev-root') { throw 'STOP: lazaret-dev-root encore dans admit job' }
    Write-Host 'L2 negatif : plugins -> :8200 timeout'
    $prev = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    $p8200 = & kubectl --context $KindContext exec -n aiforall-plugins gold-curl -- curl -sS -o /dev/null -w '%{http_code}' --max-time 3 "http://${HostGw}:8200/v1/sys/health" 2>&1 | Out-String
    $pCode = $LASTEXITCODE
    $ErrorActionPreference = $prev
    if ($pCode -eq 0 -and "$p8200".Trim() -eq '200') {
        throw "STOP: plugins atteignent OpenBao :8200 (attendu timeout)"
    }
    Write-Host "L2 plugins :8200 curl_exit=$pCode"
    Write-Host 'L2 negatif : Job admit sans projection token -> fail'
    if (-not $admitJobFile -or -not (Test-Path -LiteralPath $admitJobFile)) {
        throw 'STOP: admit job yaml introuvable pour negatif notoken'
    }
    $ntName = "apparatus-admit-notoken-$Stamp"
    $ntYaml = (Get-Content -LiteralPath $admitJobFile -Raw).Replace("apparatus-admit-gold-$Stamp", $ntName).Replace('automountServiceAccountToken: true', 'automountServiceAccountToken: false')
    $ntFile = Join-Path $env:TEMP "$ntName.yaml"
    Set-Content -LiteralPath $ntFile -Value $ntYaml -Encoding utf8
    $ErrorActionPreference = 'Continue'
    & kubectl --context $KindContext -n aiforall-apparatus delete job $ntName --ignore-not-found 2>$null | Out-Null
    $ErrorActionPreference = $prev
    Invoke-Kubectl -KubectlArgs @('apply', '-f', $ntFile)
    $ntFailed = $false
    $deadlineNt = (Get-Date).AddSeconds(90)
    while ((Get-Date) -lt $deadlineNt) {
        $js = & kubectl --context $KindContext -n aiforall-apparatus get job $ntName -o json 2>$null
        if ($LASTEXITCODE -eq 0 -and $js) {
            try {
                $st = ($js | ConvertFrom-Json).status
                if ($st.failed -ge 1) { $ntFailed = $true; break }
                if ($st.succeeded -ge 1) { throw 'STOP: Job notoken a reussi (attendu fail-closed sans token SA)' }
            } catch {
                if ("$_" -match '^STOP:') { throw }
            }
        }
        Start-Sleep -Seconds 2
    }
    $ErrorActionPreference = 'Continue'
    & kubectl --context $KindContext -n aiforall-apparatus logs -l "job-name=$ntName" --tail=20 2>&1 | Write-Host
    & kubectl --context $KindContext -n aiforall-apparatus delete job $ntName --ignore-not-found 2>$null | Out-Null
    $ErrorActionPreference = $prev
    if (-not $ntFailed) { throw "STOP: Job $ntName n a pas fail-closed en 90s" }
    Write-Host "L2 Job $ntName failed as expected"
}

function Ensure-AdmitImage {
    Write-Host "ensure admit image $AdmitImage"
    $prev = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    $null = docker image inspect $AdmitImage 2>$null
    $inspectCode = $LASTEXITCODE
    $ErrorActionPreference = $prev
    if ($inspectCode -ne 0) {
        & docker build --platform linux/amd64 --provenance=false --sbom=false -t $AdmitImage -f apparatus-operator/Dockerfile.admit .
        if ($LASTEXITCODE -ne 0) { throw "STOP: docker build $AdmitImage failed" }
    }
    & kind load docker-image $AdmitImage --name aiforall-local
    if ($LASTEXITCODE -ne 0) { throw "STOP: kind load $AdmitImage failed" }
}

function Ensure-CriKindLoad {
    Write-Host "CRI plugin kind load $CriTag (pin $CriImage)"
    $prev = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    $null = docker image inspect $CriTag 2>$null
    $inspectCode = $LASTEXITCODE
    $ErrorActionPreference = $prev
    if ($inspectCode -ne 0) {
        Write-Host "rebuild $CriTag via Dockerfile.gold-cri (enroll HTTP)"
        & docker build --platform linux/amd64 --provenance=false --sbom=false -t $CriTag -f apparatus-reference-kv/Dockerfile.gold-cri .
        if ($LASTEXITCODE -ne 0) { throw "STOP: docker build $CriTag failed" }
    }
    $id = (docker image inspect $CriTag --format '{{.Id}}' | Out-String).Trim()
    if ($id -and $CriImage -notmatch [regex]::Escape($id.Replace('sha256:', ''))) {
        Write-Host "WARN: refreshing CriImage pin to $id"
        $script:CriImage = ("apparatus-reference-kv@{0}" -f $id)
    }
    & kind load docker-image $CriTag --name aiforall-local
    if ($LASTEXITCODE -ne 0) { throw "STOP: kind load $CriTag failed" }
    # ctr tag @sha256 so kubelet IfNotPresent resolves local (sinon ImagePullBackOff registry).
    $node = 'aiforall-local-control-plane'
    $prev = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    $ls = & docker exec $node ctr -n k8s.io images ls 2>&1 | Out-String
    $source = $null
    foreach ($line in ($ls -split "`n")) {
        if ($line -match [regex]::Escape($CriTag) -and $line -notmatch '@sha256:') {
            $source = ($line -split '\s+')[0]
            break
        }
    }
    if ($source) {
        foreach ($pin in @($script:CriImage, "docker.io/library/$($script:CriImage)")) {
            & docker exec $node ctr -n k8s.io images tag $source $pin 2>$null | Out-Null
            if ($LASTEXITCODE -eq 0) { Write-Host "  ctr pin: $pin" }
        }
    }
    else {
        Write-Host 'WARN: ctr source tag introuvable apres kind load'
    }
    $ErrorActionPreference = $prev
}

Write-Host 'ensure curl pod gold-curl'
& kubectl --context $KindContext -n aiforall-plugins delete pod gold-curl --ignore-not-found --wait=true --timeout=90s 2>$null | Out-Null
Invoke-Kubectl -KubectlArgs @(
    'run', 'gold-curl', '-n', 'aiforall-plugins',
    '--restart=Never', "--image=$CurlImage", '--image-pull-policy=Never',
    '--command', '--', 'sleep', '600'
)
Invoke-Kubectl -KubectlArgs @('wait', '-n', 'aiforall-plugins', 'pod/gold-curl', '--for=condition=Ready', '--timeout=90s')

$health = Invoke-InClusterCurl -Method GET -Url "$MonolithHttp/health"
if ($health.Code -ne 200) {
    Write-Observed
    throw "STOP: nest in-cluster /health = $($health.Code) (gold path = Kind, pas curl hote :8080)"
}

$signup = Invoke-InClusterCurl -Method POST -Url "$MonolithHttp/iam/api/auth/signup" -Body (@{ email = $Email; password = $Password } | ConvertTo-Json -Compress)
if (@(200, 202) -notcontains $signup.Code) { Write-Observed; throw "STOP: signup HTTP $($signup.Code)" }
$reg = ($signup.Body | ConvertFrom-Json).registration_token
$complete = Invoke-InClusterCurl -Method POST -Url "$MonolithHttp/iam/api/auth/complete-registration" -Body (@{ registration_token = $reg; username = $Username } | ConvertTo-Json -Compress)
$login = Invoke-InClusterCurl -Method POST -Url "$MonolithHttp/iam/api/auth/login" -Body (@{ email = $Email; password = $Password } | ConvertTo-Json -Compress)
$token = $null
try { $token = ($login.Body | ConvertFrom-Json).access_token } catch { $token = $null }
if (-not $token -or $login.Code -eq 401 -or $login.Code -eq 423) {
    $verifyToken = Get-DbVerificationToken -TargetEmail $Email
    if (-not $verifyToken) {
        Write-Observed
        throw 'STOP: pas de verification_token iam_dev (Telegraph mail absent sur J3)'
    }
    $encEmail = [uri]::EscapeDataString($Email)
    $encToken = [uri]::EscapeDataString($verifyToken)
    $null = Invoke-InClusterCurl -Method GET -Url "$MonolithHttp/iam/api/auth/verify?email=$encEmail&token=$encToken"
    if ($complete.Code -ne 200 -and $reg) {
        $complete = Invoke-InClusterCurl -Method POST -Url "$MonolithHttp/iam/api/auth/complete-registration" -Body (@{ registration_token = $reg; username = $Username } | ConvertTo-Json -Compress)
    }
    $login = Invoke-InClusterCurl -Method POST -Url "$MonolithHttp/iam/api/auth/login" -Body (@{ email = $Email; password = $Password } | ConvertTo-Json -Compress)
    try { $token = ($login.Body | ConvertFrom-Json).access_token } catch { $token = $null }
}
if (-not $token) {
    try { $token = ($complete.Body | ConvertFrom-Json).access_token } catch { $token = $null }
}
if (-not $token) { Write-Observed; throw 'STOP: pas de access_token IAM' }
$auth = @("Authorization: Bearer $token")

$project = Invoke-InClusterCurl -Method POST -Url "$MonolithHttp/manifesto/api/projects" -Body (@{ name = $ProjectName; owner_type = 'personal'; visibility = 'private' } | ConvertTo-Json -Compress) -Headers $auth
if ($project.Code -ne 201) { Write-Observed; throw "STOP: POST projects HTTP $($project.Code)" }
$projectId = ($project.Body | ConvertFrom-Json).id

$comp = $null
for ($i = 0; $i -lt 60; $i++) {
    $comp = Invoke-InClusterCurl -Method POST -Url "$MonolithHttp/manifesto/api/projects/$projectId/components" -Body '{"component_type":"io.aiforall.reference-kv"}' -Headers $auth
    if ($comp.Code -eq 201) { break }
    if ($comp.Code -ne 403 -and $comp.Code -ne 500) { Write-Observed; throw "STOP: POST components HTTP $($comp.Code)" }
    Start-Sleep -Seconds 2
}
if ($comp.Code -ne 201) { Write-Observed; throw 'STOP: POST components encore 403 (sentinel-sync)' }
$componentId = ($comp.Body | ConvertFrom-Json).id

$lastConsent = $null
foreach ($cap in @('project.read', 'storage.kv.read', 'storage.kv.write')) {
    $put = Invoke-InClusterCurl -Method PUT -Url "$MonolithHttp/manifesto/api/projects/$projectId/bindings/$componentId/consents" -Body (@{ capability = $cap; status = 'consented' } | ConvertTo-Json -Compress) -Headers $auth
    if ($put.Code -ne 200) { Write-Observed; throw "STOP: T5 PUT $cap HTTP $($put.Code)" }
    $lastConsent = $put
}
if (-not $lastConsent.Body -or $lastConsent.Body -notmatch '98ba747f') {
    Write-Observed
    throw 'STOP: digest catalogue absent du snapshot consent (image oodhive-monolith:j3 stale vs attach_from_catalog)'
}

# pending → configured → active (sinon invoke 403 component_inactive)
foreach ($st in @('configured', 'active')) {
    $act = Invoke-InClusterCurl -Method PATCH -Url "$MonolithHttp/manifesto/api/projects/$projectId/components/$componentId" -Body (@{ status = $st } | ConvertTo-Json -Compress) -Headers $auth
    if ($act.Code -ne 200) { Write-Observed; throw "STOP: PATCH component status=$st HTTP $($act.Code)" }
}

Write-Host "kubectl get pod/svc $PluginHost (diagnostic, leftover schedule-reference-kv n est pas Adm-A)"
$prevErr = $ErrorActionPreference
$ErrorActionPreference = 'Continue'
& kubectl --context $KindContext -n apparatus-plugins get pod $PluginHost 2>&1 | Write-Host
& kubectl --context $KindContext -n apparatus-plugins get svc $PluginHost 2>&1 | Write-Host
& kubectl --context $KindContext -n apparatus-system get admissionrecord 2>&1 | Write-Host
$ErrorActionPreference = $prevErr

Write-Host 'VALID Adm-A : Job in-cluster SA admit-sign (pas cargo run apparatus-admit)'
# Purger leftover schedule-reference-kv / ancien Pod (pas Adm-A) avant Job.
& kubectl --context $KindContext -n apparatus-plugins delete pod $PluginHost --ignore-not-found --wait=true --timeout=60s 2>$null | Out-Null
& kubectl --context $KindContext -n apparatus-plugins delete svc $PluginHost --ignore-not-found 2>$null | Out-Null
& kubectl --context $KindContext -n apparatus-plugins delete svc apparatus-reference-kv --ignore-not-found 2>$null | Out-Null
& kubectl --context $KindContext -n apparatus-system delete admissionrecord --all --ignore-not-found 2>$null | Out-Null
Ensure-TransitCosign
$hostGwForAdmit = $gwIp
if (-not $hostGwForAdmit) { $hostGwForAdmit = '192.168.65.254' }
$null = Ensure-ZotInCluster -HostGw $hostGwForAdmit
Ensure-AdmitImage
Ensure-ControllerImage
Ensure-CriKindLoad

# Shared store volume + enroll URL on controller (hostPath Kind)
& kubectl --context $KindContext apply -k deploy/p4 | Out-Null
Patch-OpenBaoEgressCidr -Ip $hostGwForAdmit
Ensure-OpenBaoKubernetes
# imagePullPolicy Never : kind load du tag j2 ne recycle pas le Pod tout seul.
& kubectl --context $KindContext -n aiforall-apparatus rollout restart deploy/apparatus-operator 2>$null | Out-Null
& kubectl --context $KindContext set env deploy/apparatus-operator -n aiforall-apparatus "APPARATUS_PLUGIN_ENROLL_URL=$EnrollUrl" 2>$null | Out-Null
& kubectl --context $KindContext -n aiforall-apparatus rollout status deploy/apparatus-operator --timeout=180s | Out-Null
& kubectl --context $KindContext -n aiforall-platform rollout status deploy/zot --timeout=120s | Out-Null
Assert-ZotAuthNegatives

$admitJobFile = Join-Path $env:TEMP "apparatus-admit-gold-$Stamp.yaml"
@"
apiVersion: batch/v1
kind: Job
metadata:
  name: apparatus-admit-gold-$Stamp
  namespace: aiforall-apparatus
  labels:
    app.kubernetes.io/name: apparatus-admit
    app.kubernetes.io/part-of: aiforall
    aiforall.io/gold: "true"
spec:
  backoffLimit: 1
  ttlSecondsAfterFinished: 600
  template:
    metadata:
      labels:
        app.kubernetes.io/name: apparatus-admit
        app.kubernetes.io/part-of: aiforall
        aiforall.io/gold: "true"
    spec:
      restartPolicy: Never
      serviceAccountName: admit-sign
      automountServiceAccountToken: true
      securityContext:
        fsGroup: 65534
        runAsNonRoot: true
        runAsUser: 65534
        runAsGroup: 65534
      hostAliases:
        - ip: "$hostGwForAdmit"
          hostnames: ["host.docker.internal"]
      initContainers:
        - name: fix-store-perms
          image: aiforall-apparatus-admit:gold
          imagePullPolicy: Never
          command: ["/bin/sh", "-c", "mkdir -p /var/lib/apparatus && chown -R 65534:65534 /var/lib/apparatus"]
          securityContext:
            runAsUser: 0
            runAsNonRoot: false
          volumeMounts:
            - name: admission-store
              mountPath: /var/lib/apparatus
      containers:
        - name: admit
          image: aiforall-apparatus-admit:gold
          imagePullPolicy: Never
          env:
            - name: APPARATUS_ADMISSION_STORE_PATH
              value: /var/lib/apparatus/admission.json
            - name: APPARATUS_COSIGN_BIN
              value: /usr/local/bin/cosign
            - name: APPARATUS_ORAS_BIN
              value: /usr/local/bin/oras
            - name: APPARATUS_REGISTRY_HOST
              value: zot.aiforall-platform.svc.cluster.local:5000
            - name: APPARATUS_REGISTRY_NETWORK
              value: zot.aiforall-platform.svc.cluster.local:5000
            - name: APPARATUS_DOCKER_NETWORK
              value: unused-host-bins
            - name: APPARATUS_VAULT_ADDR
              value: http://host.docker.internal:8200
            - name: APPARATUS_VAULT_ADDR_NETWORK
              value: http://host.docker.internal:8200
            - name: APPARATUS_VAULT_AUTH
              value: kubernetes
            - name: APPARATUS_VAULT_K8S_ROLE
              value: admit-sign
            - name: APPARATUS_TRANSIT_KEY
              value: apparatus-p4-cosign
            - name: APPARATUS_REGISTRY_USER
              valueFrom:
                secretKeyRef:
                  name: zot-registry-accounts
                  key: signer-user
            - name: APPARATUS_REGISTRY_PASSWORD
              valueFrom:
                secretKeyRef:
                  name: zot-registry-accounts
                  key: signer-password
            - name: APPARATUS_REPOSITORY
              value: apparatus/envelope
            - name: APPARATUS_ENVELOPE_TAG
              value: gold
            - name: APPARATUS_CRI_IMAGE
              value: "$CriImage"
            - name: APPARATUS_DESCRIPTOR_DIGEST
              value: "$Digest"
            - name: APPARATUS_REPORT_DIGEST
              value: "$ReportDigest"
            - name: APPARATUS_PROJECT
              value: "$projectId"
            - name: APPARATUS_BINDING
              value: "$componentId"
          volumeMounts:
            - name: admission-store
              mountPath: /var/lib/apparatus
      volumes:
        - name: admission-store
          hostPath:
            path: /var/lib/apparatus-admission
            type: DirectoryOrCreate
"@ | Set-Content -LiteralPath $admitJobFile -Encoding utf8

& kubectl --context $KindContext -n aiforall-apparatus delete job "apparatus-admit-gold-$Stamp" --ignore-not-found 2>$null | Out-Null
Invoke-Kubectl -KubectlArgs @('apply', '-f', $admitJobFile)

Write-Host "attendre Job apparatus-admit-gold-$Stamp Complete"
$deadlineJob = (Get-Date).AddMinutes(8)
$jobDone = $false
while ((Get-Date) -lt $deadlineJob) {
    $js = & kubectl --context $KindContext -n aiforall-apparatus get job "apparatus-admit-gold-$Stamp" -o json 2>$null
    if ($LASTEXITCODE -eq 0 -and $js) {
        try {
            $st = ($js | ConvertFrom-Json).status
            if ($st.succeeded -ge 1) { $jobDone = $true; break }
            if ($st.failed -ge 1) {
                & kubectl --context $KindContext -n aiforall-apparatus logs -l "job-name=apparatus-admit-gold-$Stamp" --tail=80
                throw "STOP: Job apparatus-admit-gold-$Stamp failed"
            }
        } catch {
            if ("$_" -match '^STOP:') { throw }
        }
    }
    Start-Sleep -Seconds 5
}
if (-not $jobDone) {
    & kubectl --context $KindContext -n aiforall-apparatus logs -l "job-name=apparatus-admit-gold-$Stamp" --tail=80
    Write-Observed
    throw "STOP: Job apparatus-admit-gold-$Stamp timeout"
}
Invoke-Kubectl -KubectlArgs @('logs', '-n', 'aiforall-apparatus', '-l', "job-name=apparatus-admit-gold-$Stamp", '--tail=40')

$deadline = (Get-Date).AddMinutes(3)
$ready = $false
while ((Get-Date) -lt $deadline) {
    $podJson = & kubectl --context $KindContext -n apparatus-plugins get pod $PluginHost -o json 2>$null
    if ($LASTEXITCODE -eq 0 -and $podJson) {
        try {
            $phase = ($podJson | ConvertFrom-Json).status.phase
            if ($phase -eq 'Running') { $ready = $true; break }
        } catch { }
    }
    Start-Sleep -Seconds 3
}
if (-not $ready) {
    Write-Observed
    $prevD = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    & kubectl --context $KindContext -n apparatus-plugins describe pod $PluginHost 2>&1 | Write-Host
    & kubectl --context $KindContext -n aiforall-apparatus logs -l app.kubernetes.io/name=apparatus-operator -c operator --tail=40 2>&1 | Write-Host
    $ErrorActionPreference = $prevD
    throw "STOP: pod $PluginHost pas Running apres admit Job"
}
$prevSvc = $ErrorActionPreference
$ErrorActionPreference = 'Continue'
& kubectl --context $KindContext -n apparatus-plugins get svc $PluginHost 2>&1 | Write-Host
$ErrorActionPreference = $prevSvc
$svcOk = $false
for ($i = 0; $i -lt 30; $i++) {
    $svcJson = & kubectl --context $KindContext -n apparatus-plugins get svc $PluginHost -o name 2>$null
    if ($LASTEXITCODE -eq 0 -and $svcJson) { $svcOk = $true; break }
    Start-Sleep -Seconds 2
}
if (-not $svcOk) {
    Write-Observed
    throw "STOP: Service $PluginHost absent apres admit (controller J2 stale? rebuild deploy-j2)"
}

Write-Host 'attendre enroll workload (logs plugin)'
$enrolled = $false
for ($i = 0; $i -lt 20; $i++) {
    $logs = & kubectl --context $KindContext logs -n apparatus-plugins $PluginHost --tail=40 2>$null
    if ("$logs" -match 'enroll HTTP' -or "$logs" -match 'already enrolled') { $enrolled = $true; break }
    Start-Sleep -Seconds 2
}
Invoke-Kubectl -KubectlArgs @('logs', '-n', 'apparatus-plugins', $PluginHost, '--tail=40')
if (-not $enrolled) {
    Write-Observed
    Write-Host "PREUVE 200 ECHOUEE : enroll workload absent des logs $PluginHost"
    exit 3
}

$certPem = (& kubectl --context $KindContext exec -n apparatus-plugins $PluginHost -- cat /tmp/lazaret-workload-cert.pem | Out-String)
$keyPem = (& kubectl --context $KindContext exec -n apparatus-plugins $PluginHost -- cat /tmp/lazaret-workload-key.pem | Out-String)
if ($certPem -notmatch 'BEGIN CERTIFICATE' -or $keyPem -notmatch 'BEGIN') {
    Write-Observed
    Write-Host 'PREUVE 200 ECHOUEE : PEM workload introuvable dans le Pod plugin'
    exit 3
}
$certPem | & kubectl --context $KindContext exec -i -n aiforall-plugins gold-curl -- sh -c 'cat > /tmp/lazaret-workload-cert.pem'
$keyPem | & kubectl --context $KindContext exec -i -n aiforall-plugins gold-curl -- sh -c 'cat > /tmp/lazaret-workload-key.pem'

$session = Invoke-InClusterCurl -Method POST -Url "$MonolithHttps/lazaret/session" -Extra @(
    '--cert', '/tmp/lazaret-workload-cert.pem', '--key', '/tmp/lazaret-workload-key.pem'
)
if ($session.Code -ne 200) {
    Write-Observed
    Write-Host "PREUVE 200 ECHOUEE : session mTLS HTTP $($session.Code) body=$($session.Body)"
    exit 3
}
$sessionToken = $null
try { $sessionToken = ($session.Body | ConvertFrom-Json).session_token } catch { $sessionToken = $null }
if (-not $sessionToken) { Write-Observed; throw 'STOP: session_token absent' }

$opId = [guid]::NewGuid().ToString()
$invokeBody = (@{
        binding_id   = $componentId
        operation_id = $opId
        operation    = 'kv.get'
        params       = @{ key = 'gold' }
    } | ConvertTo-Json -Compress -Depth 5)
$invoke = Invoke-InClusterCurl -Method POST -Url "$MonolithHttps/lazaret/invoke?project_id=$projectId" -Body $invokeBody -Headers @(
    "Authorization: Bearer $sessionToken"
) -Extra @(
    '--cert', '/tmp/lazaret-workload-cert.pem', '--key', '/tmp/lazaret-workload-key.pem'
)
Write-Observed
if ($invoke.Code -eq 200 -and $invoke.Body -match $PluginHost) {
    Assert-L2Negatives -HostGw $hostGwForAdmit
    Write-Host "GOLD PATH OK HTTP 200 hostname=$PluginHost digest=$Digest component_id=$componentId"
    exit 0
}

Write-Host "PREUVE 200 ECHOUEE : invoke HTTP $($invoke.Code) body=$($invoke.Body)"
if ($invoke.Code -ne 200) {
    Write-Host "Cause exacte : invoke in-cluster HTTPS $($invoke.Code) (attendu 200 + hostname=$PluginHost)."
} else {
    Write-Host "Cause exacte : HTTP 200 mais hostname absent ou different de $PluginHost."
}
exit 3
