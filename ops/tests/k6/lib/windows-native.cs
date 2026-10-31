// Windows-only interop for run-windows.ps1. No public command/executable interface.
// Compiled in memory by Windows PowerShell 5.1; no native dependency or installation.
using System;
using System.IO;
using System.Text;
using System.Linq;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Security.Principal;
using System.Threading;
using System.Diagnostics;

namespace K6Windows {
    public static class Native {
        [StructLayout(LayoutKind.Sequential)] internal struct SA { public int size; public IntPtr descriptor; public int inherit; }
        [StructLayout(LayoutKind.Sequential, CharSet=CharSet.Unicode)] internal struct SI {
            public int cb; public string reserved, desktop, title; public int x,y,xs,ys,xc,yc,fill,flags;
            public short show, reserved2; public IntPtr reservedPtr, input, output, error;
        }
        [StructLayout(LayoutKind.Sequential)] internal struct SIX { public SI si; public IntPtr attrs; }
        [StructLayout(LayoutKind.Sequential)] internal struct PI { public IntPtr process, thread; public uint pid, tid; }
        [StructLayout(LayoutKind.Sequential)] internal struct FileInfo {
            public uint attributes; public System.Runtime.InteropServices.ComTypes.FILETIME created, accessed, written;
            public uint volume, sizeHigh, sizeLow, links, indexHigh, indexLow;
        }
        [StructLayout(LayoutKind.Sequential)] internal struct Accounting {
            public long user, kernel, periodUser, periodKernel;
            public uint faults, total, active, terminated;
        }
        [StructLayout(LayoutKind.Sequential)] internal struct Unicode { public ushort length, maximum; public IntPtr text; }
        [StructLayout(LayoutKind.Sequential)] internal struct ObjectAttributes {
            public int length; public IntPtr root, name; public uint attributes; public IntPtr security, qos;
        }
        [StructLayout(LayoutKind.Sequential)] internal struct IoStatus { public IntPtr status; public UIntPtr information; }
        [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)] internal static extern IntPtr CreateFileW(string p,uint a,uint share,IntPtr sa,uint disposition,uint flags,IntPtr template);
        [DllImport("kernel32.dll",SetLastError=true)] internal static extern bool GetFileInformationByHandle(IntPtr h,out FileInfo i);
        [DllImport("kernel32.dll",SetLastError=true)] internal static extern bool CloseHandle(IntPtr h);
        [DllImport("kernel32.dll",CharSet=CharSet.Unicode,SetLastError=true)] internal static extern IntPtr CreateJobObjectW(IntPtr sa,string name);
        [DllImport("kernel32.dll",SetLastError=true)] internal static extern bool QueryInformationJobObject(IntPtr job,int kind,IntPtr data,uint length,out uint returned);
        [DllImport("kernel32.dll",SetLastError=true)] internal static extern bool InitializeProcThreadAttributeList(IntPtr list,int count,int flags,ref IntPtr size);
        [DllImport("kernel32.dll",SetLastError=true)] internal static extern bool UpdateProcThreadAttribute(IntPtr list,uint flags,IntPtr attr,IntPtr value,IntPtr size,IntPtr previous,IntPtr returned);
        [DllImport("kernel32.dll")] internal static extern void DeleteProcThreadAttributeList(IntPtr list);
        [DllImport("kernel32.dll",CharSet=CharSet.Unicode,SetLastError=true)] internal static extern bool CreateProcessW(string app,StringBuilder cmd,IntPtr psa,IntPtr tsa,bool inherit,uint flags,IntPtr env,string cwd,ref SIX si,out PI pi);
        [DllImport("kernel32.dll",SetLastError=true)] internal static extern bool CreatePipe(out IntPtr read,out IntPtr write,ref SA sa,uint size);
        [DllImport("kernel32.dll",SetLastError=true)] internal static extern bool SetHandleInformation(IntPtr h,uint mask,uint flags);
        [DllImport("kernel32.dll",SetLastError=true)] internal static extern uint ResumeThread(IntPtr h);
        [DllImport("kernel32.dll",SetLastError=true)] internal static extern uint WaitForSingleObject(IntPtr h,uint ms);
        [DllImport("kernel32.dll",SetLastError=true)] internal static extern bool GetExitCodeProcess(IntPtr h,out uint exit);
        [DllImport("kernel32.dll",SetLastError=true)] internal static extern IntPtr OpenProcess(uint access,bool inherit,uint pid);
        [DllImport("kernel32.dll",SetLastError=true)] internal static extern bool IsProcessInJob(IntPtr h,IntPtr job,out bool inside);
        [DllImport("kernel32.dll",SetLastError=true)] internal static extern bool GetProcessTimes(IntPtr h,out long created,out long exited,out long kernel,out long user);
        [DllImport("kernel32.dll",CharSet=CharSet.Unicode,SetLastError=true)] internal static extern bool QueryFullProcessImageNameW(IntPtr h,uint flags,StringBuilder image,ref uint size);
        [DllImport("kernel32.dll",SetLastError=true)] internal static extern bool GenerateConsoleCtrlEvent(uint kind,uint group);
        [DllImport("kernel32.dll",SetLastError=true)] static extern bool IsWow64Process2(IntPtr process,out ushort processMachine,out ushort nativeMachine);
        [DllImport("kernel32.dll",SetLastError=true)] internal static extern bool PeekNamedPipe(IntPtr pipe,IntPtr buffer,uint size,IntPtr read,out uint available,IntPtr remaining);
        [DllImport("kernel32.dll",SetLastError=true)] internal static extern bool ReadFile(IntPtr file,byte[] data,uint length,out uint read,IntPtr overlapped);
        [DllImport("ntdll.dll")] internal static extern int NtQueryInformationProcess(IntPtr process,int kind,IntPtr data,int size,out int needed);
        [DllImport("ntdll.dll")] internal static extern int NtCreateFile(out IntPtr handle,uint access,ref ObjectAttributes attributes,out IoStatus status,IntPtr allocation,uint fileAttributes,uint share,uint disposition,uint options,IntPtr ea,uint eaLength);
        [DllImport("kernel32.dll",SetLastError=true)] internal static extern bool SetFileInformationByHandle(IntPtr handle,int kind,byte[] value,uint size);
        [DllImport("shell32.dll",CharSet=CharSet.Unicode,SetLastError=true)] static extern IntPtr CommandLineToArgvW(string command,out int count);
        [DllImport("kernel32.dll")] static extern IntPtr LocalFree(IntPtr memory);
        internal static void Need(bool value) { if(!value) throw new InvalidOperationException("NATIVE_UNAVAILABLE"); }
        public static bool Elevated { get { return new WindowsPrincipal(WindowsIdentity.GetCurrent()).IsInRole(WindowsBuiltInRole.Administrator); } }
        public static bool NativeX64 { get { ushort process,native; return IsWow64Process2(new IntPtr(-1),out process,out native) && process==0 && native==0x8664; } }
        internal static string Image(IntPtr p) {
            StringBuilder s=new StringBuilder(32768); uint n=32768; Need(QueryFullProcessImageNameW(p,0,s,ref n)); return s.ToString();
        }
        internal static string Command(IntPtr p) {
            int n; NtQueryInformationProcess(p,60,IntPtr.Zero,0,out n); Need(n>0 && n<=65536);
            IntPtr b=Marshal.AllocHGlobal(n); try {
                Need(NtQueryInformationProcess(p,60,b,n,out n)==0);
                Unicode u=(Unicode)Marshal.PtrToStructure(b,typeof(Unicode));
                long offset=u.text.ToInt64()-b.ToInt64(); Need(offset>=0 && offset+u.length<=n && u.length%2==0);
                return Marshal.PtrToStringUni(u.text,u.length/2);
            } finally { Marshal.FreeHGlobal(b); }
        }
        internal static string[] Arguments(IntPtr process) {
            int count; IntPtr p=CommandLineToArgvW(Command(process),out count); Need(p!=IntPtr.Zero);
            try { Need(count>0 && count<512); string[] a=new string[count]; for(int i=0;i<count;i++)a[i]=Marshal.PtrToStringUni(Marshal.ReadIntPtr(p,i*IntPtr.Size)); return a; }
            finally { LocalFree(p); }
        }
        public static string Quote(string s) {
            if(s==null || s.IndexOf('\0')>=0 || s.IndexOf('\r')>=0 || s.IndexOf('\n')>=0) throw new InvalidOperationException("INPUT_INVALID");
            StringBuilder b=new StringBuilder("\""); int slashes=0;
            foreach(char c in s) { if(c=='\\') { slashes++; continue; } b.Append('\\',c=='"'?slashes*2+1:slashes); b.Append(c); slashes=0; }
            b.Append('\\',slashes*2); return b.Append('"').ToString();
        }
        public static string Absolute(string path) {
            if(String.IsNullOrEmpty(path) || path.Length>240 || !System.Text.RegularExpressions.Regex.IsMatch(path,@"^[A-Za-z]:\\") || path.IndexOf(':',2)>=0 || path.Contains("/") || path.Contains("\"")) throw new InvalidOperationException("PATH_INVALID");
            string full=System.IO.Path.GetFullPath(path);
            if(!String.Equals(full.TrimEnd('\\'),path.TrimEnd('\\'),StringComparison.OrdinalIgnoreCase)) throw new InvalidOperationException("PATH_INVALID");
            foreach(string part in path.Substring(3).Split('\\')) if(part.EndsWith(" ") || part.EndsWith(".") || part.IndexOfAny(new char[]{'*','?','\r','\n','\0'})>=0) throw new InvalidOperationException("PATH_INVALID");
            return full.TrimEnd('\\').Length==2?full:full.TrimEnd('\\');
        }
        public static string Hash(string path) {
            // Ordinary read, closed immediately. No executable/DLL or ancestor lease.
            using(FileStream f=new FileStream(Absolute(path),FileMode.Open,FileAccess.Read,FileShare.ReadWrite|FileShare.Delete))
            using(var sha=System.Security.Cryptography.SHA256.Create()) {
                if(f.Length>536870912) throw new InvalidOperationException("PIN_INVALID");
                return BitConverter.ToString(sha.ComputeHash(f)).Replace("-","").ToLowerInvariant();
            }
        }
    }

    // Local creation/cleanup capability. FILE_CREATE returns the actual new
    // object's handle atomically; neither a pathname nor a bool is authority.
    public sealed class OwnedDirectory : IDisposable {
        IntPtr handle;
        bool bound;
        public string Path { get; private set; }
        public string Identity { get; private set; }
        OwnedDirectory(string path) {
            Path=Native.Absolute(path);
            string directory=System.IO.Path.GetDirectoryName(Path), leaf=System.IO.Path.GetFileName(Path);
            if(String.IsNullOrEmpty(directory) || String.IsNullOrEmpty(leaf)) throw new InvalidOperationException("PATH_INVALID");
            IntPtr name=IntPtr.Zero, text=IntPtr.Zero, parent=IntPtr.Zero;
            try {
                // Existing atomic creation, with ordinary inherited permissions and
                // full sharing. No ACL/owner checks or retained parent/name locks.
                parent=Native.CreateFileW(directory,0x80000000,7,IntPtr.Zero,3,0x02000000,IntPtr.Zero);
                Native.Need(parent!=new IntPtr(-1));
                text=Marshal.StringToHGlobalUni(leaf);
                var unicode=new Native.Unicode {length=(ushort)(leaf.Length*2),maximum=(ushort)(leaf.Length*2+2),text=text};
                name=Marshal.AllocHGlobal(Marshal.SizeOf(typeof(Native.Unicode))); Marshal.StructureToPtr(unicode,name,false);
                var attributes=new Native.ObjectAttributes {length=Marshal.SizeOf(typeof(Native.ObjectAttributes)),root=parent,name=name,attributes=0x40};
                Native.IoStatus io;
                int status=Native.NtCreateFile(out handle,0x80110000,ref attributes,out io,IntPtr.Zero,0x10,7,2,0x200021,IntPtr.Zero,0);
                if(status==unchecked((int)0xc0000035)) throw new InvalidOperationException("OUTPUT_EXISTS");
                if(status!=0 || handle==IntPtr.Zero || io.information.ToUInt64()!=2) throw new InvalidOperationException("OUTPUT_CREATE_REFUSED");
            } catch { Dispose(); throw; }
            finally {
                foreach(IntPtr p in new IntPtr[]{name,text}) if(p!=IntPtr.Zero) Marshal.FreeHGlobal(p);
                if(parent!=IntPtr.Zero && parent!=new IntPtr(-1)) Native.CloseHandle(parent);
            }
        }
        void Bind() {
            try {
                Native.FileInfo info; Native.Need(Native.GetFileInformationByHandle(handle,out info));
                Identity=info.volume+":"+info.indexHigh+":"+info.indexLow;
                bound=true; AssertBound();
            } catch { bound=false; throw new InvalidOperationException("OUTPUT_BINDING_UNPROVEN"); }
        }
        public static OwnedDirectory Create(string path) {
            var owned=new OwnedDirectory(path);
            try { owned.Bind(); return owned; } catch { owned.Dispose(); throw; }
        }
        public void AssertBound() {
            if(!bound || handle==IntPtr.Zero) throw new InvalidOperationException("CLEANUP_UNPROVEN");
            Native.FileInfo original, current;
            if(!Native.GetFileInformationByHandle(handle,out original) || (original.attributes&0x410)!=0x10 ||
                Identity!=original.volume+":"+original.indexHigh+":"+original.indexLow) throw new InvalidOperationException("CLEANUP_UNPROVEN");
            IntPtr check=Native.CreateFileW(Path,0x80020080,7,IntPtr.Zero,3,0x02200000,IntPtr.Zero);
            if(check==new IntPtr(-1)) throw new InvalidOperationException("CLEANUP_UNPROVEN");
            try {
                if(!Native.GetFileInformationByHandle(check,out current) || Identity!=current.volume+":"+current.indexHigh+":"+current.indexLow ||
                    (current.attributes&0x410)!=0x10) throw new InvalidOperationException("CLEANUP_UNPROVEN");
            } finally { Native.CloseHandle(check); }
        }
        public void DeleteEmpty() {
            AssertBound();
            if(!Native.SetFileInformationByHandle(handle,4,new byte[]{1},1)) throw new InvalidOperationException("CLEANUP_UNPROVEN");
            IntPtr h=handle; handle=IntPtr.Zero; bound=false;
            bool closed=Native.CloseHandle(h);
            if(!closed) throw new InvalidOperationException("CLEANUP_UNPROVEN");
        }
        public void Dispose() {
            bound=false;
            if(handle!=IntPtr.Zero) { Native.CloseHandle(handle); handle=IntPtr.Zero; }
        }
    }

    public sealed class ProcessIdentity {
        public uint Pid; public long Created; public string Image;
        internal IntPtr Handle;
        public bool Ended { get { return Native.WaitForSingleObject(Handle,0)==0; } }
    }
    public sealed class ChildResult {
        public string Stdout="", Stderr="", Reason="NONE";
        public bool Cleanup, IdentityComplete, CaptureComplete, Overflow, Interrupted;
        public uint ExitCode=0xffffffff, Total, Active;
        public ProcessIdentity[] Identities;
    }

    // One owner, one job, synchronous bounded capture. Closing this job never kills.
    public sealed class Child : IDisposable {
        IntPtr job, root, thread, stdout, stderr;
        uint pid; readonly List<ProcessIdentity> identities=new List<ProcessIdentity>();
        bool unknown, resumed, disposed; volatile bool cancel;
        ConsoleCancelEventHandler handler;
        public ChildResult Result=new ChildResult();
        public ProcessIdentity[] Identities { get { return identities.ToArray(); } }
        public void Cancel() { cancel=true; }
        static IntPtr PointerArray(IntPtr[] a) { IntPtr p=Marshal.AllocHGlobal(a.Length*IntPtr.Size); Marshal.Copy(a,0,p,a.Length); return p; }
        static string EnvironmentBlock(IDictionary<string,string> env) {
            return String.Join("\0",env.OrderBy(k=>k.Key,StringComparer.OrdinalIgnoreCase).Select(k=>k.Key+"="+k.Value).ToArray())+"\0\0";
        }
        public Child(string exe,string[] args,IDictionary<string,string> env,string cwd) {
            IntPtr attrs=IntPtr.Zero,size=IntPtr.Zero,jobs=IntPtr.Zero,list=IntPtr.Zero,block=IntPtr.Zero;
            IntPtr outWrite=IntPtr.Zero,errWrite=IntPtr.Zero,inRead=IntPtr.Zero,inWrite=IntPtr.Zero;
            Native.PI pi=new Native.PI();
            try {
                job=Native.CreateJobObjectW(IntPtr.Zero,null); Native.Need(job!=IntPtr.Zero);
                Accounting(); Native.Need(Result.Total==0 && Result.Active==0);
                Native.SA sa=new Native.SA { size=Marshal.SizeOf(typeof(Native.SA)),inherit=1 };
                Native.Need(Native.CreatePipe(out stdout,out outWrite,ref sa,0)); Native.Need(Native.SetHandleInformation(stdout,1,0));
                Native.Need(Native.CreatePipe(out stderr,out errWrite,ref sa,0)); Native.Need(Native.SetHandleInformation(stderr,1,0));
                Native.Need(Native.CreatePipe(out inRead,out inWrite,ref sa,0)); Native.Need(Native.SetHandleInformation(inWrite,1,0));
                Native.InitializeProcThreadAttributeList(IntPtr.Zero,2,0,ref size); Native.Need(size.ToInt64()>0);
                attrs=Marshal.AllocHGlobal(size); Native.Need(Native.InitializeProcThreadAttributeList(attrs,2,0,ref size));
                jobs=PointerArray(new IntPtr[]{job}); list=PointerArray(new IntPtr[]{inRead,outWrite,errWrite});
                Native.Need(Native.UpdateProcThreadAttribute(attrs,0,new IntPtr(0x2000d),jobs,new IntPtr(IntPtr.Size),IntPtr.Zero,IntPtr.Zero));
                Native.Need(Native.UpdateProcThreadAttribute(attrs,0,new IntPtr(0x20002),list,new IntPtr(3*IntPtr.Size),IntPtr.Zero,IntPtr.Zero));
                Native.SIX si=new Native.SIX { attrs=attrs,si=new Native.SI {cb=Marshal.SizeOf(typeof(Native.SIX)),flags=0x100,input=inRead,output=outWrite,error=errWrite} };
                string command=Native.Quote(exe)+" "+String.Join(" ",args.Select(Native.Quote).ToArray());
                block=Marshal.StringToHGlobalUni(EnvironmentBlock(env));
                // JOB_LIST assigns atomically: failure creates no unowned suspended process.
                const uint flags=0x80000 /* EXTENDED_STARTUPINFO_PRESENT */ | 0x400 /* UNICODE_ENVIRONMENT */ |
                    0x200 /* NEW_PROCESS_GROUP: CTRL_BREAK must not reach the parent */ | 0x4 /* SUSPENDED */;
                Native.Need(Native.CreateProcessW(exe,new StringBuilder(command),IntPtr.Zero,IntPtr.Zero,true,flags,block,cwd,ref si,out pi));
                root=pi.process; thread=pi.thread; pid=pi.pid;
                Observe(root,pid,false);
                Native.Need(String.Equals(identities[0].Image,exe,StringComparison.OrdinalIgnoreCase));
                handler=(s,e)=>{e.Cancel=true;cancel=true;}; Console.CancelKeyPress+=handler;
            } catch {
                // If observation failed, the same handle still owns the created process.
                // Resume only to permit normal exit/CTRL_BREAK; never terminate a suspended child.
                if(root!=IntPtr.Zero) { unknown=true; if(Native.ResumeThread(thread)!=0xffffffff) resumed=true; Finish(1500,1500,null); }
                Dispose(); throw new InvalidOperationException("PROCESS_START_REFUSED");
            } finally {
                foreach(IntPtr h in new IntPtr[]{outWrite,errWrite,inRead,inWrite}) if(h!=IntPtr.Zero) Native.CloseHandle(h);
                if(attrs!=IntPtr.Zero) { Native.DeleteProcThreadAttributeList(attrs); Marshal.FreeHGlobal(attrs); }
                foreach(IntPtr p in new IntPtr[]{jobs,list,block}) if(p!=IntPtr.Zero) Marshal.FreeHGlobal(p);
            }
        }
        void Observe(IntPtr h,uint id,bool ownHandle) {
            try {
                bool inside; Native.Need(Native.IsProcessInJob(h,job,out inside) && inside);
                Native.Need(Native.WaitForSingleObject(h,0)==258);
                long c,e,k,u; Native.Need(Native.GetProcessTimes(h,out c,out e,out k,out u));
                string image=Native.Image(h);
                Native.Need(Native.WaitForSingleObject(h,0)==258);
                identities.Add(new ProcessIdentity {Pid=id,Created=c,Image=image,Handle=h});
            } catch { if(ownHandle) Native.CloseHandle(h); throw; }
        }
        void Snapshot(string chrome) {
            IntPtr b=Marshal.AllocHGlobal(8+4096*IntPtr.Size);
            try {
                uint n; Native.Need(Native.QueryInformationJobObject(job,3,b,(uint)(8+4096*IntPtr.Size),out n));
                int assigned=Marshal.ReadInt32(b), count=Marshal.ReadInt32(b,4); Native.Need(assigned==count && count>=0 && count<=4096);
                for(int i=0;i<count;i++) {
                    uint id=(uint)Marshal.ReadIntPtr(b,8+i*IntPtr.Size).ToInt64();
                    ProcessIdentity observed=identities.FirstOrDefault(x=>x.Pid==id && !x.Ended);
                    if(observed==null) {
                        IntPtr h=Native.OpenProcess(0x101000,false,id);
                        if(h==IntPtr.Zero) { unknown=true; continue; }
                        try { Observe(h,id,true); observed=identities[identities.Count-1]; } catch { unknown=true; continue; }
                    }
                    if(chrome!=null && String.Equals(observed.Image,chrome,StringComparison.OrdinalIgnoreCase)) {
                        try {
                            string[] args=Native.Arguments(observed.Handle);
                            bool forbidden=args.Any(a=>System.Text.RegularExpressions.Regex.IsMatch(a,@"^--(?:no-sandbox|disable-.*sandbox|ignore-certificate|allow-insecure|disable-web-security|remote-debugging-address|remote-debugging-pipe)",System.Text.RegularExpressions.RegexOptions.IgnoreCase) ||
                                (a.StartsWith("--remote-debugging-port",StringComparison.OrdinalIgnoreCase) && a!="--remote-debugging-port=0"));
                            if(forbidden) { Result.Reason="BROWSER_ARGUMENTS_REFUSED"; cancel=true; }
                        } catch { /* A process can exit during command-line observation. */ }
                    }
                }
            } finally { Marshal.FreeHGlobal(b); }
        }
        void Accounting() {
            int size=Marshal.SizeOf(typeof(Native.Accounting)); IntPtr b=Marshal.AllocHGlobal(size);
            try { uint n; Native.Need(Native.QueryInformationJobObject(job,1,b,(uint)size,out n)); var a=(Native.Accounting)Marshal.PtrToStructure(b,typeof(Native.Accounting)); Result.Total=a.total; Result.Active=a.active; }
            finally { Marshal.FreeHGlobal(b); }
        }
        bool Drain(IntPtr pipe,MemoryStream capture) {
            uint available; if(!Native.PeekNamedPipe(pipe,IntPtr.Zero,0,IntPtr.Zero,out available,IntPtr.Zero)) {
                int e=Marshal.GetLastWin32Error(); if(e==109) return true; throw new InvalidOperationException("CAPTURE_UNAVAILABLE");
            }
            // Never allocate/capture more than 64 KiB per stream; overflow is sticky.
            if(available>0) {
                byte[] data=new byte[Math.Min(available,4096)]; uint read;
                Native.Need(Native.ReadFile(pipe,data,(uint)data.Length,out read,IntPtr.Zero));
                if(capture.Length+read>65536) Result.Overflow=true;
                int keep=(int)Math.Min(read,65536-capture.Length); capture.Write(data,0,keep);
            }
            return false;
        }
        public ChildResult Finish(int timeoutMs,int graceMs,string chrome) {
            if(timeoutMs<1 || timeoutMs>60000 || graceMs<1 || graceMs>10000) throw new InvalidOperationException("INPUT_INVALID");
            Stopwatch clock=Stopwatch.StartNew(); long stopAt=-1; bool signalled=false, outEnd=false,errEnd=false;
            using(MemoryStream output=new MemoryStream()) using(MemoryStream error=new MemoryStream()) {
                try {
                    if(!resumed) { Native.Need(Native.ResumeThread(thread)!=0xffffffff); resumed=true; }
                    while(true) {
                        Snapshot(chrome); Accounting(); outEnd=Drain(stdout,output);errEnd=Drain(stderr,error);
                        if(Result.Active==0 && Native.WaitForSingleObject(root,0)==0 && outEnd && errEnd) break;
                        if(cancel || Result.Overflow || clock.ElapsedMilliseconds>=timeoutMs) {
                            if(stopAt<0) { stopAt=clock.ElapsedMilliseconds; Result.Interrupted=cancel; if(Result.Reason=="NONE") Result.Reason=Result.Overflow?"OUTPUT_LIMIT":cancel?"INTERRUPTED":"TIMEOUT"; }
                            if(!signalled) { signalled=true; Native.GenerateConsoleCtrlEvent(1,pid); }
                            if(clock.ElapsedMilliseconds-stopAt>=graceMs) break;
                        }
                        Thread.Sleep(5);
                    }
                } catch { Result.Reason="PROCESS_OBSERVATION_UNAVAILABLE"; unknown=true; Native.GenerateConsoleCtrlEvent(1,pid); }
                finally {
                    // On every path, bounded graceful join, then same-handle + whole-job inventory.
                    Stopwatch join=Stopwatch.StartNew();
                    do {
                        try { Snapshot(chrome); Accounting(); outEnd=Drain(stdout,output); errEnd=Drain(stderr,error); } catch { unknown=true; }
                        if(Result.Active==0 && Native.WaitForSingleObject(root,0)==0 && outEnd && errEnd) break;
                        if(!signalled) { signalled=true; Native.GenerateConsoleCtrlEvent(1,pid); }
                        Thread.Sleep(5);
                    } while(join.ElapsedMilliseconds<graceMs);
                    try { Accounting(); } catch { unknown=true; }
                    bool ended=Native.WaitForSingleObject(root,0)==0;
                    if(ended) Native.GetExitCodeProcess(root,out Result.ExitCode);
                    Result.Identities=identities.ToArray();
                    Result.IdentityComplete=!unknown && identities.Count==Result.Total && identities.Count>0;
                    Result.Cleanup=ended && Result.Active==0 && identities.All(x=>x.Ended) && Result.IdentityComplete;
                    Result.CaptureComplete=outEnd && errEnd;
                    try { var utf8=new UTF8Encoding(false,true); Result.Stdout=utf8.GetString(output.ToArray()); Result.Stderr=utf8.GetString(error.ToArray()); }
                    catch { Result.Reason="CAPTURE_INVALID"; }
                }
            }
            return Result;
        }
        public void Dispose() {
            if(disposed) return; disposed=true;
            if(handler!=null) Console.CancelKeyPress-=handler;
            foreach(var p in identities) if(p.Handle!=root) Native.CloseHandle(p.Handle);
            foreach(IntPtr h in new IntPtr[]{root,thread,stdout,stderr,job}) if(h!=IntPtr.Zero) Native.CloseHandle(h);
        }
    }

    // Duplicate members (including case aliases) must be rejected before PS5 ConvertFrom-Json.
    public static class JsonGuard {
        public static void Check(string text) { new Reader(text).Run(); }
        sealed class Reader {
            readonly string s; int i,depth;
            internal Reader(string text) { s=text; if(s==null || s.Length>65536) Bad(); }
            static void Bad() { throw new InvalidOperationException("JSON_INVALID"); }
            void Space() { while(i<s.Length && " \r\n\t".IndexOf(s[i])>=0)i++; }
            bool Take(char c) { Space(); if(i<s.Length && s[i]==c){i++;return true;} return false; }
            string Str() {
                Space(); if(i>=s.Length || s[i++]!='"') Bad(); StringBuilder b=new StringBuilder();
                while(i<s.Length) {
                    char c=s[i++]; if(c=='"')return b.ToString(); if(c<32)Bad();
                    if(c=='\\') { if(i>=s.Length)Bad(); c=s[i++]; if(c=='u') { if(i+4>s.Length)Bad(); int n; if(!Int32.TryParse(s.Substring(i,4),System.Globalization.NumberStyles.HexNumber,null,out n))Bad(); b.Append((char)n);i+=4;continue; } if("\"\\/bfnrt".IndexOf(c)<0)Bad(); }
                    b.Append(c);
                } Bad(); return null;
            }
            void Value() {
                Space(); if(++depth>16 || i>=s.Length)Bad();
                if(Take('{')) { var keys=new HashSet<string>(StringComparer.OrdinalIgnoreCase); if(!Take('}'))do { if(!keys.Add(Str()) || !Take(':'))Bad();Value();if(Take('}'))break;if(!Take(','))Bad(); }while(true); }
                else if(Take('[')) { if(!Take(']'))do { Value();if(Take(']'))break;if(!Take(','))Bad(); }while(true); }
                else if(s[i]=='"')Str();
                else { int start=i; while(i<s.Length && ",]} \t\r\n".IndexOf(s[i])<0)i++; string token=s.Substring(start,i-start); if(token!="null" && token!="true" && token!="false" && !System.Text.RegularExpressions.Regex.IsMatch(token,@"^-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?$"))Bad(); }
                depth--;
            }
            internal void Run() { Value();Space();if(i!=s.Length)Bad(); }
        }
    }
}
