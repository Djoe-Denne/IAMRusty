#Requires -Version 5.1
# Pure PowerShell/AST checks: no Cargo, Docker, Kind or fixture startup.
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$path = Join-Path $PSScriptRoot 'it-native.ps1'
$tokens = $null
$errors = $null
$ast = [Management.Automation.Language.Parser]::ParseFile($path, [ref]$tokens, [ref]$errors)
if ($errors.Count) { throw "it-native.ps1 parse failed: $errors" }
$resolver = $ast.Find({ param($node)
    $node -is [Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq 'Resolve-ItTestThreads'
}, $true)
if ($null -eq $resolver) { throw 'Missing thread policy resolver' }
. ([scriptblock]::Create($resolver.Extent.Text))
foreach ($case in @(@{ input = $null; expected = 2 }, @{ input = ''; expected = 2 },
                    @{ input = '1'; expected = 1 }, @{ input = '4'; expected = 4 },
                    @{ input = '2147483647'; expected = 2147483647 })) {
    if ((Resolve-ItTestThreads $case.input) -ne $case.expected) { throw 'Incorrect thread cap' }
}
foreach ($invalid in @('0', '-1', '1.5', 'abc', ' ', ' 2', '2 ', '+2', '2147483648', '99999999999999999999')) {
    $rejected = $false
    try { $null = Resolve-ItTestThreads $invalid } catch { $rejected = $true }
    if (-not $rejected) { throw "Invalid thread override accepted: $invalid" }
}

# Exercise the launcher's actual environment save/set/restore statements without
# running its runtime code. Both unset/preset environments and exceptions matter.
$save = $ast.Find({ param($node)
    $node -is [Management.Automation.Language.ForEachStatementAst] -and
    $node.Extent.Text.Contains('$previous[$name] =')
}, $true)
$set = $ast.Find({ param($node)
    $node -is [Management.Automation.Language.AssignmentStatementAst] -and
    $node.Left.Extent.Text -eq '$env:RUST_TEST_THREADS'
}, $true)
$restore = $ast.Find({ param($node)
    $node -is [Management.Automation.Language.ForEachStatementAst] -and
    $node.Extent.Text.StartsWith('foreach ($name in $previous.Keys)')
}, $true)
if ($null -eq $save -or $null -eq $set -or $null -eq $restore) { throw 'Missing environment lifecycle statements' }
if (-not $save.Extent.Text.Contains("'RUST_TEST_THREADS'")) { throw 'Thread cap not included in environment snapshot' }
$original = $env:RUST_TEST_THREADS
try {
    foreach ($ambient in @($null, '7')) {
        foreach ($fail in @($false, $true)) {
            $env:RUST_TEST_THREADS = $ambient
            $previous = @{}
            . ([scriptblock]::Create($save.Extent.Text))
            try {
                $testThreads = Resolve-ItTestThreads ''
                . ([scriptblock]::Create($set.Extent.Text))
                if ($env:RUST_TEST_THREADS -ne '2') { throw 'Ambient value defeated reproducible default' }
                if ($fail) { throw 'injected suite failure' }
            } catch {
                if (-not $fail -or $_.Exception.Message -ne 'injected suite failure') { throw }
            } finally { . ([scriptblock]::Create($restore.Extent.Text)) }
            if ($env:RUST_TEST_THREADS -ne $ambient) { throw 'Thread environment not restored' }
        }
    }
} finally { $env:RUST_TEST_THREADS = $original }
$source = $ast.Extent.Text
if ($source.IndexOf('$testThreads = Resolve-ItTestThreads') -gt $source.IndexOf('Get-Command cargo')) {
    throw 'Invalid override must fail before Cargo/runtime preflight'
}
if ($set.Extent.StartOffset -gt $source.IndexOf('foreach ($selected in $Suite)')) {
    throw 'Thread cap must be set before the batch'
}
Write-Host 'PASS: thread policy (5 valid/10 invalid), ambient isolation and restoration (4 success/failure cases), placement, PowerShell parsing'
