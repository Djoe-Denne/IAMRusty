$ErrorActionPreference = 'Stop'
$revisions = @(
    @{ name = 'current'; spec = '9b43bd99e11b01303191a399e4bc5b9ad9563579:services/Hive/application/src/dto/common.rs'; hash = 'b634f2801fe3e709dae12bbbe1bbe41bec654361' },
    @{ name = 'historical'; spec = '419f206457a37489a2b8e9782f4904614250397e:Hive/application/src/dto/common.rs'; hash = '7295f653d4677da9477a7bbd18d3ab311f9ca5b5' }
)

foreach ($revision in $revisions) {
    $hash = git -C $PSScriptRoot rev-parse $revision.spec
    if ($LASTEXITCODE -ne 0 -or $hash -ne $revision.hash) { throw "Missing or unexpected source: $($revision.spec). Use a clone with full history." }
    $source = (git -C $PSScriptRoot show $revision.spec) -join "`n"
    if ($LASTEXITCODE -ne 0) { throw "Could not read $($revision.spec)" }

    $patterns = '(?ms)^#\[derive\([^\n]+\)\]\npub struct PaginationResponse \{.*?^\}', '(?ms)^impl PaginationResponse \{.*?^\}'
    $parts = foreach ($pattern in $patterns) {
        $hits = [regex]::Matches($source, $pattern)
        if ($hits.Count -ne 1) { throw "No unique matching Rust block in $($revision.spec)" }
        $hits[0].Value
    }
    $destination = Join-Path $PSScriptRoot "src/$($revision.name).rs"
    [IO.File]::WriteAllText($destination, ($parts -join "`n`n") + "`n")
    "Exported $($revision.name) from Git blob $hash"
}
