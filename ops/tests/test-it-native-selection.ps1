#Requires -Version 5.1
# Pure PowerShell/AST checks: no Cargo executable, Docker, Kind or fixtures.
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$path = Join-Path $PSScriptRoot 'it-native.ps1'
$tokens = $null
$errors = $null
$ast = [Management.Automation.Language.Parser]::ParseFile($path, [ref]$tokens, [ref]$errors)
if ($errors.Count) { throw 'IT launcher parse failed' }
$resolver = $ast.Find({ param($node)
    $node -is [Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq 'Resolve-ItTestBinary'
}, $true)
if ($null -eq $resolver) { throw 'Missing named selection resolver' }
. ([scriptblock]::Create($resolver.Extent.Text))
$names = @(
    'apparatus_p3_t4_gateway', 'apparatus_p3_t5_consent',
    'apparatus_p1_t1_migration', 'apparatus_p2_t2_migration', 'apparatus_p3_t2_migration',
    'apparatus_p2_t4_resume', 'apparatus_p2_t7_cleanup', 'transaction_readiness_tests'
)
foreach ($name in $names) {
    if ((Resolve-ItTestBinary -Suites @('manifesto') -Binary $name) -cne $name) { throw 'Valid named selection rejected' }
    foreach ($badName in @($name.ToUpperInvariant(), (' ' + $name), ($name + ' '), ('tests/' + $name + '.rs'), ($name + '; anything'))) {
        try { $null = Resolve-ItTestBinary -Suites @('manifesto') -Binary $badName; throw 'unexpected-accept' }
        catch { if ($_.Exception.Message -eq 'unexpected-accept') { throw 'Non-exact Manifesto binary accepted' } }
    }
    foreach ($badSuites in @(@('iam'), @('Manifesto'), @('manifesto','manifesto'), @('manifesto','hive'))) {
        try { $null = Resolve-ItTestBinary -Suites $badSuites -Binary $name; throw 'unexpected-accept' }
        catch { if ($_.Exception.Message -eq 'unexpected-accept') { throw 'Cross-suite Manifesto selection accepted' } }
    }
}
if ((Resolve-ItTestBinary -Suites @('lazaret') -Binary 'redis_tls') -cne 'redis_tls') {
    throw 'Valid exact Lazaret Redis TLS selection rejected'
}
foreach ($suites in @(@('iam'), @('manifesto'), @('Lazaret'), @('lazaret','iam'), @('lazaret','lazaret'))) {
    try { $null = Resolve-ItTestBinary -Suites $suites -Binary 'redis_tls'; throw 'unexpected-accept' }
    catch { if ($_.Exception.Message -eq 'unexpected-accept') { throw 'Cross-suite Redis TLS selection accepted' } }
}
foreach ($binary in @('REDIS_TLS', ' redis_tls', 'redis_tls ', 'redis_tls.exe', 'tests/redis_tls.rs', 'redis_tls; anything')) {
    try { $null = Resolve-ItTestBinary -Suites @('lazaret') -Binary $binary; throw 'unexpected-accept' }
    catch { if ($_.Exception.Message -eq 'unexpected-accept') { throw 'Non-exact Redis TLS selection accepted' } }
}
$oauthBinary = 'oauth_browser_transactions'
if ((Resolve-ItTestBinary -Suites @('telegraph') -Binary 'outbox_runtime_tests') -cne 'outbox_runtime_tests') {
    throw 'Valid exact Telegraph outbox selection rejected'
}
foreach ($badSuites in @(@('iam'), @('manifesto'), @('Telegraph'), @('telegraph','telegraph'), @('telegraph','hive'))) {
    try { $null = Resolve-ItTestBinary -Suites $badSuites -Binary 'outbox_runtime_tests'; throw 'unexpected-accept' }
    catch { if ($_.Exception.Message -eq 'unexpected-accept') { throw 'Cross-suite Telegraph selection accepted' } }
}
foreach ($badName in @('OUTBOX_RUNTIME_TESTS', ' outbox_runtime_tests', 'outbox_runtime_tests ', 'tests/outbox_runtime_tests.rs', 'outbox_runtime_tests; anything')) {
    try { $null = Resolve-ItTestBinary -Suites @('telegraph') -Binary $badName; throw 'unexpected-accept' }
    catch { if ($_.Exception.Message -eq 'unexpected-accept') { throw 'Non-exact Telegraph binary accepted' } }
}
if ((Resolve-ItTestBinary -Suites @('iam') -Binary $oauthBinary) -cne $oauthBinary) { throw 'Valid IAM OAuth selection rejected' }
$iamBinaries = @(
    $oauthBinary, 'auth_transactions_postgres', 'oauth_cleanup_lifecycle',
    'auth_oauth_start', 'auth_oauth_callback', 'auth_oauth_provider_failures',
    'relink_provider', 'verified_idp_https', 'organization_account_guard',
    'app_cleanup_supervision', 'request_trace_redaction', 'signing_epoch_writer'
)
foreach ($name in $iamBinaries) {
    if ((Resolve-ItTestBinary -Suites @('iam') -Binary $name) -cne $name) { throw 'Valid exact IAM selection rejected' }
    foreach ($suiteName in @('manifesto', 'hive', 'telegraph', 'lazaret', 'IAM')) {
        try { $null = Resolve-ItTestBinary -Suites @($suiteName) -Binary $name; throw 'unexpected-accept' }
        catch { if ($_.Exception.Message -eq 'unexpected-accept') { throw 'Cross-suite IAM selection accepted' } }
    }
    foreach ($badName in @($name.ToUpperInvariant(), (' ' + $name), ($name + ' '), ('tests/' + $name + '.rs'), ($name + '; anything'))) {
        try { $null = Resolve-ItTestBinary -Suites @('iam') -Binary $badName; throw 'unexpected-accept' }
        catch { if ($_.Exception.Message -eq 'unexpected-accept') { throw 'Non-exact IAM binary accepted' } }
    }
    foreach ($badSuites in @(@('iam', 'iam'), @('iam', 'hive'))) {
        try { $null = Resolve-ItTestBinary -Suites $badSuites -Binary $name; throw 'unexpected-accept' }
        catch { if ($_.Exception.Message -eq 'unexpected-accept') { throw 'Multiple suites accepted' } }
    }
}
foreach ($suites in @(@('manifesto'), @('iam'), @('iam', 'hive'))) {
    if ((Resolve-ItTestBinary -Suites $suites -Binary '') -ne '') { throw 'Default suite selection changed' }
}
foreach ($case in @(
    @{suite=@('iam'); binary=$names[0]}, @{suite=@('manifesto','hive'); binary=$names[0]},
    @{suite=@(); binary=$names[0]}, @{suite=@('manifesto'); binary='unknown'},
    @{suite=@('manifesto'); binary=' apparatus_p3_t4_gateway'},
    @{suite=@('manifesto'); binary='apparatus_p3_t4_gateway '},
    @{suite=@('manifesto'); binary='APPARATUS_P3_T4_GATEWAY'},
    @{suite=@('manifesto'); binary='--jobs=1'}, @{suite=@('manifesto'); binary='-j 1'},
    @{suite=@('manifesto'); binary='apparatus_p3_t4_gateway; anything'},
    @{suite=@('manifesto'); binary=$oauthBinary}, @{suite=@('hive'); binary=$oauthBinary},
    @{suite=@('telegraph'); binary=$oauthBinary}, @{suite=@('lazaret'); binary=$oauthBinary},
    @{suite=@('IAM'); binary=$oauthBinary}, @{suite=@('iam','iam'); binary=$oauthBinary},
    @{suite=@('iam','hive'); binary=$oauthBinary}, @{suite=@(); binary=$oauthBinary},
    @{suite=@('iam'); binary='OAUTH_BROWSER_TRANSACTIONS'},
    @{suite=@('iam'); binary=' oauth_browser_transactions'},
    @{suite=@('iam'); binary='oauth_browser_transactions '},
    @{suite=@('iam'); binary='oauth_browser_transactions.exe'},
    @{suite=@('iam'); binary='tests/oauth_browser_transactions.rs'},
    @{suite=@('iam'); binary='--jobs=1'}, @{suite=@('iam'); binary='-j 1'},
    @{suite=@('iam'); binary='oauth_browser_transactions; anything'},
    @{suite=@('iam'); binary='sqs_event_routing_tests'},
    @{suite=@('iam'); binary='signup_sqs'}, @{suite=@('iam'); binary='signup_kafka'}
)) {
    $rejected = $false
    try { $null = Resolve-ItTestBinary -Suites $case.suite -Binary $case.binary } catch { $rejected = $true }
    if (-not $rejected) { throw 'Invalid named selection accepted' }
}
if ($ast.Extent.Text.IndexOf('$testBinary = Resolve-ItTestBinary') -gt $ast.Extent.Text.IndexOf('Get-Command cargo')) {
    throw 'Named selection must be validated before any runtime preflight'
}
foreach ($probe in @('Get-Command docker', '$observer = Start-Job')) {
    if ($ast.Extent.Text.IndexOf('$testBinary = Resolve-ItTestBinary') -gt $ast.Extent.Text.IndexOf($probe)) {
        throw 'Named selection validation must precede runtime or observer'
    }
}
$branch = $ast.Find({ param($node)
    $node -is [Management.Automation.Language.IfStatementAst] -and $node.Extent.Text.StartsWith('if ($testBinary)')
}, $true)
if ($null -eq $branch) { throw 'Missing actual Cargo selection branch' }
function cargo { $script:seen = @($args) }
$packages = @{manifesto='manifesto-service'; iam='iam-service'; lazaret='lazaret-service'; telegraph='telegraph-service'}
foreach ($selected in @('manifesto', 'iam', 'lazaret', 'telegraph')) {
    $selections = if ($selected -eq 'iam') { @('') + $iamBinaries }
                  elseif ($selected -eq 'lazaret') { @('', 'redis_tls') }
                  elseif ($selected -eq 'telegraph') { @('', 'outbox_runtime_tests') }
                  else { @('') + $names }
    foreach ($testBinary in $selections) {
        $script:seen = @()
        . ([scriptblock]::Create($branch.Extent.Text))
        $expected = if ($testBinary) { @('test','--locked','-p',$packages[$selected],'--test',$testBinary) }
                    else { @('test','-p',$packages[$selected],'--tests') }
        if (($script:seen -join '|') -cne ($expected -join '|')) { throw 'Actual Cargo arguments do not match the closed selection' }
    }
}
Write-Host 'PASS: exact IAM OAuth/Manifesto/Lazaret Redis TLS selections, default compatibility, refusals, placement and actual Cargo branches; no runtime executed'
