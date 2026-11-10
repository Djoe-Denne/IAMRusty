# The native local-lab entry. No Python, deployer state, backup or recovery engine.
param([ValidateSet('keep-data', 'reset-data')][string]$Mode = 'keep-data')
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$OutputEncoding = [Text.UTF8Encoding]::new($false)
$root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$context = 'kind-aiforall-local-full'
$namespace = 'aiforall-local-full'
$cluster = 'aiforall-local-full'
$prefix = @('--context', $context, '--namespace', $namespace)
$helm = if ($env:AIFORALL_HELM) { $env:AIFORALL_HELM } else { 'helm.exe' }
$chart = Join-Path $PSScriptRoot 'charts/kind-minimal'
$inventory = Join-Path $PSScriptRoot 'build/targets.json'
$private = $env:KIND_LOCAL_PRIVATE_DIR
if (-not $private -or -not [IO.Path]::IsPathRooted($private)) { throw 'KIND_LOCAL_PRIVATE_DIR must designate your absolute private input directory.' }
$private = [IO.Path]::GetFullPath($private)
if ($private.StartsWith($root + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase) -or $private -eq $root) {
    throw 'Keep complete private inputs outside the Git checkout and Docker build context.'
}
$certs = if ($env:KIND_LOCAL_CERT_DIR) { [IO.Path]::GetFullPath($env:KIND_LOCAL_CERT_DIR) } else { Join-Path $private 'pki' }
if ($certs.StartsWith($root + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase) -or $certs -eq $root) {
    throw 'Keep the PKI outside the Git checkout and Docker build context.'
}
$tag = if ($env:KIND_LOCAL_BUILD_TAG) { $env:KIND_LOCAL_BUILD_TAG } else { 'local-' + [DateTime]::UtcNow.ToString('yyyyMMdd-HHmmss') }
$p4Registry = $env:KIND_LOCAL_ENABLE_P4_REGISTRY -eq '1'
if ($tag -cnotmatch '^[A-Za-z0-9_][A-Za-z0-9_.-]{0,127}$') { throw 'Invalid public build tag.' }

function K {
    $input | & kubectl.exe @prefix @args
    if ($LASTEXITCODE -ne 0) { throw 'kubectl failed; no automatic rollback, data replacement or retry is performed.' }
}
function Need([string]$Path) {
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { throw "Required complete input is missing: $Path" }
}
function Render([string]$Template, [string[]]$Settings) {
    $arguments = @('template', 'kind-local', $chart, '--namespace', $namespace, '-f', $inventory, '--show-only', "templates/$Template")
    if ($env:KIND_LOCAL_PUBLIC_VALUES) { $arguments += @('-f', $env:KIND_LOCAL_PUBLIC_VALUES) }
    foreach ($setting in $Settings) { $arguments += @('--set', $setting) }
    $result = & $helm @arguments
    if ($LASTEXITCODE -ne 0) { throw 'Helm rendering failed.' }
    return $result
}
function Secret([string]$Name, [string[]]$Files, [switch]$OnlyMissing, [string]$EnvFile) {
    if ($OnlyMissing -and (K get secret $Name --ignore-not-found=true -o name)) { return }
    $arguments = @('create', 'secret', 'generic', $Name, '--dry-run=client', '-o', 'json')
    foreach ($file in $Files) { $arguments += "--from-file=$file" }
    if ($EnvFile) { $arguments += "--from-env-file=$EnvFile" }
    # Native file/stdin transfer only. Never echo private JSON or --from-literal secrets.
    $manifest = & kubectl.exe @prefix @arguments
    if ($LASTEXITCODE -ne 0) { throw "Unable to read the complete private inputs for $Name." }
    $null = $manifest | K apply -f -
}
function LeafSecret([string]$Name, [string]$Leaf) {
    Secret $Name @("tls.crt=$(Join-Path $certs "$Leaf.crt")", "tls.key=$(Join-Path $certs "$Leaf.key")") -OnlyMissing
}
function CreateData([string]$Name, [string]$Template, [string]$Setting) {
    if (-not (K get deployment $Name --ignore-not-found=true -o name)) {
        $manifest = Render $Template @($Setting)
        # kubectl create prints separate JSON objects, not one List, for a
        # multi-document input. Let the native decoder consume one Helm document
        # at a time; the declarations' document separators are at column zero.
        foreach ($document in (($manifest -join "`n") -split '(?m)^---[ \t]*\r?$')) {
            if ([string]::IsNullOrWhiteSpace($document)) { continue }
            $object = ConvertFrom-Json -InputObject (($document | K create --dry-run=client --validate=false -f - -o json) -join "`n")
            # A surviving PVC/Service/ConfigMap is not an invitation to resize,
            # replace or reconfigure it just because its Deployment is absent.
            $kind = $object.kind.ToLowerInvariant()
            if (-not (K get $kind $object.metadata.name --ignore-not-found=true -o name)) {
                $null = (ConvertTo-Json -InputObject $object -Depth 100 -Compress) | K create -f -
            }
        }
    }
}
function CompleteJob([string]$Name) {
    $null = K wait --for=condition=complete --timeout=240s "job/$Name"
}
function PauseCompute($Deployments) {
    # Quiesce only declared compute, never data Deployments or PVCs.
    foreach ($deployment in $Deployments) {
        $old = K get deployment $deployment.metadata.name --ignore-not-found=true -o json
        if ($old) {
            $current = ConvertFrom-Json -InputObject ($old -join "`n")
            $selector = (@($current.spec.selector.matchLabels.PSObject.Properties | ForEach-Object { "$($_.Name)=$($_.Value)" }) -join ',')
            $null = K scale deployment $deployment.metadata.name --replicas=0
            $pods = ConvertFrom-Json -InputObject ((K get pods --selector $selector -o json) -join "`n")
            if (@($pods.items).Count) { $null = K wait --for=delete pods --selector $selector --timeout=180s }
        }
    }
}

Push-Location $root
try {
    $clusters = & kind.exe get clusters
    if ($LASTEXITCODE -ne 0) { throw 'kind get clusters failed; this is not an absent lab.' }
    $exists = @($clusters) -contains $cluster
    $fresh = -not $exists -or $Mode -eq 'reset-data'
    Need (Join-Path $private 'postgres.password')
    Need (Join-Path $private 'openfga.env')
    if ($p4Registry) { Need (Join-Path $private 'zot/config.json'); Need (Join-Path $private 'zot/htpasswd') }
    if (-not $env:KIND_LOCAL_OPENBAO_CA_FILE) { throw 'KIND_LOCAL_OPENBAO_CA_FILE must explicitly designate the coffer CA.' }
    if ($fresh) {
        if ($env:KIND_LOCAL_INITIALIZE_EMPTY -ne '1') { throw 'A fresh installation requires KIND_LOCAL_INITIALIZE_EMPTY=1.' }
        foreach ($path in @($env:KIND_LOCAL_OPENBAO_INIT_OUT, $env:KIND_LOCAL_OPENBAO_PRODUCT_OUT_DIR)) {
            if (-not $path -or -not [IO.Path]::IsPathRooted($path) -or (Test-Path -LiteralPath $path)) {
                throw 'Before a fresh installation/reset, choose NEW absolute OpenBao init and product output paths.'
            }
            $absolute = [IO.Path]::GetFullPath($path)
            if ($absolute.StartsWith($root + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase) -or $absolute -eq $root) {
                throw 'OpenBao private outputs must remain outside the checkout and Docker build context.'
            }
        }
        if (-not $env:KIND_LOCAL_BAO) { throw 'A fresh installation requires KIND_LOCAL_BAO (official 2.6.2 bao.exe).' }
        Need $env:KIND_LOCAL_BAO
        Need (Join-Path $private 'openbao/ops-token.json')
    }
    if ($exists -and -not (Test-Path -LiteralPath (Join-Path $certs 'mesh/ca.key') -PathType Leaf)) {
        throw 'An existing lab requires its existing PKI. Do not generate a replacement authority for preserved data.'
    }
    & (Join-Path $PSScriptRoot 'kind-local-pki.ps1') -Directory $certs
    if (-not $?) { throw 'PKI preparation failed.' }
    Need $env:KIND_LOCAL_OPENBAO_CA_FILE
    $appsText = Render 'applications.yaml' @('applications.enabled=true', "buildTag=$tag")
    # Each Helm document is one compact JSON object (valid YAML), not a YAML
    # parser or private configuration composer. Ignore only Helm's framing.
    $objects = @($appsText | Where-Object { $_ -and $_ -notmatch '^(---\s*|# Source: .*)$' } |
        ForEach-Object { ConvertFrom-Json -InputObject $_ -ErrorAction Stop })
    $deployments = @($objects | Where-Object { $_.kind -eq 'Deployment' })
    if (-not $deployments.Count) { throw 'The application declarations contain no compute Deployment.' }
    $enabled = @($deployments | Where-Object { $_.spec.replicas -gt 0 })
    # For reset, validate every supplied application input before destroying nodes.
    # A fresh coffer may instead stop at credentials handback before applications.
    if (-not $fresh -or $Mode -eq 'reset-data') {
        foreach ($deployment in $enabled) {
            $folder = Join-Path $private "apps/$($deployment.metadata.name)"
            $container = $deployment.spec.template.spec.containers[0]
            if ($container.PSObject.Properties['envFrom']) { Need (Join-Path $folder 'environment.env') }
            $configFile = $deployment.metadata.annotations.'aiforall.dev/config-file'
            if ($configFile) { Need (Join-Path $folder $configFile) }
        }
    }
    & (Join-Path $PSScriptRoot 'kind-local-images.ps1') -Action build -BuildTag $tag
    if (-not $?) { throw 'Application image build failed.' }
    if ($exists) { PauseCompute $deployments }
    if ($Mode -eq 'reset-data' -and $exists) {
        Write-Host "Explicit reset: deleting only Kind lab $cluster and its node-local data. Host PKI/private files and other clusters stay intact."
        & kind.exe delete cluster --name $cluster
        if ($LASTEXITCODE -ne 0) { throw 'Kind lab deletion failed.' }
        $exists = $false
    }
    if (-not $exists) {
        & kind.exe create cluster --name $cluster --config (Join-Path $PSScriptRoot 'kind/cluster-local-full.yaml')
        if ($LASTEXITCODE -ne 0) { throw 'Kind creation failed.' }
        $null = K apply -f (Join-Path $PSScriptRoot 'third-party/calico/base/calico.yaml')
        $null = K -n kube-system rollout status daemonset/calico-node --timeout=240s
    }
    & (Join-Path $PSScriptRoot 'kind-local-images.ps1') -Action import -BuildTag $tag
    if (-not $?) { throw 'Native image import failed; no deployments applied.' }
    $ns = & kubectl.exe --context $context create namespace $namespace --dry-run=client -o json
    if ($LASTEXITCODE -ne 0) { throw 'Namespace preparation failed.' }
    $null = $ns | K apply -f -
    $null = (Render 'network.yaml' @('network.create=true')) | K apply -f -
    Secret 'transport-ca' @("ca.crt=$(Join-Path $certs 'mesh/ca.crt')") -OnlyMissing
    Secret 'postgres-credentials' @("password=$(Join-Path $private 'postgres.password')") -OnlyMissing
    LeafSecret 'postgres-server-tls' 'mesh/postgres'
    LeafSecret 'redis-server-tls' 'mesh/redis'
    CreateData 'postgres' 'postgres.yaml' 'postgres.create=true'
    CreateData 'redis' 'redis.yaml' 'redis.create=true'
    $null = K rollout status deployment/postgres --timeout=240s
    $null = K rollout status deployment/redis --timeout=240s
    $null = K delete job kind-local-postgres-initialize --ignore-not-found=true --wait=true
    $null = (Render 'postgres-initialize.yaml' @('postgres.initialize=true')) | K apply -f -
    CompleteJob 'kind-local-postgres-initialize'
    if (-not (K get deployment openfga --ignore-not-found=true -o name)) {
        Secret 'openfga-datastore' @() -EnvFile (Join-Path $private 'openfga.env') -OnlyMissing
        LeafSecret 'openfga-server-tls' 'mesh/openfga'
        CreateData 'openfga' 'openfga.yaml' 'openfga.create=true'
    }
    $null = K rollout status deployment/openfga --timeout=240s
    $null = & (Join-Path $PSScriptRoot 'kind-local-fga.ps1') -Helm $helm -InitializeEmpty:($env:KIND_LOCAL_INITIALIZE_EMPTY -eq '1')
    LeafSecret 'localstack-server-tls' 'mesh/localstack'
    CreateData 'localstack' 'localstack.yaml' 'localstack.create=true'
    $null = K rollout status deployment/localstack --timeout=240s
    $null = K delete job kind-local-sqs-initialize --ignore-not-found=true --wait=true
    $null = (Render 'localstack.yaml' @('localstack.create=false', 'localstack.initialize=true')) | K apply -f -
    CompleteJob 'kind-local-sqs-initialize'
    if ($p4Registry) {
        if (-not (K get deployment zot --ignore-not-found=true -o name)) {
            Secret 'kind-local-zot-config' @("config.json=$(Join-Path $private 'zot/config.json')", "htpasswd=$(Join-Path $private 'zot/htpasswd')") -OnlyMissing
            LeafSecret 'zot-server-tls' 'mesh/zot'
            CreateData 'zot' 'zot.yaml' 'zot.create=true'
        }
        $null = K rollout status deployment/zot --timeout=240s
    }
    # Even when P4 is not selected, any existing zot/admission-store resources
    # remain untouched. This is not the removed application-image registry.
    # An existing coffer stays completely outside apply/restart/reseed.
    $newBao = -not (K get deployment openbao --ignore-not-found=true -o name)
    if ($newBao) {
        LeafSecret 'openbao-server-tls' 'mesh/openbao'
        CreateData 'openbao' 'openbao.yaml' 'openbao.create=true'
    }
    if ($newBao -or $env:KIND_LOCAL_OPENBAO_UNSEAL_INPUT) {
        $null = K wait --for=create pods --selector app.kubernetes.io/name=openbao --timeout=180s
        $pod = ((K get pods --selector app.kubernetes.io/name=openbao -o 'jsonpath={.items[0].metadata.name}') -join '').Trim()
        $null = K wait '--for=jsonpath={.status.phase}=Running' "pod/$pod" --timeout=180s
        $forward = [Diagnostics.Process]::new()
        $forward.StartInfo = [Diagnostics.ProcessStartInfo]::new('kubectl.exe', "--context $context --namespace $namespace port-forward --address 127.0.0.1 pod/$pod 18444:8443")
        $forward.StartInfo.UseShellExecute = $false
        $forward.StartInfo.RedirectStandardOutput = $true
        $forward.StartInfo.RedirectStandardError = $true
        $forwardStarted = $false
        try {
            $forwardStarted = $forward.Start()
            $errors = $forward.StandardError.ReadToEndAsync()
            $line = $forward.StandardOutput.ReadLineAsync()
            if (-not $line.Wait(30000) -or $line.Result -notlike 'Forwarding from 127.0.0.1:*') { throw 'Native OpenBao port-forward did not become available.' }
            Import-Module (Join-Path $PSScriptRoot 'openbao-cli.psm1') -Force
            if (-not $env:KIND_LOCAL_BAO) { throw 'KIND_LOCAL_BAO must explicitly select the official 2.6.2 bao executable.' }
            $connection = @{ Executable = $env:KIND_LOCAL_BAO; Address = 'https://127.0.0.1:18444'; CaFile = $env:KIND_LOCAL_OPENBAO_CA_FILE; ServerName = 'openbao.aiforall-local-full.svc.cluster.local' }
            $status = Get-KindOpenBaoStatus -Connection $connection
            if (-not $status.initialized) {
                if ($env:KIND_LOCAL_INITIALIZE_EMPTY -ne '1') { throw 'Uninitialized OpenBao requires explicit KIND_LOCAL_INITIALIZE_EMPTY=1 and new private outputs.' }
                Initialize-KindOpenBao -Connection $connection -OutputFile $env:KIND_LOCAL_OPENBAO_INIT_OUT
                $null = Unlock-KindOpenBao -Connection $connection -InputFile $env:KIND_LOCAL_OPENBAO_INIT_OUT
                Invoke-KindOpenBaoBootstrap -Connection $connection -InitFile $env:KIND_LOCAL_OPENBAO_INIT_OUT -TokenConsumers lazaret `
                    -ProductOutputDirectory $env:KIND_LOCAL_OPENBAO_PRODUCT_OUT_DIR -SeedFiles @{'ops/token' = (Join-Path $private 'openbao/ops-token.json')}
                Write-Host 'Coffer initialized; root revoked and private product credentials issued. Complete the application inputs with these credentials, then rerun. Applications have NOT been started.'
                return
            }
            if ($status.sealed) { $null = Unlock-KindOpenBao -Connection $connection -InputFile $env:KIND_LOCAL_OPENBAO_UNSEAL_INPUT }
        } finally {
            if ($forwardStarted -and -not $forward.HasExited) { $forward.Kill(); $forward.WaitForExit() }
            if ($forwardStarted) { $null = $errors.GetAwaiter().GetResult() }
            $forward.Dispose()
        }
    }
    $null = K rollout status deployment/openbao --timeout=180s
    Secret 'kind-local-openbao-client-ca' @("ca.crt=$env:KIND_LOCAL_OPENBAO_CA_FILE")
    Secret 'lazaret-identity-ca' @("ca.crt=$(Join-Path $certs 'workload/ca.crt')", "ca.key=$(Join-Path $certs 'workload/ca.key')", "server.crt=$(Join-Path $certs 'workload/server.crt')", "server.key=$(Join-Path $certs 'workload/server.key')")
    foreach ($deployment in $enabled) {
        $name = $deployment.metadata.name
        $folder = Join-Path $private "apps/$name"
        $container = $deployment.spec.template.spec.containers[0]
        if ($container.PSObject.Properties['envFrom']) { Secret "kind-local-$name-env" @() -EnvFile (Join-Path $folder 'environment.env') }
        if ($deployment.metadata.annotations.'aiforall.dev/config-file') { Secret "kind-local-$name-config" @($folder) }
        $files = @("ca.crt=$(Join-Path $certs 'mesh/ca.crt')", "ca-bundle.crt=$(Join-Path $certs 'ca-bundle.crt')")
        $leaf = $deployment.metadata.annotations.'aiforall.dev/tls-leaf'
        if ($leaf) {
            $base = Split-Path $leaf -Leaf
            $files += @("$base.crt=$(Join-Path $certs "$leaf.crt")", "$base.key=$(Join-Path $certs "$leaf.key")", "tls.crt=$(Join-Path $certs "$leaf.crt")", "tls.key=$(Join-Path $certs "$leaf.key")")
        }
        if ($container.PSObject.Properties['readinessProbe'] -and $container.readinessProbe.PSObject.Properties['exec'] -and $container.readinessProbe.exec.command -contains '--cert') {
            $files += @("mesh-client.crt=$(Join-Path $certs 'mesh/mesh-client.crt')", "mesh-client.key=$(Join-Path $certs 'mesh/mesh-client.key')")
        }
        Secret "kind-local-$name-tls" $files
    }
    $null = $appsText | K apply -f -
    foreach ($deployment in $enabled) { $null = K rollout status "deployment/$($deployment.metadata.name)" --timeout=300s }
    if ($Mode -eq 'reset-data') { Write-Host 'Dedicated lab reset completed; host private inputs and PKI were retained.' }
    else { Write-Host 'Native lab workloads are available. Existing data Deployments and volumes were retained.' }
    Write-Host "HTTPS access (foreground): kubectl --context $context -n $namespace port-forward --address 127.0.0.1 service/envoy-mesh 18443:10000"
} finally { Pop-Location }
