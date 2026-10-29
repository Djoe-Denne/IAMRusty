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
$names = @('apparatus_p3_t4_gateway', 'apparatus_p3_t5_consent')
foreach ($name in $names) {
    if ((Resolve-ItTestBinary -Suites @('manifesto') -Binary $name) -cne $name) { throw 'Valid named selection rejected' }
}
$oauthBinary = 'oauth_browser_transactions'
if ((Resolve-ItTestBinary -Suites @('iam') -Binary $oauthBinary) -cne $oauthBinary) { throw 'Valid IAM OAuth selection rejected' }
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
    @{suite=@('iam'); binary='oauth_browser_transactions; anything'}
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
$packages = @{manifesto='manifesto-service'; iam='iam-service'}
foreach ($selected in @('manifesto', 'iam')) {
    $selections = if ($selected -eq 'iam') { @('', $oauthBinary) } else { @('', $names[0], $names[1]) }
    foreach ($testBinary in $selections) {
        $script:seen = @()
        . ([scriptblock]::Create($branch.Extent.Text))
        $expected = if ($testBinary) { @('test','--locked','-p',$packages[$selected],'--test',$testBinary) }
                    else { @('test','-p',$packages[$selected],'--tests') }
        if (($script:seen -join '|') -cne ($expected -join '|')) { throw 'Actual Cargo arguments do not match the closed selection' }
    }
}
Write-Host 'PASS: exact IAM OAuth/Manifesto selections, default compatibility, refusals, placement and actual Cargo branches; no runtime executed'
