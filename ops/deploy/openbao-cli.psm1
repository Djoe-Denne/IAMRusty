# Native Windows CLI primitives, not a complete startup/bootstrap command.
# A caller that initializes must bootstrap and revoke root before handback.
Set-StrictMode -Version Latest
. (Join-Path $PSScriptRoot 'private-path.ps1')

function Start-KindBaoCommand {
    param([hashtable]$Connection, [string]$Arguments, [string]$InputJson = '', [IO.Stream]$Destination,
          [string]$Token = '')
    $address = [Uri]$Connection.Address
    if (-not $address.IsAbsoluteUri -or $address.Scheme -cne 'https' -or
        $address.UserInfo -or $address.Fragment -or $address.Query -or $address.AbsolutePath -ne '/') {
        throw 'OpenBao requires an explicit HTTPS root address without credentials or URL options.'
    }
    if (-not (Test-Path -LiteralPath $Connection.CaFile -PathType Leaf)) {
        throw 'An explicit readable OpenBao CA file is required.'
    }
    $info = [Diagnostics.ProcessStartInfo]::new()
    $info.FileName = $Connection.Executable
    $info.Arguments = $Arguments # Internal constant commands; private bodies use stdin.
    $info.UseShellExecute = $false
    $info.CreateNoWindow = $true
    $info.RedirectStandardInput = $true
    $info.RedirectStandardOutput = $true
    $info.RedirectStandardError = $true
    foreach ($name in @($info.EnvironmentVariables.Keys)) {
        if ($name.StartsWith('BAO_', [StringComparison]::OrdinalIgnoreCase) -or
            $name.StartsWith('VAULT_', [StringComparison]::OrdinalIgnoreCase)) {
            $info.EnvironmentVariables.Remove($name)
        }
    }
    $info.EnvironmentVariables['BAO_ADDR'] = $address.AbsoluteUri
    $info.EnvironmentVariables['BAO_CACERT'] = [IO.Path]::GetFullPath($Connection.CaFile)
    $info.EnvironmentVariables['BAO_TLS_SERVER_NAME'] = $Connection.ServerName
    $info.EnvironmentVariables['BAO_SKIP_VERIFY'] = 'false'
    $info.EnvironmentVariables['BAO_DISABLE_REDIRECTS'] = 'true'
    $info.EnvironmentVariables['BAO_CLIENT_TIMEOUT'] = '15s'
    $info.EnvironmentVariables['BAO_MAX_RETRIES'] = '0'
    # Native CLI configuration and token helper files are explicitly empty.
    # Do not load the operator's ~/.bao or ~/.vault-token or a custom helper.
    $info.EnvironmentVariables['BAO_CONFIG_PATH'] = 'NUL'
    $info.EnvironmentVariables['BAO_TOKEN_PATH'] = 'NUL'
    $info.EnvironmentVariables['BAO_TOKEN'] = $Token
    $process = [Diagnostics.Process]::new()
    $process.StartInfo = $info
    $started = $false
    try {
        $started = $process.Start()
        $errors = $process.StandardError.ReadToEndAsync()
        $output = if ($Destination) { $process.StandardOutput.BaseStream.CopyToAsync($Destination) }
                  else { $process.StandardOutput.ReadToEndAsync() }
        if ($InputJson) { $process.StandardInput.Write($InputJson) }
        $process.StandardInput.Close()
        if (-not $process.WaitForExit(30000)) { throw 'CLI timeout' }
        $text = $output.GetAwaiter().GetResult()
        $diagnostic = $errors.GetAwaiter().GetResult()
        # Retain only native CLI classifications, never private diagnostics.
        $httpCode = if ($diagnostic -match '(?m)^Code: ([1-5][0-9]{2})\. Errors:\s*$') { [int]$Matches[1] } else { 0 }
        $missingPath = if ($diagnostic.Trim() -cmatch '^No value found at ([a-z0-9/-]+)$') { $Matches[1] } else { '' }
        return @{ ExitCode = $process.ExitCode; Output = $(if (-not $Destination) { $text });
                  HttpCode = $httpCode; MissingPath = $missingPath }
    } catch {
        throw 'Native OpenBao invocation failed; private output is not printed and no retry is performed.'
    } finally {
        $info.EnvironmentVariables.Remove('BAO_TOKEN')
        if ($started -and -not $process.HasExited) { $process.Kill(); $process.WaitForExit() }
        $process.Dispose()
    }
}

function Invoke-KindBaoCommand {
    param([hashtable]$Connection, [string]$Arguments, [string]$InputJson = '', [IO.Stream]$Destination,
          [string]$Token = '')
    $version = Start-KindBaoCommand -Connection $Connection -Arguments 'version'
    if ($version.ExitCode -ne 0 -or $version.Output.Trim() -notmatch '^OpenBao v2\.6\.2(\s|$)') {
        throw 'The explicitly selected native OpenBao CLI must be version 2.6.2.'
    }
    Start-KindBaoCommand -Connection $Connection -Arguments $Arguments -InputJson $InputJson -Destination $Destination -Token $Token
}

function ConvertFrom-KindBaoJson([string]$Text) {
    try { return ConvertFrom-Json -InputObject $Text -ErrorAction Stop }
    catch { throw 'Invalid OpenBao JSON; private content is not printed.' }
}

function Read-KindBaoBundle([string]$Path, [switch]$NewInit) {
    try { $text = [IO.File]::ReadAllText([IO.Path]::GetFullPath($Path)) }
    catch { throw 'Cannot read the explicitly designated private OpenBao bundle.' }
    $bundle = ConvertFrom-KindBaoJson $text
    $parts = $bundle.PSObject.Properties['unseal_keys_b64']
    $threshold = $bundle.PSObject.Properties['unseal_threshold']
    if (-not $parts -or -not $threshold -or $parts.Value -isnot [array] -or
        ($threshold.Value -isnot [int] -and $threshold.Value -isnot [long])) {
        throw 'Invalid native unseal bundle fields; private values are not printed.'
    }
    if ($threshold.Value -lt 1 -or $threshold.Value -gt $parts.Value.Count) {
        throw 'Incomplete unseal bundle; no other keys are searched.'
    }
    foreach ($part in $parts.Value) {
        if ($part -isnot [string] -or [string]::IsNullOrWhiteSpace($part)) {
            throw 'Invalid unseal part; private values are not printed.'
        }
    }
    if ($NewInit) {
        $root = $bundle.PSObject.Properties['root_token']
        $shares = $bundle.PSObject.Properties['unseal_shares']
        if (-not $root -or $root.Value -isnot [string] -or [string]::IsNullOrWhiteSpace($root.Value) -or
            -not $shares -or ($shares.Value -isnot [int] -and $shares.Value -isnot [long]) -or
            $shares.Value -ne 1 -or $threshold.Value -ne 1 -or $parts.Value.Count -ne 1) {
            throw 'Init output lacks the expected native Shamir1/1/root fields; preserve it without retrying init.'
        }
    }
    return $bundle
}

function Get-KindOpenBaoStatus {
    param([Parameter(Mandatory = $true)][hashtable]$Connection)
    $result = Invoke-KindBaoCommand -Connection $Connection -Arguments 'status -format=json'
    if ($result.ExitCode -notin @(0, 2)) { throw 'OpenBao status failed; this is not an absent or uninitialized coffer.' }
    $value = ConvertFrom-KindBaoJson $result.Output
    $initialized = $value.PSObject.Properties['initialized']
    $sealed = $value.PSObject.Properties['sealed']
    if (-not $initialized -or -not $sealed -or $initialized.Value -isnot [bool] -or $sealed.Value -isnot [bool]) {
        throw 'OpenBao status lacks native initialized/sealed booleans.'
    }
    if (($result.ExitCode -eq 2) -ne $sealed.Value) { throw 'OpenBao status and native exit code disagree.' }
    [pscustomobject]@{ initialized = $initialized.Value; sealed = $sealed.Value }
}

function Initialize-KindOpenBao {
    param([Parameter(Mandatory = $true)][hashtable]$Connection,
          [Parameter(Mandatory = $true)][string]$OutputFile)
    if (-not [IO.Path]::IsPathRooted($OutputFile)) { throw 'An explicit absolute init-output path is required.' }
    $path = [IO.Path]::GetFullPath($OutputFile)
    if (Test-Path -LiteralPath $path) { throw 'Init output already exists; no initialization or overwrite is allowed.' }
    $status = Get-KindOpenBaoStatus -Connection $Connection
    if ($status.initialized) { throw 'OpenBao is already initialized; do not initialize an existing backend.' }
    $parent = [IO.Path]::GetDirectoryName($path)
    $null = [IO.Directory]::CreateDirectory($parent)
    Set-KindPrivatePath $parent -Folder
    $stream = [IO.File]::Open($path, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::None)
    try {
        Set-KindPrivatePath $path
        $result = Invoke-KindBaoCommand -Connection $Connection -Arguments 'operator init -format=json -key-shares=1 -key-threshold=1' -Destination $stream
        $stream.Flush($true)
        if ($result.ExitCode -ne 0) { throw 'OpenBao init failed; preserve the output and backend, never automatically repeat init.' }
    } finally { $stream.Dispose() }
    $null = Read-KindBaoBundle $path -NewInit
    # This is only the native init primitive. Root remains active until the
    # caller completes bootstrap and explicitly verifies its revocation.
}

function Unlock-KindOpenBao {
    param([Parameter(Mandatory = $true)][hashtable]$Connection, [string]$InputFile)
    $status = Get-KindOpenBaoStatus -Connection $Connection
    if (-not $status.initialized) { throw 'OpenBao is not initialized; unseal must not initialize it.' }
    if (-not $status.sealed) { return $status }
    if (-not $InputFile -or -not (Test-Path -LiteralPath $InputFile -PathType Leaf)) {
        throw 'Sealed OpenBao requires the explicitly designated private unseal input.'
    }
    $bundle = Read-KindBaoBundle $InputFile
    $keys = @($bundle.unseal_keys_b64)
    $threshold = [int]$bundle.unseal_threshold
    if ($threshold -lt 1 -or $threshold -gt $keys.Count) { throw 'Incomplete native unseal bundle; no other keys are searched.' }
    for ($i = 0; $i -lt $threshold; $i++) {
        if ([string]::IsNullOrWhiteSpace($keys[$i])) { throw 'An unseal part is missing.' }
        $body = @{ key = $keys[$i] } | ConvertTo-Json -Compress
        $result = Invoke-KindBaoCommand -Connection $Connection -Arguments 'write sys/unseal -' -InputJson $body
        if ($result.ExitCode -ne 0) { throw 'OpenBao unseal failed; no key or private diagnostic is printed.' }
    }
    $status = Get-KindOpenBaoStatus -Connection $Connection
    if ($status.sealed) { throw 'OpenBao remains sealed; no automatic retry, reset or alternate key lookup.' }
    return $status
}

function Read-KindBaoData {
    param([hashtable]$Connection, [string]$Path, [string]$Token, [switch]$AllowMissing)
    $result = Invoke-KindBaoCommand -Connection $Connection -Arguments "read -format=json $Path" -Token $Token
    if ($AllowMissing -and $result.ExitCode -eq 2 -and $result.MissingPath -ceq $Path) { return $null }
    if ($result.ExitCode -ne 0) { throw 'OpenBao read failed; an authorization or transport error is not absence.' }
    $value = ConvertFrom-KindBaoJson $result.Output
    $data = $value.PSObject.Properties['data']
    if (-not $data -or $null -eq $data.Value) { throw 'OpenBao read lacks native data.' }
    return $data.Value
}

function Write-KindBaoData {
    param([hashtable]$Connection, [string]$Path, $Body, [string]$Token)
    $result = Invoke-KindBaoCommand -Connection $Connection -Arguments "write $Path -" -Token $Token `
        -InputJson (ConvertTo-Json -InputObject $Body -Depth 20 -Compress)
    if ($result.ExitCode -ne 0) { throw 'OpenBao write failed; preserve partial state without automatic retry.' }
}

function Get-KindBaoDuration([string]$Value) {
    if ($Value -cnotmatch '^([1-9][0-9]{0,5})([smh])$') { throw 'A bounded public token duration is required.' }
    $multiplier = @{ s = 1; m = 60; h = 3600 }[$Matches[2]]
    return [int]$Matches[1] * $multiplier
}

function Assert-KindBaoFields($Actual, $Expected) {
    foreach ($field in $Expected.Keys) {
        $property = $Actual.PSObject.Properties[$field]
        if (-not $property) { throw 'Existing OpenBao configuration is incomplete; explicit modification is required.' }
        $wanted = $Expected[$field]
        $found = $property.Value
        if ($wanted -is [array]) {
            if (@(Compare-Object @($wanted | Sort-Object) @($found | Sort-Object) -CaseSensitive).Count) {
                throw 'Existing OpenBao configuration differs; no overwrite is allowed.'
            }
        } elseif ($wanted -is [string]) {
            if ($found -isnot [string] -or $found.Trim() -cne $wanted.Trim()) {
                throw 'Existing OpenBao configuration differs; no overwrite is allowed.'
            }
        } elseif ($found -cne $wanted) { throw 'Existing OpenBao configuration differs; no overwrite is allowed.' }
    }
}

function Test-KindBaoProductToken {
    param([hashtable]$Connection, [string]$Token, [string]$Policy, [int]$MaxSeconds)
    $result = Invoke-KindBaoCommand -Connection $Connection -Arguments 'token lookup -format=json' -Token $Token
    if ($result.ExitCode -ne 0) { throw 'Product token lookup failed.' }
    $value = ConvertFrom-KindBaoJson $result.Output
    $data = $value.data
    Assert-KindBaoFields $data @{ policies = @($Policy); orphan = $true; renewable = $false }
    foreach ($field in @('ttl', 'explicit_max_ttl')) {
        $property = $data.PSObject.Properties[$field]
        if (-not $property -or ($property.Value -isnot [int] -and $property.Value -isnot [long]) -or
            $property.Value -le 0 -or $property.Value -gt $MaxSeconds) { throw 'Product token lifetime is not bounded as requested.' }
    }
}

function Invoke-KindOpenBaoBootstrap {
    [CmdletBinding(DefaultParameterSetName = 'NewInit')]
    param([Parameter(Mandatory = $true)][hashtable]$Connection,
          [Parameter(Mandatory = $true, ParameterSetName = 'NewInit')][string]$InitFile,
          [Parameter(Mandatory = $true, ParameterSetName = 'Admin')][string]$AdminTokenFile,
          [ValidateSet('lazaret')][string[]]$TokenConsumers = @(),
          [string]$ProductOutputDirectory,
          [string[]]$KubernetesRoles = @(), [string]$KubernetesCaFile,
          [hashtable]$SeedFiles = @{})
    # Explicit bootstrap only. The launcher must never invoke this on normal
    # rebuild, or on an existing compatible coffer merely because it is running.
    $ErrorActionPreference = 'Stop'
    $token = ''
    $products = @()
    $revokeRoot = $PSCmdlet.ParameterSetName -ceq 'NewInit'
    try {
        if ($TokenConsumers.Count) {
            if (-not $ProductOutputDirectory -or -not [IO.Path]::IsPathRooted($ProductOutputDirectory) -or
                (Test-Path -LiteralPath $ProductOutputDirectory)) { throw 'A new absolute product output directory is required.' }
        }
        $publicFile = Join-Path $PSScriptRoot 'openbao/bootstrap.json'
        $public = ConvertFrom-KindBaoJson ([IO.File]::ReadAllText($publicFile))
        $policies = @()
        foreach ($name in @($TokenConsumers) + @($KubernetesRoles)) {
            if ($name -cnotmatch '^[a-z][a-z0-9-]*$' -or -not $public.roles.PSObject.Properties[$name]) {
                throw 'Only explicitly selected declared consumers/roles are supported.'
            }
            $role = $public.roles.PSObject.Properties[$name].Value
            $policyField = $role.PSObject.Properties['token_policies']
            if (-not $policyField) { $policyField = $role.PSObject.Properties['policies'] }
            $policies += $policyField.Value -split ','
        }
        $policies = @($policies | Sort-Object -Unique)
        if ($KubernetesRoles.Count -and (-not $KubernetesCaFile -or -not [IO.Path]::IsPathRooted($KubernetesCaFile))) {
            throw 'Selected Kubernetes roles require the explicit host Kubernetes CA file.'
        }
        foreach ($path in $SeedFiles.Keys) {
            if ($path -cnotmatch '^[a-z0-9-]+(/[a-z0-9-]+)*$' -or -not [IO.Path]::IsPathRooted($SeedFiles[$path])) {
                throw 'Seed paths and absolute private input files must be explicit.'
            }
        }
        if ($revokeRoot) {
            $token = (Read-KindBaoBundle $InitFile -NewInit).root_token
        } else {
            if (-not [IO.Path]::IsPathRooted($AdminTokenFile)) { throw 'An absolute admin token input is required.' }
            $token = [IO.File]::ReadAllText($AdminTokenFile).Trim()
        }
        if ([string]::IsNullOrWhiteSpace($token)) { throw 'An explicit nonempty bootstrap credential is required.' }
        $status = Get-KindOpenBaoStatus -Connection $Connection
        if (-not $status.initialized -or $status.sealed) { throw 'Bootstrap requires an initialized unsealed server.' }
        $session = Invoke-KindBaoCommand -Connection $Connection -Arguments 'token lookup -format=json' -Token $token
        if ($session.ExitCode -ne 0) { throw 'Explicit bootstrap credential lookup failed.' }
        if ($revokeRoot) { Assert-KindBaoFields (ConvertFrom-KindBaoJson $session.Output).data @{ policies = @('root') } }
        if ($TokenConsumers.Count) {
            $null = New-Item -ItemType Directory -Path $ProductOutputDirectory -ErrorAction Stop
            Set-KindPrivatePath $ProductOutputDirectory -Folder
        }
        # DEV supplied secret/ implicitly; the persistent server does not.
        $mountsResult = Invoke-KindBaoCommand -Connection $Connection -Arguments 'secrets list -format=json' -Token $token
        if ($mountsResult.ExitCode -ne 0) { throw 'OpenBao mounts listing failed.' }
        $mounts = ConvertFrom-KindBaoJson $mountsResult.Output
        $requiredMounts = @{ secret = @{ type = 'kv'; options = @{ version = '2' } } }
        $needsTransit = @($policies | Where-Object { $_ -in @('local-full-transit-sign', 'local-full-transit-verify') }).Count -gt 0
        if ($needsTransit) { $requiredMounts['transit'] = @{ type = $public.mounts.transit.type } }
        foreach ($name in $requiredMounts.Keys) {
            $existing = $mounts.PSObject.Properties["$name/"]
            if (-not $existing) { Write-KindBaoData $Connection "sys/mounts/$name" $requiredMounts[$name] $token }
            else {
                Assert-KindBaoFields $existing.Value @{ type = $requiredMounts[$name].type }
                if ($name -ceq 'secret') { Assert-KindBaoFields $existing.Value.options @{ version = '2' } }
            }
        }
        foreach ($name in $policies) {
            $expected = @{ policy = $public.policies.PSObject.Properties[$name].Value }
            $existing = Read-KindBaoData $Connection "sys/policies/acl/$name" $token -AllowMissing
            if ($null -eq $existing) { Write-KindBaoData $Connection "sys/policies/acl/$name" $expected $token }
            else { Assert-KindBaoFields $existing $expected }
        }
        if ($needsTransit) {
            $request = $public.requests.'transit-key'
            $existing = Read-KindBaoData $Connection $request.path $token -AllowMissing
            if ($null -eq $existing) { Write-KindBaoData $Connection $request.path $request.body $token }
            else { Assert-KindBaoFields $existing @{ type = $request.body.type } }
        }
        if ($KubernetesRoles.Count) {
            $authResult = Invoke-KindBaoCommand -Connection $Connection -Arguments 'auth list -format=json' -Token $token
            if ($authResult.ExitCode -ne 0) { throw 'OpenBao auth listing failed.' }
            $auth = ConvertFrom-KindBaoJson $authResult.Output
            if (-not $auth.PSObject.Properties['kubernetes/']) {
                Write-KindBaoData $Connection 'sys/auth/kubernetes' @{ type = $public.auth.kubernetes.type } $token
            } else { Assert-KindBaoFields $auth.'kubernetes/' @{ type = 'kubernetes' } }
            $config = @{
                kubernetes_host = $public.authConfig.fields.kubernetes_host
                kubernetes_ca_cert = [IO.File]::ReadAllText($KubernetesCaFile).Trim()
                disable_local_ca_jwt = $false
            }
            $existing = Read-KindBaoData $Connection $public.authConfig.path $token -AllowMissing
            if ($null -eq $existing -or -not $existing.kubernetes_host) {
                Write-KindBaoData $Connection $public.authConfig.path $config $token
            } else { Assert-KindBaoFields $existing $config }
            foreach ($name in $KubernetesRoles) {
                $expected = @{}
                foreach ($field in $public.roles.PSObject.Properties[$name].Value.PSObject.Properties) {
                    $key = $field.Name; $value = $field.Value
                    if ($key -ceq 'policies') { $key = 'token_policies' }
                    if ($key -ceq 'ttl') { $key = 'token_ttl' }
                    if ($key -in @('bound_service_account_names', 'bound_service_account_namespaces', 'token_policies')) { $value = @($value -split ',') }
                    if ($key.EndsWith('_ttl')) { $value = Get-KindBaoDuration $value }
                    $expected[$key] = $value
                }
                $path = "auth/kubernetes/role/$name"
                $existing = Read-KindBaoData $Connection $path $token -AllowMissing
                if ($null -eq $existing) { Write-KindBaoData $Connection $path $expected $token }
                else { Assert-KindBaoFields $existing $expected }
            }
        }
        foreach ($path in $SeedFiles.Keys) {
            $existing = Read-KindBaoData $Connection "secret/metadata/$path" $token -AllowMissing
            if ($null -eq $existing) {
                $data = ConvertFrom-KindBaoJson ([IO.File]::ReadAllText($SeedFiles[$path]))
                if ($data -isnot [pscustomobject]) { throw 'A seed input must be a native KV JSON object.' }
                Write-KindBaoData $Connection "secret/data/$path" @{ options = @{ cas = 0 }; data = $data } $token
            }
        }
        foreach ($name in @($TokenConsumers | Select-Object -Unique)) {
            $role = $public.roles.PSObject.Properties[$name].Value
            $policy = $role.token_policies
            $maxSeconds = Get-KindBaoDuration $role.token_explicit_max_ttl
            $path = Join-Path $ProductOutputDirectory "$name.json"
            $stream = [IO.File]::Open($path, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::None)
            try {
                Set-KindPrivatePath $path
                $args = "token create -format=json -orphan -no-default-policy -renewable=false -ttl=$($role.token_ttl) -explicit-max-ttl=$($role.token_explicit_max_ttl) -policy=$policy"
                $result = Invoke-KindBaoCommand -Connection $Connection -Arguments $args -Token $token -Destination $stream
                $stream.Flush($true)
                if ($result.ExitCode -ne 0) { throw 'Product token creation failed; preserve the partial private response.' }
            } finally { $stream.Dispose() }
            $value = ConvertFrom-KindBaoJson ([IO.File]::ReadAllText($path))
            $productToken = $value.auth.client_token
            if ($productToken -isnot [string] -or [string]::IsNullOrWhiteSpace($productToken)) { throw 'Native product token output is incomplete.' }
            Test-KindBaoProductToken $Connection $productToken $policy $maxSeconds
            $products += @{ Token = $productToken; Policy = $policy; MaxSeconds = $maxSeconds }
        }
        if ($revokeRoot) {
            $result = Invoke-KindBaoCommand -Connection $Connection -Arguments 'token revoke -self' -Token $token
            if ($result.ExitCode -ne 0) { throw 'Root revocation failed.' }
            $lookup = Invoke-KindBaoCommand -Connection $Connection -Arguments 'token lookup -format=json' -Token $token
            if ($lookup.ExitCode -ne 2 -or $lookup.HttpCode -ne 403) { throw 'Root revocation is not verified by a native permission denial.' }
            foreach ($product in $products) {
                Test-KindBaoProductToken $Connection $product.Token $product.Policy $product.MaxSeconds
            }
        }
    } catch {
        if ($revokeRoot) { throw 'Bootstrap incomplete; root may remain active. Preserve private files and inspect explicitly; no automatic retry or rollback.' }
        throw 'Explicit admin bootstrap incomplete. Preserve private files and partial state; supplied admin is not revoked.'
    } finally {
        $token = ''; $productToken = ''; $products = @()
    }
}

Export-ModuleMember -Function Get-KindOpenBaoStatus, Initialize-KindOpenBao, Unlock-KindOpenBao, Invoke-KindOpenBaoBootstrap
