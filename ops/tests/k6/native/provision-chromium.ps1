# Expand the COMPLETE, already downloaded/pinned CfT distribution, not chrome.exe
# alone. No execution, acquisition, global installation or modification of cache.
param(
    [Parameter(Mandatory = $true)][string]$Archive,
    [Parameter(Mandatory = $true)][string]$Directory
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.IO.Compression.FileSystem
$pin = (ConvertFrom-Json ([IO.File]::ReadAllText((Join-Path $PSScriptRoot '../config/tools.windows.lock.json')))).chromium
if (-not [IO.Path]::IsPathRooted($Archive) -or -not [IO.Path]::IsPathRooted($Directory) -or $Directory.StartsWith('\\')) { throw 'EXPLICIT_LOCAL_PATHS_REQUIRED' }
if ((Get-FileHash -LiteralPath $Archive -Algorithm SHA256).Hash.ToLowerInvariant() -cne $pin.archive_sha256) { throw 'CHROMIUM_ARCHIVE_PIN_INVALID' }
$Directory = [IO.Path]::GetFullPath($Directory).TrimEnd('\', '/')
$root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../../../..'))
if ($Directory.Equals($root, [StringComparison]::OrdinalIgnoreCase) -or $Directory.StartsWith($root + '\', [StringComparison]::OrdinalIgnoreCase) -or (Test-Path -LiteralPath $Directory)) { throw 'FRESH_EXTERNAL_DIRECTORY_REQUIRED' }
$ancestor = [IO.DirectoryInfo]::new($Directory)
while ($ancestor) {
    if ($ancestor.Exists -and ($ancestor.Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw 'REPARSE_POINT_REFUSED' }
    $ancestor = $ancestor.Parent
}
$zip = [IO.Compression.ZipFile]::OpenRead($Archive)
try {
    $seen = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    foreach ($entry in $zip.Entries) {
        $name = $entry.FullName
        if (-not $name.StartsWith('chrome-win64/', [StringComparison]::Ordinal) -or $name.Contains('\') -or $name.Contains(':') -or $name -match '(^|/)\.\.?(/|$)' -or -not $seen.Add($name)) { throw 'ARCHIVE_ENTRY_REFUSED' }
    }
} finally { $zip.Dispose() }
$null = New-Item -ItemType Directory -Path $Directory
$sid = [Security.Principal.WindowsIdentity]::GetCurrent().User
$acl = [Security.AccessControl.DirectorySecurity]::new(); $acl.SetOwner($sid); $acl.SetAccessRuleProtection($true, $false)
$acl.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new($sid, 'FullControl', 'ContainerInherit,ObjectInherit', 'None', 'Allow'))
Set-Acl -LiteralPath $Directory -AclObject $acl
[IO.Compression.ZipFile]::ExtractToDirectory($Archive, $Directory)
$exe = Join-Path $Directory $pin.executable
if ((Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash.ToLowerInvariant() -cne $pin.executable_sha256) { throw 'CHROMIUM_EXE_PIN_INVALID' }
$files = @(Get-ChildItem -LiteralPath (Join-Path $Directory 'chrome-win64') -Recurse -File | ForEach-Object {
    @{path = $_.FullName.Substring($Directory.Length + 1); sha256 = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()}
})
$manifest = Join-Path $Directory 'bundle.json'
[IO.File]::WriteAllText($manifest, (@{version = 1; archive_sha256 = $pin.archive_sha256; files = $files} | ConvertTo-Json -Depth 5), [Text.UTF8Encoding]::new($false))
@{path = $exe; sha256 = $pin.executable_sha256; bundle_manifest = $manifest; bundle_sha256 = (Get-FileHash -LiteralPath $manifest -Algorithm SHA256).Hash.ToLowerInvariant(); files = $files.Count} | ConvertTo-Json -Compress
