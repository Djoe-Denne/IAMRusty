"""Closed, non-instrumented native-build bundles; never compiles or runs a service."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import struct
import subprocess
import tarfile
import tomllib

ROOT = Path(__file__).resolve().parents[4]
SCHEMA = "aiforall.k6-binaries/v2"
ROLES = {"iam": "Build (IAMRusty)", "oauth": "Build (OAuthConnect)",
         "ext-authz": "Build (Ext Authz)"}
BUILD_JOBS = {"iam": "build-iam", "oauth": "build-oauth", "ext-authz": "build-ext-authz"}
FLAG_NAMES = ("RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "RUSTC_WRAPPER",
              "RUSTC_WORKSPACE_WRAPPER", "RUSTC", "CARGO_BUILD_RUSTFLAGS",
              "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS")
MAX_BINARY = 2 * 1024 ** 3
MAX_BUNDLE = 6 * 1024 ** 3


class Refused(ValueError):
    """Bounded public reason; no raw command/API output."""


def require(condition, reason):
    if not condition:
        raise Refused(reason)


def sha(path):
    digest = hashlib.sha256()
    with Path(path).open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def strict_json(raw):
    def pairs(items):
        value = {}
        for key, item in items:
            require(key not in value, "DUPLICATE_JSON_KEY")
            value[key] = item
        return value

    def constant(_):
        raise Refused("NONFINITE_JSON")

    try:
        return json.loads(raw, object_pairs_hook=pairs, parse_constant=constant)
    except (ValueError, UnicodeError, RecursionError):
        raise Refused("INVALID_JSON") from None


def command(args, cwd=ROOT):
    result = subprocess.run(args, cwd=cwd, capture_output=True, timeout=60)
    require(result.returncode == 0 and len(result.stdout) < 4 * 1024 ** 2, "NATIVE_INSPECTION_FAILED")
    return result.stdout.decode("utf-8", errors="strict").strip()


def inventory(root, role):
    require(role in ROLES, "UNKNOWN_BINARY_ROLE")
    path = Path(root) / "ops/deploy/build/targets.json"
    target = strict_json(path.read_bytes())["variable"]["inventory"]["default"]["targets"][role]
    outputs = target["outputs"]
    require(isinstance(outputs, dict) and bool(outputs), "BINARY_INVENTORY")
    for package, name in outputs.items():
        require(re.fullmatch(r"[a-z][a-z0-9-]+", package) and
                re.fullmatch(r"[a-z][a-z0-9-]+", name), "BINARY_INVENTORY")
    return outputs


def flags(environment, root, cache_attest=None):
    values = {name: environment.get(name, "") for name in FLAG_NAMES}
    require(not any(value for name, value in values.items() if name != "RUSTC_WRAPPER"),
            "BUILD_FLAGS_OR_WRAPPER")
    require(values["RUSTC_WRAPPER"] in ("", "sccache"), "BUILD_FLAGS_OR_WRAPPER")
    cache = None
    if values["RUSTC_WRAPPER"]:
        if cache_attest is None:
            cache_attest = compiler_cache_module().attest
        cache = cache_attest()
    require(not any(name.startswith("CARGO_PROFILE_") and value for name, value in environment.items()),
            "PROFILE_OVERRIDE")
    # Capture/reject Cargo configuration overrides as well as environment flags.
    config_hashes = {}
    candidates = [Path(root) / ".cargo/config.toml", Path(root) / ".cargo/config"]
    home = Path(environment.get("CARGO_HOME", str(Path.home() / ".cargo")))
    candidates += [home / "config.toml", home / "config"]
    for index, path in enumerate(candidates):
        if not path.exists():
            continue
        config = tomllib.loads(path.read_text(encoding="utf-8"))
        # No target/rustflags/wrapper/env indirection in the accepted producer.
        require(set(config) <= {"build", "net", "registries", "registry", "alias"}, "CARGO_CONFIG_OVERRIDE")
        require(set(config.get("build", {})) <= {"jobs", "target-dir", "incremental"}, "CARGO_CONFIG_OVERRIDE")
        if "alias" in config:
            # Only the repository's lint shortcuts. The producer invokes the
            # built-in build/metadata commands, never these aliases. Do not
            # admit shell aliases, alternate build commands or home overrides.
            aliases = config["alias"]
            require(index == 0 and isinstance(aliases, dict)
                    and set(aliases) <= {"clippy-ci-production", "clippy-ci-tests"}
                    and all(isinstance(args, list) and bool(args) and args[0] == "clippy"
                            and all(isinstance(arg, str) for arg in args) for args in aliases.values()), "CARGO_CONFIG_OVERRIDE")
        config_hashes[str(index)] = sha(path)
    return {"environment": values, "cargo_config_sha256": config_hashes, "compiler_cache": cache}


def compiler_cache_module():
    # Import only repository-owned policy code, never a path supplied by a bundle.
    import importlib.util
    path = ROOT / "ops/ci/compiler_cache.py"
    spec = importlib.util.spec_from_file_location("aiforall_compiler_cache", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def elf(path, inspect=command):
    return _inspect_elf(path, inspect, coverage=False)


def coverage_elf(path, inspect=command):
    """Pilot-only ELF inspection; canonical bundle admission still uses elf()."""
    return _inspect_elf(path, inspect, coverage=True)


def _inspect_elf(path, inspect, *, coverage):
    path = Path(path)
    require(path.is_file() and not path.is_symlink(), "MISSING_BINARY")
    require(64 <= path.stat().st_size <= MAX_BINARY, "BINARY_SIZE")
    with path.open("rb") as stream:
        header = stream.read(64)
        require(header[:6] == b"\x7fELF\x02\x01" and struct.unpack_from("<H", header, 18)[0] == 62
                and struct.unpack_from("<H", header, 16)[0] in (2, 3), "NOT_LINUX_AMD64_ELF")
        stream.seek(0)
        tail = b""
        marked = False
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            data = tail + block
            if any(marker in data for marker in (b"__llvm_profile_", b"__llvm_prf", b"__llvm_cov")):
                require(coverage, "INSTRUMENTED_BINARY")
                marked = True
                break
            tail = data[-64:]
        require(not coverage or marked, "MISSING_PILOT_INSTRUMENTATION")
    program = inspect(["readelf", "--program-headers", "--wide", str(path)])
    dynamic = inspect(["readelf", "--dynamic", "--wide", str(path)])
    versions = inspect(["readelf", "--version-info", "--wide", str(path)])
    interpreter = re.findall(r"Requesting program interpreter: ([^\]]+)", program)
    require(interpreter == ["/lib64/ld-linux-x86-64.so.2"], "ELF_INTERPRETER")
    needed = sorted(re.findall(r"\(NEEDED\).*?\[([^\]]+)\]", dynamic))
    require(bool(needed) and all(re.fullmatch(r"[A-Za-z0-9_.+-]+", name) for name in needed), "ELF_DEPENDENCIES")
    required = sorted(set(re.findall(r"Name: ((?:GLIBC|GLIBCXX|CXXABI|GCC|OPENSSL)_[A-Za-z0-9_.]+)", versions)))
    require(bool(required), "ELF_VERSIONS")
    return {"architecture": "x86_64", "interpreter": interpreter[0],
            "needed": needed, "required_versions": required}


def identity(environment, role):
    values = {name: environment.get(name, "") for name in
              ("GITHUB_REPOSITORY", "GITHUB_REPOSITORY_ID", "GITHUB_SHA", "GITHUB_RUN_ID", "GITHUB_RUN_ATTEMPT")}
    require(re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", values["GITHUB_REPOSITORY"]), "PRODUCER_REPOSITORY")
    for name in ("GITHUB_REPOSITORY_ID", "GITHUB_RUN_ID", "GITHUB_RUN_ATTEMPT"):
        require(re.fullmatch(r"[1-9][0-9]*", values[name]), "PRODUCER_ID")
    require(re.fullmatch(r"[a-f0-9]{40}", values["GITHUB_SHA"]), "PRODUCER_SHA")
    require(role in BUILD_JOBS and environment.get("GITHUB_JOB") == BUILD_JOBS[role], "PRODUCER_JOB")
    values["GITHUB_JOB"] = environment["GITHUB_JOB"]
    return values


def build_packages(args, *, allow_all_targets):
    """Admit only production commands or the explicitly supported historical v2 form."""
    require(isinstance(args, list) and args[:2] == ["build", "--locked"], "MANIFEST_BUILD_ARGS")
    offset = 3 if allow_all_targets and args[2:3] == ["--all-targets"] else 2
    require(len(args) >= offset + 2 and (len(args) - offset) % 2 == 0, "MANIFEST_BUILD_ARGS")
    require(all(args[index] == "--package" and isinstance(args[index + 1], str)
                and re.fullmatch(r"[a-z][a-z0-9_-]+", args[index + 1])
                for index in range(offset, len(args), 2)), "MANIFEST_BUILD_ARGS")
    return args[offset + 1::2]


def publish(root, role, output, environment=None):
    env = dict(os.environ if environment is None else environment)
    root, output = Path(root).resolve(), Path(output)
    require(not output.exists() and output.is_absolute(), "BUNDLE_OUTPUT_EXISTS")
    outputs = inventory(root, role)
    source = identity(env, role)
    require(platform.system() == "Linux" and platform.machine() == "x86_64", "PRODUCER_PLATFORM")
    os_release = platform.freedesktop_os_release()
    require(os_release.get("ID") == "ubuntu" and os_release.get("VERSION_ID") == "24.04", "PRODUCER_ABI")
    require(command(["git", "rev-parse", "HEAD"], root) == source["GITHUB_SHA"], "PRODUCER_CHECKOUT")
    sdk = command(["git", "rev-parse", "HEAD:rustycog"], root)
    require(re.fullmatch(r"[a-f0-9]{40}", sdk), "SDK_GITLINK")
    build_flags = flags(env, root)
    require(build_flags["compiler_cache"] == compiler_cache_module().expected_attestation(),
            "MISSING_COMPILER_CACHE_ATTESTATION")
    args = strict_json(env.get("K6_BUILD_ARGS_JSON", "null"))
    packages = build_packages(args, allow_all_targets=False)
    require(set(outputs) <= set(packages), "OUTPUT_PACKAGE_NOT_BUILT")
    target = Path(env["CARGO_TARGET_DIR"]).resolve()
    require(target == root / "target", "BINARY_TARGET_DIRECTORY")
    files = []
    for package, name in outputs.items():
        path = target / "debug" / name
        files.append({"package": package, "name": name, "path": "bin/" + name,
                      "size": path.stat().st_size if path.exists() else 0,
                      "sha256": sha(path) if path.is_file() else "", "elf": elf(path)})
    manifest = {
        "schema": SCHEMA, "role": role, "repository": source["GITHUB_REPOSITORY"],
        "repository_id": int(source["GITHUB_REPOSITORY_ID"]), "workflow_path": ".github/workflows/ci.yml",
        "source_sha": source["GITHUB_SHA"], "sdk_sha": sdk,
        "run_id": int(source["GITHUB_RUN_ID"]), "run_attempt": int(source["GITHUB_RUN_ATTEMPT"]),
        "job": source["GITHUB_JOB"], "job_name": ROLES[role], "cargo_profile": "dev", "output_directory": "debug",
        "target": "x86_64-unknown-linux-gnu", "producer_os": "ubuntu-24.04",
        "libc": command(["getconf", "GNU_LIBC_VERSION"]), "cargo": command(["cargo", "--version"]),
        "rustc": command(["rustc", "-vV"]), "instrumentation": "none", "flags": build_flags,
        "build_args": args,
        "cargo_lock_sha256": sha(root / "Cargo.lock"),
        "inventory_sha256": sha(root / "ops/deploy/build/targets.json"), "files": files,
    }
    # The trusted workflow injects its immutable workflow ID from the API.
    workflow_id = env.get("K6_CI_WORKFLOW_ID", "")
    require(re.fullmatch(r"[1-9][0-9]*", workflow_id), "PRODUCER_WORKFLOW_ID")
    manifest["workflow_id"] = int(workflow_id)
    output.mkdir(mode=0o700)
    (output / "manifest.json").write_text(json.dumps(manifest, sort_keys=True), encoding="utf-8")
    with tarfile.open(output / "bundle.tar.gz", "w:gz", compresslevel=1) as archive:
        archive.add(output / "manifest.json", arcname="manifest.json", recursive=False)
        for item in files:
            archive.add(target / "debug" / item["name"], arcname=item["path"], recursive=False)
    (output / "manifest.json").unlink()


MANIFEST_FIELDS = frozenset((
    "schema", "role", "repository", "repository_id", "workflow_path", "workflow_id", "source_sha", "sdk_sha",
    "run_id", "run_attempt", "job", "job_name", "cargo_profile", "output_directory", "target", "producer_os",
    "libc", "cargo", "rustc", "instrumentation", "flags", "build_args", "cargo_lock_sha256", "inventory_sha256", "files"))


def validate_manifest(value, expected, outputs):
    require(isinstance(value, dict) and value.keys() == MANIFEST_FIELDS, "MANIFEST_FIELDS")
    for key, item in expected.items():
        require(key in value and type(value[key]) is type(item) and value[key] == item, "MANIFEST_PROVENANCE")
    for key, item in {"schema": SCHEMA, "workflow_path": ".github/workflows/ci.yml",
                      "cargo_profile": "dev", "output_directory": "debug", "producer_os": "ubuntu-24.04",
                      "target": "x86_64-unknown-linux-gnu", "instrumentation": "none"}.items():
        require(value[key] == item, "MANIFEST_BUILD_CONTRACT")
    require(value["role"] in ROLES and value["job_name"] == ROLES[value["role"]]
            and value["job"] == BUILD_JOBS[value["role"]], "MANIFEST_JOB")
    flag_values = value["flags"]
    require(isinstance(flag_values, dict) and flag_values.keys() ==
            {"environment", "cargo_config_sha256", "compiler_cache"}, "MANIFEST_FLAGS")
    require(flag_values["environment"] ==
            {name: "sccache" if name == "RUSTC_WRAPPER" else "" for name in FLAG_NAMES}, "MANIFEST_FLAGS")
    require(flag_values["compiler_cache"] == compiler_cache_module().expected_attestation(), "MANIFEST_COMPILER_CACHE")
    configs = flag_values["cargo_config_sha256"]
    require(isinstance(configs, dict) and all(key in ("0", "1", "2", "3") and
            isinstance(value, str) and re.fullmatch(r"[a-f0-9]{64}", value)
            for key, value in configs.items()), "MANIFEST_FLAGS")
    args = value["build_args"]
    packages = build_packages(args, allow_all_targets=True)
    require(set(outputs) <= set(packages), "MANIFEST_BUILD_ARGS")
    require(isinstance(value["files"], list) and len(value["files"]) == len(outputs), "MANIFEST_FILES")
    names = set()
    for item in value["files"]:
        require(isinstance(item, dict) and item.keys() == {"package", "name", "path", "size", "sha256", "elf"}, "MANIFEST_FILES")
        require(isinstance(item["package"], str) and isinstance(item["name"], str) and
                outputs.get(item["package"]) == item["name"] and item["name"] not in names, "MANIFEST_FILES")
        require(item["path"] == "bin/" + item["name"], "MANIFEST_PATH")
        require(type(item["size"]) is int and 64 <= item["size"] <= MAX_BINARY, "MANIFEST_SIZE")
        require(isinstance(item["sha256"], str) and re.fullmatch(r"[a-f0-9]{64}", item["sha256"]), "MANIFEST_HASH")
        names.add(item["name"])
    return value


def unpack(bundle, destination, expected, outputs, inspect=elf):
    destination = Path(destination)
    require(destination.is_absolute() and not destination.exists(), "EXTRACTION_COLLISION")
    try:
        with tarfile.open(bundle, "r:gz") as archive:
            required_names = {"manifest.json", *("bin/" + name for name in outputs.values())}
            members = []
            total = 0
            for member in archive:
                require(len(members) < len(required_names), "ARCHIVE_MEMBERS")
                require(member.name in required_names and member.isfile() and not member.issparse()
                        and 0 < member.size <= MAX_BINARY, "ARCHIVE_TYPE_OR_SIZE")
                total += member.size
                require(total <= MAX_BUNDLE, "ARCHIVE_SIZE")
                members.append(member)
            names = [member.name for member in members]
            require(len(names) == len(required_names) and set(names) == required_names, "ARCHIVE_MEMBERS")
            require(all(member.isfile() and not member.issparse() and 0 < member.size <= MAX_BINARY
                        for member in members), "ARCHIVE_TYPE_OR_SIZE")
            require(sum(member.size for member in members) <= MAX_BUNDLE, "ARCHIVE_SIZE")
            metadata = archive.getmember("manifest.json")
            require(metadata.size <= 65536, "MANIFEST_SIZE")
            manifest = validate_manifest(strict_json(archive.extractfile(metadata).read()), expected, outputs)
            destination.mkdir(mode=0o700)
            for item in manifest["files"]:
                member = archive.getmember(item["path"])
                require(member.size == item["size"], "ARCHIVE_BINARY_SIZE")
                path = destination / item["name"]
                with archive.extractfile(member) as source, path.open("xb") as target:
                    shutil.copyfileobj(source, target, 1024 * 1024)
                require(path.stat().st_size == item["size"] and sha(path) == item["sha256"], "BINARY_HASH")
                path.chmod(0o755)
                require(inspect(path) == item["elf"], "BINARY_ELF_METADATA")
            return manifest
    except (tarfile.TarError, EOFError, OSError):
        raise Refused("ARCHIVE_INVALID") from None


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--role", choices=tuple(ROLES), required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    try:
        publish(ROOT, args.role, args.output)
    except Refused as error:
        raise SystemExit("Native binary publication refused: " + str(error)) from None
    except (Refused, OSError, ValueError, KeyError):
        raise SystemExit("Native binary publication refused; no fallback build") from None
    print("Closed native binary bundle published; no compilation performed")


if __name__ == "__main__":
    main()
