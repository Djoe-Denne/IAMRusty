"""Isolated IAM executable producer and compiler-free libtest replay."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tarfile
import time
import tomllib

import pipeline as ci

WORKFLOW = ".github/workflows/iam-compile-once.yml"
BRANCH = "ci-iam-compile-once"
SCHEMA = "aiforall.iam-compile-once/v1"
MAX_BINARY = 2 * 1024**3
MAX_BUNDLE = 6 * 1024**3
SERVICE_OUTPUTS = {"iam-service": "iam-service", "iam-migration": "iammigration"}


def require(ok, reason):
    if not ok:
        raise ValueError(reason)


def identity(env=os.environ, root=ci.ROOT):
    require(env.get("GITHUB_ACTIONS") == "true" and env.get("GITHUB_REPOSITORY") == "Djoe-Denne/IAMRusty"
            and env.get("GITHUB_EVENT_NAME") == "push" and env.get("GITHUB_REF") == f"refs/heads/{BRANCH}",
            "Not the reserved IAM pilot push")
    ref = f"{env.get('GITHUB_REPOSITORY')}/{WORKFLOW}@refs/heads/{BRANCH}"
    require(env.get("GITHUB_WORKFLOW_REF") == ref, "Not the IAM pilot workflow")
    value = ci.evidence_identity(env)
    revision = subprocess.check_output(["git", "rev-parse", "HEAD", "HEAD:rustycog"],
                                       cwd=root, text=True).splitlines()
    require(len(revision) == 2 and revision[0] == value["GITHUB_SHA"], "Pilot checkout mismatch")
    return {**value, "sdk": revision[1], "workflow": WORKFLOW,
            "repository": env["GITHUB_REPOSITORY"], "repository_id": env["GITHUB_REPOSITORY_ID"],
            "lock": digest(root / "Cargo.lock"), "toolchain": digest(root / "rust-toolchain.toml"),
            "inventory": digest(root / ".github/ci/inventory.json")}


def digest(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def plan(metadata, root=ci.ROOT):
    return {f"{row['lane']}-{row['shard']}": ci.target_assignment(
        metadata, "iam", row["lane"], row["shard"], root)
        for row in ci.matrix_for(metadata, "iam", root)["include"]}


def cargo_artifacts(args, root, log):
    with log.open("x", encoding="utf-8") as out:
        subprocess.run(["cargo", *args, "--message-format=json-render-diagnostics"],
                       cwd=root, stdout=out, check=True)
    with log.open(encoding="utf-8") as out:
        for line in out:
            if line.startswith("{"):
                event = json.loads(line)
                if event.get("reason") == "compiler-artifact":
                    yield event


def select_binaries(events, packages, wanted, target):
    tests, services = {}, {}
    for item in events:
        manifest = Path(item["manifest_path"]).resolve()
        if manifest not in packages:
            continue
        package = packages[manifest]
        require(not {"test-mode", "qa-mode"}.intersection(item["features"]),
                "Unexpected IAM test/QA feature in producer")
        if not item.get("executable"):
            continue
        kind = item["target"]["kind"][0]
        name = item["target"]["name"]
        key = f"{package}|{kind}|{name}"
        path = Path(item["executable"]).resolve(strict=True)
        require(path.is_relative_to(target) and path.is_file(), "Cargo executable outside the pilot target")
        if item["profile"]["test"] and key in wanted:
            require(path.name != name, "Expected a Cargo test harness, not a server executable")
            require(tests.setdefault(key, (path, manifest.parent)) == (path, manifest.parent),
                    "Multiple executable variants for one test target")
        if not item["profile"]["test"] and kind == "bin" and SERVICE_OUTPUTS.get(package) == name:
            require(services.setdefault(name, path) == path, "Multiple variants of an E2E binary")
    return tests, services


def bundle(output, provenance, partition, files):
    rows = [{"key": key, "cwd": cwd, "name": name, "sha256": digest(path),
             "bytes": path.stat().st_size} for key, cwd, name, path in files]
    validate_manifest({"identity": provenance, "partition": partition, "schema": SCHEMA, "files": rows},
                      provenance, partition, set(row["key"] for row in rows))
    manifest = output.parent / (partition + ".json")
    manifest.write_text(json.dumps({"identity": provenance, "partition": partition,
                                    "schema": SCHEMA, "files": rows}, sort_keys=True), encoding="utf-8")
    with tarfile.open(output, "w:gz", compresslevel=1) as archive:
        archive.add(manifest, arcname="manifest.json", recursive=False)
        for row, (_, _, _, path) in zip(rows, files):
            archive.add(path, arcname=row["name"], recursive=False)
    require(output.stat().st_size < 4 * 1024**3, "Pilot compressed bundle exceeds download limit")
    print(f"{partition}: {len(rows)} files, {sum(r['bytes'] for r in rows)} uncompressed bytes,"
          f" {output.stat().st_size} compressed bytes")


def produce(output, matrix_file, root=ci.ROOT):
    provenance = identity(root=root)
    target = Path(os.environ["CARGO_TARGET_DIR"]).resolve()
    require(not target.exists() or (target.is_dir() and not any(target.iterdir())),
            "Pilot requires a fresh instrumented target directory")
    coverage_flags = os.environ.get("CARGO_ENCODED_RUSTFLAGS", "").split("\x1f")
    coverage_flags += os.environ.get("RUSTFLAGS", "").split()
    require(any("instrument-coverage" in flag for flag in coverage_flags), "Coverage flags missing")
    data = ci.metadata(root)
    partitions = plan(data, root)
    wanted = set(key for values in partitions.values() for key in values)
    packages = {Path(p["manifest_path"]).resolve(): p["name"] for p in data["packages"]
                if p["name"] in ci.INVENTORY["builds"]["iam"]}
    output.mkdir(mode=0o700)
    args = [a for p in ci.INVENTORY["coverage"]["iam"] for a in ("--package", p)]
    started = time.monotonic()
    events = list(cargo_artifacts(["test", "--locked", "--no-run", "--lib", "--bins", "--tests", *args],
                                  root, output / "cargo-tests.jsonl"))
    tests, services = select_binaries(events, packages, wanted, target)
    require("iam-service" in services, "Cargo test build did not emit the normal IAM server")
    if "iammigration" not in services:
        events.extend(cargo_artifacts(["build", "--locked", "-p", "iam-migration", "--bin", "iammigration"],
                                      root, output / "cargo-migration.jsonl"))
        tests, services = select_binaries(events, packages, wanted, target)
    require(set(tests) == wanted and set(services) == set(SERVICE_OUTPUTS.values()),
            "Incomplete pilot executable set")
    print(f"Producer Cargo duration: {time.monotonic() - started:.1f}s")
    for key, keys in partitions.items():
        folder = output / key
        folder.mkdir()
        files = [(name, str(tests[name][1].relative_to(root).as_posix()), f"test-{i:03d}", tests[name][0])
                 for i, name in enumerate(keys)]
        bundle(folder / "bundle.tar.gz", provenance, key, files)
    folder = output / "e2e"
    folder.mkdir()
    bundle(folder / "bundle.tar.gz", provenance, "e2e",
           [(name, ".", name, path) for name, path in sorted(services.items())])
    matrix_file.write_text(json.dumps({"include": [{"part": key, "targets": json.dumps(keys)}
                                                   for key, keys in partitions.items()]}), encoding="utf-8")
    units = [(item["package_id"], tuple(item["filenames"])) for item in events if not item["fresh"]]
    require(len(units) == len(set(units)), "Cargo rebuilt a unit with the same selected outputs")
    print(f"Cargo artifact events: {len(events)}; non-fresh units: {len(units)};"
          " no repeated non-fresh output sets")


def validate_manifest(value, provenance, part, expected):
    require(isinstance(value, dict) and set(value) == {"identity", "schema", "partition", "files"}
            and value["identity"] == provenance and value["partition"] == part and value["schema"] == SCHEMA,
            "Pilot manifest identity mismatch")
    rows = value["files"]
    require(isinstance(rows, list) and bool(expected) and len(rows) == len(expected), "Pilot target count mismatch")
    for row in rows:
        require(isinstance(row, dict) and set(row) == {"key", "cwd", "name", "sha256", "bytes"}
                and all(isinstance(row[k], str) for k in ("key", "cwd", "name", "sha256"))
                and (row["name"] == row["key"] and re.fullmatch(r"[a-z][a-z0-9-]+", row["name"])
                     if part in {"e2e", "oauth", "ext-authz"} else re.fullmatch(r"test-[0-9]{3}", row["name"]))
                and re.fullmatch(r"[a-f0-9]{64}", row["sha256"])
                and type(row["bytes"]) is int and 64 <= row["bytes"] <= MAX_BINARY,
                "Invalid pilot executable record")
    require({r["key"] for r in rows} == expected and len({r["name"] for r in rows}) == len(rows)
            and sum(r["bytes"] for r in rows) <= MAX_BUNDLE, "Pilot artifact set mismatch")
    return rows


def unpack(archive_path, output, provenance, part, expected):
    with tarfile.open(archive_path, "r|gz") as archive:
        manifest = archive.next()
        require(manifest is not None and manifest.name == "manifest.json" and manifest.isfile()
                and not manifest.issparse() and 0 < manifest.size <= 65536, "Invalid pilot manifest member")
        rows = validate_manifest(json.load(archive.extractfile(manifest)), provenance, part, expected)
        output.mkdir(mode=0o700)
        for row in rows:
            member = archive.next()
            require(member is not None and member.name == row["name"] and member.isfile()
                    and not member.issparse() and member.size == row["bytes"], "Unexpected pilot archive member")
            destination = output / member.name
            with destination.open("xb") as out, archive.extractfile(member) as src:
                for block in iter(lambda: src.read(1024 * 1024), b""):
                    out.write(block)
            require(destination.stat().st_size == row["bytes"] and digest(destination) == row["sha256"],
                    "Pilot executable digest mismatch")
            require(elf_amd64(destination), "Pilot artifact is not a Linux amd64 executable")
            destination.chmod(0o755)
        require(archive.next() is None, "Unexpected trailing pilot archive member")
    return rows


def elf_amd64(path):
    with path.open("rb") as stream:
        header = stream.read(20)
    return header[:6] == b"\x7fELF\x02\x01" and header[16:20] in (b"\x02\x00\x3e\x00", b"\x03\x00\x3e\x00")


def execution_directory(root, row):
    cwd = (root / row["cwd"]).resolve()
    require(cwd.is_relative_to(root / "services/IAMRusty") and (cwd / "Cargo.toml").is_file(),
            "Invalid IAM test working directory")
    with (cwd / "Cargo.toml").open("rb") as stream:
        package = tomllib.load(stream)["package"]["name"]
    require(package == row["key"].split("|")[0], "Pilot test package directory mismatch")
    return cwd


def replay(archive, output, part, expected, root=ci.ROOT):
    provenance = identity(root=root)
    require(re.fullmatch(r"unit-0|tests-[0-3]", part), "Unknown IAM pilot shard")
    output.mkdir(mode=0o700)
    objects = output / "objects"
    rows = unpack(archive, objects, provenance, part, expected)
    directories = [execution_directory(root, row) for row in rows]
    profiles = output / "profiles"
    profiles.mkdir()
    status = 0
    results = []
    for row, cwd in zip(rows, directories, strict=True):
        elf = subprocess.check_output(["readelf", "--program-headers", "--wide", str(objects / row["name"])],
                                      text=True)
        require("Requesting program interpreter: /lib64/ld-linux-x86-64.so.2" in elf,
                "Unexpected pilot test program interpreter")
        resolution = subprocess.check_output(["ldd", str(objects / row["name"])], text=True)
        require("not found" not in resolution, "Pilot test has an unresolved runtime library")
        env = {**os.environ, "CARGO_MANIFEST_DIR": str(cwd), "CARGO_PKG_NAME": row["key"].split("|")[0],
               "LLVM_PROFILE_FILE": str(profiles / (row["name"] + "-%m-%p.profraw"))}
        result = subprocess.run([str(objects / row["name"])], cwd=cwd, env=env, check=False)
        results.append({"key": row["key"], "sha256": row["sha256"], "status": result.returncode})
        status = status or result.returncode
    receipt = output / "receipt.json"
    receipt.write_text(json.dumps({"identity": provenance, "part": part, "results": results,
                                   "canonical": False}), encoding="utf-8")
    profdata = output / "merged.profdata"
    raw = sorted(profiles.glob("*.profraw"))
    require(raw and all(p.stat().st_size for p in raw), "Missing pilot profiles")
    subprocess.run(["llvm-profdata", "merge", "-sparse", *map(str, raw), "-o", str(profdata)], check=True)
    binaries = [str(objects / row["name"]) for row in rows]
    with (output / "pilot.lcov").open("x", encoding="utf-8") as out:
        subprocess.run(["llvm-cov", "export", "--format=lcov", f"--instr-profile={profdata}",
                        "--ignore-filename-regex=/\\.cargo/registry/|/rustc/",
                        binaries[0], *[f"--object={path}" for path in binaries[1:]]],
                       stdout=out, check=True)
    require((output / "pilot.lcov").stat().st_size > 0, "Empty pilot LCOV")
    return status


def fetch(part, destination, root=ci.ROOT):
    """Fetch only the trusted producer's exact same-attempt, digest-checked archive."""
    sys.path.insert(0, str(root / "ops/tests/k6/ci"))
    from artifacts import GitHub, positive, unwrap_zip, validate_artifact

    require(re.fullmatch(r"unit-0|tests-[0-3]|e2e|oauth|ext-authz", part), "Unknown pilot artifact")
    known = identity(root=root)
    run_id, attempt = (positive(known[key]) for key in ("GITHUB_RUN_ID", "GITHUB_RUN_ATTEMPT"))
    api = GitHub(known["repository"], os.environ["GH_TOKEN"])
    workflow = api.json(f"actions/workflows/{Path(WORKFLOW).name}")
    run = api.json(f"actions/runs/{run_id}/attempts/{attempt}")
    require(workflow.get("path") == WORKFLOW and workflow.get("id") == run.get("workflow_id")
            and run.get("path") == WORKFLOW and run.get("head_sha") == known["GITHUB_SHA"]
            and run.get("event") == "push" and run.get("head_branch") == BRANCH
            and run.get("id") == run_id and run.get("run_attempt") == attempt,
            "Foreign IAM pilot run")
    for key in ("repository", "head_repository"):
        require(run.get(key, {}).get("id") == positive(known["repository_id"])
                and run[key].get("full_name") == known["repository"], "Foreign pilot repository")
    job_name = {"oauth": "Pilot OAuth native build", "ext-authz": "Pilot Ext Authz native build"}.get(
        part, "Pilot IAM instrumented compile")
    jobs = [j for j in api.pages(f"actions/runs/{run_id}/attempts/{attempt}/jobs", "jobs") if j["name"] == job_name]
    require(len(jobs) == 1 and jobs[0]["status"] == "completed" and jobs[0]["conclusion"] == "success",
            "Pilot producer not successful")
    name = f"iam-pilot-{part}-{attempt}"
    items = [a for a in api.pages(f"actions/runs/{run_id}/artifacts", "artifacts") if a["name"] == name]
    require(len(items) == 1, "Missing or duplicate pilot artifact")
    record = validate_artifact(items[0], run_id, known["GITHUB_SHA"])
    destination.mkdir(mode=0o700, parents=True)
    api.download(record, destination / "archive.zip")
    unwrap_zip(destination / "archive.zip", destination / "bundle.tar.gz")
    (destination / "github-artifact.json").write_text(json.dumps(record), encoding="utf-8")
    (destination / "archive.zip").unlink()


def companion(part, output, root=ci.ROOT):
    name = {"oauth": "oauth-connect-service", "ext-authz": "ext-authz"}[part]
    expected_job = {"oauth": "native-oauth", "ext-authz": "native-ext-authz"}[part]
    require(os.environ.get("GITHUB_JOB") == expected_job, "Unexpected pilot companion producer")
    known = identity(root=root)
    target = Path(os.environ["CARGO_TARGET_DIR"]) / "debug" / name
    require(elf_amd64(target), "Invalid pilot companion binary")
    output.mkdir(mode=0o700)
    bundle(output / "bundle.tar.gz", known, part, [(name, ".", name, target)])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("operation", choices=("produce", "replay", "fetch", "companion"))
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--matrix", type=Path)
    parser.add_argument("--archive", type=Path)
    parser.add_argument("--part")
    parser.add_argument("--targets")
    args = parser.parse_args()
    if args.operation == "produce":
        produce(args.output, args.matrix)
        return 0
    if args.operation == "fetch":
        fetch(args.part, args.output)
        return 0
    if args.operation == "companion":
        companion(args.part, args.output)
        return 0
    targets = json.loads(args.targets)
    require(isinstance(targets, list) and len(set(targets)) == len(targets), "Invalid trusted matrix targets")
    return replay(args.archive, args.output, args.part, set(targets))


if __name__ == "__main__":
    raise SystemExit(main())
