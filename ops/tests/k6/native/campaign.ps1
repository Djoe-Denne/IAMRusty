# Native prepare/run/clean for the three retained scenarios. Does not own the lab.
param(
    [Parameter(Mandatory = $true)][ValidateSet('prepare', 'run', 'clean')][string]$Action,
    [Parameter(Mandatory = $true)][string]$Directory,
    [string]$Config,
    [switch]$TrustLabCaForThisRun,
    [switch]$ExclusiveDatabaseDdlForThisRun
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../../../..'))
$utf8 = [Text.UTF8Encoding]::new($false)
Add-Type -AssemblyName System.Net.Http
if (-not ('K6Windows.Child' -as [type])) { Add-Type -Path (Join-Path $PSScriptRoot '../lib/windows-native.cs') }
if (-not [K6Windows.Native]::NativeX64 -or [K6Windows.Native]::Elevated) { throw 'NATIVE_X64_NON_ELEVATED_SESSION_REQUIRED' }
. (Join-Path $PSScriptRoot '../lib/windows.ps1')
. (Join-Path $PSScriptRoot '../lib/windows-auth.ps1')
. (Join-Path $PSScriptRoot 'helpers.ps1')
$script:sequence = 0; $script:forwards = @()
$sid = [Security.Principal.WindowsIdentity]::GetCurrent().User
if (-not [IO.Path]::IsPathRooted($Directory) -or $Directory.StartsWith('\\')) { throw 'ABSOLUTE_LOCAL_DIRECTORY_REQUIRED' }
$Directory = [IO.Path]::GetFullPath($Directory).TrimEnd('\', '/')
if ($Directory.Equals($root, [StringComparison]::OrdinalIgnoreCase) -or $Directory.StartsWith($root + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'PRIVATE_DIRECTORY_OUTSIDE_GIT_REQUIRED' }
$ancestor = [IO.DirectoryInfo]::new($Directory)
while ($ancestor) {
    if ($ancestor.Exists -and ($ancestor.Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw 'REPARSE_POINT_REFUSED' }
    $ancestor = $ancestor.Parent
}
if ($Action -eq 'prepare') {
    if (Test-Path -LiteralPath $Directory) { throw 'FRESH_DIRECTORY_REQUIRED' }
    $null = New-Item -ItemType Directory -Path $Directory
    $acl = [Security.AccessControl.DirectorySecurity]::new(); $acl.SetOwner($sid); $acl.SetAccessRuleProtection($true, $false)
    $acl.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new($sid, 'FullControl', 'ContainerInherit,ObjectInherit', 'None', 'Allow'))
    Set-Acl -LiteralPath $Directory -AclObject $acl
    $cfg = ConvertFrom-Json ([IO.File]::ReadAllText($Config))
    $run = [Guid]::NewGuid().ToString('N')
    $script:state = [pscustomobject]@{version = 1; run_id = $run; name = 'k6-' + $run; config = $cfg; prepared = $false; attempted = $false; cleaned = $false
        resources = @(); target = $null; postgres = $null; database = 'k6_' + $run; database_pending = $false; database_oid = $null; database_dropped = $false; process_cleanup_proven = $true
        certificate_phase = 'good'; restoration = 'NOT_REQUIRED'; ca_owned = $false; ca_thumbprint = $null; edge_identity = $null
        source_hashes = @{}; input_hashes = @{}; artifact_hashes = @{}; verdict = 'NOT_RUN'; cleanup = 'NOT_RUN'}
    Save-NativeState
} else {
    $script:state = Read-NativeJson 'state.json'; $cfg = $state.config
    if ($state.version -ne 1 -or $state.run_id -cnotmatch '^[a-f0-9]{32}$' -or $state.name -cne ('k6-' + $state.run_id) -or $state.database -cne ('k6_' + $state.run_id)) { throw 'CAMPAIGN_INVALID' }
}
$acl = Get-Acl -LiteralPath $Directory
if ($acl.GetOwner([Security.Principal.SecurityIdentifier]).Value -cne $sid.Value) { throw 'FOREIGN_DIRECTORY_OWNER' }
foreach ($rule in $acl.Access) {
    if ($rule.AccessControlType -eq 'Allow' -and $rule.IdentityReference.Translate([Security.Principal.SecurityIdentifier]).Value -notin @($sid.Value, 'S-1-5-18')) { throw 'DIRECTORY_NOT_PRIVATE' }
}
$lockPath = Join-Path $Directory '.active.lock'
$lock = [IO.File]::Open($lockPath, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::None)
$prefix = @('--kubeconfig', $cfg.kubeconfig, '--context', 'kind-aiforall-local-full', '--namespace', 'aiforall-local-full')
$inputs = $cfg.operator_directory
$mesh = Join-Path $inputs 'pki/mesh'
$files = @('campaign.ps1', 'helpers.ps1', 'trust-ca.ps1', 'provision-chromium.ps1', 'chart/Chart.yaml', 'chart/templates/fixtures.yaml', '../config/tools.windows.lock.json', '../lib/campaign-target.js', '../lib/summary.js', '../lib/windows.ps1', '../lib/windows-auth.ps1', '../lib/windows-native.cs', '../scenarios/smoke/iam-health.js', '../scenarios/auth/oauth-browser.js', '../scenarios/auth/envoy-san.js', '../../support/oauth-idp-simulator.py')

function Read-Input([string]$Relative) { [IO.File]::ReadAllText((Join-Path $inputs $Relative)) }
function Rollout-Iam([string]$Phase) {
    $name = $state.name + '-iam'
    $receipt = @($state.resources | Where-Object { $_.kind -ceq 'Deployment' -and $_.name -ceq $name })[0]
    Save-NativeJson @(@{op = 'test'; path = '/metadata/uid'; value = $receipt.uid},
        @{op = 'replace'; path = '/spec/template/metadata/annotations/aiforall.dev~1certificate-phase'; value = $Phase}) 'rollout.json'
    $null = K @('patch', 'deployment', $name, '--type=json', '--patch-file', (Join-Path $Directory 'rollout.json'), '-o', 'name')
    $null = K @('rollout', 'status', ('deployment/' + $name), '--timeout=120s') 150000
    $pod = Get-NativePod 'iam'
    Save-NativeJson @{phase = $Phase; uid = $pod.metadata.uid; patch = $true; rollout = $true} ('ack-' + $Phase + '.json')
}
function Set-IamCertificate([string]$Phase) {
    $material = Read-NativeJson 'certificate-material.json'
    $name = $state.name + '-iam-tls'
    $receipt = @($state.resources | Where-Object { $_.kind -ceq 'Secret' -and $_.name -ceq $name })[0]
    # Allow recovery after an uncertain response without GET Secret: first test
    # the expected preimage; if that fails, test the already-applied target.
    $from = if ($Phase -ceq 'bad') { 'good' } else { 'bad' }
    $patch = @(@{op = 'test'; path = '/metadata/uid'; value = $receipt.uid},
        @{op = 'test'; path = '/data/iam-service.crt'; value = $material.$from.crt},
        @{op = 'test'; path = '/data/iam-service.key'; value = $material.$from.key},
        @{op = 'replace'; path = '/data/iam-service.crt'; value = $material.$Phase.crt},
        @{op = 'replace'; path = '/data/iam-service.key'; value = $material.$Phase.key})
    Save-NativeJson $patch 'certificate-patch.json'
    try { $null = K @('patch', 'secret', $name, '--type=json', '--patch-file', (Join-Path $Directory 'certificate-patch.json'), '-o', 'name') }
    catch {
        if ($Phase -ceq 'bad') { throw }
        Save-NativeJson @(@{op = 'test'; path = '/metadata/uid'; value = $receipt.uid},
            @{op = 'test'; path = '/data/iam-service.crt'; value = $material.good.crt},
            @{op = 'test'; path = '/data/iam-service.key'; value = $material.good.key}) 'certificate-patch.json'
        $null = K @('patch', 'secret', $name, '--type=json', '--patch-file', (Join-Path $Directory 'certificate-patch.json'), '-o', 'name')
    }
    Rollout-Iam $Phase
    $state.certificate_phase = $Phase; Save-NativeState
}
function Invoke-Scenario([string]$Case) {
    Assert-NativeForwards
    $script = if ($Case -ceq 'iam-health') { 'smoke/iam-health.js' } elseif ($Case -ceq 'auth-oauth-browser') { 'auth/oauth-browser.js' } else { 'auth/envoy-san.js' }
    $summary = Join-Path $Directory ($Case + '.json')
    if (Test-Path -LiteralPath ($summary + '.attempt')) { throw 'SCENARIO_ALREADY_ATTEMPTED' }
    Save-NativeText 'attempted' ($Case + '.json.attempt')
    $chrome = if ($Case -ceq 'auth-oauth-browser') { Pin-NativeTool 'chromium' } else { $null }
    $version = if ($chrome) { '156.0.8078.73' } else { $null }
    $envBlock = New-K6Environment $Directory $state.run_id $chrome @('native-private-sentinel') $version
    $envBlock.Add('K6_ORIGIN', 'https://127.0.0.1:18443')
    $envBlock.Add('K6_CAMPAIGN_FILE', (Join-Path $Directory 'target.json'))
    $envBlock['K6_SUMMARY_PATH'] = $summary
    if ($Case.StartsWith('auth-envoy-san-')) { $envBlock.Add('K6_AUTH_SAN_PHASE', $Case.Substring('auth-envoy-san-'.Length)) }
    $child = [K6Windows.Child]::new((Pin-NativeTool 'k6'), @('run', '--quiet', '--no-usage-report', '--address', '127.0.0.1:0', '--config', (Join-Path $Directory 'k6-options.json'), (Join-Path $PSScriptRoot ('../scenarios/' + $script))), $envBlock, $Directory)
    try {
        $r = $child.Finish(150000, 10000, $chrome)
        Save-NativeText $r.Stdout ($Case + '.stdout.log'); Save-NativeText $r.Stderr ($Case + '.stderr.log')
        Save-NativeJson @{cleanup = $r.Cleanup; identities_complete = $r.IdentityComplete; active = $r.Active; total = $r.Total; exit = $r.ExitCode; reason = $r.Reason} ($Case + '.process.json')
        if (-not $r.Cleanup) { $state.process_cleanup_proven = $false; Save-NativeState }
        if (-not $r.Cleanup -or -not $r.CaptureComplete -or $r.Overflow -or $r.Reason -cne 'NONE') { throw 'SCENARIO_PROCESS_UNPROVEN' }
        $value = Read-NativeJson ($Case + '.json')
        if ($Case -cne 'iam-health') { Assert-K6AuthSummary $value $state.run_id $Case $r.ExitCode }
        else {
            if ($r.ExitCode -ne 0 -or $value.run_id -cne $state.run_id -or $value.cases.Count -ne 1 -or $value.cases[0] -cne $Case -or
                $value.requests -ne 1 -or $value.started -ne 1 -or $value.completed -ne 1 -or $value.iterations -ne 1 -or $value.expected -ne 1) { throw 'HEALTH_SUMMARY_INVALID' }
            Assert-K6Fields $value.checks @('iam-status', 'iam-body')
            foreach ($c in $value.checks.PSObject.Properties.Value) { if ($c.passes -ne 1 -or $c.fails -ne 0) { throw 'HEALTH_CHECK_FAILED' } }
        }
    } finally { $child.Dispose() }
}
function Snapshot([string]$Phase) {
    $identity = Get-NativeIdentity 'envoy'
    if (($identity | ConvertTo-Json -Compress) -cne ($state.edge_identity | ConvertTo-Json -Compress)) { throw 'EDGE_INCARNATION_CHANGED' }
    $server = Get-NativeHttpJson 'http://127.0.0.1:19901/server_info'
    if ($server.state -cne 'LIVE' -or $server.version -cnotmatch '^[a-f0-9]{40}/1\.31\.2/(Clean|Modified)/(RELEASE|DEBUG)/[A-Za-z0-9_.-]+$' -or
        $null -eq $server.command_line_options.PSObject.Properties['restart_epoch']) { throw 'EDGE_VERSION_OR_STATE_UNPROVEN' }
    # Request counters only, as the legacy Admin reader did. Unfiltered Envoy
    # JSON also contains a histogram block with no name/value pair.
    $filter = '^cluster\.(iam|ext_authz|hive|telegraph|manifesto)\.(upstream_rq_total|upstream_rq_5xx|upstream_cx_connect_fail|upstream_cx_connect_timeout|ssl\.(handshake|fail_verify_san|fail_verify_error))$'
    $stats = Get-NativeHttpJson ('http://127.0.0.1:19901/stats?format=json&filter=' + [Uri]::EscapeDataString($filter))
    $counters = @{}
    foreach ($s in $stats.stats) {
        Assert-K6Fields $s @('name', 'value')
        if ($s.name -cnotmatch $filter -or $counters.ContainsKey($s.name) -or ($s.value -isnot [int] -and $s.value -isnot [long]) -or $s.value -lt 0) { throw 'ADMIN_COUNTER_INVALID' }
        $counters[$s.name] = [long]$s.value
    }
    $result = @{identity = $identity; restart_epoch = $server.command_line_options.restart_epoch; stats = $counters}
    Save-NativeJson $result ('snapshot-' + $Phase + '.json')
    return $result
}
function Restore-NativeIam {
    if ($state.certificate_phase -ceq 'good' -or $state.certificate_phase -ceq 'restored') { return }
    Set-IamCertificate 'restored'
    Invoke-Scenario 'auth-envoy-san-restored'
    $null = Snapshot 'restored'
    $state.restoration = 'PASS'; Save-NativeState
}
function Cleanup-NativeFixtures {
    $failed = @()
    if ($state.PSObject.Properties['process_cleanup_proven'] -and -not $state.process_cleanup_proven) { $failed += 'SCENARIO_PROCESS_CLEANUP_UNPROVEN' }
    try { Restore-NativeIam } catch { $failed += 'SAN_RESTORATION_FAILED' }
    # A failed physical restoration is not hidden by deleting the evidence.
    if ($state.certificate_phase -ceq 'good' -or ($state.certificate_phase -ceq 'restored' -and $state.restoration -ceq 'PASS')) {
        foreach ($receipt in @($state.resources | Where-Object kind -ceq 'Deployment')) {
            try { Remove-NativeObject $receipt } catch { $failed += ('DELETE_' + $receipt.kind) }
        }
        if ($state.database_oid -and -not $state.database_dropped) {
            try {
                if (-not $ExclusiveDatabaseDdlForThisRun) { throw 'DDL_EXCLUSIVITY_REQUIRED_FOR_DROP' }
                $db = $state.database; $oid = [long]$state.database_oid
                $guard = "DO `$`$ BEGIN IF NOT EXISTS (SELECT 1 FROM pg_database WHERE datname='$db' AND oid=$oid) OR EXISTS (SELECT 1 FROM pg_stat_activity WHERE datname='$db') THEN RAISE EXCEPTION 'database identity or sessions changed'; END IF; END; `$`$"
                $null = Invoke-NativeSql @('SELECT pg_advisory_lock(1860202611)', $guard, ('DROP DATABASE ' + $db))
                $state.database_dropped = $true; Save-NativeState
            } catch { $failed += 'DATABASE_REMAINS' }
        } elseif ($state.database_pending -and -not $state.database_oid) {
            try {
                $remaining = Invoke-NativeSql @("SELECT count(*) FROM pg_database WHERE datname='$($state.database)'")
                if ($remaining -cne '0') { throw 'DATABASE_WITHOUT_RECEIPT' }
                $state.database_pending = $false; Save-NativeState
            } catch { $failed += 'DATABASE_CREATE_UNCERTAIN' }
        }
        foreach ($receipt in @($state.resources | Where-Object kind -cne 'Deployment')) {
            try { Remove-NativeObject $receipt } catch { $failed += ('DELETE_' + $receipt.kind) }
        }
    } else { $failed += 'RESTORATION_UNPROVEN_FIXTURES_RETAINED' }
    # This rollback is independent of DB, certificate or resource cleanup errors.
    if ($state.ca_owned) {
        try {
            if (Test-Path ('Cert:\CurrentUser\Root\' + $state.ca_thumbprint)) {
                Remove-Item -LiteralPath ('Cert:\CurrentUser\Root\' + $state.ca_thumbprint)
            }
            if (Test-Path ('Cert:\CurrentUser\Root\' + $state.ca_thumbprint)) { throw 'CA_REMAINS' }
            $state.ca_owned = $false; Save-NativeState
        } catch { $failed += 'CA_REMOVAL_FAILED' }
    }
    foreach ($f in $script:forwards) {
        try {
            $f.child.Cancel(); $r = $f.task.GetAwaiter().GetResult()
            Save-NativeJson @{role = $f.role; port = $f.port; cleanup = $r.Cleanup; reason = $r.Reason; exit = $r.ExitCode; active = $r.Active} ('forward-' + $f.port + '.json')
            if (-not $r.Cleanup -or -not $r.CaptureComplete -or $r.Overflow -or $r.Reason -cnotin @('NONE', 'INTERRUPTED') -or $r.ExitCode -ne 0) { throw 'FORWARD_NOT_JOINED' }
        } catch { $failed += 'FORWARD_CLEANUP_FAILED' } finally { $f.child.Dispose() }
    }
    $script:forwards = @()
    $state.cleanup = if ($failed.Count) { 'FAIL' } else { 'PASS' }
    $state.cleaned = $failed.Count -eq 0
    Save-NativeState
    Save-NativeJson @{run_id = $state.run_id; status = $state.cleanup; failures = $failed; database_dropped = $state.database_dropped; owned_ca_absent = -not $state.ca_owned; restoration = $state.restoration} 'cleanup.json'
    if ($failed.Count) { throw 'CAMPAIGN_CLEANUP_FAILED_SEE_PRIVATE_RECEIPT' }
    foreach ($name in @('certificate-material.json', 'certificate-patch.json', 'create.json', 'iam-overlay.toml', 'iam-env.json', 'oauth.toml', 'provider.json', 'render-values.json', 'fixtures.json', 'oauth.key', 'idp.key')) {
        $p = Join-Path $Directory $name
        if (Test-Path -LiteralPath $p -PathType Leaf) { Remove-Item -LiteralPath $p }
    }
}

try {
    if ($Action -eq 'clean') {
        if (-not $state.cleaned) { $null = Assert-NativeTarget; Cleanup-NativeFixtures }
        Write-Output 'Native k6 cleanup complete; lab kept running.'
        return
    }
    foreach ($tool in @('k6', 'chromium', 'kubectl', 'docker', 'helm', 'openssl')) { $null = Pin-NativeTool $tool }
    if ((Invoke-NativeTool 'k6' @('version')) -cnotmatch '^k6(?:\.exe)? v2\.3\.0\b') { throw 'K6_VERSION_INVALID' }
    if (-not [IO.Path]::IsPathRooted($inputs) -or $inputs.StartsWith($root, [StringComparison]::OrdinalIgnoreCase)) { throw 'EXPLICIT_EXTERNAL_OPERATOR_INPUTS_REQUIRED' }
    if ($Action -eq 'prepare') {
        foreach ($f in $files) { $state.source_hashes[$f] = (Get-FileHash -LiteralPath (Join-Path $PSScriptRoot $f) -Algorithm SHA256).Hash }
        foreach ($f in @('apps/iam/development.toml', 'apps/iam/environment.env', 'apps/iam/platform-signing.pem', 'apps/iam/platform-signing.pub', 'apps/envoy-mesh/envoy.yaml', 'pki/ca-bundle.crt', 'pki/mesh/ca.crt', 'pki/mesh/ca.key', 'pki/mesh/iam-service.crt', 'pki/mesh/iam-service.key', 'pki/mesh/hive-service.crt', 'pki/mesh/hive-service.key', 'pki/mesh/envoy-mesh.crt', 'pki/mesh/envoy-mesh.key')) {
            $state.input_hashes[$f] = (Get-FileHash -LiteralPath (Join-Path $inputs $f) -Algorithm SHA256).Hash
        }
        $state.target = Assert-NativeTarget
        $envLines = (Read-Input 'apps/iam/environment.env') -split '\r?\n' | Where-Object { $_ -and -not $_.StartsWith('#') }
        $dbLines = @($envLines | Where-Object { $_.StartsWith('DATABASE_URL=') })
        if ($dbLines.Count -ne 1 -or @($envLines | Where-Object { $_ -cnotmatch '^(DATABASE_URL|PGSSLMODE|PGSSLROOTCERT)=' }).Count) { throw 'ENVIRONMENT_PROFILE_NOT_CLOSED' }
        $url = [UriBuilder]::new($dbLines[0].Substring('DATABASE_URL='.Length))
        if ($url.Scheme -notin @('postgres', 'postgresql') -or $url.Host -cne 'postgres.aiforall-local-full.svc.cluster.local' -or $url.Port -ne 5432 -or $url.Path -cne '/iam_dev') { throw 'DATABASE_SOURCE_PROFILE_INVALID' }
        $url.Path = '/' + $state.database
        Save-NativeJson @{DATABASE_URL = $url.Uri.AbsoluteUri} 'iam-env.json'
        $hmac = [Guid]::NewGuid().ToString('N') + [Guid]::NewGuid().ToString('N')
        $secret = [Guid]::NewGuid().ToString('N') + [Guid]::NewGuid().ToString('N')
        $client = 'native-' + $state.run_id
        $oauthHost = $state.name + '-oauth.aiforall-local-full.svc.cluster.local'
        $idpHost = $state.name + '-idp.aiforall-local-full.svc.cluster.local'
        $redirects = '["https://127.0.0.1:18443/iam/api/auth/gitlab/callback", "https://127.0.0.1:18443/iam/api/auth/gitlab/relink-callback"]'
        $overlay = @"
[database]
db = "$($state.database)"
[[idp.connectors]]
id = "gitlab"
base_url = "https://${oauthHost}:8443/oauth-connect/gitlab"
hmac_secret = "$hmac"
pkce_supported = true
redirect_uris = $redirects
"@
        Save-NativeText $overlay 'iam-overlay.toml'
        $oauth = @"
[server]
host = "0.0.0.0"
port = 8080
tls_enabled = true
tls_port = 8443
tls_cert_path = "/app/certs/oauth.crt"
tls_key_path = "/app/certs/oauth.key"
[logging]
level = "error"
[[providers]]
id = "gitlab"
client_id = "$client"
client_secret = "$secret"
hmac_secret = "$hmac"
redirect_uris = $redirects
scopes = ["read_user"]
pkce_supported = true
secrets_mode = "direct"
transport_security = "verified"
ca_cert_file = "/app/certs/ca.crt"
token_auth_method = "client_secret_post"
[providers.profile]
kind = "oauth"
authorization_url = "https://localhost:18445/oauth/authorize"
token_url = "https://${idpHost}:8080/oauth/token"
[providers.profile.apis.user]
url = "https://${idpHost}:8080/user"
access_token_auth = "bearer"
[providers.profile.claims]
sub = {api = "user", json_path = "$.id"}
preferred_username = {api = "user", json_path = "$.username"}
email = {api = "user", json_path = "$.email"}
email_verified = {api = "user", json_path = "$.email_verified"}
"@
        Save-NativeText $oauth 'oauth.toml'
        Save-NativeJson @{run_id = $state.run_id; client_id = $client; client_secret = $secret; edge_origin = 'https://127.0.0.1:18443'; tls_cert_file = '/app/certs/idp.crt'; tls_key_file = '/app/certs/idp.key'} 'provider.json'
        foreach ($role in @('oauth', 'idp')) {
            $dns = $state.name + '-' + $role + '.aiforall-local-full.svc.cluster.local'
            $publicSan = if ($role -ceq 'idp') { ',DNS:localhost' } else { '' }
            Save-NativeText "basicConstraints=critical,CA:FALSE`nkeyUsage=critical,digitalSignature,keyEncipherment`nextendedKeyUsage=serverAuth`nsubjectAltName=DNS:$dns,IP:127.0.0.1$publicSan`n" ($role + '.ext')
            $null = Invoke-NativeTool 'openssl' @('req', '-new', '-newkey', 'rsa:2048', '-nodes', '-subj', ('/CN=' + $state.name + '-' + $role), '-keyout', (Join-Path $Directory ($role + '.key')), '-out', (Join-Path $Directory ($role + '.csr')))
            $null = Invoke-NativeTool 'openssl' @('x509', '-req', '-in', (Join-Path $Directory ($role + '.csr')), '-CA', (Join-Path $mesh 'ca.crt'), '-CAkey', (Join-Path $mesh 'ca.key'), '-set_serial', ('0x' + [Guid]::NewGuid().ToString('N')), '-days', '2', '-sha256', '-extfile', (Join-Path $Directory ($role + '.ext')), '-out', (Join-Path $Directory ($role + '.crt')))
            $null = Invoke-NativeTool 'openssl' @('verify', '-CAfile', (Join-Path $mesh 'ca.crt'), '-verify_hostname', $dns, (Join-Path $Directory ($role + '.crt')))
        }
        $material = @{}
        foreach ($phase in @('good', 'bad')) {
            $leaf = if ($phase -ceq 'good') { 'iam-service' } else { 'hive-service' }
            $material[$phase] = @{crt = [Convert]::ToBase64String([IO.File]::ReadAllBytes((Join-Path $mesh ($leaf + '.crt')))); key = [Convert]::ToBase64String([IO.File]::ReadAllBytes((Join-Path $mesh ($leaf + '.key'))))}
        }
        $material.restored = $material.good; Save-NativeJson $material 'certificate-material.json'
        $null = Invoke-NativeTool 'openssl' @('verify', '-CAfile', (Join-Path $mesh 'ca.crt'), '-verify_hostname', 'iam.aiforall-local-full.svc.cluster.local', (Join-Path $mesh 'iam-service.crt'))
        $null = Invoke-NativeTool 'openssl' @('verify', '-CAfile', (Join-Path $mesh 'ca.crt'), (Join-Path $mesh 'hive-service.crt'))
        $null = Invoke-NativeTool 'openssl' @('verify', '-CAfile', (Join-Path $mesh 'ca.crt'), '-verify_hostname', 'iam.aiforall-local-full.svc.cluster.local', (Join-Path $mesh 'hive-service.crt')) 60000 2
        Save-NativeJson @{name = $state.name; runId = $state.run_id; images = $cfg.images; envoySource = Read-Input 'apps/envoy-mesh/envoy.yaml'; simulatorSource = [IO.File]::ReadAllText((Join-Path $PSScriptRoot '../../support/oauth-idp-simulator.py'))} 'render-values.json'
        $rendered = Invoke-NativeTool 'helm' @('template', 'k6-native', (Join-Path $PSScriptRoot 'chart'), '-f', (Join-Path $Directory 'render-values.json'))
        $start = $rendered.IndexOf('{'); if ($start -lt 0) { throw 'HELM_JSON_LIST_MISSING' }
        $render = ConvertFrom-Json $rendered.Substring($start)
        if ($render.kind -cne 'List' -or $render.items.Count -ne 10) { throw 'UNEXPECTED_FIXTURE_INVENTORY' }
        Save-NativeJson $render 'fixtures.json'
        Save-NativeJson @{version = 1; profile = 'iam-native-parity'; run_id = $state.run_id; origin = 'https://127.0.0.1:18443'; provider = 'https://localhost:18445/oauth/authorize'; issuer = 'https://127.0.0.1:18443/iam'; audience = 'aiforall'} 'target.json'
        Save-NativeJson @{} 'k6-options.json'
        foreach ($f in @('iam-env.json', 'iam-overlay.toml', 'oauth.toml', 'provider.json', 'certificate-material.json', 'fixtures.json', 'target.json', 'k6-options.json', 'oauth.crt', 'oauth.key', 'idp.crt', 'idp.key')) {
            $state.artifact_hashes[$f] = (Get-FileHash -LiteralPath (Join-Path $Directory $f) -Algorithm SHA256).Hash
        }
        $state.prepared = $true; Save-NativeState
        Write-Output 'Native fixtures prepared offline; no DB, workload, CA trust or scenario created.'
        return
    }
    if (-not $state.prepared -or $state.attempted) { throw 'FRESH_PREPARED_CAMPAIGN_REQUIRED' }
    if (-not $ExclusiveDatabaseDdlForThisRun) { throw 'EXPLICIT_DATABASE_DDL_EXCLUSIVITY_REQUIRED' }
    foreach ($property in $state.source_hashes.PSObject.Properties) {
        if ((Get-FileHash -LiteralPath (Join-Path $PSScriptRoot $property.Name) -Algorithm SHA256).Hash -cne $property.Value) { throw 'SOURCES_CHANGED_AFTER_PREPARE' }
    }
    foreach ($property in $state.input_hashes.PSObject.Properties) {
        if ((Get-FileHash -LiteralPath (Join-Path $inputs $property.Name) -Algorithm SHA256).Hash -cne $property.Value) { throw 'INPUTS_CHANGED_AFTER_PREPARE' }
    }
    foreach ($property in $state.artifact_hashes.PSObject.Properties) {
        if ((Get-FileHash -LiteralPath (Join-Path $Directory $property.Name) -Algorithm SHA256).Hash -cne $property.Value) { throw 'ARTIFACTS_CHANGED_AFTER_PREPARE' }
    }
    $null = Assert-NativeTarget
    # Validate trust before creating a DB or starting any timed child. The
    # default path requires preinstalled trust; install-profile is separate.
    $caText = Read-Input 'pki/mesh/ca.crt'
    if ($caText -cnotmatch '(?s)^\s*-----BEGIN CERTIFICATE-----\s*([A-Za-z0-9+/=\s]+)-----END CERTIFICATE-----\s*$') { throw 'ONE_PUBLIC_CA_REQUIRED' }
    $ca = [Security.Cryptography.X509Certificates.X509Certificate2]::new([Convert]::FromBase64String($Matches[1]))
    $state.ca_thumbprint = $ca.Thumbprint; Save-NativeState
    if (-not (Test-Path ('Cert:\CurrentUser\Root\' + $ca.Thumbprint)) -and -not (Test-Path ('Cert:\LocalMachine\Root\' + $ca.Thumbprint)) -and -not $TrustLabCaForThisRun) {
        throw 'CA_NOT_PRETRUSTED_INSTALL_PROFILE_BEFORE_RUN'
    }
    $state.attempted = $true; Save-NativeState
    try {
        if (-not (Test-Path ('Cert:\CurrentUser\Root\' + $ca.Thumbprint)) -and -not (Test-Path ('Cert:\LocalMachine\Root\' + $ca.Thumbprint))) {
            $state.ca_owned = $true; Save-NativeState
            $store = [Security.Cryptography.X509Certificates.X509Store]::new('Root', 'CurrentUser')
            try { $store.Open([Security.Cryptography.X509Certificates.OpenFlags]::ReadWrite); $store.Add($ca) }
            finally { $store.Close() }
            if (-not (Test-Path ('Cert:\CurrentUser\Root\' + $ca.Thumbprint))) { throw 'CA_TRUST_NOT_INSTALLED' }
        }
        foreach ($port in @(18443, 18445, 19901)) {
            $listener = [Net.Sockets.TcpListener]::new([Net.IPAddress]::Loopback, $port)
            try { $listener.Start() } finally { $listener.Stop() }
        }
        $pods = ConvertFrom-Json (K @('get', 'pods', '-l', 'app.kubernetes.io/name=postgres', '-o', 'json'))
        if ($pods.items.Count -ne 1 -or $pods.items[0].status.phase -cne 'Running') { throw 'ONE_POSTGRES_REQUIRED' }
        $state.postgres = @{pod = $pods.items[0].metadata.name; uid = $pods.items[0].metadata.uid}; Save-NativeState
        $db = $state.database
        $state.database_pending = $true; Save-NativeState
        $oid = Invoke-NativeSql @('SELECT pg_advisory_lock(1860202611)', ('CREATE DATABASE ' + $db), ("SELECT oid FROM pg_database WHERE datname='$db'"))
        $oidLines = @($oid -split '\r?\n' | Where-Object { $_ -cmatch '^[0-9]+$' })
        if ($oidLines.Count -ne 1) { throw 'DATABASE_OID_UNCERTAIN' }
        $state.database_oid = [long]$oidLines[0]; Save-NativeState
        $baseTls = @{'ca.crt' = $caText; 'ca-bundle.crt' = Read-Input 'pki/ca-bundle.crt'}
        foreach ($role in @('iam', 'oauth', 'envoy', 'idp')) {
            $data = $baseTls.Clone()
            if ($role -in @('iam', 'envoy')) {
                $leaf = if ($role -ceq 'iam') { 'iam-service' } else { 'envoy-mesh' }
                $data[$leaf + '.crt'] = Read-Input ('pki/mesh/' + $leaf + '.crt'); $data[$leaf + '.key'] = Read-Input ('pki/mesh/' + $leaf + '.key')
            } else {
                $data[$role + '.crt'] = [IO.File]::ReadAllText((Join-Path $Directory ($role + '.crt'))); $data[$role + '.key'] = [IO.File]::ReadAllText((Join-Path $Directory ($role + '.key')))
            }
            Create-NativeSecret ($role + '-tls') $data
        }
        Create-NativeSecret 'iam-config' @{'development.toml' = Read-Input 'apps/iam/development.toml'; 'k6.toml' = [IO.File]::ReadAllText((Join-Path $Directory 'iam-overlay.toml')); 'platform-signing.pem' = Read-Input 'apps/iam/platform-signing.pem'; 'platform-signing.pub' = Read-Input 'apps/iam/platform-signing.pub'}
        Create-NativeSecret 'iam-env' @{DATABASE_URL = (Read-NativeJson 'iam-env.json').DATABASE_URL}
        Create-NativeSecret 'oauth-config' @{'development.toml' = [IO.File]::ReadAllText((Join-Path $Directory 'oauth.toml'))}
        Create-NativeSecret 'idp-config' @{'provider.json' = [IO.File]::ReadAllText((Join-Path $Directory 'provider.json'))}
        foreach ($o in (Read-NativeJson 'fixtures.json').items) { Create-NativeObject $o }
        foreach ($role in @('idp', 'oauth', 'iam', 'envoy')) { $null = K @('rollout', 'status', ('deployment/' + $state.name + '-' + $role), '--timeout=120s') 150000 }
        $state.edge_identity = Get-NativeIdentity 'envoy'; Save-NativeState
        Start-NativeForward 'envoy' 18443 10000
        Start-NativeForward 'idp' 18445 8080
        Start-NativeForward 'envoy' 19901 9901
        $dump = Get-NativeHttpJson 'http://127.0.0.1:19901/config_dump'
        $loaded = @($dump.configs | Where-Object { $_.'@type' -ceq 'type.googleapis.com/envoy.admin.v3.ClustersConfigDump' })
        $iamCluster = @($loaded.static_clusters.cluster | Where-Object name -ceq 'iam')
        if ($iamCluster.Count -ne 1 -or $iamCluster[0].load_assignment.endpoints[0].lb_endpoints[0].endpoint.address.socket_address.address -cne ($state.name + '-iam.aiforall-local-full.svc.cluster.local')) { throw 'LOADED_IAM_DESTINATION_MISMATCH' }
        Save-NativeJson @{edge = $state.edge_identity; loaded_iam_destination = $true; sources = $state.source_hashes} 'admission.json'
        # Compare real JWKS through verified mTLS, without disturbing edge counters.
        $iamPod = Get-NativePod 'iam'
        $jwks = @()
        foreach ($address in @('127.0.0.1', 'iam.aiforall-local-full.svc.cluster.local')) {
            $args = @('exec', $iamPod.metadata.name, '-c', 'iam', '--', 'curl', '--silent', '--show-error', '--fail', '--cacert', '/app/certs/ca.crt', '--cert', '/app/certs/iam-service.crt', '--key', '/app/certs/iam-service.key')
            if ($address -ceq '127.0.0.1') { $args += @('--resolve', 'iam.aiforall-local-full.svc.cluster.local:8443:127.0.0.1') }
            $args += 'https://iam.aiforall-local-full.svc.cluster.local:8443/iam/.well-known/jwks.json'
            $jwks += ConvertFrom-Json (K $args)
        }
        $keys = @($jwks[0].keys | Where-Object { $_.kty -ceq 'RSA' })
        if ($keys.Count -ne 1 -or @($jwks[1].keys | Where-Object { $_.kid -ceq $keys[0].kid -and $_.n -ceq $keys[0].n -and $_.e -ceq $keys[0].e }).Count -ne 1) { throw 'NOMINAL_JWKS_INCOMPATIBLE' }
        Invoke-Scenario 'iam-health'
        Invoke-Scenario 'auth-envoy-san-good'
        $good = Snapshot 'good'
        try {
            $state.certificate_phase = 'uncertain'; Save-NativeState
            Set-IamCertificate 'bad'
            Invoke-Scenario 'auth-envoy-san-bad'
            $bad = Snapshot 'bad'
            if ((Get-K6AuthDelta $good $bad 'cluster.iam.ssl.fail_verify_san') -ne 1 -or (Get-K6AuthDelta $good $bad 'cluster.iam.ssl.fail_verify_error') -ne 0) { throw 'SAN_CAUSALITY_FAILED' }
        } finally { Restore-NativeIam }
        $restored = Snapshot 'restoration-proof'
        if ((Get-K6AuthDelta $good $restored 'cluster.iam.ssl.fail_verify_san') -ne 1 -or (Get-K6AuthDelta $good $restored 'cluster.iam.ssl.handshake') -lt 1) { throw 'SAN_RESTORATION_ORACLE_FAILED' }
        $before = Snapshot 'oauth-before'
        $providerBefore = Get-NativeHttpJson 'https://127.0.0.1:18445/observations'
        $countSql = "SELECT json_build_object('users',(SELECT count(*) FROM users),'outbox',(SELECT count(*) FROM rustycog_outbox_events))"
        $dbBefore = ConvertFrom-Json (Invoke-NativeSql @($countSql) $state.database)
        Save-NativeJson $dbBefore 'oauth-database-before.json'
        Invoke-Scenario 'auth-oauth-browser'
        $providerAfter = Get-NativeHttpJson 'https://127.0.0.1:18445/observations'
        $dbAfter = ConvertFrom-Json (Invoke-NativeSql @($countSql) $state.database)
        Save-NativeJson $dbAfter 'oauth-database-after.json'
        if (($dbAfter.users - $dbBefore.users) -ne 1 -or ($dbAfter.outbox - $dbBefore.outbox) -ne 0) { throw 'OAUTH_DATABASE_EFFECTS_UNEXPECTED' }
        $after = Snapshot 'oauth-after'
        foreach ($name in @('authorize', 'token', 'profile', 'pkce_validated')) {
            if ($providerBefore.run_id -cne $state.run_id -or $providerAfter.run_id -cne $state.run_id -or ($providerAfter.$name - $providerBefore.$name) -ne 3) { throw 'PROVIDER_CORRELATION_FAILED' }
        }
        if ((Get-K6AuthDelta $before $after 'cluster.iam.upstream_rq_total') -ne 7 -or (Get-K6AuthDelta $before $after 'cluster.ext_authz.upstream_rq_total') -ne 4 -or (Get-K6AuthDelta $before $after 'cluster.iam.upstream_rq_5xx') -ne 0) { throw 'OAUTH_EDGE_CARDINALITY_FAILED' }
        foreach ($cluster in @('iam', 'ext_authz', 'hive', 'telegraph', 'manifesto')) {
            foreach ($counter in @('upstream_cx_connect_fail', 'upstream_cx_connect_timeout', 'ssl.fail_verify_san', 'ssl.fail_verify_error')) {
                if ((Get-K6AuthDelta $before $after ('cluster.' + $cluster + '.' + $counter)) -ne 0) { throw 'OAUTH_TRANSPORT_FAILED' }
            }
        }
        Save-NativeJson @{run_id = $state.run_id; san_delta = 1; restoration = 'PASS'; iam_delta = 7; ext_authz_delta = 4; provider_delta = 3; pkce_delta = 3; user_delta = 1; outbox_delta = 0; jwks_compatible = $true} 'evidence.json'
        $state.verdict = 'PASS'; Save-NativeState
    } catch {
        Save-NativeJson @{message = $_.Exception.Message; stack = $_.ScriptStackTrace} 'failure-private.json'
        $state.verdict = 'FAIL'; Save-NativeState; throw
    }
    finally { Cleanup-NativeFixtures }
    Write-Output ('Three native k6 scenarios PASS; cleanup PASS; run ' + $state.run_id + '. Lab kept running.')
} finally { $lock.Dispose(); Remove-Item -LiteralPath $lockPath }
