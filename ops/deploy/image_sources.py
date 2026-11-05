"""Content identity of public image inputs, including a dirty, consumed SDK.

Snapshots never consult the operator's .dockerignore and never copy .git or .env.
The image builder consumes the captured bytes, not the live checkout.
"""
import hashlib
import json
import os
from pathlib import Path
import posixpath
import stat
import subprocess
import tomllib

from image_catalog import TARGETS

ROOT = Path(__file__).resolve().parents[2]
ROOT_FILES = {"Cargo.toml", "Cargo.lock", ".cargo/config.toml", "Dockerfile.build", "Dockerfile.build.dockerignore"}
SOURCE_DIRS = {"services", "workers", "crates", "runtime"}
BUILD_FILES = {"ops/deploy/image_catalog.py", "ops/deploy/image_sources.py"}
PRIVATE_DIRS = {".git", ".tmp", ".worktrees", ".cursor", ".codex", ".opencode", ".serena", "obsidian",
                "target", "node_modules", "__pycache__", ".venv", ".cache", ".pytest_cache", ".mypy_cache",
                ".docker", ".aws", ".azure", ".ssh", ".gnupg", "secrets", "private", "certs", "keys"}
PRIVATE_SUFFIXES = {".pem", ".key", ".p12", ".pfx", ".keystore", ".jks"}
PUBLIC_SUFFIXES = {".rs", ".toml", ".lock", ".sh", ".sql", ".proto", ".json", ".yaml", ".yml", ".py",
                   ".html", ".txt", ".css", ".js", ".ts", ".tsx", ".cjs", ".c", ".h", ".cc", ".cpp", ".hpp",
                   ".cmake", ".wit", ".wasm", ".svg", ".png", ".hbs", ".tera", ".mustache", ".dockerignore"}
PUBLIC_NAMES = {"Cargo.toml", "Cargo.lock", "CMakeLists.txt", "Makefile", "README.md"}
# Public synthetic fixtures shipped in the service config COPY, not general JWT
# or credential-file permission. Private/generated tokens remain inadmissible.
PUBLIC_FIXTURES = {"services/Hive/config/test-oidc.jwt", "services/IAMRusty/config/test-oidc.jwt"}
# Public SDK code, not permission to traverse arbitrary credential directories.
# The directory entry permits walking to these two files; all neighbors stay private.
SDK_CONFIG_PUBLIC_PATHS = {"rustycog-config/src/secrets", "rustycog-config/src/secrets/mod.rs",
                           "rustycog-config/src/secrets/tests.rs"}


def git(root, *args):
    result = subprocess.run(["git", "-C", str(root), *args], capture_output=True, timeout=30)
    if result.returncode:
        raise ValueError("cannot inspect image source Git state")
    return result.stdout.decode("utf-8", errors="strict")


def public_input(path, sdk=False):
    parts = Path(path).parts
    name = parts[-1].lower()
    public_sdk_code = sdk and Path(path).as_posix() in SDK_CONFIG_PUBLIC_PATHS
    return ((public_sdk_code or not any(part.lower() in PRIVATE_DIRS for part in parts))
            and not any(part.lower().startswith(".env") for part in parts) and Path(name).suffix not in PRIVATE_SUFFIXES
            and not name.startswith("kubeconfig")
            and name not in {"credentials", "credentials.toml", "credentials.json"})


def included(path, sdk=False):
    if not public_input(path, sdk=sdk):
        return False
    name = Path(path).name
    if not (Path(path).suffix.lower() in PUBLIC_SUFFIXES or name in PUBLIC_NAMES
            or name.startswith("Dockerfile") or path in PUBLIC_FIXTURES):
        return False
    if sdk:
        return path in ROOT_FILES or path.split("/")[0] == "src" or path.split("/")[0].startswith("rustycog-")
    return (path in ROOT_FILES or path in BUILD_FILES or path.split("/")[0] in SOURCE_DIRS
            or path.startswith("ops/deploy/build/"))


def ordinary_path(path, root):
    if path.is_symlink() or path.resolve() != path.absolute():
        raise ValueError("image inputs must be ordinary paths inside the source root")
    if os.name == "nt" and any(getattr(parent.stat(), "st_file_attributes", 0) & stat.FILE_ATTRIBUTE_REPARSE_POINT
                               for parent in [path, *path.parents] if parent != root.parent):
        raise ValueError("reparse image input refused")


def paths(root, sdk=False):
    """Enumerate admissible sources/assets, including Git-ignored public inputs."""
    ordinary_path(root, root)
    directories = ([root / "src", *root.glob("rustycog-*")] if sdk else
                   [*(root / name for name in sorted(SOURCE_DIRS)), root / "ops/deploy/build"])
    candidates = [root / name for name in ROOT_FILES | (set() if sdk else BUILD_FILES)]

    def walk(directory):
        ordinary_path(directory, root)
        for entry in sorted(directory.iterdir()):
            relative = entry.relative_to(root).as_posix()
            # Prune private directories BEFORE traversing or reading any bytes.
            if not public_input(relative, sdk=sdk):
                continue
            ordinary_path(entry, root)
            if entry.is_dir():
                walk(entry)
            elif included(relative, sdk):
                candidates.append(entry)

    for directory in directories:
        if directory.exists() or directory.is_symlink():
            walk(directory)
    result, seen = [], set()
    for file in sorted(set(candidates)):
        name = file.relative_to(root).as_posix()
        if not included(name, sdk) or (not file.exists() and not file.is_symlink()):
            continue
        ordinary_path(file, root)
        if not file.is_file() or name.casefold() in seen:
            raise ValueError("special file or case-colliding public image input refused")
        seen.add(name.casefold())
        result.append(name)
    return result


def identity(entries):
    return hashlib.sha256(json.dumps(entries, sort_keys=True, separators=(",", ":")).encode()).hexdigest()


def dependency_paths(manifests):
    """Conservatively include local dependencies, even behind optional/test features."""
    packages = {data["package"]["name"]: path for path, data in manifests.items() if "package" in data}
    workspace = manifests["Cargo.toml"].get("workspace", {}).get("dependencies", {})

    def children(path):
        data = manifests[path]
        tables = [data, *data.get("target", {}).values()]
        for table in tables:
            for key in ("dependencies", "build-dependencies", "dev-dependencies"):
                for name, dependency in table.get(key, {}).items():
                    base = posixpath.dirname(path)
                    if isinstance(dependency, dict) and dependency.get("workspace"):
                        dependency, base = workspace.get(name), ""
                    if not isinstance(dependency, dict) or "path" not in dependency:
                        continue
                    relative = dependency["path"].replace("\\", "/")
                    resolved = posixpath.normpath(posixpath.join(base, relative, "Cargo.toml"))
                    if posixpath.isabs(relative) or ":" in relative or resolved.startswith("../"):
                        raise ValueError("external path dependency is outside the image snapshot")
                    if resolved not in manifests:
                        raise ValueError("path dependency manifest is missing from the public image inputs")
                    yield resolved

    result = {}
    for name, item in TARGETS.items():
        pending = [packages[p] for p in item["outputs"] if p in packages]
        seen = set()
        while pending:
            path = pending.pop()
            if path in seen:
                continue
            seen.add(path)
            pending.extend(children(path))
        result[name] = sorted({posixpath.dirname(path) + "/" for path in seen})
    return result


def capture(root=ROOT, destination=None):
    root = Path(root).resolve()
    sdk_root = root / "rustycog"
    ordinary_path(sdk_root, root)
    manifest_file = root / "Cargo.toml"
    ordinary_path(manifest_file, root)
    # The path-patched SDK, not an arbitrary sibling checkout, is the input.
    if not (sdk_root / "Cargo.toml").is_file():
        raise ValueError("consumed rustycog submodule is unavailable")
    if not manifest_file.is_file():
        raise ValueError("ordinary root Cargo manifest required")
    manifest = tomllib.loads(manifest_file.read_text(encoding="utf-8"))
    if manifest.get("patch", {}).get("crates-io", {}).get("rustycog-framework", {}).get("path") != "rustycog":
        raise ValueError("image source policy requires the consumed rustycog path patch")
    entries, manifests = [], {}
    for source, prefix in ((root, ""), (sdk_root, "rustycog/")):
        for relative in paths(source, sdk=bool(prefix)):
            file = source / relative
            before = file.stat()
            contents = file.read_bytes()
            after = file.stat()
            if (before.st_size, before.st_mtime_ns) != (after.st_size, after.st_mtime_ns):
                raise ValueError("image input changed during capture")
            name = prefix + relative
            if relative == "Cargo.toml" or relative.endswith("/Cargo.toml"):
                manifests[name] = tomllib.loads(contents.decode("utf-8"))
            mode = 0o755 if before.st_mode & stat.S_IXUSR else 0o644
            entries.append({"path": name, "sha256": hashlib.sha256(contents).hexdigest(), "mode": mode})
            if destination is not None:
                output = Path(destination) / name
                output.parent.mkdir(parents=True, exist_ok=True)
                with output.open("xb") as stream:
                    stream.write(contents)
                output.chmod(mode)
                os.utime(output, ns=(before.st_atime_ns, before.st_mtime_ns))
    entries.sort(key=lambda item: item["path"])
    return {"sha256": identity(entries), "files": entries, "target_paths": dependency_paths(manifests),
            "revision": git(root, "rev-parse", "HEAD").strip(),
            "sdk_revision": git(sdk_root, "rev-parse", "HEAD").strip()}


def target_identity(snapshot, name):
    item = TARGETS[name]
    inputs = (*item["inputs"], *snapshot.get("target_paths", {}).get(name, []))
    entries = [entry for entry in snapshot["files"]
               if entry["path"].startswith((*inputs, "rustycog/", "crates/", "ops/deploy/build/"))
               or entry["path"] in ROOT_FILES | BUILD_FILES
               or entry["path"].endswith("/Cargo.toml")]
    return identity(entries)


def require_unchanged(snapshot, root=ROOT):
    fresh = capture(root)
    if any(fresh[key] != snapshot[key] for key in ("sha256", "revision", "sdk_revision")):
        raise ValueError("image inputs changed during build; capture and build again")
