# Windows DACL primitive shared by the two native private-file producers.
function Set-KindPrivatePath([string]$Path, [switch]$Folder) {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent().User
    $acl = Get-Acl -LiteralPath $Path
    $acl.SetAccessRuleProtection($true, $false)
    foreach ($existing in @($acl.GetAccessRules($true, $false, [Security.Principal.SecurityIdentifier]))) {
        $acl.RemoveAccessRuleSpecific($existing)
    }
    if ($Folder) {
        $rule = [Security.AccessControl.FileSystemAccessRule]::new(
            $identity, 'FullControl', 'ContainerInherit,ObjectInherit', 'None', 'Allow')
    } else {
        $rule = [Security.AccessControl.FileSystemAccessRule]::new($identity, 'FullControl', 'Allow')
    }
    $acl.AddAccessRule($rule)
    # Persist DACL changes only, avoiding the Windows PowerShell5.1 Set-Acl
    # provider's audit fallback; do not change owner/SACL or request elevation.
    (Get-Item -LiteralPath $Path).SetAccessControl($acl)
}
