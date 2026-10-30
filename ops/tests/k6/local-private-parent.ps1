param([switch]$LibraryOnly, [string]$Mode='preflight', [string]$Profile)
# Fixed Windows controller, WindowsPowerShell5.1. No runtime starts when dot-sourced
# with -LibraryOnly. No interop child, kill, policy bypass, network or site reader.
# Private configuration keys are closed below. expected contains the observed
# SID/GUID/name/BasePath/UID/version, native file identities and exact ACL baseline.
# linux_profile/companion are reviewed native Linux paths; lease and journal must
# be NEW files beneath a private Windows campaign directory, never old inventory.
# controller_sha256 pins this reviewed file, wsl_image the actual expected system
# image. These comparison bounds never replace Get-WindowsSnapshot or the held
# process/file handles. The lead coordinates shared runtime ownership separately;
# this program neither proves absence of other consumers nor stops a distribution.
Set-StrictMode -Version Latest
$ErrorActionPreference='Stop'
$script:ControllerPath=$PSCommandPath

function Stop-Parent { throw 'PARENT_REFUSED' }
function Require-Parent([bool]$Condition) { if (!$Condition) { Stop-Parent } }

function Initialize-Native {
    if ('K6Parent.Native' -as [type]) { return }
    Add-Type -TypeDefinition @'
using System;
using System.IO;
using System.Text;
using System.Threading.Tasks;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Security.Principal;
using Microsoft.Win32.SafeHandles;
namespace K6Parent {
 public sealed class FrameReader {
  readonly Stream stream; readonly byte[] one=new byte[1];
  readonly MemoryStream bytes=new MemoryStream(); Task<int> pending;
  public bool Ended {get;private set;} public long Seen {get;private set;}
  public FrameReader(Stream value) {stream=value;}
  public string Poll(int milliseconds) {
   var timer=Stopwatch.StartNew();
   do {
    if(Ended) return null;
    if(pending==null) pending=stream.ReadAsync(one,0,1);
    if(!pending.Wait(Math.Max(0,milliseconds-(int)timer.ElapsedMilliseconds))) return null;
    int n=pending.GetAwaiter().GetResult();pending=null;
    if(n==0) {Ended=true;if(bytes.Length!=0)throw new Exception("PARENT_REFUSED");return null;}
    Seen++;if(Seen>33554432)throw new Exception("PARENT_REFUSED");
    if(one[0]==10) {var raw=bytes.ToArray();bytes.SetLength(0);return new UTF8Encoding(false,true).GetString(raw);}
    bytes.WriteByte(one[0]);if(bytes.Length>=4096)throw new Exception("PARENT_REFUSED");
   }while(timer.ElapsedMilliseconds<=milliseconds);
   return null;
  }
 }
 public sealed class ProcessFact {
  public uint Pid; public long Created; public long Exited; public string Image; public string Sid; public uint Wait;
 }
 public static class Native {
  [StructLayout(LayoutKind.Sequential,Pack=4)] struct Info {
   public uint Attributes; public long Created,Accessed,Written; public uint Volume,SizeHigh,SizeLow,Links,IndexHigh,IndexLow;
  }
  [DllImport("kernel32.dll",CharSet=CharSet.Unicode,SetLastError=true)] static extern SafeFileHandle CreateFile(
   string name,uint access,uint share,IntPtr security,uint disposition,uint flags,IntPtr template);
  [DllImport("kernel32.dll",SetLastError=true)] static extern bool GetFileInformationByHandle(SafeFileHandle file,out Info info);
  [DllImport("kernel32.dll",SetLastError=true)] static extern bool GetProcessTimes(IntPtr h,out long created,out long ended,out long kernel,out long user);
  [DllImport("kernel32.dll")] static extern uint GetProcessId(IntPtr h);
  [DllImport("kernel32.dll")] static extern uint WaitForSingleObject(IntPtr h,uint timeout);
  [DllImport("kernel32.dll",CharSet=CharSet.Unicode,SetLastError=true)] static extern bool QueryFullProcessImageName(IntPtr h,uint flags,StringBuilder image,ref int length);
  [DllImport("advapi32.dll",SetLastError=true)] static extern bool OpenProcessToken(IntPtr h,uint access,out IntPtr token);
  [DllImport("kernel32.dll")] static extern bool CloseHandle(IntPtr h);
  [DllImport("shell32.dll",CharSet=CharSet.Unicode)] static extern IntPtr CommandLineToArgvW(string value,out int count);
  [DllImport("kernel32.dll")] static extern IntPtr LocalFree(IntPtr value);
  public static string[] Arguments(string value) {
   int count;IntPtr block=CommandLineToArgvW("x.exe "+value,out count);
   if(block==IntPtr.Zero)throw new Exception("PARENT_REFUSED");
   try{var result=new string[count-1];for(int i=1;i<count;i++)result[i-1]=Marshal.PtrToStringUni(Marshal.ReadIntPtr(block,i*IntPtr.Size));return result;}
   finally{LocalFree(block);}
  }
  public static string FileIdentity(string path) {
   using(var file=CreateFile(path,0x20080,7,IntPtr.Zero,3,0x02200000,IntPtr.Zero)) {
    Info value;if(file.IsInvalid||!GetFileInformationByHandle(file,out value)||(value.Attributes&0x400)!=0)
     throw new Exception("PARENT_REFUSED");
    return value.Volume.ToString("x8")+":"+value.IndexHigh.ToString("x8")+value.IndexLow.ToString("x8");
   }
  }
  public static ProcessFact Observe(IntPtr h) {
   long c,e,k,u;int length=32768;var image=new StringBuilder(length);IntPtr token;
   if(!GetProcessTimes(h,out c,out e,out k,out u)||!QueryFullProcessImageName(h,0,image,ref length)||!OpenProcessToken(h,8,out token))
    throw new Exception("PARENT_REFUSED");
   string sid;try{using(var identity=new WindowsIdentity(token)){sid=identity.User.Value;}}finally{CloseHandle(token);}
   return new ProcessFact{Pid=GetProcessId(h),Created=c,Exited=e,Image=image.ToString(),Sid=sid,Wait=WaitForSingleObject(h,0)};
  }
 }
}
'@
}

function Convert-Table($Value) {
    $result=@{}
    if ($Value -is [Collections.IDictionary]) { foreach($key in $Value.Keys){$result[$key]=$Value[$key]} }
    else { foreach($item in $Value.PSObject.Properties){$result[$item.Name]=$item.Value} }
    return $result
}

function Read-JsonStrict([string]$Text,[int]$Maximum=4095) {
    Require-Parent ([Text.Encoding]::UTF8.GetByteCount($Text) -le $Maximum)
    Add-Type -AssemblyName System.Web.Extensions
    $parser=New-Object Web.Script.Serialization.JavaScriptSerializer
    $keys=New-Object 'Collections.Generic.HashSet[string]' ([StringComparer]::Ordinal)
    # This closed configuration/protocol intentionally uses no repeated property
    # names even in nested objects. Decode escaped names before duplicate checks.
    foreach($match in [regex]::Matches($Text,'"(?:[^"\\]|\\.)*"\s*:')) {
        $key=$parser.DeserializeObject($match.Value.Substring(0,$match.Value.LastIndexOf(':')).Trim())
        Require-Parent ($keys.Add([string]$key))
    }
    $value=$parser.DeserializeObject($Text)
    Require-Parent ($value -is [Collections.IDictionary])
    return $value
}

function Test-Integer($Value) { return ($Value -is [int] -or $Value -is [long]) }
function Assert-Keys($Value,[string[]]$Names) {
    Require-Parent ($Value -is [Collections.IDictionary] -and $Value.Count -eq $Names.Count)
    foreach($key in $Value.Keys) { Require-Parent ($Names -ccontains [string]$key) }
}

function Read-FrameText([string]$Text) {
    Require-Parent (!$Text.Contains('\') -and !$Text.Contains("`n") -and !$Text.Contains("`r"))
    $v=Read-JsonStrict $Text
    Assert-Keys $v @('v','kind','seq','session','browser','run','deadline','home','mode','cleanup','diag')
    Require-Parent ((Test-Integer $v.v) -and $v.v -eq 1 -and (Test-Integer $v.seq) -and $v.seq -ge 0 -and $v.seq -le 8192)
    foreach($key in @('session','browser','run')) {
        Require-Parent ($v[$key] -is [string] -and (($key -eq 'run' -and $v[$key] -ceq '') -or $v[$key] -cmatch '^[a-f0-9]{32}$'))
    }
    Require-Parent ($v.kind -is [string] -and @('INIT','HELLO','AUTHORIZE','WRITE','ALLOW','DENY','READY','POLL','WAIT','ACK','CLOSED','PREFLIGHT','ABORT') -ccontains $v.kind)
    Require-Parent ($v.mode -is [string] -and @('capture','preflight') -ccontains $v.mode)
    Require-Parent ($v.home -is [Array] -and $v.home.Count -eq 3)
    foreach($number in $v.home) {Require-Parent ((Test-Integer $number) -and $number -ge 0)}
    Require-Parent ($v.cleanup -is [string] -and @('NOT_STARTED','PASS','FAIL') -ccontains $v.cleanup)
    Require-Parent ($v.diag -is [string] -and @('NONE','OBSERVERS_ONLY','NATIVE_NON_SUCCESS','INPUT_INVALID','CLEANUP_FAILED','RETENTION_UNPROVEN') -ccontains $v.diag)
    Require-Parent ($v.deadline -is [string] -and $v.deadline -cmatch '^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$')
    [void][datetime]::ParseExact($v.deadline,"yyyy-MM-dd'T'HH:mm:ss'Z'",[Globalization.CultureInfo]::InvariantCulture)
    return $v
}

function Write-FrameText($Value) {
    $text=ConvertTo-Json -InputObject $Value -Compress -Depth 5
    [void](Read-FrameText $text)
    return $text
}

function Quote-WindowsArgument([string]$Value) {
    Require-Parent ($Value.IndexOf([char]0) -lt 0 -and !$Value.Contains("`r") -and !$Value.Contains("`n"))
    # CommandLineToArgvW/CRT escaping; no shell is involved.
    $escaped=[regex]::Replace($Value,'(\\*)"', '$1$1\"')
    $escaped=[regex]::Replace($escaped,'(\\+)$','$1$1')
    return '"'+$escaped+'"'
}

function Assert-Snapshot($Expected,$Actual) {
    $keys=@('sid','guid','name','base','uid','version','directory','disk','acl')
    Assert-Keys $Expected $keys
    Assert-Keys $Actual $keys
    foreach($key in $keys) { Require-Parent ($Expected[$key].GetType() -eq $Actual[$key].GetType() -and $Expected[$key] -ceq $Actual[$key]) }
}

function Read-UtcDeadline([string]$Value) {
    Require-Parent ($Value -cmatch '^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$')
    return [datetime]::ParseExact($Value,"yyyy-MM-dd'T'HH:mm:ss'Z'",[Globalization.CultureInfo]::InvariantCulture,
        [Globalization.DateTimeStyles]::AssumeUniversal -bor [Globalization.DateTimeStyles]::AdjustToUniversal)
}

function Assert-Elapsed([double]$Seconds) {Require-Parent ($Seconds -ge 0 -and $Seconds -le 30)}

function Assert-ProtocolStep($State,$Init,$Request) {
    Require-Parent ($Request.seq -eq $State.seq+1)
    if($Request.kind -ceq 'AUTHORIZE') {
        Require-Parent ($State.phase -eq 0 -and $Request.run -cmatch '^[a-f0-9]{32}$')
        $State.run=$Request.run;$State.phase=1
    } elseif($Request.kind -ceq 'WRITE') {Require-Parent ($State.phase -eq 1);$State.phase=2}
    $bound=@{}+$Init;$bound.seq=$Request.seq;$bound.run=$State.run
    $bound.cleanup=$Request.cleanup;$bound.diag=$Request.diag
    Assert-ReplyBinding $bound $Request @('AUTHORIZE','WRITE','READY','POLL','PREFLIGHT','CLOSED','ABORT')
    $State.seq=$Request.seq
}

function Get-PrivateAcl([string]$Path,[string]$Sid) {
    $acl=Get-Acl -LiteralPath $Path
    Require-Parent ($acl.GetOwner([Security.Principal.SecurityIdentifier]).Value -ceq $Sid)
    Require-Parent ($acl.AreAccessRulesProtected)
    foreach($ace in $acl.GetAccessRules($true,$true,[Security.Principal.SecurityIdentifier])) {
        Require-Parent (@($Sid,'S-1-5-18','S-1-5-32-544') -ccontains $ace.IdentityReference.Value)
    }
    return $acl.GetSecurityDescriptorSddlForm([Security.AccessControl.AccessControlSections]::All)
}

function Get-WindowsSnapshot($Expected) {
    Initialize-Native
    Require-Parent ($Expected.name -ceq 'AIForAll' -and $Expected.guid -cmatch '^\{[a-fA-F0-9-]{36}\}$')
    Require-Parent ($Expected.base -is [string] -and $Expected.base -match '^(\\\\\?\\)?[A-Za-z]:\\')
    $drive=$Expected.base -replace '^\\\\\?\\',''
    Require-Parent ((New-Object IO.DriveInfo($drive.Substring(0,3))).DriveType -eq [IO.DriveType]::Fixed)
    $sid=[Security.Principal.WindowsIdentity]::GetCurrent().User.Value
    $root='HKCU:\Software\Microsoft\Windows\CurrentVersion\Lxss'
    $entries=@(Get-ChildItem -LiteralPath $root | Where-Object {
        (Get-ItemProperty -LiteralPath $_.PSPath).DistributionName -ceq 'AIForAll'
    })
    Require-Parent ($entries.Count -eq 1 -and $entries[0].PSChildName -ieq $Expected.guid)
    $reg=Get-ItemProperty -LiteralPath $entries[0].PSPath
    $disk=Join-Path $Expected.base 'ext4.vhdx'
    $directoryId=[K6Parent.Native]::FileIdentity($Expected.base)
    $diskId=[K6Parent.Native]::FileIdentity($disk)
    $acl=(Get-PrivateAcl $Expected.base $sid)+'|'+(Get-PrivateAcl $disk $sid)
    Require-Parent ($directoryId -ceq [K6Parent.Native]::FileIdentity($Expected.base) -and $diskId -ceq [K6Parent.Native]::FileIdentity($disk))
    $actual=@{sid=$sid;guid=$entries[0].PSChildName.ToLowerInvariant();name=[string]$reg.DistributionName;
        base=[string]$reg.BasePath;uid=[int]$reg.DefaultUid;version=[int]$reg.Version;directory=$directoryId;disk=$diskId;acl=$acl}
    Assert-Snapshot $Expected $actual
    return $actual
}

function Get-ProcessObservation($Process) {
    Initialize-Native
    Require-Parent ($null -ne $Process -and !$Process.SafeHandle.IsInvalid -and !$Process.SafeHandle.IsClosed)
    return [K6Parent.Native]::Observe($Process.SafeHandle.DangerousGetHandle())
}

function Assert-Process($Before,$Now,[bool]$Ended=$false) {
    foreach($key in @('Pid','Created','Image','Sid')) {Require-Parent ($Before.$key -ceq $Now.$key)}
    if ($Ended) { Require-Parent ($Now.Wait -eq 0 -and $Now.Exited -gt 0) }
    else { Require-Parent ($Now.Wait -eq 258 -and $Now.Exited -eq 0) }
}

function Read-BoundedFile([string]$Path,[int]$Maximum=32768) {
    $stream=[IO.File]::Open($Path,[IO.FileMode]::Open,[IO.FileAccess]::Read,[IO.FileShare]::Read)
    try {
        $bytes=New-Object byte[] ($Maximum+1)
        $count=0
        while($count -lt $bytes.Length) {
            $n=$stream.Read($bytes,$count,$bytes.Length-$count)
            if($n -eq 0){break};$count+=$n
        }
        Require-Parent ($count -le $Maximum)
        return (New-Object Text.UTF8Encoding($false,$true)).GetString($bytes,0,$count)
    } finally {$stream.Dispose()}
}

function Get-ConfigHash([string]$Path) {
    $hash=[Security.Cryptography.SHA256]::Create()
    try {return [BitConverter]::ToString($hash.ComputeHash([Text.Encoding]::UTF8.GetBytes((Read-BoundedFile $Path)))).Replace('-','')}
    finally {$hash.Dispose()}
}

function Write-CampaignEvent($Journal,[string]$Event,[string]$Session,$Fact) {
    Require-Parent (@('started','joined','unjoined') -ccontains $Event)
    $entry=@{v=1;event=$Event;session=$Session;utc=[datetime]::UtcNow.ToString('o');pid=0;created=0;image='UNKNOWN';sid='UNKNOWN';ended=0}
    if($null -ne $Fact){$entry.pid=$Fact.Pid;$entry.created=$Fact.Created;$entry.image=$Fact.Image;$entry.sid=$Fact.Sid;$entry.ended=$Fact.Exited}
    $raw=[Text.Encoding]::UTF8.GetBytes((ConvertTo-Json -InputObject $entry -Compress)+"`n")
    Require-Parent ($raw.Length -le 4096)
    $Journal.Write($raw,0,$raw.Length);$Journal.Flush()
}

function Test-Ci {
    foreach($entry in [Environment]::GetEnvironmentVariables().Keys) {
        if($entry -match '^(CI|CONTINUOUS_INTEGRATION|GITLAB_CI|TF_BUILD|JENKINS_URL|BUILDKITE|CIRCLECI|TRAVIS|APPVEYOR|TEAMCITY_VERSION)$|^(GITHUB_|RUNNER_)'){return $true}
    }
    return $false
}

function Send-Frame($Process,$Frame,[int]$Milliseconds=5000) {
    Require-Parent ($Milliseconds -gt 0 -and $Milliseconds -le 5000)
    $raw=(New-Object Text.UTF8Encoding($false,$true)).GetBytes((Write-FrameText $Frame)+"`n")
    $task=$Process.StandardInput.BaseStream.WriteAsync($raw,0,$raw.Length)
    Require-Parent ($task.Wait($Milliseconds));$task.GetAwaiter().GetResult()
}

function Receive-Frame($Out,$Errors,$Process,[int]$Milliseconds) {
    $timer=[Diagnostics.Stopwatch]::StartNew()
    while($timer.ElapsedMilliseconds -lt $Milliseconds) {
        [void]$Errors.Poll(0)
        Require-Parent ($Errors.Seen -eq 0)
        $text=$Out.Poll([Math]::Min(25,[Math]::Max(0,$Milliseconds-[int]$timer.ElapsedMilliseconds)))
        if($null -ne $text){return (Read-FrameText $text)}
        Require-Parent (!$Out.Ended)
    }
    Stop-Parent
}

function Assert-ReplyBinding($Expected,$Actual,[string[]]$Kinds) {
    Require-Parent ($Kinds -ccontains $Actual.kind)
    foreach($key in @('v','seq','session','browser','run','deadline','mode','cleanup','diag')) {
        Require-Parent ($Expected[$key] -ceq $Actual[$key])
    }
    Require-Parent (($Expected.home -join ',') -ceq ($Actual.home -join ','))
}

function Read-LocalAck([string]$Run,[ref]$Buffer) {
    Require-Parent (![Console]::IsInputRedirected -and ![Console]::IsOutputRedirected)
    while([Console]::KeyAvailable) {
        $key=[Console]::ReadKey($true)
        if($key.Key -eq [ConsoleKey]::Enter) {
            $line=$Buffer.Value;$Buffer.Value=''
            if($line -ceq ('LU '+$Run)){return $true}
        } elseif($key.Key -eq [ConsoleKey]::Backspace) {
            if($Buffer.Value.Length -gt 0){$Buffer.Value=$Buffer.Value.Substring(0,$Buffer.Value.Length-1)}
        } elseif($Buffer.Value.Length -lt 80) {$Buffer.Value+=$key.KeyChar}
        else {Stop-Parent}
    }
    return $false
}

function Complete-OwnedInvocation($Process,$Before,$Out,$Errors,$Journal,[string]$Session,[bool]$ProtocolComplete,[int]$Budget=30000) {
    Require-Parent ($Budget -gt 0 -and $Budget -le 30000)
    try{$Process.StandardInput.Close()}catch{$ProtocolComplete=$false}
    if(!$ProtocolComplete){try{$Process.StandardOutput.Close();$Process.StandardError.Close()}catch{}}
    $timer=[Diagnostics.Stopwatch]::StartNew();$joined=$false;$fact=$Before;$nativeExit=2
    while($timer.ElapsedMilliseconds -lt $Budget) {
        if($ProtocolComplete) {
            try {Require-Parent ($null -eq $Out.Poll(10) -and $null -eq $Errors.Poll(10) -and $Errors.Seen -eq 0)}
            catch {$ProtocolComplete=$false;try{$Process.StandardOutput.Close();$Process.StandardError.Close()}catch{}}
        }
        if($Process.WaitForExit(0) -and (!$ProtocolComplete -or ($Out.Ended -and $Errors.Ended))) {
            $joined=$true
            try {
                $fact=Get-ProcessObservation $Process
                Require-Parent ($null -ne $Before)
                Assert-Process $Before $fact $true
                $nativeExit=$Process.ExitCode
            } catch {$ProtocolComplete=$false}
            break
        }
        Start-Sleep -Milliseconds 10
    }
    try {Write-CampaignEvent $Journal $(if($joined){'joined'}else{'unjoined'}) $Session $fact}
    finally {if($joined){$Process.Dispose()}}
    return @{Joined=$joined;Clean=$ProtocolComplete;ExitCode=$nativeExit}
}

function Invoke-PrivateParent([string]$ConfigPath,[string]$Operation) {
    Require-Parent (!(Test-Ci))
    Require-Parent (@('preflight','capture') -ccontains $Operation)
    Initialize-Native
    Require-Parent ([IO.Path]::GetFileName($ConfigPath) -cne 'site.json' -and $ConfigPath -notmatch 'k6-causal')
    [void](Get-PrivateAcl $ConfigPath ([Security.Principal.WindowsIdentity]::GetCurrent().User.Value))
    $config=Read-JsonStrict (Read-BoundedFile $ConfigPath) 32768
    Assert-Keys $config @('v','expected','browser','deadline','linux_profile','companion','lease','journal','wsl_image','controller_sha256')
    Require-Parent ((Test-Integer $config.v) -and $config.v -eq 1)
    $expected=Convert-Table $config.expected
    [void](Get-PrivateAcl $ConfigPath $expected.sid)
    [void](Get-PrivateAcl ([IO.Path]::GetDirectoryName($config.lease)) $expected.sid)
    [void](Get-PrivateAcl ([IO.Path]::GetDirectoryName($config.journal)) $expected.sid)
    $configIdentity=[K6Parent.Native]::FileIdentity($ConfigPath)
    $configHash=Get-ConfigHash $ConfigPath
    Require-Parent ($config.controller_sha256 -is [string] -and $config.controller_sha256 -cmatch '^[a-f0-9]{64}$')
    $controllerIdentity=[K6Parent.Native]::FileIdentity($script:ControllerPath)
    Require-Parent ($config.controller_sha256 -ieq (Get-ConfigHash $script:ControllerPath))
    $deadline=Read-UtcDeadline $config.deadline
    $seconds=($deadline-[datetime]::UtcNow).TotalSeconds
    Require-Parent ($seconds -gt 0 -and $seconds -le 3600)
    if($Operation -ceq 'capture') {
        Require-Parent (![Console]::IsInputRedirected -and ![Console]::IsOutputRedirected)
        # Actual operator readiness, not a manifest/CLI consent field. No capture
        # starts until the agreed slot and separate local reading terminal exist.
        [Console]::WriteLine('PRIVATE_TERMINALS_READY? Type PRET')
        $ready='';$readyClock=[Diagnostics.Stopwatch]::StartNew()
        while($readyClock.Elapsed.TotalSeconds -lt [Math]::Min(60,$seconds)) {
            if([Console]::KeyAvailable){$k=[Console]::ReadKey($true);if($k.Key -eq [ConsoleKey]::Enter){break};Require-Parent ($ready.Length -lt 8);$ready+=$k.KeyChar}
            else{Start-Sleep -Milliseconds 25}
        }
        Require-Parent ($ready -ceq 'PRET')
    }
    [void](Get-WindowsSnapshot $expected)
    $lease=[IO.File]::Open($config.lease,[IO.FileMode]::CreateNew,[IO.FileAccess]::ReadWrite,[IO.FileShare]::Read)
    $process=$null;$before=$null;$code=2;$journal=$null;$joinResult=$null;$out=$null;$errors=$null
    $clock=[Diagnostics.Stopwatch]::StartNew()
    try {
        $leaseIdentity=[K6Parent.Native]::FileIdentity($config.lease)
        $session=[guid]::NewGuid().ToString('N')
        $journal=[IO.File]::Open($config.journal,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::Read)
        $record=[Text.Encoding]::UTF8.GetBytes(('session='+$session+';owner='+$PID+';active=true'))
        $lease.Write($record,0,$record.Length);$lease.Flush()
        foreach($path in @($config.companion,$config.linux_profile)) {
            Require-Parent ($path -is [string] -and $path -cmatch '^/home/[A-Za-z0-9_./-]+$' -and $path -notmatch '/\.\.?(/|$)' -and $path -notmatch '/k6-causal/')
        }
        $wsl=Join-Path $env:SystemRoot 'System32\wsl.exe'
        Require-Parent ((Get-AuthenticodeSignature -LiteralPath $wsl).Status -eq 'Valid')
        $start=New-Object Diagnostics.ProcessStartInfo
        $start.FileName=$wsl;$start.UseShellExecute=$false;$start.CreateNoWindow=$true
        $start.RedirectStandardInput=$true;$start.RedirectStandardOutput=$true;$start.RedirectStandardError=$true
        $start.EnvironmentVariables.Clear()
        foreach($name in @('SystemRoot','WINDIR','TEMP','TMP')) {
            $value=[Environment]::GetEnvironmentVariable($name)
            Require-Parent (![string]::IsNullOrEmpty($value));$start.EnvironmentVariables[$name]=$value
        }
        $start.EnvironmentVariables['PATH']=Join-Path $env:SystemRoot 'System32'
        $argv=@('--distribution-id',$expected.guid,'--user','aiforall','--exec','/usr/bin/env','-i','PATH=/usr/bin:/bin','LANG=C.UTF-8',
            '/usr/bin/python3','-I','-B',$config.companion,'--mode',$Operation,'--profile',$config.linux_profile)
        $start.Arguments=($argv | ForEach-Object {Quote-WindowsArgument ([string]$_)}) -join ' '
        $process=New-Object Diagnostics.Process;$process.StartInfo=$start
        Require-Parent ($process.Start())
        $before=Get-ProcessObservation $process
        Write-CampaignEvent $journal 'started' $session $before
        Require-Parent ($before.Pid -eq $process.Id -and $before.Sid -ceq $expected.sid -and $before.Image -ceq $config.wsl_image)
        Assert-Process $before (Get-ProcessObservation $process)
        [void](Get-WindowsSnapshot $expected)
        $out=New-Object K6Parent.FrameReader($process.StandardOutput.BaseStream)
        $errors=New-Object K6Parent.FrameReader($process.StandardError.BaseStream)
        $init=@{v=1;kind='INIT';seq=0;session=$session;browser=$config.browser;run='';deadline=$config.deadline;
            home=@(0,0,0);mode=$Operation;cleanup='NOT_STARTED';diag='NONE'}
        Send-Frame $process $init
        $hello=Receive-Frame $out $errors $process 30000
        Require-Parent ($hello.home[2] -eq $expected.uid -and $hello.home[0] -gt 0 -and $hello.home[1] -gt 0)
        $init.home=$hello.home
        Assert-ReplyBinding $init $hello @('HELLO')
        $state=@{seq=0;phase=0;run=''};$readBuffer='';$finished=$false;$lastPoll=-1.0
        while(!$finished) {
            $remaining=[Math]::Min($seconds-$clock.Elapsed.TotalSeconds,($deadline-[datetime]::UtcNow).TotalSeconds)
            Require-Parent ($remaining -gt -5)
            $request=Receive-Frame $out $errors $process ([int]([Math]::Max(1,[Math]::Min(35,$remaining+5))*1000))
            Assert-ProtocolStep $state $init $request
            if(@('AUTHORIZE','WRITE','READY','POLL') -ccontains $request.kind) {
                $check=[Diagnostics.Stopwatch]::StartNew()
                Require-Parent ($remaining -gt 0 -and !$lease.SafeFileHandle.IsClosed -and $lease.CanWrite)
                Require-Parent ($configIdentity -ceq [K6Parent.Native]::FileIdentity($ConfigPath) -and $leaseIdentity -ceq [K6Parent.Native]::FileIdentity($config.lease))
                Require-Parent ($configHash -ceq (Get-ConfigHash $ConfigPath))
                Require-Parent ($controllerIdentity -ceq [K6Parent.Native]::FileIdentity($script:ControllerPath) -and
                    $config.controller_sha256 -ieq (Get-ConfigHash $script:ControllerPath))
                [void](Get-PrivateAcl $ConfigPath $expected.sid)
                [void](Get-PrivateAcl ([IO.Path]::GetDirectoryName($config.lease)) $expected.sid)
                if($Operation -ceq 'capture'){Require-Parent (![Console]::IsInputRedirected -and ![Console]::IsOutputRedirected)}
                [void](Get-WindowsSnapshot $expected)
                Assert-Process $before (Get-ProcessObservation $process)
                $answer=@{}+$request
                if(@('AUTHORIZE','WRITE') -ccontains $request.kind) {
                    Require-Parent ($request.cleanup -ceq 'NOT_STARTED' -and $request.diag -ceq 'NONE');$answer.kind='ALLOW'
                } else {
                    Require-Parent ($Operation -ceq 'capture' -and $state.phase -eq 2 -and $request.cleanup -ceq 'PASS' -and $request.diag -ceq 'NONE')
                    Require-Parent ($lastPoll -lt 0 -or $clock.Elapsed.TotalSeconds-$lastPoll -ge 0.5)
                    if($request.kind -ceq 'READY') {Require-Parent ($lastPoll -lt 0);[Console]::WriteLine('READY '+$state.run+' '+$config.deadline)}
                    else {Require-Parent ($lastPoll -ge 0)}
                    $lastPoll=$clock.Elapsed.TotalSeconds
                    $answer.kind=if(Read-LocalAck $state.run ([ref]$readBuffer)){'ACK'}else{'WAIT'}
                }
                Assert-Elapsed $check.Elapsed.TotalSeconds
                Require-Parent ([datetime]::UtcNow -lt $deadline -and $clock.Elapsed.TotalSeconds -lt $seconds)
                Send-Frame $process $answer ([int][Math]::Max(1,[Math]::Min(5000,30000-$check.ElapsedMilliseconds)))
                Assert-Elapsed $check.Elapsed.TotalSeconds
            } else {
                if($request.kind -ceq 'PREFLIGHT') {
                    Require-Parent ($Operation -ceq 'preflight' -and $state.phase -eq 2 -and $request.diag -ceq 'OBSERVERS_ONLY' -and $request.cleanup -ceq 'NOT_STARTED');$code=0
                } elseif($request.kind -ceq 'CLOSED') {
                    Require-Parent ($Operation -ceq 'capture' -and $state.phase -ge 1 -and $request.diag -ceq 'NATIVE_NON_SUCCESS' -and $request.cleanup -ceq 'PASS');$code=2
                } else {Stop-Parent}
                $finished=$true
            }
        }
        $joinResult=Complete-OwnedInvocation $process $before $out $errors $journal $session $true
        Require-Parent ($joinResult.Joined -and $joinResult.Clean)
        Require-Parent (($Operation -ceq 'preflight' -and $joinResult.ExitCode -eq 0) -or ($Operation -ceq 'capture' -and @(1,2) -contains $joinResult.ExitCode))
        [Console]::WriteLine($(if($Operation -ceq 'preflight'){'OBSERVERS_ONLY_CLOSED'}else{'NATIVE_NON_SUCCESS_CLOSED'}))
        return $code
    } finally {
        try {
            if($null -ne $process -and $null -eq $joinResult) {
                $joinResult=Complete-OwnedInvocation $process $before $out $errors $journal $session $false
            }
            if($null -ne $joinResult -and !$joinResult.Joined){[Console]::WriteLine('WSL_PROCESS_UNJOINED')}
        } finally {
            if($null -ne $journal){$journal.Dispose()}
            $lease.Dispose() # retained file is inactive evidence, never reusable authority
        }
    }
}

if(!$LibraryOnly) {
    try {exit (Invoke-PrivateParent $Profile $Mode)}
    catch {[Console]::WriteLine('PARENT_REFUSED');exit 2}
}
