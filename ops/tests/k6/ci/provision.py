"""Explicit, checksum-verified CI CLI acquisition; no runtime startup."""
import argparse
import hashlib
import http.client
import io
import json
import os
from pathlib import Path
import ssl
import sys
import tarfile
import time
from urllib.error import HTTPError, URLError
import urllib.request

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "lib"))
from safety import new_output


def download(name, url):
    for attempt in range(1, 4):
        try:
            with urllib.request.urlopen(url, timeout=60) as response:
                return response.read(100 * 1024 ** 2 + 1)
        except HTTPError as error:
            reason = f"HTTP_{error.code}"
            transient = error.code in (408, 500, 502, 503, 504)
        except URLError as error:
            reason = type(error.reason).__name__
            transient = isinstance(error.reason, OSError) and not isinstance(error.reason, ssl.SSLError)
        except (TimeoutError, ConnectionError, http.client.IncompleteRead) as error:
            reason = type(error).__name__
            transient = True
        # Never print network exceptions themselves: they may include URLs.
        if not transient or attempt == 3:
            raise ValueError(f"CI_CLI_DOWNLOAD {name} {reason}") from None
        print(f"Retrying pinned CLI {name} after {reason} (attempt {attempt} of 3)", flush=True)
        time.sleep(2 * attempt)
    raise AssertionError("unreachable download state")


def acquire(output):
    lock = json.loads((Path(__file__).parent / "tools.lock.json").read_text(encoding="utf-8"))
    for name in ("kind", "kubectl", "helm"):
        pin = lock[name]
        raw = download(name, pin["url"])
        if len(raw) > 100 * 1024 ** 2 or hashlib.sha256(raw).hexdigest() != pin["sha256"]:
            print(f"Pinned CLI checksum or size mismatch: {name}", flush=True)
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
        print(f"Pinned CLI verified: {name}", flush=True)
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
    except ValueError as error:
        if str(error).startswith("CI_CLI_DOWNLOAD "):
            print(str(error), file=sys.stderr, flush=True)
        raise SystemExit("Pinned CI CLI acquisition failed; no runtime started") from None
    except Exception:
        raise SystemExit("Pinned CI CLI acquisition failed; no runtime started") from None
    print("Kind/Helm/kubectl verified and acquired; not a runtime qualification")


if __name__ == "__main__":
    main()
