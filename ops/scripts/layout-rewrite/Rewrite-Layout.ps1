#Requires -Version 5.1
param(
    [switch]$Apply,
    [switch]$SelfCheck,
    [string]$RepoRoot = '',
    [switch]$Verbose,
    [string]$ReportPath = ''
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$script:Moves = @{}
$script:FromKeys = @()
$script:DestPrefixes = @()
$script:NeverMove = @()
$script:AddWorkspaceMembers = @()
$script:PreconditionDirs = @()
$script:NeedleRegex = $null
$script:RxComposeContext = $null
$script:ScanKeysLongest = @()
$script:BinExt = @(
    '.png', '.jpg', '.jpeg', '.gif', '.webp', '.ico',
    '.exe', '.dll', '.so', '.dylib', '.rlib', '.rmeta', '.o', '.a',
    '.zip', '.gz', '.7z', '.woff', '.woff2', '.ttf', '.pdf', '.lock',
    '.crt', '.key', '.pem', '.p12', '.pfx'
)
$script:ExcludeDirNames = @{
    'target'              = $true
    '.git'                = $true
    'rustycog'            = $true
    '.grepai'             = $true
    'node_modules'        = $true
    'docker-build-stage'  = $true
    '.terraform'          = $true
    '.vs'                 = $true
}

function Show-Usage {
    Write-Host 'layout-rewrite : usage'
    Write-Host '  powershell.exe -NoProfile -File ops/scripts/layout-rewrite/Rewrite-Layout.ps1'
    Write-Host '  powershell.exe -NoProfile -File ops/scripts/layout-rewrite/Rewrite-Layout.ps1 -SelfCheck'
    Write-Host '  powershell.exe -NoProfile -File ops/scripts/layout-rewrite/Rewrite-Layout.ps1 -Apply'
    Write-Host '  options : -RepoRoot <chemin>  -Verbose  -ReportPath <fichier>'
    Write-Host '  defaut : dry-run (aucune ecriture). -Apply est bloque si les dossiers cibles n existent pas.'
}

function Find-RepoRootFromScript {
    $dir = $PSScriptRoot
    while (-not [string]::IsNullOrEmpty($dir)) {
        $cargo = Join-Path $dir 'Cargo.toml'
        $git = Join-Path $dir '.git'
        if ((Test-Path -LiteralPath $cargo) -and (Test-Path -LiteralPath $git)) {
            return [System.IO.Path]::GetFullPath($dir)
        }
        $parent = [System.IO.Path]::GetDirectoryName($dir)
        if ([string]::IsNullOrEmpty($parent) -or $parent -eq $dir) { break }
        $dir = $parent
    }
    return $null
}

function ConvertTo-SlashPath {
    param([string]$Path)
    if ([string]::IsNullOrEmpty($Path)) { return '' }
    return ($Path -replace '\\', '/')
}

function Add-PathSegments {
    param(
        [string]$Path,
        [System.Collections.Generic.List[string]]$Target
    )
    $Target.Clear()
    $n = ConvertTo-SlashPath $Path
    if ([string]::IsNullOrWhiteSpace($n)) { return }
    $n = $n.Trim()
    while ($n.Length -ge 2 -and $n.Substring(0, 2) -eq './') {
        $n = $n.Substring(2)
    }
    $n = $n.Trim('/')
    if ([string]::IsNullOrEmpty($n)) { return }
    foreach ($part in $n.Split(@('/'), [System.StringSplitOptions]::RemoveEmptyEntries)) {
        [void]$Target.Add($part)
    }
}

function Get-SegCount {
    param([string]$Path)
    $tmp = New-Object System.Collections.Generic.List[string]
    Add-PathSegments -Path $Path -Target $tmp
    return [int]$tmp.Count
}

function Get-ParentRepoPath {
    param([string]$RepoRelative)
    $n = ConvertTo-SlashPath $RepoRelative
    $n = $n.TrimEnd('/')
    $idx = $n.LastIndexOf('/')
    if ($idx -lt 0) { return '' }
    return $n.Substring(0, $idx)
}

function Strip-DotSlash {
    param([string]$Path)
    $n = ConvertTo-SlashPath $Path
    if ([string]::IsNullOrEmpty($n)) { return '' }
    while ($n.Length -ge 2 -and $n.Substring(0, 2) -eq './' -and -not ($n.Length -ge 3 -and $n.Substring(0, 3) -eq '../')) {
        $n = $n.Substring(2)
    }
    return $n
}

function Test-PathHasPrefix {
    param([string]$Path, [string]$Prefix)
    if ([string]::IsNullOrEmpty($Prefix)) { return $false }
    if ($Path -eq $Prefix) { return $true }
    if ($Path.StartsWith($Prefix + '/')) { return $true }
    return $false
}

function Resolve-VirtualRepoPath {
    param([string]$Dir, [string]$Token)
    $tokenSlash = ConvertTo-SlashPath $Token
    if ([string]::IsNullOrWhiteSpace($tokenSlash)) { return $null }
    $t = Strip-DotSlash $tokenSlash
    if ($t.StartsWith('/')) { return $null }

    $acc = New-Object System.Collections.Generic.List[string]
    Add-PathSegments -Path $Dir -Target $acc

    $rawParts = $t.Split(@('/'), [System.StringSplitOptions]::None)
    foreach ($part in $rawParts) {
        if ($part -eq '' -or $part -eq '.') { continue }
        if ($part -eq '..') {
            if ($acc.Count -eq 0) { return $null }
            $acc.RemoveAt($acc.Count - 1)
            continue
        }
        [void]$acc.Add($part)
    }
    if ($acc.Count -eq 0) { return '' }
    return ($acc -join '/')
}

function Get-RelativizedPath {
    param([string]$FromDir, [string]$ToPath)
    $from = New-Object System.Collections.Generic.List[string]
    $to = New-Object System.Collections.Generic.List[string]
    Add-PathSegments -Path $FromDir -Target $from
    Add-PathSegments -Path $ToPath -Target $to
    $i = 0
    $fromN = $from.Count
    $toN = $to.Count
    while ($i -lt $fromN -and $i -lt $toN -and $from[$i] -eq $to[$i]) {
        $i++
    }
    $parts = New-Object System.Collections.Generic.List[string]
    for ($u = 0; $u -lt ($fromN - $i); $u++) { [void]$parts.Add('..') }
    for ($j = $i; $j -lt $toN; $j++) { [void]$parts.Add($to[$j]) }
    if ($parts.Count -eq 0) { return '.' }
    return ($parts -join '/')
}

function Test-SegmentPrefixMatch {
    param([string]$Path, [string]$Prefix)
    return (Test-PathHasPrefix (ConvertTo-SlashPath $Path) (ConvertTo-SlashPath $Prefix))
}

function Test-MatchesMappingPrefix {
    param([string]$RepoRelative)
    $n = ConvertTo-SlashPath $RepoRelative
    $n = (Strip-DotSlash $n).Trim('/')
    if ([string]::IsNullOrEmpty($n)) { return $false }
    foreach ($dest in $script:DestPrefixes) {
        if (Test-PathHasPrefix $n $dest) { return $true }
    }
    foreach ($from in $script:FromKeys) {
        if (Test-PathHasPrefix $n $from) { return $true }
    }
    return $false
}

function Test-HasDotDotSegment {
    param([string]$Token)
    $n = ConvertTo-SlashPath $Token
    $n = Strip-DotSlash $n
    $tmp = New-Object System.Collections.Generic.List[string]
    Add-PathSegments -Path $n -Target $tmp
    foreach ($s in $tmp) {
        if ($s -eq '..') { return $true }
    }
    if ($n.StartsWith('../') -or $n -eq '..') { return $true }
    return $false
}

function Get-FirstSegment {
    param([string]$Path)
    $tmp = New-Object System.Collections.Generic.List[string]
    Add-PathSegments -Path $Path -Target $tmp
    if ($tmp.Count -eq 0) { return '' }
    return $tmp[0]
}

function Test-IsFromKey {
    param([string]$Segment)
    if ([string]::IsNullOrEmpty($Segment)) { return $false }
    return $script:Moves.ContainsKey($Segment)
}

function Invoke-MapPath {
    param([string]$RepoRelative)
    $n = ConvertTo-SlashPath $RepoRelative
    $n = Strip-DotSlash $n
    $n = $n.Trim('/')
    if ([string]::IsNullOrEmpty($n)) { return '' }

    foreach ($dest in $script:DestPrefixes) {
        if (Test-PathHasPrefix $n $dest) { return $n }
    }

    $segs = New-Object System.Collections.Generic.List[string]
    Add-PathSegments -Path $n -Target $segs
    $bestLen = 0
    $bestTo = $null
    foreach ($from in $script:FromKeys) {
        $fromSegs = New-Object System.Collections.Generic.List[string]
        Add-PathSegments -Path $from -Target $fromSegs
        $flen = $fromSegs.Count
        if ($flen -le $bestLen) { continue }
        if ($segs.Count -lt $flen) { continue }
        $ok = $true
        for ($i = 0; $i -lt $flen; $i++) {
            if ($segs[$i] -ne $fromSegs[$i]) { $ok = $false; break }
        }
        if ($ok) {
            $bestLen = $flen
            $bestTo = [string]$script:Moves[$from]
        }
    }

    if ($null -eq $bestTo) { return $n }

    $toSegs = New-Object System.Collections.Generic.List[string]
    Add-PathSegments -Path $bestTo -Target $toSegs
    $out = New-Object System.Collections.Generic.List[string]
    foreach ($s in $toSegs) { [void]$out.Add($s) }
    for ($k = $bestLen; $k -lt $segs.Count; $k++) { [void]$out.Add($segs[$k]) }
    return ($out -join '/')
}

function Test-SkipToken {
    param([string]$Token)
    if ([string]::IsNullOrWhiteSpace($Token)) { return $true }
    $t = $Token.Trim()
    $tl = $t.ToLowerInvariant()
    if ($tl.StartsWith('http://') -or $tl.StartsWith('https://') -or $tl.StartsWith('mailto:')) { return $true }
    if ($t -match '^[A-Za-z]:' -or $t.StartsWith('\\')) { return $true }
    return $false
}

function Test-RepoPathExists {
    param([string]$Root, [string]$Rel)
    if ([string]::IsNullOrWhiteSpace($Root) -or [string]::IsNullOrWhiteSpace($Rel)) { return $false }
    $rel = (ConvertTo-SlashPath $Rel).Trim('/')
    if ([string]::IsNullOrEmpty($rel) -or $rel.Contains('..')) { return $false }
    $p = $Root
    foreach ($seg in ($rel -split '/')) {
        if ([string]::IsNullOrEmpty($seg) -or $seg -eq '.' -or $seg -eq '..') { return $false }
        $p = Join-Path $p $seg
    }
    return Test-Path -LiteralPath $p
}

function Invoke-RewriteRelativeToken {
    param(
        [string]$Token,
        [string]$CurrentFileRepoPath,
        [string]$RepoRoot = '',
        [bool]$RequireExists = $false
    )
    $result = @{ Changed = $false; NewToken = $Token }
    if (Test-SkipToken $Token) { return $result }

    $usedBackslash = $Token.Contains('\')
    $slashToken = ConvertTo-SlashPath $Token
    $hadTrailingSlash = $slashToken.EndsWith('/') -and $slashToken -ne '/'
    $hadDotSlash = $slashToken.Length -ge 2 -and $slashToken.Substring(0, 2) -eq './' -and -not ($slashToken.Length -ge 3 -and $slashToken.Substring(0, 3) -eq '../')

    $stripped = Strip-DotSlash $slashToken
    if ($hadTrailingSlash -and $stripped.EndsWith('/')) {
        $stripped = $stripped.TrimEnd('/')
    }
    elseif ($hadTrailingSlash) {
        $stripped = $stripped.TrimEnd('/')
    }

    $currentDir = Get-ParentRepoPath $CurrentFileRepoPath
    $futureFile = Invoke-MapPath $CurrentFileRepoPath
    $futureDir = Get-ParentRepoPath $futureFile
    $resolved = Resolve-VirtualRepoPath -Dir $currentDir -Token $slashToken

    $repoRelPrime = $null
    $useRelativize = $false
    $hasDotDot = Test-HasDotDotSegment $slashToken

    if ($hasDotDot) {
        if ($null -eq $resolved) { return $result }
        $resolvedFirst = Get-FirstSegment $resolved
        $isRepoRoot = ($resolved -eq '')
        if ($isRepoRoot -or (Test-MatchesMappingPrefix $resolved) -or (Test-IsFromKey $resolvedFirst)) {
            if ($isRepoRoot) { $repoRelPrime = '' } else { $repoRelPrime = Invoke-MapPath $resolved }
            $useRelativize = $true
        }
        elseif ($resolvedFirst -eq 'rustycog' -or $resolvedFirst -eq 'config') {
            $repoRelPrime = $resolved
            $useRelativize = $true
        }
        else {
            return $result
        }
    }
    elseif (Test-MatchesMappingPrefix $stripped) {
        $repoRelPrime = Invoke-MapPath $stripped
        $useRelativize = $false
    }
    elseif ($null -ne $resolved -and (Test-IsFromKey (Get-FirstSegment $resolved))) {
        $repoRelPrime = Invoke-MapPath $resolved
        $useRelativize = $true
    }
    elseif (Test-IsFromKey (Get-FirstSegment $stripped)) {
        $repoRelPrime = Invoke-MapPath $stripped
        $useRelativize = $false
    }
    elseif ($null -ne $resolved -and ((Get-FirstSegment $resolved) -eq 'rustycog' -or (Get-FirstSegment $resolved) -eq 'config')) {
        $repoRelPrime = $resolved
        $useRelativize = $true
    }
    else {
        return $result
    }

    if ([string]::IsNullOrEmpty($repoRelPrime) -and $repoRelPrime -ne '') { return $result }

    if ($useRelativize) {
        $newToken = Get-RelativizedPath -FromDir $futureDir -ToPath $repoRelPrime
        if ($hadDotSlash -and -not $newToken.StartsWith('..')) {
            if ($newToken -eq '.') {
                $newToken = '.'
            }
            elseif (-not $newToken.StartsWith('./')) {
                $newToken = './' + $newToken
            }
        }
    }
    else {
        $newToken = $repoRelPrime
        if ($hadDotSlash -and -not $newToken.StartsWith('..')) {
            if (-not $newToken.StartsWith('./')) {
                $newToken = './' + $newToken
            }
        }
    }

    if ($hadTrailingSlash -and -not $newToken.EndsWith('/')) {
        $newToken = $newToken + '/'
    }

    if ($usedBackslash) {
        $newToken = $newToken -replace '/', '\'
    }

    if ($RequireExists) {
        $probe = $stripped
        if ($hasDotDot) { $probe = $resolved }
        $srcExists = Test-RepoPathExists -Root $RepoRoot -Rel $probe
        $dstExists = Test-RepoPathExists -Root $RepoRoot -Rel $repoRelPrime
        if (-not $srcExists -and -not $dstExists) { return $result }
    }

    $normOld = ConvertTo-SlashPath $Token
    $normNew = ConvertTo-SlashPath $newToken
    if ($normOld -eq $normNew) { return $result }

    $result.Changed = $true
    $result.NewToken = $newToken
    return $result
}

function Initialize-Mapping {
    param([string]$MappingPath)
    $raw = [System.IO.File]::ReadAllText($MappingPath, [System.Text.Encoding]::UTF8)
    $json = $raw | ConvertFrom-Json
    $script:Moves = @{}
    foreach ($p in $json.moves.PSObject.Properties) {
        $script:Moves[$p.Name] = [string]$p.Value
    }
    $script:FromKeys = @($script:Moves.Keys | Sort-Object { - (Get-SegCount $_) }, { - $_.Length })
    $destSet = @{}
    foreach ($v in $script:Moves.Values) {
        $destSet[[string]$v] = $true
    }
    $script:DestPrefixes = @($destSet.Keys | Sort-Object { - $_.Length })
    $script:NeverMove = @()
    if ($null -ne $json.never_move) {
        foreach ($n in @($json.never_move)) { $script:NeverMove += [string]$n }
    }
    $script:AddWorkspaceMembers = @()
    if ($null -ne $json.add_workspace_members) {
        foreach ($n in @($json.add_workspace_members)) { $script:AddWorkspaceMembers += [string]$n }
    }
    $script:PreconditionDirs = @()
    if ($null -ne $json.apply_precondition_dirs) {
        foreach ($n in @($json.apply_precondition_dirs)) { $script:PreconditionDirs += [string]$n }
    }

    $script:ScanKeysLongest = @()
    foreach ($d in $script:DestPrefixes) { $script:ScanKeysLongest += $d }
    foreach ($f in ($script:FromKeys | Sort-Object { - $_.Length })) { $script:ScanKeysLongest += $f }
    $needles = New-Object System.Collections.Generic.List[string]
    foreach ($k in $script:ScanKeysLongest) { [void]$needles.Add([regex]::Escape($k)) }
    [void]$needles.Add([regex]::Escape('../'))
    $script:NeedleRegex = New-Object System.Text.RegularExpressions.Regex(($needles -join '|'), [System.Text.RegularExpressions.RegexOptions]::Compiled)
    $compiled = [System.Text.RegularExpressions.RegexOptions]::Compiled
    $script:RxPathEq = New-Object System.Text.RegularExpressions.Regex('path\s*=\s*(["''])([^"'']*)\1', $compiled)
    $script:RxMembers = New-Object System.Text.RegularExpressions.Regex('members\s*=\s*\[', $compiled)
    $script:RxQuoted = New-Object System.Text.RegularExpressions.Regex('(["''])([^"'']*)\1', $compiled)
    $script:RxInclude = New-Object System.Text.RegularExpressions.Regex('include_(?:str|bytes)!\s*\(\s*(["''])([^"'']*)\1', $compiled)
    $script:RxComposeContext = New-Object System.Text.RegularExpressions.Regex('(?m)^[ \t]*context:[ \t]*(?<q>["'']?)(?<v>\.\.(?:/\.\.)*|\.)(?:\k<q>)(?=\s|#|$)', $compiled)
}

function Get-LineNumber {
    param([string]$Text, [int]$Index)
    if ($Index -le 0) { return 1 }
    $sliceLen = [Math]::Min($Index, $Text.Length)
    $slice = $Text.Substring(0, $sliceLen)
    return (@($slice -split "`n", [System.StringSplitOptions]::None)).Count
}

function Test-IsLineCommentToml {
    param([string]$Text, [int]$Index)
    $lineStart = 0
    if ($Index -gt 0) {
        $prevNl = $Text.LastIndexOf("`n", $Index - 1)
        if ($prevNl -ge 0) { $lineStart = $prevNl + 1 }
    }
    $len = $Index - $lineStart
    if ($len -le 0) { return $false }
    $prefix = $Text.Substring($lineStart, $len)
    return ($prefix.Contains('#'))
}

function New-Replacement {
    param(
        [string]$FileRel,
        [string]$Zone,
        [int]$Start,
        [int]$Length,
        [string]$OldToken,
        [string]$NewToken,
        [string]$Kind = 'replace',
        [int]$Line = 0,
        [string]$Extra = ''
    )
    return New-Object psobject -Property @{
        FileRel  = $FileRel
        Zone     = $Zone
        Start    = $Start
        Length   = $Length
        OldToken = $OldToken
        NewToken = $NewToken
        Kind     = $Kind
        Line     = $Line
        Extra    = $Extra
    }
}

function ConvertTo-SafeArray {
    param($Value)
    if ($null -eq $Value) { return ,@() }
    if ($Value -is [string]) { return @($Value) }
    $tmp = New-Object System.Collections.Generic.List[object]
    if ($Value -is [System.Collections.IList]) {
        foreach ($item in $Value) { [void]$tmp.Add($item) }
    }
    else {
        [void]$tmp.Add($Value)
    }
    if ($tmp.Count -eq 0) { return ,@() }
    $a = New-Object object[] $tmp.Count
    for ($i = 0; $i -lt $tmp.Count; $i++) { $a[$i] = $tmp[$i] }
    return ,$a
}

function Add-TokenMatch {
    param(
        [System.Collections.Generic.List[object]]$Bag,
        [string]$Text,
        [string]$FileRel,
        [string]$Zone,
        [int]$Start,
        [int]$Length,
        [string]$Token,
        [string]$RepoRoot = '',
        [bool]$RequireExists = $false
    )
    if ($Length -le 0) { return }
    $rw = Invoke-RewriteRelativeToken -Token $Token -CurrentFileRepoPath $FileRel -RepoRoot $RepoRoot -RequireExists $RequireExists
    if (-not $rw.Changed) { return }
    [void]$Bag.Add((New-Replacement -FileRel $FileRel -Zone $Zone -Start $Start -Length $Length -OldToken $Token -NewToken $rw.NewToken -Kind 'replace' -Line 0))
}

function Find-MatchingBracket {
    param([string]$Text, [int]$OpenIndex)
    $depth = 0
    $inStr = $false
    $q = [char]0
    for ($i = $OpenIndex; $i -lt $Text.Length; $i++) {
        $c = $Text[$i]
        if ($inStr) {
            if ($c -eq $q) { $inStr = $false }
            continue
        }
        if ($c -eq [char]34 -or $c -eq [char]39) { $inStr = $true; $q = $c; continue }
        if ($c -eq '[') { $depth++ }
        elseif ($c -eq ']') {
            $depth--
            if ($depth -eq 0) { return $i }
        }
    }
    return -1
}

function Add-CargoMatches {
    param(
        [System.Collections.Generic.List[object]]$Bag,
        [string]$Text,
        [string]$FileRel,
        [string]$Zone
    )
    foreach ($m in $script:RxPathEq.Matches($Text)) {
        if (Test-IsLineCommentToml -Text $Text -Index $m.Index) { continue }
        $q = $m.Groups[1].Value
        $val = $m.Groups[2].Value
        $valStart = $m.Groups[2].Index
        Add-TokenMatch -Bag $Bag -Text $Text -FileRel $FileRel -Zone $Zone -Start $valStart -Length $val.Length -Token $val
    }

    foreach ($mm in $script:RxMembers.Matches($Text)) {
        if (Test-IsLineCommentToml -Text $Text -Index $mm.Index) { continue }
        $open = $mm.Index + $mm.Length - 1
        $close = Find-MatchingBracket -Text $Text -OpenIndex $open
        if ($close -lt 0) { continue }
        $inner = $Text.Substring($open + 1, $close - $open - 1)
        $memberValues = New-Object System.Collections.Generic.List[string]
        foreach ($sm in $script:RxQuoted.Matches($inner)) {
            $val = $sm.Groups[2].Value
            [void]$memberValues.Add($val)
            $absStart = $open + 1 + $sm.Groups[2].Index
            Add-TokenMatch -Bag $Bag -Text $Text -FileRel $FileRel -Zone $Zone -Start $absStart -Length $val.Length -Token $val
        }

        if ($FileRel -eq 'Cargo.toml' -and $Text.Contains('[workspace]')) {
            foreach ($add in $script:AddWorkspaceMembers) {
                $present = $false
                foreach ($mv in $memberValues) {
                    $n = ConvertTo-SlashPath $mv
                    if ($n -eq $add -or (Invoke-MapPath $n) -eq $add) { $present = $true; break }
                }
                if ($present) { continue }
                $insertAt = $close
                $before = $Text.Substring($open, $close - $open)
                $nl = "`n"
                if ($before.Contains("`r`n")) { $nl = "`r`n" }
                $insertText = ''
                if ($inner.Contains("`n")) {
                    $indent = '    '
                    $indentM = [regex]::Match($inner, '\n([ \t]+)"')
                    if ($indentM.Success) { $indent = $indentM.Groups[1].Value }
                    $trimmedInner = $inner.TrimEnd()
                    $needComma = -not ($trimmedInner.EndsWith(',') -or $trimmedInner -eq '')
                    if ($needComma) { $insertText += ',' }
                    $insertText += $nl + $indent + '"' + $add + '",'
                }
                else {
                    $insertText = ', "' + $add + '"'
                }
                $line = Get-LineNumber -Text $Text -Index $insertAt
                [void]$Bag.Add((New-Replacement -FileRel $FileRel -Zone $Zone -Start $insertAt -Length 0 -OldToken '(absent)' -NewToken $add -Kind 'insert-member' -Line $line -Extra $insertText))
            }
        }
    }
}

function Test-IsIdentChar {
    param([char]$c)
    if (($c -ge 'A' -and $c -le 'Z') -or ($c -ge 'a' -and $c -le 'z') -or ($c -ge '0' -and $c -le '9')) { return $true }
    if ($c -eq '_' -or $c -eq '-' -or $c -eq '.' -or $c -eq '/' -or $c -eq '\') { return $true }
    return $false
}

function Test-PathStartBoundary {
    param([string]$Text, [int]$Index)
    if ($Index -le 0) { return $true }
    return -not (Test-IsIdentChar $Text[$Index - 1])
}

function Get-ExtendedPathEnd {
    param([string]$Text, [int]$From)
    $i = $From
    $n = $Text.Length
    while ($i -lt $n) {
        $c = $Text[$i]
        if ($c -eq '/' -or $c -eq '\') { $i++; continue }
        if (($c -ge 'A' -and $c -le 'Z') -or ($c -ge 'a' -and $c -le 'z') -or ($c -ge '0' -and $c -le '9') -or $c -eq '_' -or $c -eq '-') {
            $i++; continue
        }
        if ($c -eq '.') {
            if (($i + 1) -lt $n) {
                $n1 = $Text[$i + 1]
                if (($n1 -ge 'A' -and $n1 -le 'Z') -or ($n1 -ge 'a' -and $n1 -le 'z') -or ($n1 -ge '0' -and $n1 -le '9')) {
                    $i++; continue
                }
            }
            break
        }
        break
    }
    return $i
}

function Test-TextNeedsPathScan {
    param([string]$Text)
    if ($null -eq $script:NeedleRegex) { return $true }
    return $script:NeedleRegex.IsMatch($Text)
}

function Add-IndexedPathMatches {
    param(
        [System.Collections.Generic.List[object]]$Bag,
        [string]$Text,
        [string]$FileRel,
        [string]$Zone,
        [string]$RepoRoot = ''
    )
    $ordinal = [System.StringComparison]::Ordinal
    foreach ($key in $script:ScanKeysLongest) {
        $requireSlash = -not $key.Contains('/')
        $searchFrom = 0
        while ($searchFrom -lt $Text.Length) {
            $idx = $Text.IndexOf($key, $searchFrom, $ordinal)
            if ($idx -lt 0) { break }
            $tokenStart = $idx
            if ($idx -ge 2 -and $Text.Substring($idx - 2, 2) -eq './' -and (Test-PathStartBoundary -Text $Text -Index ($idx - 2))) {
                $tokenStart = $idx - 2
            }
            elseif (-not (Test-PathStartBoundary -Text $Text -Index $idx)) {
                $searchFrom = $idx + 1
                continue
            }
            $after = $idx + $key.Length
            if ($requireSlash) {
                if ($after -ge $Text.Length) {
                    $searchFrom = $idx + 1
                    continue
                }
                $sep = $Text[$after]
                if ($sep -ne '/' -and $sep -ne '\') {
                    $searchFrom = $idx + 1
                    continue
                }
            }
            $tokenEnd = Get-ExtendedPathEnd -Text $Text -From $after
            $len = $tokenEnd - $tokenStart
            if ($len -gt 0) {
                $token = $Text.Substring($tokenStart, $len)
                Add-TokenMatch -Bag $Bag -Text $Text -FileRel $FileRel -Zone $Zone -Start $tokenStart -Length $len -Token $token -RepoRoot $RepoRoot -RequireExists $true
            }
            $searchFrom = $idx + 1
        }
    }

    $searchFrom = 0
    while ($searchFrom -lt $Text.Length) {
        $idx = $Text.IndexOf('../', $searchFrom, $ordinal)
        if ($idx -lt 0) { break }
        if (-not (Test-PathStartBoundary -Text $Text -Index $idx)) {
            $searchFrom = $idx + 1
            continue
        }
        $i = $idx
        while (($i + 3) -le $Text.Length -and $Text.Substring($i, 3) -eq '../') { $i += 3 }
        $tokenEnd = Get-ExtendedPathEnd -Text $Text -From $i
        if ($tokenEnd -gt $i) {
            $len = $tokenEnd - $idx
            $token = $Text.Substring($idx, $len)
            Add-TokenMatch -Bag $Bag -Text $Text -FileRel $FileRel -Zone $Zone -Start $idx -Length $len -Token $token -RepoRoot $RepoRoot -RequireExists $true
        }
        $searchFrom = $idx + 1
    }
}

function Add-ComposeContextMatches {
    param(
        [System.Collections.Generic.List[object]]$Bag,
        [string]$Text,
        [string]$FileRel,
        [string]$Zone
    )
    if ($null -eq $script:RxComposeContext) { return }
    foreach ($m in $script:RxComposeContext.Matches($Text)) {
        $val = $m.Groups['v'].Value
        if ([string]::IsNullOrEmpty($val)) { continue }
        Add-TokenMatch -Bag $Bag -Text $Text -FileRel $FileRel -Zone $Zone -Start $m.Groups['v'].Index -Length $val.Length -Token $val
    }
}

function Add-IncludeMacroMatches {
    param(
        [System.Collections.Generic.List[object]]$Bag,
        [string]$Text,
        [string]$FileRel,
        [string]$Zone
    )
    foreach ($m in $script:RxInclude.Matches($Text)) {
        $val = $m.Groups[2].Value
        Add-TokenMatch -Bag $Bag -Text $Text -FileRel $FileRel -Zone $Zone -Start $m.Groups[2].Index -Length $val.Length -Token $val
    }
}

function Merge-Replacements {
    param([System.Collections.Generic.List[object]]$Bag)
    $arr = ConvertTo-SafeArray $Bag
    if ($arr.Count -eq 0) { return @() }
    $sorted = $arr | Sort-Object Start, @{ Expression = { $_.Length }; Descending = $true }
    $out = New-Object System.Collections.Generic.List[object]
    $end = -1
    foreach ($m in $sorted) {
        if ($m.Kind -eq 'insert' -or $m.Kind -eq 'insert-member') {
            [void]$out.Add($m)
            continue
        }
        if ($m.Start -lt $end) { continue }
        [void]$out.Add($m)
        $end = $m.Start + $m.Length
    }
    return $out
}

function Get-FileZone {
    param([string]$Rel)
    $n = ConvertTo-SlashPath $Rel
    $leaf = $n
    $idx = $n.LastIndexOf('/')
    if ($idx -ge 0) { $leaf = $n.Substring($idx + 1) }
    if ($leaf -eq 'Cargo.toml') { return 'cargo' }
    if ($leaf -match '^docker-compose.*\.yml$' -or $leaf -eq 'compose.yaml') { return 'compose' }
    if ($leaf -like 'Dockerfile*') { return 'docker' }
    if ($leaf -like '*.rs') { return 'rust' }
    if ($leaf -eq 'justfile') { return 'justfile' }
    if ($n.StartsWith('.github/')) { return 'ci' }
    if ($leaf -eq 'sonar-project.properties') { return 'sonar' }
    if ($n.StartsWith('docs/') -or $leaf -eq 'AGENTS.md' -or $leaf -like 'README*') { return 'docs' }
    if ($n.StartsWith('obsidian/')) { return 'obsidian' }
    if ($n.StartsWith('.serena/')) { return 'serena' }
    if ($n.StartsWith('.cursor/skills/') -or $n.StartsWith('.agents/')) { return 'skills' }
    if ($leaf -match '\.(ps1|sh|bat)$') { return 'scripts' }
    return 'other'
}

function Test-ExcludedDirName {
    param([string]$Name)
    if ($script:ExcludeDirNames.ContainsKey($Name)) { return $true }
    if ($Name.StartsWith('.tmp-')) { return $true }
    return $false
}

function Test-IsToolkitDirRel {
    param([string]$Rel)
    $n = ConvertTo-SlashPath $Rel
    if ($n -eq 'scripts/layout-rewrite' -or $n.StartsWith('scripts/layout-rewrite/')) { return $true }
    if ($n -eq 'ops/scripts/layout-rewrite' -or $n.StartsWith('ops/scripts/layout-rewrite/')) { return $true }
    return $false
}

function Test-IsCacheDirRel {
    param([string]$Rel)
    if ([string]::IsNullOrWhiteSpace($Rel)) { return $false }
    $n = ConvertTo-SlashPath $Rel
    if ($n -eq 'docker-build-stage' -or $n.StartsWith('docker-build-stage/')) { return $true }
    if ($n -eq 'ops/docker/build-stage' -or $n.StartsWith('ops/docker/build-stage/')) { return $true }
    return $false
}

function Get-RepoRelativeFromRoot {
    param([string]$FullPath, [string]$RootAbs)
    $f = ConvertTo-SlashPath $FullPath
    $r = (ConvertTo-SlashPath $RootAbs).TrimEnd('/')
    if ($f.Length -lt $r.Length) { return $null }
    $head = $f.Substring(0, $r.Length)
    if ($head.ToLowerInvariant() -ne $r.ToLowerInvariant()) { return $null }
    if ($f.Length -eq $r.Length) { return '' }
    if ($f[$r.Length] -ne '/') { return $null }
    return $f.Substring($r.Length + 1)
}

function Test-IsIncludedFile {
    param([string]$Rel, [string]$Name)
    if ($Name -eq 'Cargo.toml') { return $true }
    if ($Name -like '*.rs') { return $true }
    if ($Name -like 'Dockerfile*') { return $true }
    if ($Name -like 'docker-compose*.yml') { return $true }
    if ($Name -eq 'compose.yaml') { return $true }
    if ($Name -eq 'justfile') { return $true }
    if ($Name -like '*.ps1' -or $Name -like '*.sh' -or $Name -like '*.bat') { return $true }
    if ($Rel.StartsWith('.github/')) { return $true }
    if ($Name -eq 'sonar-project.properties') { return $true }
    if ($Name -like 'README*') { return $true }
    if ($Name -eq 'AGENTS.md') { return $true }
    if ($Rel.StartsWith('docs/')) { return $true }
    if ($Rel.StartsWith('obsidian/')) { return $true }
    if ($Rel.StartsWith('.serena/')) { return $true }
    if ($Rel.StartsWith('.cursor/skills/')) { return $true }
    if ($Rel.StartsWith('.agents/')) { return $true }
    if ($Name -like '*.md') { return $true }
    if ($Name -like '*.yml' -or $Name -like '*.yaml') { return $true }
    if ($Name -like '*.toml') { return $true }
    if ($Name -like '*.json') { return $true }
    if ($Name -like '*.properties') { return $true }
    return $false
}

function Test-IsBinaryFile {
    param([string]$AbsPath, [string]$Name)
    $ext = [System.IO.Path]::GetExtension($Name)
    $extLower = ''
    if (-not [string]::IsNullOrEmpty($ext)) { $extLower = $ext.ToLowerInvariant() }
    foreach ($b in $script:BinExt) {
        if ($extLower -eq $b) { return $true }
    }
    if ($extLower -in @('.rs', '.md', '.toml', '.yml', '.yaml', '.json', '.ps1', '.sh', '.bat', '.properties', '.txt')) {
        return $false
    }
    try {
        $fs = [System.IO.File]::Open($AbsPath, [System.IO.FileMode]::Open, [System.IO.FileAccess]::Read, [System.IO.FileShare]::ReadWrite)
        try {
            $len = [Math]::Min(8192, [int]$fs.Length)
            if ($len -le 0) { return $false }
            $buf = New-Object byte[] $len
            $read = $fs.Read($buf, 0, $len)
            for ($i = 0; $i -lt $read; $i++) {
                if ($buf[$i] -eq 0) { return $true }
            }
        }
        finally { $fs.Close() }
    }
    catch { return $true }
    return $false
}

function Get-ScanFilePaths {
    param([string]$RootAbs)
    $files = New-Object System.Collections.Generic.List[string]
    $stack = New-Object System.Collections.Generic.Stack[string]
    $visited = New-Object 'System.Collections.Generic.HashSet[string]' ([StringComparer]::OrdinalIgnoreCase)
    $stack.Push($RootAbs)
    [void]$visited.Add((ConvertTo-SlashPath $RootAbs))
    while ($stack.Count -gt 0) {
        $dir = $stack.Pop()
        $relDir = Get-RepoRelativeFromRoot -FullPath $dir -RootAbs $RootAbs
        if ($null -ne $relDir -and (Test-IsToolkitDirRel $relDir)) { continue }
        if ($null -ne $relDir -and (Test-IsCacheDirRel $relDir)) { continue }
        try {
            $dinfo = New-Object System.IO.DirectoryInfo $dir
            foreach ($sub in $dinfo.GetDirectories()) {
                if (Test-ExcludedDirName $sub.Name) { continue }
                if (($sub.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { continue }
                $full = $sub.FullName
                $key = ConvertTo-SlashPath $full
                if (-not $visited.Add($key)) { continue }
                $childRel = Get-RepoRelativeFromRoot -FullPath $full -RootAbs $RootAbs
                if (Test-IsToolkitDirRel $childRel) { continue }
                if (Test-IsCacheDirRel $childRel) { continue }
                $stack.Push($full)
            }
            foreach ($f in $dinfo.GetFiles()) {
                if ($f.Name -eq 'nul') { continue }
                [void]$files.Add($f.FullName)
            }
        }
        catch { }
    }
    return $files
}

function Get-FileMatches {
    param([string]$Text, [string]$FileRel, [string]$Zone, [string]$RepoRoot = '')
    $bag = New-Object System.Collections.Generic.List[object]
    $leaf = $FileRel
    $idx = (ConvertTo-SlashPath $FileRel).LastIndexOf('/')
    if ($idx -ge 0) { $leaf = $FileRel.Substring($idx + 1) }
    if ($leaf -eq 'Cargo.toml') {
        Add-CargoMatches -Bag $bag -Text $Text -FileRel $FileRel -Zone $Zone
    }
    if ($leaf -like '*.rs') {
        Add-IncludeMacroMatches -Bag $bag -Text $Text -FileRel $FileRel -Zone $Zone
    }
    if ($Zone -eq 'compose') {
        Add-ComposeContextMatches -Bag $bag -Text $Text -FileRel $FileRel -Zone $Zone
    }
    if (Test-TextNeedsPathScan -Text $Text) {
        Add-IndexedPathMatches -Bag $bag -Text $Text -FileRel $FileRel -Zone $Zone -RepoRoot $RepoRoot
    }
    return (Merge-Replacements $bag)
}

function Apply-ReplacementsToText {
    param([string]$Text, $Matches)
    $arr = ConvertTo-SafeArray $Matches
    if ($arr.Count -eq 0) { return $Text }
    $sorted = $arr | Sort-Object @{ Expression = { $_.Start }; Descending = $true }, @{ Expression = { $_.Length }; Descending = $false }
    $sb = New-Object System.Text.StringBuilder $Text
    foreach ($m in $sorted) {
        $replacement = $m.NewToken
        if ($m.Kind -eq 'insert-member') {
            $replacement = [string]$m.Extra
            if ([string]::IsNullOrEmpty($replacement)) { $replacement = ', "' + $m.NewToken + '"' }
        }
        elseif ($m.Kind -eq 'insert') {
            $replacement = $m.NewToken
        }
        [void]$sb.Remove($m.Start, $m.Length)
        [void]$sb.Insert($m.Start, $replacement)
    }
    return $sb.ToString()
}

function Format-Report {
    param(
        [string]$RootAbs,
        [bool]$DoApply,
        [bool]$DumpAll,
        [int]$FilesScanned,
        $Replacements
    )
    $nl = [Environment]::NewLine
    $lines = New-Object System.Collections.Generic.List[string]
    $mode = 'dry-run (aucune ecriture)'
    if ($DoApply) { $mode = 'apply (ecriture)' }
    [void]$lines.Add("layout-rewrite : $mode")
    [void]$lines.Add("racine : $RootAbs")
    [void]$lines.Add("fichiers scannes : $FilesScanned")
    [void]$lines.Add('')

    $zones = @(
        'cargo', 'compose', 'docker', 'rust', 'justfile', 'scripts',
        'ci', 'sonar', 'docs', 'obsidian', 'serena', 'skills', 'other'
    )
    $byZone = @{}
    foreach ($z in $zones) { $byZone[$z] = New-Object System.Collections.Generic.List[object] }
    $filesByZone = @{}
    foreach ($z in $zones) { $filesByZone[$z] = @{} }

    $hitFiles = @{}
    $replAll = ConvertTo-SafeArray $Replacements
    foreach ($r in $replAll) {
        $z = [string]$r.Zone
        if (-not $byZone.ContainsKey($z)) { $byZone[$z] = New-Object System.Collections.Generic.List[object] }
        [void]$byZone[$z].Add($r)
        if (-not $filesByZone.ContainsKey($z)) { $filesByZone[$z] = @{} }
        $filesByZone[$z][$r.FileRel] = $true
        $hitFiles[$r.FileRel] = $true
    }

    foreach ($z in $zones) {
        $list = $byZone[$z]
        $fileCount = 0
        if ($filesByZone.ContainsKey($z)) { $fileCount = $filesByZone[$z].Count }
        $replCount = $list.Count
        [void]$lines.Add(("zone {0} : {1} fichiers, {2} remplacements" -f $z, $fileCount, $replCount))
        $shown = 0
        foreach ($e in $list) {
            if (-not $DumpAll -and $shown -ge 8) { break }
            $arrow = '{0}:{1} -> {2}' -f $e.FileRel, $e.OldToken, $e.NewToken
            [void]$lines.Add('  ' + $arrow)
            $shown++
        }
        if (-not $DumpAll -and $replCount -gt 8) {
            [void]$lines.Add(('  ... ({0} autres, relancer avec -Verbose)' -f ($replCount - 8)))
        }
        [void]$lines.Add('')
    }

    $insertCount = 0
    foreach ($r in $replAll) {
        if ($r.Kind -eq 'insert-member' -or $r.Kind -eq 'insert') { $insertCount++ }
    }
    [void]$lines.Add('RESUME')
    [void]$lines.Add(("  fichiers scannes : {0}" -f $FilesScanned))
    [void]$lines.Add(("  fichiers touches : {0}" -f $hitFiles.Count))
    [void]$lines.Add(("  remplacements : {0}" -f $replAll.Count))
    [void]$lines.Add(("  insertions workspace : {0}" -f $insertCount))
    return ($lines -join $nl)
}

function Invoke-LayoutRewrite {
    param(
        [string]$RootAbs,
        [bool]$DoApply,
        [bool]$DumpAll,
        [string]$OutReportPath
    )
    $all = New-Object System.Collections.Generic.List[object]
    $scanned = 0
    Write-Host ("layout-rewrite : scan {0}" -f $RootAbs)
    $fileList = ConvertTo-SafeArray (Get-ScanFilePaths -RootAbs $RootAbs)
    Write-Host ("layout-rewrite : {0} chemins enumeres, filtrage..." -f $fileList.Count)
    $utf8 = New-Object System.Text.UTF8Encoding $false
    foreach ($abs in $fileList) {
        $rel = Get-RepoRelativeFromRoot -FullPath $abs -RootAbs $RootAbs
        if ([string]::IsNullOrEmpty($rel)) { continue }
        $rel = ConvertTo-SlashPath $rel
        if (Test-IsToolkitDirRel $rel) { continue }
        $name = [System.IO.Path]::GetFileName($abs)
        if ($name -eq 'mapping.json' -or $name -eq 'Rewrite-Layout.ps1') { continue }
        if (-not (Test-IsIncludedFile -Rel $rel -Name $name)) { continue }
        if (Test-IsBinaryFile -AbsPath $abs -Name $name) { continue }
        $scanned++
        if (($scanned % 100) -eq 0) {
            Write-Host ("layout-rewrite : ... {0} fichiers textes" -f $scanned)
        }
        $text = [System.IO.File]::ReadAllText($abs, $utf8)
        $zone = Get-FileZone $rel
        $matches = ConvertTo-SafeArray (Get-FileMatches -Text $text -FileRel $rel -Zone $zone -RepoRoot $RootAbs)
        foreach ($m in $matches) { [void]$all.Add($m) }
        if ($DoApply -and $matches.Count -gt 0) {
            $newText = Apply-ReplacementsToText -Text $text -Matches $matches
            if ($newText -ne $text) {
                [System.IO.File]::WriteAllText($abs, $newText, $utf8)
            }
        }
    }

    $replArr = ConvertTo-SafeArray $all
    $report = Format-Report -RootAbs $RootAbs -DoApply $DoApply -DumpAll $DumpAll -FilesScanned $scanned -Replacements $replArr
    Write-Host $report
    if (-not [string]::IsNullOrWhiteSpace($OutReportPath)) {
        $dir = Split-Path -Parent $OutReportPath
        if (-not [string]::IsNullOrEmpty($dir) -and -not (Test-Path -LiteralPath $dir)) {
            New-Item -ItemType Directory -Path $dir | Out-Null
        }
        $utf8 = New-Object System.Text.UTF8Encoding $false
        [System.IO.File]::WriteAllText($OutReportPath, $report, $utf8)
    }
    return @{
        FilesScanned  = $scanned
        Replacements  = $replArr
        Report        = $report
    }
}

function Assert-Eq {
    param([string]$Name, $Actual, $Expected)
    $a = [string]$Actual
    $e = [string]$Expected
    if ($a -ne $e) {
        Write-Host ("self-check ECHEC : {0}" -f $Name)
        Write-Host ("  obtenu  : {0}" -f $a)
        Write-Host ("  attendu : {0}" -f $e)
        $script:FailCount++
    }
    else {
        Write-Host ("self-check OK : {0}" -f $Name)
    }
}

function Assert-True {
    param([string]$Name, [bool]$Cond)
    if (-not $Cond) {
        Write-Host ("self-check ECHEC : {0}" -f $Name)
        $script:FailCount++
    }
    else {
        Write-Host ("self-check OK : {0}" -f $Name)
    }
}

function Test-HasReplacement {
    param($Replacements, [string]$FileRel, [string]$Old, [string]$New)
    foreach ($r in @($Replacements)) {
        if ($r.FileRel -eq $FileRel -and $r.OldToken -eq $Old -and $r.NewToken -eq $New) { return $true }
        if ($r.FileRel -eq $FileRel -and $r.Kind -eq 'insert-member' -and $r.NewToken -eq $New) { return $true }
    }
    return $false
}

function Count-FileHits {
    param($Replacements, [string]$FileRel)
    $n = 0
    foreach ($r in @($Replacements)) {
        if ($r.FileRel -eq $FileRel) { $n++ }
    }
    return $n
}

function Invoke-SelfCheck {
    $script:FailCount = 0
    $discovered = Find-RepoRootFromScript
    Assert-True 'decouverte racine Cargo.toml' (-not [string]::IsNullOrWhiteSpace($discovered) -and (Test-Path -LiteralPath (Join-Path $discovered 'Cargo.toml')))
    $discSlash = ConvertTo-SlashPath $discovered
    Assert-True 'decouverte racine hors fixture' (-not $discSlash.EndsWith('fixtures/mini-repo'))

    $fixtureRoot = Join-Path $PSScriptRoot 'fixtures\mini-repo'
    if (-not (Test-Path -LiteralPath $fixtureRoot)) {
        Write-Host 'self-check ECHEC : fixtures/mini-repo introuvable'
        return 1
    }

    Assert-Eq 'map nested IAMRusty/scripts (pas ops/scripts)' (Invoke-MapPath 'IAMRusty/scripts/foo.sh') 'services/IAMRusty/scripts/foo.sh'
    Assert-Eq 'map root scripts/' (Invoke-MapPath 'scripts/foo.sh') 'ops/scripts/foo.sh'
    Assert-Eq 'map already services/IAMRusty' (Invoke-MapPath 'services/IAMRusty/src/main.rs') 'services/IAMRusty/src/main.rs'
    Assert-Eq 'map already ops/deploy' (Invoke-MapPath 'ops/deploy/mesh') 'ops/deploy/mesh'
    Assert-Eq 'map docker-build-stage' (Invoke-MapPath 'docker-build-stage/obj') 'ops/docker/build-stage/obj'

    $r = Invoke-RewriteRelativeToken -Token '../../hive-events' -CurrentFileRepoPath 'Hive/domain/Cargo.toml'
    Assert-Eq 'cargo hive-events' $r.NewToken '../../../crates/hive-events'
    Assert-True 'cargo hive-events changed' ([bool]$r.Changed)

    $r = Invoke-RewriteRelativeToken -Token '../IAMRusty/application' -CurrentFileRepoPath 'monolith/Cargo.toml'
    Assert-Eq 'cargo monolith IAMRusty' $r.NewToken '../../services/IAMRusty/application'

    $r = Invoke-RewriteRelativeToken -Token '../readiness' -CurrentFileRepoPath 'monolith/Cargo.toml'
    Assert-Eq 'cargo monolith readiness' $r.NewToken '../../crates/readiness'

    $r = Invoke-RewriteRelativeToken -Token './domain' -CurrentFileRepoPath 'IAMRusty/Cargo.toml'
    Assert-True 'hexagon ./domain inchange' (-not $r.Changed)
    Assert-Eq 'hexagon ./domain token' $r.NewToken './domain'

    $r = Invoke-RewriteRelativeToken -Token 'http' -CurrentFileRepoPath 'IAMRusty/Cargo.toml'
    Assert-True 'hexagon http inchange' (-not $r.Changed)

    $r = Invoke-RewriteRelativeToken -Token 'rustycog' -CurrentFileRepoPath 'Cargo.toml'
    Assert-True 'rustycog path inchange' (-not $r.Changed)

    $r = Invoke-RewriteRelativeToken -Token 'IAMRusty' -CurrentFileRepoPath 'Cargo.toml'
    Assert-Eq 'workspace IAMRusty' $r.NewToken 'services/IAMRusty'
    $r = Invoke-RewriteRelativeToken -Token 'monolith' -CurrentFileRepoPath 'Cargo.toml'
    Assert-Eq 'workspace monolith' $r.NewToken 'runtime/monolith'
    $r = Invoke-RewriteRelativeToken -Token 'ext-authz' -CurrentFileRepoPath 'Cargo.toml'
    Assert-Eq 'workspace ext-authz' $r.NewToken 'workers/ext-authz'
    $r = Invoke-RewriteRelativeToken -Token 'iam-events' -CurrentFileRepoPath 'Cargo.toml'
    Assert-Eq 'workspace iam-events' $r.NewToken 'crates/iam-events'

    $r = Invoke-RewriteRelativeToken -Token '../../openfga/model.json' -CurrentFileRepoPath 'Hive/tests/common.rs'
    Assert-Eq 'include_str openfga' $r.NewToken '../../../ops/openfga/model.json'

    $r = Invoke-RewriteRelativeToken -Token '..' -CurrentFileRepoPath 'IAMRusty/docker-compose.yml'
    Assert-Eq 'compose context .. depuis service' $r.NewToken '../..'
    Assert-True 'compose context .. changed' ([bool]$r.Changed)
    $r = Invoke-RewriteRelativeToken -Token '.' -CurrentFileRepoPath 'docker-compose.yml'
    Assert-True 'compose context . a la racine inchange' (-not $r.Changed)

    $once = Invoke-MapPath 'IAMRusty/http'
    $twice = Invoke-MapPath $once
    Assert-Eq 'idempotence MapPath' $twice $once
    $r = Invoke-RewriteRelativeToken -Token '../../../crates/hive-events' -CurrentFileRepoPath 'services/Hive/domain/Cargo.toml'
    Assert-True 'idempotence rewrite deja mappe' (-not $r.Changed)
    $r = Invoke-RewriteRelativeToken -Token 'services/IAMRusty/src/main.rs' -CurrentFileRepoPath 'services/IAMRusty/src/main.rs'
    Assert-True 'idempotence dest prefix nested' (-not $r.Changed)

    $scan = Invoke-LayoutRewrite -RootAbs ([System.IO.Path]::GetFullPath($fixtureRoot)) -DoApply $false -DumpAll $true -OutReportPath ''
    $repl = $scan.Replacements

    Assert-True 'scan hive-events' (Test-HasReplacement $repl 'Hive/domain/Cargo.toml' '../../hive-events' '../../../crates/hive-events')
    Assert-True 'scan monolith IAMRusty' (Test-HasReplacement $repl 'monolith/Cargo.toml' '../IAMRusty/application' '../../services/IAMRusty/application')
    Assert-True 'scan monolith readiness' (Test-HasReplacement $repl 'monolith/Cargo.toml' '../readiness' '../../crates/readiness')
    Assert-True 'scan hexagon domain 0' ((Count-FileHits $repl 'IAMRusty/Cargo.toml') -eq 0)
    Assert-True 'scan rustycog absent' (-not (Test-HasReplacement $repl 'Cargo.toml' 'rustycog' 'rustycog'))
    $cargoHits = @($repl | Where-Object { $_.FileRel -eq 'Cargo.toml' })
    $hasRusty = $false
    $hasMono = $false
    $hasExt = $false
    $hasIamEv = $false
    $hasInsert = $false
    foreach ($h in $cargoHits) {
        if ($h.OldToken -eq 'IAMRusty' -and $h.NewToken -eq 'services/IAMRusty') { $hasRusty = $true }
        if ($h.OldToken -eq 'monolith' -and $h.NewToken -eq 'runtime/monolith') { $hasMono = $true }
        if ($h.OldToken -eq 'ext-authz' -and $h.NewToken -eq 'workers/ext-authz') { $hasExt = $true }
        if ($h.OldToken -eq 'iam-events' -and $h.NewToken -eq 'crates/iam-events') { $hasIamEv = $true }
        if ($h.Kind -eq 'insert-member' -and $h.NewToken -eq 'crates/apparatus-events') { $hasInsert = $true }
    }
    Assert-True 'scan workspace IAMRusty' $hasRusty
    Assert-True 'scan workspace monolith' $hasMono
    Assert-True 'scan workspace ext-authz' $hasExt
    Assert-True 'scan workspace iam-events' $hasIamEv
    Assert-True 'scan insert apparatus-events' $hasInsert
    $r = Invoke-RewriteRelativeToken -Token 'Lazaret/README.md' -CurrentFileRepoPath 'docs/already-moved.md' -RepoRoot ([System.IO.Path]::GetFullPath($fixtureRoot)) -RequireExists $true
    Assert-Eq 'already-moved dest-only token' $r.NewToken 'services/Lazaret/README.md'
    Assert-True 'already-moved dest-only changed' ([bool]$r.Changed)

    Assert-True 'prose 0 hits' ((Count-FileHits $repl 'docs/prose.md') -eq 0)
    Assert-True 'image openfga inchange' (-not (Test-HasReplacement $repl 'docs/prose.md' 'openfga/openfga' 'ops/openfga/openfga'))
    Assert-True 'liste Hive/IAM inchange' (-not (Test-HasReplacement $repl 'docs/prose.md' 'Hive/IAM/Telegraph/GitHub' 'services/Hive/IAM/Telegraph/GitHub'))
    Assert-True 'feature apparatus-contracts/test-harness inchange' (-not (Test-HasReplacement $repl 'docs/prose.md' 'apparatus-contracts/test-harness' 'crates/apparatus-contracts/test-harness'))
    Assert-True 'hyphen kind-demo-monolith inchange' (-not (Test-HasReplacement $repl 'docs/prose.md' 'monolith/' 'runtime/monolith/'))
    Assert-True 'already-moved dest-only scan' (Test-HasReplacement $repl 'docs/already-moved.md' 'Lazaret/README.md' 'services/Lazaret/README.md')
    Assert-True 'prefix deploy/mesh' (Test-HasReplacement $repl 'prefix.md' 'deploy/mesh' 'ops/deploy/mesh')
    Assert-True 'prefix ./certs/x' (Test-HasReplacement $repl 'prefix.md' './certs/x' './ops/certs/x')
    Assert-True 'prefix docker-build-stage' (Test-HasReplacement $repl 'prefix.md' 'docker-build-stage/obj' 'ops/docker/build-stage/obj')
    Assert-True 'already services/IAMRusty file 0' ((Count-FileHits $repl 'services/IAMRusty/src/main.rs') -eq 0)
    Assert-True 'already ops/deploy/mesh 0' ((Count-FileHits $repl 'ops/deploy/mesh.yaml') -eq 0)
    Assert-True 'nested scripts citation' (Test-HasReplacement $repl 'justfile' 'IAMRusty/scripts/foo.sh' 'services/IAMRusty/scripts/foo.sh')
    Assert-True 'include_str scan' (Test-HasReplacement $repl 'Hive/tests/common.rs' '../../openfga/model.json' '../../../ops/openfga/model.json')
    Assert-True 'compose context ..' (Test-HasReplacement $repl 'IAMRusty/docker-compose.yml' '..' '../..')
    Assert-True 'compose dockerfile service' (Test-HasReplacement $repl 'IAMRusty/docker-compose.yml' 'IAMRusty/Dockerfile' 'services/IAMRusty/Dockerfile')
    Assert-True 'compose volume certs' (Test-HasReplacement $repl 'IAMRusty/docker-compose.yml' '../certs/platform-mesh' '../../ops/certs/platform-mesh')
    Assert-True 'obsidian repo path' (Test-HasReplacement $repl 'obsidian/note.md' 'Manifesto/http/handler' 'services/Manifesto/http/handler')
    Assert-True 'obsidian vault projects/hive 0' (-not (Test-HasReplacement $repl 'obsidian/note.md' 'projects/hive/overview' 'projects/hive/overview'))
    $obsidianBad = $false
    foreach ($h in @($repl | Where-Object { $_.FileRel -eq 'obsidian/note.md' })) {
        if ($h.OldToken -like 'projects/*') { $obsidianBad = $true }
    }
    Assert-True 'obsidian vault inchange' (-not $obsidianBad)

    if ($script:FailCount -gt 0) {
        Write-Host ("self-check : {0} echec(s)" -f $script:FailCount)
        return 1
    }
    Write-Host 'self-check : OK'
    return 0
}

if ($Apply -and $SelfCheck) {
    Show-Usage
    exit 2
}

$mappingFile = Join-Path $PSScriptRoot 'mapping.json'
if (-not (Test-Path -LiteralPath $mappingFile)) {
    Write-Host 'layout-rewrite : mapping.json introuvable'
    Show-Usage
    exit 2
}

Initialize-Mapping -MappingPath $mappingFile

if ($SelfCheck) {
    $code = Invoke-SelfCheck
    exit $code
}

if ([string]::IsNullOrWhiteSpace($RepoRoot)) {
    $RepoRoot = Find-RepoRootFromScript
    if ([string]::IsNullOrWhiteSpace($RepoRoot)) {
        Write-Host 'layout-rewrite : racine du depot introuvable (Cargo.toml + .git)'
        Show-Usage
        exit 2
    }
}
else {
    $RepoRoot = [System.IO.Path]::GetFullPath($RepoRoot)
}

if (-not (Test-Path -LiteralPath $RepoRoot -PathType Container)) {
    Write-Host ("layout-rewrite : racine introuvable : {0}" -f $RepoRoot)
    Show-Usage
    exit 2
}

if ($Apply) {
    $missing = New-Object System.Collections.Generic.List[string]
    foreach ($d in $script:PreconditionDirs) {
        $p = Join-Path $RepoRoot (($d -replace '/', [IO.Path]::DirectorySeparatorChar))
        if (-not (Test-Path -LiteralPath $p)) {
            [void]$missing.Add($d)
        }
    }
    if ($missing.Count -gt 0) {
        Write-Host 'layout-rewrite : apply refuse - repertoires de destination manquants (aucun fichier ecrit). Deplacez d abord l arborescence :'
        foreach ($d in $missing) {
            Write-Host ("  - {0}" -f $d)
        }
        exit 1
    }
}

$null = Invoke-LayoutRewrite -RootAbs $RepoRoot -DoApply ([bool]$Apply) -DumpAll ([bool]$Verbose) -OutReportPath $ReportPath
exit 0
