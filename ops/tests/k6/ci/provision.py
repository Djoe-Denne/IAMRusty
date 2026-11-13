"""Explicit, checksum-verified CI CLI acquisition; no runtime startup."""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import sys
import tarfile
import urllib.request

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "lib"))
from safety import new_output


def acquire(output):
    lock = json.loads((Path(__file__).parent / "tools.lock.json").read_text(encoding="utf-8"))
    for name in ("kind", "kubectl", "helm"):
        pin = lock[name]
        with urllib.request.urlopen(pin["url"], timeout=60) as response:
            raw = response.read(100 * 1024 ** 2 + 1)
        if len(raw) > 100 * 1024 ** 2 or hashlib.sha256(raw).hexdigest() != pin["sha256"]:
            raise ValueError("CI_CLI_CHECKSUM")
        if name == "helm":
            with tarfile.open(fileobj=io.BytesIO(raw), mode="r:gz") as archive:
                member = archive.getmember("linux-amd64/helm")
                if not member.isfile() or not 0 < member.size < 100 * 1024 ** 2:
                    raise ValueError("CI_CLI_ARCHIVE")
                raw = archive.extractfile(member).read()
        path = output / name
        path.write_bytes(raw)
        path.chmod(0o500)
    (output / "hashes.json").write_text(json.dumps({name: hashlib.sha256((output / name).read_bytes()).hexdigest()
                                                   for name in ("kind", "kubectl", "helm")}), encoding="utf-8")
    (output / "hashes.json").chmod(0o400)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    try:
        if sys.platform != "linux" or os.getuid() == 0:
            raise ValueError("LINUX_NONROOT_REQUIRED")
        acquire(new_output(args.output))
    except Exception:
        raise SystemExit("Pinned CI CLI acquisition failed; no runtime started") from None
    print("Kind/Helm/kubectl verified and acquired; not a runtime qualification")


if __name__ == "__main__":
    main()
