# Native first-install adapter. Existing references are read-only; no HTTP client
# or deployer state. Requires an already reachable HTTPS OpenFGA server and CA.
param(
    [switch]$InitializeEmpty,
    [string]$Kubectl = 'kubectl.exe',
    [string]$Helm = $env:AIFORALL_HELM,
    [ValidatePattern('^[a-z0-9][a-z0-9.-]{0,252}$')]
    [string]$CaSecret = 'transport-ca',
    [ValidatePattern('^[a-z0-9][a-z0-9.-]{0,252}$')]
    [string]$ModelConfigMap = 'kind-local-fga-model'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$prefix = @('--context', 'kind-aiforall-local-full', '--namespace', 'aiforall-local-full')
if (-not $Helm) { $Helm = 'helm.exe' }

function Invoke-FgaTool([string]$Executable, [string[]]$Arguments, [string]$InputText = '') {
    $info = [Diagnostics.ProcessStartInfo]::new()
    $info.FileName = $Executable
    # These public internal arguments never contain quotes, control characters
    # or trailing backslashes; JSON/YAML bodies travel on stdin, not argv.
    $info.Arguments = (@(foreach ($argument in $Arguments) {
        if ($argument -match '["\r\n]' -or $argument.EndsWith('\')) { throw 'Unsupported native argument.' }
        '"' + $argument + '"'
    }) -join ' ')
    $info.WorkingDirectory = $root
    $info.UseShellExecute = $false
    $info.CreateNoWindow = $true
    $info.RedirectStandardInput = $true
    $info.RedirectStandardOutput = $true
    $info.RedirectStandardError = $true
    $process = [Diagnostics.Process]::new()
    $process.StartInfo = $info
    $started = $false
    try {
        $started = $process.Start()
        $output = $process.StandardOutput.ReadToEndAsync()
        $errors = $process.StandardError.ReadToEndAsync()
        if ($InputText) { $process.StandardInput.Write($InputText) }
        $process.StandardInput.Close()
        if (-not $process.WaitForExit(240000)) { throw 'Native command timed out; remote state is not rolled back.' }
        return @{ ExitCode = $process.ExitCode; Output = $output.GetAwaiter().GetResult(); Error = $errors.GetAwaiter().GetResult() }
    } finally {
        if ($started -and -not $process.HasExited) { $process.Kill(); $process.WaitForExit() }
        $process.Dispose()
    }
}

function Invoke-FgaKubectl([string[]]$Arguments, [string]$InputText = '') {
    $result = Invoke-FgaTool $Kubectl ($prefix + $Arguments) $InputText
    if ($result.ExitCode -ne 0) { throw 'Native kubectl failed; an API error is not absence or an empty store.' }
    return $result.Output
}

function Read-FgaJsonObject([string]$Text) {
    # Parse the entire stream as ONE JSON object. No prefix/suffix extraction,
    # diagnostic-line removal or salvage of a partially written CLI result.
    try { $value = ConvertFrom-Json -InputObject $Text -ErrorAction Stop }
    catch { throw 'Native FGA result is not one complete JSON object.' }
    if ($value -isnot [pscustomobject]) { throw 'Native FGA result is not an object.' }
    return $value
}

function Read-FgaReferences($Value, [switch]$NativeResult) {
    if ($NativeResult) {
        $store = $Value.store.id
        $model = $Value.model.authorization_model_id
    } else {
        $store = $Value.data.'store-id'
        $model = $Value.data.'model-id'
    }
    if ($store -isnot [string] -or [string]::IsNullOrWhiteSpace($store) -or
        $model -isnot [string] -or [string]::IsNullOrWhiteSpace($model)) { throw 'Both native OpenFGA references must be nonempty strings.' }
    if ($NativeResult -and ($store -cnotmatch '^[0-9A-HJKMNP-TV-Z]{26}$' -or $model -cnotmatch '^[0-9A-HJKMNP-TV-Z]{26}$')) {
        throw 'Native OpenFGA identifiers are invalid; no partial references are published.'
    }
    return [pscustomobject]@{ 'store-id' = $store; 'model-id' = $model }
}

function New-FgaJob([string]$Kind) {
    $chart = Join-Path $PSScriptRoot 'charts/kind-minimal'
    $arguments = @('template', 'kind-local', $chart, '--namespace', 'aiforall-local-full',
        '--show-only', 'templates/openfga-jobs.yaml', '--set', "openfga.$Kind=true",
        '--set-string', "openfga.caSecret=$CaSecret", '--set-string', "openfga.modelConfigMap=$ModelConfigMap")
    $rendered = Invoke-FgaTool $Helm $arguments
    if ($rendered.ExitCode -ne 0 -or [string]::IsNullOrWhiteSpace($rendered.Output)) { throw 'Native Helm Job rendering failed.' }
    $null = Invoke-FgaKubectl @('create', '-f', '-') $rendered.Output
}

function Read-FgaJobResult([string]$Name) {
    $null = Invoke-FgaKubectl @('wait', '--for=condition=complete', '--timeout=180s', "job/$Name")
    # K8s 1.32.2 standard logs interleave container stdout/stderr. Accept only a
    # whole valid native JSON object after Complete, not a separated-stream claim.
    # Host kubectl stderr is captured independently by Invoke-FgaTool.
    $text = Invoke-FgaKubectl @('logs', "job/$Name", '--container', 'cli', '--timestamps=false', '--tail=-1')
    return Read-FgaJsonObject $text
}

$existing = Invoke-FgaKubectl @('get', 'configmap', 'local-full-fga', '--ignore-not-found=true', '-o', 'json')
if (-not [string]::IsNullOrWhiteSpace($existing)) {
    # Do not rewrite the object or discard its extra data. A partial object is
    # an operator error, never permission to create a store or replay a model.
    return Read-FgaReferences (Read-FgaJsonObject $existing)
}
if (-not $InitializeEmpty) { throw 'Missing OpenFGA references require an explicit first installation (-InitializeEmpty).' }

try {
    $previousJob = Invoke-FgaKubectl @('get', 'job', 'kind-local-fga-initialize', '--ignore-not-found=true', '-o', 'json')
    if (-not [string]::IsNullOrWhiteSpace($previousJob)) { throw 'An existing mutator Job must be preserved and resolved explicitly.' }
    $modelFile = Join-Path $root 'ops/openfga/model.fga'
    if (-not (Test-Path -LiteralPath $modelFile -PathType Leaf)) { throw 'The canonical OpenFGA DSL file is required.' }
    # Only the read-only inspection may be recreated. Never delete/retry the
    # mutator after an ambiguous result, even on a later explicit invocation.
    $null = Invoke-FgaKubectl @('delete', 'job', 'kind-local-fga-inspect', '--ignore-not-found=true', '--wait=true')
    New-FgaJob 'inspect'
    $inspection = Read-FgaJobResult 'kind-local-fga-inspect'
    $stores = $inspection.PSObject.Properties['stores']
    if (-not $stores -or $stores.Value -isnot [array]) { throw 'Native inspection lacks its stores array.' }
    if ($stores.Value.Count -ne 0) { throw 'Stores already exist; recover their references explicitly, never select one by name.' }
    # CLI list aggregates up to 20 pages and omits continuation-token metadata.
    # This is not an exhaustive inventory claim, only the explicit empty-install gate.
    $null = Invoke-FgaKubectl @('create', 'configmap', $ModelConfigMap, "--from-file=model.fga=$modelFile")
    New-FgaJob 'initialize'
    $created = Read-FgaJobResult 'kind-local-fga-initialize'
    $references = Read-FgaReferences $created -NativeResult
    $null = Invoke-FgaKubectl @('create', 'configmap', 'local-full-fga',
        "--from-literal=store-id=$($references.'store-id')", "--from-literal=model-id=$($references.'model-id')")
    return $references
} catch {
    # Native Job objects/logs remain available. No rollback, ID guessing,
    # publication overwrite or automatic second store creation is attempted.
    throw 'FGA first installation incomplete; preserve native Jobs/results for explicit operator recovery. No mutator retry or rollback is performed.'
}
