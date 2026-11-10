"""Public source identity for image-only refresh; no runtime/private-file inputs."""
import hashlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def identity(root=ROOT):
    digest = hashlib.sha256()
    paths = [path for directory in ("charts/local-full", "profiles/local-full", "registry", "third-party/calico", "kind")
             for path in sorted((root / "ops/deploy" / directory).rglob("*"))]
    paths += [root / "ops/deploy/local-full.just", root / "ops/scripts/local-full-certificates.cnf"]
    for path in paths:
        if path.is_symlink() or not path.resolve().is_relative_to(root.resolve()):
            raise ValueError("declaration source escapes canonical root")
        if not path.is_file():
            continue
        if path.suffix.lower() == ".md":
            continue  # Documentation alone is not a deployment/configuration drift.
        digest.update(path.relative_to(root).as_posix().encode("utf-8") + b"\0")
        digest.update(hashlib.sha256(path.read_bytes()).digest())
    return digest.hexdigest()
