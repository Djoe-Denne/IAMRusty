#!/usr/bin/env python3
"""Local-only PKI preparation shared by Windows and Linux; no third-party modules."""
import argparse
import base64
import os
from pathlib import Path
import re
import shutil
import stat
import subprocess
import sys

AUTHORITIES = {
    "mesh": "aiforall-local-full-transport",
    "workload": "aiforall-local-full-workload",
}


def leaf_profiles(oauth_proof=False):
    profiles = []
    for name in ("iam", "hive", "telegraph", "manifesto"):
        profiles.append(("mesh", name + "-service", [
            "DNS:" + name + "-service", "DNS:" + name,
            "DNS:" + name + ".aiforall-local-full.svc.cluster.local",
            "DNS:localhost", "IP:127.0.0.1",
        ]))
    for name in ("envoy-mesh", "ext-authz", "mesh-client"):
        profiles.append(("mesh", name, [
            "DNS:" + name, "DNS:" + name + ".aiforall-local-full.svc.cluster.local",
            "DNS:localhost", "IP:127.0.0.1",
        ]))
    if oauth_proof:
        profiles.append(("mesh", "gitlab-connect", [
            "DNS:gitlab-connect", "DNS:gitlab-connect.aiforall-local-full.svc.cluster.local",
        ]))
    profiles.append(("workload", "server", [
        "DNS:lazaret-service", "DNS:lazaret", "DNS:lazaret.aiforall-local-full.svc.cluster.local",
        "DNS:localhost", "IP:127.0.0.1",
    ]))
    return profiles


def openssl_path(explicit):
    if explicit:
        executable = shutil.which(explicit)
    else:
        executable = shutil.which("openssl")
        if not executable and os.name == "nt":
            # Git for Windows bundles OpenSSL; invoke the EXE, never Git Bash/MSYS.
            git = shutil.which("git")
            candidates = [Path(git).parent.parent / "usr/bin/openssl.exe"] if git else []
            if os.environ.get("ProgramFiles"):
                candidates.append(Path(os.environ["ProgramFiles"]) / "Git/usr/bin/openssl.exe")
            executable = next((str(path) for path in candidates if path.is_file()), None)
    if not executable:
        raise ValueError("OpenSSL required; use --openssl or OPENSSL to specify its executable")
    return str(Path(executable).resolve())


def checked_path(value):
    path = Path(os.path.abspath(Path(value).expanduser()))
    for component in (path, *path.parents):
        if os.path.lexists(component):
            info = component.lstat()
            if stat.S_ISLNK(info.st_mode) or getattr(info, "st_file_attributes", 0) & 0x400:
                raise ValueError("linked/reparse certificate paths are not supported")
        if (component / ".git").exists():
            raise ValueError("certificate output must be outside Git working trees")
    if not path.parent.is_dir():
        raise ValueError("certificate parent directory must already exist")
    return path


def private_directory(path, create=False):
    if create:
        path.mkdir(mode=0o700)  # Exclusive: no merge, overwrite or adoption by name.
    if os.name == "nt":
        script = r"""
$ErrorActionPreference = 'Stop'
$path = $env:AIFORALL_CERT_ACL_PATH
$user = [Security.Principal.WindowsIdentity]::GetCurrent().User
$system = [Security.Principal.SecurityIdentifier]::new('S-1-5-18')
if ($env:AIFORALL_CERT_ACL_CREATE -eq '1') {
    # Modify only the DACL. Set-Acl on a new security descriptor can request
    # SeSecurityPrivilege (SACL), which ordinary developer sessions do not have.
    $directory = [IO.DirectoryInfo]::new($path)
    $acl = $directory.GetAccessControl([Security.AccessControl.AccessControlSections]::Access)
    $acl.SetAccessRuleProtection($true, $false)
    foreach ($rule in @($acl.Access)) { $acl.RemoveAccessRuleSpecific($rule) }
    foreach ($sid in @($user, $system)) {
        $acl.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new(
            $sid, 'FullControl', 'ContainerInherit, ObjectInherit', 'None', 'Allow'))
    }
    $directory.SetAccessControl($acl)
}
$items = @((Get-Item -LiteralPath $path -Force)) + @(Get-ChildItem -LiteralPath $path -Force -Recurse)
foreach ($item in $items) {
    if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Linked certificate entry' }
    $acl = Get-Acl -LiteralPath $item.FullName
    if ($acl.GetOwner([Security.Principal.SecurityIdentifier]).Value -ne $user.Value) { throw 'Foreign owner' }
    foreach ($rule in $acl.Access) {
        $sid = $rule.IdentityReference.Translate([Security.Principal.SecurityIdentifier]).Value
        if ($rule.AccessControlType -eq 'Allow' -and $sid -notin @($user.Value, $system.Value)) {
            throw 'Certificate access is not private'
        }
    }
}
"""
        env = {**os.environ, "AIFORALL_CERT_ACL_PATH": str(path),
               "AIFORALL_CERT_ACL_CREATE": "1" if create else "0"}
        result = subprocess.run([
            "powershell.exe", "-NoProfile", "-NonInteractive", "-EncodedCommand",
            base64.b64encode(script.encode("utf-16le")).decode("ascii"),
        ], env=env, capture_output=True, timeout=30, check=False)
        if result.returncode:
            raise ValueError("Windows certificate ACL check failed; require current user and SYSTEM only")
    else:
        for entry in [path, *path.rglob("*")]:
            info = entry.lstat()
            if not (stat.S_ISREG(info.st_mode) or stat.S_ISDIR(info.st_mode)):
                raise ValueError("certificate directory contains a non-regular entry")
            if info.st_uid != os.getuid() or info.st_mode & 0o077:
                raise ValueError("certificate files/directories must be private to their owner")


class OpenSSL:
    def __init__(self, executable, directory):
        self.executable = executable
        self.directory = directory
        self.private_output = False

    def run(self, *arguments):
        result = subprocess.run([self.executable, *map(str, arguments)], capture_output=True,
                                stdin=subprocess.DEVNULL, timeout=45, check=False)
        if result.returncode:
            if self.private_output:
                # Private ACL/umask is established before any OpenSSL operation here.
                with (self.directory / "openssl-error.log").open("ab") as log:
                    log.write(result.stdout + result.stderr)
            diagnostic = "; private diagnostic: openssl-error.log" if self.private_output else "; output redacted"
            raise ValueError(f"OpenSSL {arguments[0]} failed (exit {result.returncode})" + diagnostic)
        return result.stdout


def generate(tool, directory, profiles):
    for folder, common_name in AUTHORITIES.items():
        target = directory / folder
        # Windows children inherit the root DACL. Python 3.13's mode=0700
        # otherwise installs its own OWNER RIGHTS/Administrators entries.
        target.mkdir(mode=0o777 if os.name == "nt" else 0o700)
        tool.run("req", "-x509", "-newkey", "rsa:2048", "-noenc", "-sha256", "-days", "365",
                 "-keyout", target / "ca.key", "-out", target / "ca.crt", "-subj", "/CN=" + common_name,
                 "-addext", "basicConstraints=critical,CA:TRUE",
                 "-addext", "keyUsage=critical,keyCertSign,cRLSign")
    for folder, name, sans in profiles:
        target = directory / folder
        tool.run("req", "-new", "-newkey", "rsa:2048", "-noenc", "-sha256",
                 "-keyout", target / (name + ".key"), "-out", target / (name + ".csr"),
                 "-subj", "/CN=" + name)
        extension = target / (name + ".ext")
        extension.write_text("basicConstraints=critical,CA:FALSE\nsubjectAltName=" + ",".join(sans)
                             + "\nkeyUsage=digitalSignature,keyEncipherment\n"
                             "extendedKeyUsage=serverAuth,clientAuth\n", encoding="ascii")
        tool.run("x509", "-req", "-in", target / (name + ".csr"), "-CA", target / "ca.crt",
                 "-CAkey", target / "ca.key", "-CAcreateserial", "-sha256", "-days", "90",
                 "-out", target / (name + ".crt"), "-extfile", extension)


def validate_pair(tool, directory, name, authority=False):
    certificate = directory / (name + ".crt")
    key = directory / (name + ".key")
    tool.run("pkey", "-in", key, "-check", "-noout")
    public = tool.run("pkey", "-in", key, "-pubout")
    if public != tool.run("x509", "-in", certificate, "-pubkey", "-noout"):
        raise ValueError("certificate/private key mismatch: " + name)
    tool.run("x509", "-in", certificate, "-noout", "-checkend", "0")
    extension = tool.run("x509", "-in", certificate, "-noout", "-ext", "basicConstraints")
    if (b"CA:TRUE" if authority else b"CA:FALSE") not in extension:
        raise ValueError("invalid CA/leaf constraints: " + name)
    usage = tool.run("x509", "-in", certificate, "-noout", "-ext", "keyUsage")
    expected = (b"Certificate Sign", b"CRL Sign") if authority else (b"Digital Signature", b"Key Encipherment")
    if any(value not in usage for value in expected):
        raise ValueError("invalid certificate key usage: " + name)
    if not authority:
        extended = tool.run("x509", "-in", certificate, "-noout", "-ext", "extendedKeyUsage")
        if any(value not in extended for value in (b"TLS Web Server Authentication", b"TLS Web Client Authentication")):
            raise ValueError("invalid certificate extended key usage: " + name)
    return public


def validate(tool, directory, profiles):
    required = [directory / folder / ("ca." + suffix)
                for folder in AUTHORITIES for suffix in ("crt", "key")]
    required += [directory / folder / (name + "." + suffix)
                 for folder, name, _ in profiles for suffix in ("crt", "key")]
    if any(not path.is_file() or path.is_symlink() for path in required):
        raise ValueError("incomplete certificate directory; preserve it and use a new destination")
    private_directory(directory)
    tool.private_output = True
    public_keys = []
    for folder, common_name in AUTHORITIES.items():
        target = directory / folder
        public_keys.append(validate_pair(tool, target, "ca", authority=True))
        subject = tool.run("x509", "-in", target / "ca.crt", "-noout", "-subject", "-nameopt", "RFC2253")
        if subject.strip() != ("subject=CN=" + common_name).encode("ascii"):
            raise ValueError("unexpected local CA identity: " + folder)
        tool.run("verify", "-check_ss_sig", "-CAfile", target / "ca.crt", target / "ca.crt")
    if public_keys[0] == public_keys[1]:
        raise ValueError("transport and workload CA keys must be distinct")
    for folder, name, sans in profiles:
        target = directory / folder
        certificate = target / (name + ".crt")
        validate_pair(tool, target, name)
        extension = tool.run("x509", "-in", certificate, "-noout", "-ext", "subjectAltName").decode("ascii")
        actual = {value.strip().replace("IP Address:", "IP:")
                  for value in extension.split("\n", 1)[-1].split(",")}
        if actual != set(sans):
            raise ValueError("unexpected certificate DNS/IP identities: " + name)
        for purpose in ("sslserver", "sslclient"):
            tool.run("verify", "-CAfile", target / "ca.crt", "-purpose", purpose, certificate)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-dir", default=os.environ.get("LOCAL_FULL_CERT_DIR"), type=Path)
    parser.add_argument("--openssl", default=os.environ.get("OPENSSL"))
    parser.add_argument("--oauth-proof", action="store_true")
    args = parser.parse_args()
    if not args.output_dir:
        parser.error("--output-dir or LOCAL_FULL_CERT_DIR is required")
    if sys.version_info < (3, 11):
        parser.error("Python 3.11+ required")
    previous_umask = os.umask(0o077)
    try:
        directory = checked_path(args.output_dir)
        tool = OpenSSL(openssl_path(args.openssl), directory)
        version = tool.run("version")
        match = re.match(rb"OpenSSL (\d+)\.", version)
        if not match or int(match[1]) < 3:
            raise ValueError("OpenSSL 3+ required")
        existed = directory.exists()
        profiles = leaf_profiles(args.oauth_proof)
        if existed:
            if not directory.is_dir():
                raise ValueError("certificate output is not a directory")
            # A previously generated OAuth superset remains usable without modifying it.
            if any((directory / "mesh" / ("gitlab-connect." + suffix)).exists() for suffix in ("crt", "key")):
                profiles = leaf_profiles(True)
        else:
            private_directory(directory, create=True)
            tool.private_output = True
            generate(tool, directory, profiles)
        validate(tool, directory, profiles)
        print(("reused" if existed else "created") + " verified local-full certificates; "
              "separate transport/workload CAs; no key material printed")
        return 0
    except (OSError, ValueError, subprocess.TimeoutExpired):
        # ValueError messages originate here, never from raw native output or key parsing.
        error = sys.exc_info()[1]
        message = str(error) if isinstance(error, ValueError) else "file/tool access failed or timed out; outputs retained"
        print("Certificate preparation failed: " + message, file=sys.stderr)
        return 1
    finally:
        os.umask(previous_umask)


if __name__ == "__main__":
    sys.exit(main())
