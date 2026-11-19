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
CI_WORKFLOW = ".github/workflows/ci.yml"
CI_SCHEMA = "aiforall.ci-iam-executables/v1"
CI_PRODUCER = "Build (IAMRusty)"
CI_BRANCHES = {"master", "main"}
AB_WORKFLOW = ".github/workflows/iam-ab.yml"
AB_BRANCH = "ci-iam-ab"
AB_SCHEMA = "aiforall.iam-ab-executables/v1"
AB_E2E_PARTS = {"native-e2e", "e2e", "oauth", "ext-authz"}
AB_PRODUCERS = {
    "native-e2e": ("build-a", "A native IAM"),
    "e2e": ("build-b", "B instrumented IAM and tests"),
    "oauth": ("native-oauth", "Common native OAuth"),
    "ext-authz": ("native-ext-authz", "Common native Ext Authz"),
}
MAX_BINARY = 2 * 1024**3
MAX_BUNDLE = 6 * 1024**3
SERVICE_OUTPUTS = {"iam-service": "iam-service", "iam-migration": "iammigration"}


def require(ok, reason):
    if not ok:
        raise ValueError(reason)


def run_lane(env):
    ref = env.get("GITHUB_REF", "")
    workflow_ref = env.get("GITHUB_WORKFLOW_REF", "")
    repo = env.get("GITHUB_REPOSITORY")
    event = env.get("GITHUB_EVENT_NAME")
    require(env.get("GITHUB_ACTIONS") == "true" and repo == "Djoe-Denne/IAMRusty",
            "Not the IAM executable CI repository")
    if event == "push" and ref == f"refs/heads/{BRANCH}" and workflow_ref == f"{repo}/{WORKFLOW}@{ref}":
        return WORKFLOW, BRANCH, "pilot"
    if event == "push" and ref == f"refs/heads/{AB_BRANCH}" and workflow_ref == f"{repo}/{AB_WORKFLOW}@{ref}":
        return AB_WORKFLOW, AB_BRANCH, "ab"
    if workflow_ref == f"{repo}/{CI_WORKFLOW}@{ref}":
        if event == "push" and ref.removeprefix("refs/heads/") in CI_BRANCHES and ref.startswith("refs/heads/"):
            return CI_WORKFLOW, ref.removeprefix("refs/heads/"), "ci"
        if event == "pull_request" and re.fullmatch(r"refs/pull/[1-9][0-9]*/merge", ref) \
                and env.get("GITHUB_BASE_REF") in CI_BRANCHES:
            return CI_WORKFLOW, env.get("GITHUB_HEAD_REF", ""), "ci"
    raise ValueError("Not an admitted IAM executable workflow")


def identity(env=os.environ, root=ci.ROOT):
    workflow, _, lane = run_lane(env)
    value = ci.evidence_identity(env)
    revision = subprocess.check_output(["git", "rev-parse", "HEAD", "HEAD:rustycog"],
                                       cwd=root, text=True).splitlines()
    require(len(revision) == 2 and revision[0] == value["GITHUB_SHA"], "Pilot checkout mismatch")
    return {**value, "sdk": revision[1], "workflow": workflow, "lane": lane,
            "producer_job": {"ci": "build-iam", "ab": "build-b", "pilot": "producer"}[lane],
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


def verify_coverage_environment(env):
    # Pinned cargo-llvm-cov 0.9.1 show-env uses a chained wrapper by default,
    # not RUSTFLAGS. Require that mode to keep instrumentation scoped.
    flags = env.get("__CARGO_LLVM_COV_RUSTC_WRAPPER_RUSTFLAGS", "").split("\x1f")
    require(env.get("__CARGO_LLVM_COV_RUSTC_WRAPPER") == "1"
            and Path(env.get("RUSTC_WRAPPER", "")).name == "cargo-llvm-cov"
            and ("-C", "instrument-coverage") in zip(flags, flags[1:]),
            "Pilot requires the cargo-llvm-cov coverage wrapper")
    require(Path(env.get("__CARGO_LLVM_COV_RUSTC_WRAPPER_PRE_EXISTING", "")).name == "sccache",
            "Pilot coverage wrapper did not retain sccache")


def cargo_artifacts(args, root, log):
    started = time.monotonic()
    print(f"{log.stem} argv: {json.dumps(['cargo', *args, '--message-format=json-render-diagnostics'])}", flush=True)
    with log.open("x", encoding="utf-8") as out:
        subprocess.run(["cargo", *args, "--message-format=json-render-diagnostics"],
                       cwd=root, stdout=out, check=True)
    events = read_cargo_artifacts(log)
    fresh = sum(item["fresh"] for item in events)
    debug_levels = sorted({json.dumps(item["profile"]["debuginfo"]) for item in events})
    print(f"{log.stem}: {time.monotonic() - started:.1f}s, "
          f"{fresh} fresh and {len(events) - fresh} non-fresh Cargo artifacts; "
          f"debuginfo: {', '.join(debug_levels)}", flush=True)
    return events


def read_cargo_artifacts(log):
    events = []
    with log.open(encoding="utf-8") as out:
        for line in out:
            if line.startswith("{"):
                event = json.loads(line)
                if event.get("reason") == "compiler-artifact":
                    events.append(event)
    return events


def cargo_artifact_key(item):
    target = item["target"]
    return (item["package_id"], tuple(target["kind"]), target["name"], tuple(sorted(item["features"])),
            json.dumps(item["profile"], sort_keys=True), tuple(sorted(item["filenames"])))


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


def bundle_schema(provenance):
    return {"ci": CI_SCHEMA, "ab": AB_SCHEMA}.get(provenance.get("lane"), SCHEMA)


def part_identity(provenance, part):
    if provenance.get("lane") == "ab" and part in AB_PRODUCERS:
        return {**provenance, "producer_job": AB_PRODUCERS[part][0]}
    return provenance


def bundle(output, provenance, partition, files):
    total = sum(path.stat().st_size for _, _, _, path in files)
    require(total <= MAX_BUNDLE, f"{partition}: {total} uncompressed bytes exceeds limit {MAX_BUNDLE}")
    rows = [{"key": key, "cwd": cwd, "name": name, "sha256": digest(path),
             "bytes": path.stat().st_size} for key, cwd, name, path in files]
    schema = bundle_schema(provenance)
    content = {"identity": provenance, "partition": partition, "schema": schema, "files": rows}
    validate_manifest(content, provenance, partition, set(row["key"] for row in rows))
    manifest = output.parent / (partition + ".json")
    manifest.write_text(json.dumps(content, sort_keys=True), encoding="utf-8")
    with tarfile.open(output, "w:gz", compresslevel=1) as archive:
        archive.add(manifest, arcname="manifest.json", recursive=False)
        for row, (_, _, _, path) in zip(rows, files):
            archive.add(path, arcname=row["name"], recursive=False)
    require(output.stat().st_size < 4 * 1024**3, "Pilot compressed bundle exceeds download limit")
    print(f"{partition}: {len(rows)} files, {total} uncompressed bytes,"
          f" {output.stat().st_size} compressed bytes", flush=True)


def producer_target():
    verify_coverage_environment(os.environ)
    require(all(os.environ.get(f"CARGO_PROFILE_{profile}_DEBUG") == "line-tables-only"
                for profile in ("DEV", "TEST")), "Pilot requires matching compact dev and test debug settings")
    return Path(os.environ["CARGO_TARGET_DIR"]).resolve()


def iam_packages(data):
    return {Path(p["manifest_path"]).resolve(): p["name"] for p in data["packages"]
            if p["name"] in ci.INVENTORY["builds"]["iam"]}


def produce_e2e(output, root=ci.ROOT):
    provenance = identity(root=root)
    require(os.environ.get("GITHUB_JOB") == provenance["producer_job"],
            "Unexpected IAM executable producer")
    target = producer_target()
    require(not target.exists() or (target.is_dir() and not any(target.iterdir())),
            "Pilot requires a fresh instrumented target directory")
    packages = iam_packages(ci.metadata(root))
    output.mkdir(mode=0o700)
    args = [arg for package in SERVICE_OUTPUTS for arg in ("--package", package)]
    events = cargo_artifacts(["build", "--locked", "--bins", *args], root, output / "cargo-e2e.jsonl")
    _, services = select_binaries(events, packages, set(), target)
    require(set(services) == set(SERVICE_OUTPUTS.values()), "Incomplete IAM E2E executable set")
    folder = output / "e2e"
    folder.mkdir()
    bundle(folder / "bundle.tar.gz", provenance, "e2e",
           [(name, ".", name, path) for name, path in sorted(services.items())])


def produce(output, matrix_file, root=ci.ROOT):
    provenance = identity(root=root)
    require(os.environ.get("GITHUB_JOB") == provenance["producer_job"],
            "Unexpected IAM executable producer")
    target = producer_target()
    require(target.is_dir() and (output / "e2e/bundle.tar.gz").is_file(),
            "Test compilation requires the prior E2E build in the same target directory")
    before = {cargo_artifact_key(item) for item in read_cargo_artifacts(output / "cargo-e2e.jsonl")}
    require(before, "Missing prior Cargo build evidence")
    data = ci.metadata(root)
    partitions = plan(data, root)
    wanted = set(key for values in partitions.values() for key in values)
    args = [a for p in ci.INVENTORY["coverage"]["iam"] for a in ("--package", p)]
    events = cargo_artifacts(["test", "--locked", "--no-run", "--lib", "--bins", "--tests", *args],
                             root, output / "cargo-tests.jsonl")
    tests, _ = select_binaries(events, iam_packages(data), wanted, target)
    require(set(tests) == wanted, "Incomplete pilot test executable set")
    shared = [item for item in events if cargo_artifact_key(item) in before]
    fresh = sum(item["fresh"] for item in shared)
    print(f"Same Cargo artifact keys in E2E and test phases: {fresh} fresh, {len(shared) - fresh} non-fresh;"
          f" {len(events) - len(shared)} test-phase records with a different key", flush=True)
    if len(shared) != fresh:
        print("Visible Cargo keys are not full fingerprints; investigate non-fresh overlaps before qualification.",
              flush=True)
    for key, keys in partitions.items():
        folder = output / key
        folder.mkdir()
        files = [(name, str(tests[name][1].relative_to(root).as_posix()), f"test-{i:03d}", tests[name][0])
                 for i, name in enumerate(keys)]
        bundle(folder / "bundle.tar.gz", provenance, key, files)
    matrix_file.write_text(json.dumps({"include": [{"part": key, "targets": json.dumps(keys)}
                                                   for key, keys in partitions.items()]}), encoding="utf-8")


def validate_manifest(value, provenance, part, expected):
    schema = bundle_schema(provenance)
    require(isinstance(value, dict) and set(value) == {"identity", "schema", "partition", "files"}
            and value["identity"] == provenance and value["partition"] == part and value["schema"] == schema,
            "Pilot manifest identity mismatch")
    rows = value["files"]
    require(isinstance(rows, list) and bool(expected) and len(rows) == len(expected), "Pilot target count mismatch")
    for row in rows:
        require(isinstance(row, dict) and set(row) == {"key", "cwd", "name", "sha256", "bytes"}
                and all(isinstance(row[k], str) for k in ("key", "cwd", "name", "sha256"))
                and (row["name"] == row["key"] and re.fullmatch(r"[a-z][a-z0-9-]+", row["name"])
                      if part in AB_E2E_PARTS else re.fullmatch(r"test-[0-9]{3}", row["name"]))
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


def require_iam_coverage(report, root):
    iam_source = False
    with report.open(encoding="utf-8") as stream:
        for line in stream:
            if line.startswith("SF:"):
                path = (root / line[3:].rstrip("\n")).resolve()
                iam_source = path.is_relative_to(root / "services/IAMRusty") and path.is_file()
            elif line.startswith("DA:") and iam_source:
                if int(line.split(",")[1]) > 0:
                    return
            elif line.strip() == "end_of_record":
                iam_source = False
    raise ValueError("Pilot LCOV has no executed IAM source lines")


def replay(archive, output, part, expected, root=ci.ROOT, report=None):
    provenance = identity(root=root)
    require(re.fullmatch(r"unit-0|tests-[0-3]", part), "Unknown IAM pilot shard")
    require((report is not None) == (provenance["lane"] in {"ci", "ab"}), "Wrong IAM reporting lane")
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
        env.pop("GH_TOKEN", None)
        result = subprocess.run([str(objects / row["name"])], cwd=cwd, env=env, check=False)
        results.append({"key": row["key"], "sha256": row["sha256"], "status": result.returncode})
        status = status or result.returncode
    receipt = output / "receipt.json"
    receipt.write_text(json.dumps({
        "identity": provenance, "part": part, "results": results, "canonical": False,
        "runner": "direct-libtest", "arguments": [],
    }), encoding="utf-8")
    if report is not None:
        try:
            export_coverage(objects, rows, profiles, report, common_coverage_exclusions(root), root)
        except (ValueError, OSError, subprocess.SubprocessError):
            print("IAM coverage export failed", file=sys.stderr)
            return status, 1
        return status, 0
    export_coverage(objects, rows, profiles, output / "pilot.lcov", r"/\.cargo/registry/|/rustc/", root)
    return status


def export_coverage(objects, rows, profiles, destination, excludes, root):
    profdata = profiles.parent / "merged.profdata"
    raw = sorted(profiles.glob("*.profraw"))
    require(raw and all(p.stat().st_size for p in raw), "Missing pilot profiles")
    subprocess.run(["llvm-profdata", "merge", "-sparse", *map(str, raw), "-o", str(profdata)], check=True)
    binaries = [str(objects / row["name"]) for row in rows]
    with destination.open("x", encoding="utf-8") as out:
        subprocess.run(["llvm-cov", "export", "--format=lcov", f"--instr-profile={profdata}",
                        f"--ignore-filename-regex={excludes}", binaries[0],
                        *[f"--object={path}" for path in binaries[1:]]], stdout=out, check=True)
    require_iam_coverage(destination, root)


def common_coverage_exclusions(root):
    """Retain pinned cargo-llvm-cov Linux default exclusions for this workspace."""
    workspace = re.escape(root.as_posix())
    cargo_home = re.escape(Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo")).as_posix())
    rustup_home = re.escape(Path(os.environ.get("RUSTUP_HOME", Path.home() / ".rustup")).as_posix())
    return "|".join((
        r"/rustc/([0-9a-f]+|[0-9]+\.[0-9]+\.[0-9]+)/",
        rf"^{workspace}(/.*)?/(tests|examples|benches)/",
        rf"^{workspace}(/.*)?/(tests\.rs|[0-9A-Za-z_-]+[_-]tests\.rs)$",
        rf"^{workspace}/target($|/)",
        rf"^{cargo_home}/(registry|git)/",
        rf"^{rustup_home}/toolchains($|/)",
    ))


def coverage_replay(data, lane, shard, output, root=ci.ROOT, *, expected_lane, private_name):
    known = identity(root=root)
    require(known["lane"] == expected_lane, "Wrong IAM coverage lane")
    expected = ci.target_assignment(data, "iam", lane, shard, root)
    require(expected and len(expected) == len(set(expected)), "Invalid IAM coverage assignment")
    context = ci.coverage_context(os.environ)
    output = ci.coverage_destination(output)
    private = Path(os.environ["RUNNER_TEMP"]) / private_name
    private.mkdir(mode=0o700)
    part = f"{lane}-{shard}"
    acquired = private / "acquired"
    fetch(part, acquired, root)
    try:
        status, report_status = replay(acquired / "bundle.tar.gz", private / "replay", part,
                                       set(expected), root, report=output)
    except (ValueError, OSError, subprocess.SubprocessError):
        status, report_status = 1, 1
        print("IAM binary replay failed", file=sys.stderr)
    return ci.write_coverage_receipt(data, "iam", lane, shard, output, context, status, report_status, root)


def canonical_coverage(data, lane, shard, output, root=ci.ROOT):
    return coverage_replay(data, lane, shard, output, root, expected_lane="ci",
                           private_name="iam-coverage-private")


def fetch(part, destination, root=ci.ROOT):
    """Fetch only the trusted producer's exact same-attempt, digest-checked archive."""
    sys.path.insert(0, str(root / "ops/tests/k6/ci"))
    from artifacts import GitHub, positive, unwrap_zip, validate_artifact
    from binaries import ROLES

    require(re.fullmatch(r"unit-0|tests-[0-3]|native-e2e|e2e|oauth|ext-authz", part), "Unknown pilot artifact")
    known = identity(root=root)
    workflow_path, branch, lane = run_lane(os.environ)
    event = os.environ["GITHUB_EVENT_NAME"]
    if lane == "ci":
        expected_consumer = "e2e" if part in {"e2e", "oauth", "ext-authz"} else "coverage"
        require(os.environ.get("GITHUB_JOB") == expected_consumer, "Unexpected IAM artifact consumer")
    require(lane == "ab" or part != "native-e2e", "Benchmark artifact is not a CI input")
    if lane == "ab":
        arm = os.environ.get("IAM_AB_ARM")
        job = os.environ.get("GITHUB_JOB")
        if part in AB_E2E_PARTS:
            allowed = {"a": {"native-e2e", "oauth", "ext-authz"}, "b": {"e2e", "oauth", "ext-authz"}}
            require(job == "e2e" and part in allowed.get(arm, set()), "Wrong benchmark E2E input")
        else:
            require(job == "coverage" and arm == "b", "Wrong benchmark replay input")
    if lane == "ci" and part in {"e2e", "oauth", "ext-authz"}:
        require(event == "push" and branch == "master", "Current-CI IAM E2E requires push/master")
    run_id, attempt = (positive(known[key]) for key in ("GITHUB_RUN_ID", "GITHUB_RUN_ATTEMPT"))
    api = GitHub(known["repository"], os.environ["GH_TOKEN"])
    workflow = api.json(f"actions/workflows/{Path(workflow_path).name}")
    run = api.json(f"actions/runs/{run_id}/attempts/{attempt}")
    api_sha = run.get("head_sha")
    require(isinstance(api_sha, str) and re.fullmatch(r"[a-f0-9]{40}", api_sha), "Invalid run head")
    if event == "pull_request":
        # The API identifies the PR head; the checkout is GitHub's merge commit.
        checked_head = subprocess.check_output(["git", "rev-parse", "HEAD^2"], cwd=root, text=True).strip()
        require(api_sha in {known["GITHUB_SHA"], checked_head}, "Foreign PR head")
    else:
        require(api_sha == known["GITHUB_SHA"], "Foreign push head")
    require(workflow.get("path") == workflow_path and workflow.get("id") == run.get("workflow_id")
            and run.get("path") == workflow_path
            and run.get("event") == event and run.get("head_branch") == branch
            and run.get("id") == run_id and run.get("run_attempt") == attempt,
            "Foreign IAM pilot run")
    repo = run.get("repository") or {}
    require(repo.get("id") == positive(known["repository_id"]) and repo.get("full_name") == known["repository"],
            "Foreign IAM repository")
    if event == "push":
        head = run.get("head_repository") or {}
        require(head.get("id") == repo["id"] and head.get("full_name") == repo["full_name"], "Foreign push repository")
    job_name = {"oauth": "Pilot OAuth native build", "ext-authz": "Pilot Ext Authz native build"}.get(
        part, "Pilot IAM instrumented compile")
    if lane == "ci":
        job_name = ROLES[part] if part in {"oauth", "ext-authz"} else CI_PRODUCER
    elif lane == "ab":
        job_name = AB_PRODUCERS.get(part, AB_PRODUCERS["e2e"])[1]
    jobs = [j for j in api.pages(f"actions/runs/{run_id}/attempts/{attempt}/jobs", "jobs") if j["name"] == job_name]
    require(len(jobs) == 1 and jobs[0]["status"] == "completed" and jobs[0]["conclusion"] == "success"
            and jobs[0].get("run_id") == run_id and jobs[0].get("run_attempt") == attempt,
            "Pilot producer not successful")
    name = f"{'iam-ci' if lane == 'ci' else 'iam-pilot'}-{part}-{attempt}"
    if lane == "ci" and part in {"oauth", "ext-authz"}:
        name = f"k6-binaries-{part}-{run_id}-{attempt}"
    elif lane == "ab":
        prefix = "common" if part in {"oauth", "ext-authz"} else "a" if part == "native-e2e" else "b"
        name = f"iam-ab-{prefix}-{part}-{attempt}"
    items = [a for a in api.pages(f"actions/runs/{run_id}/artifacts", "artifacts") if a["name"] == name]
    require(len(items) == 1, "Missing or duplicate pilot artifact")
    # GitHub may expose the merge SHA in the artifact metadata and the head SHA
    # in the PR run. The embedded manifest always requires the exact merge SHA.
    origin_sha = items[0].get("workflow_run", {}).get("head_sha")
    require(origin_sha in {api_sha, known["GITHUB_SHA"]}, "Foreign IAM artifact SHA")
    record = validate_artifact(items[0], run_id, origin_sha)
    require(type(jobs[0].get("id")) is int and jobs[0]["id"] > 0, "Invalid producer job ID")
    record["workflow_id"] = workflow["id"]
    record["producer_job_id"] = jobs[0]["id"]
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


def benchmark_native(part, output, root=ci.ROOT):
    sys.path.insert(0, str(root / "ops/tests/k6/ci"))
    from binaries import elf, inventory

    known = identity(root=root)
    require(known["lane"] == "ab" and part in {"native-e2e", "oauth", "ext-authz"}
            and os.environ.get("GITHUB_JOB") == AB_PRODUCERS[part][0], "Wrong benchmark native producer")
    target = Path(os.environ["CARGO_TARGET_DIR"]).resolve()
    require(target == root / "target", "Wrong benchmark target directory")
    files = []
    for name in inventory(root, "iam" if part == "native-e2e" else part).values():
        path = target / "debug" / name
        elf(path)
        files.append((name, ".", name, path))
    output.mkdir(mode=0o700)
    bundle(output / "bundle.tar.gz", part_identity(known, part), part, files)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("operation", choices=("produce-e2e", "produce", "replay", "fetch", "companion", "benchmark-native"))
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--matrix", type=Path)
    parser.add_argument("--archive", type=Path)
    parser.add_argument("--part")
    parser.add_argument("--targets")
    args = parser.parse_args()
    if args.operation == "produce-e2e":
        produce_e2e(args.output)
        return 0
    if args.operation == "produce":
        produce(args.output, args.matrix)
        return 0
    if args.operation == "fetch":
        fetch(args.part, args.output)
        return 0
    if args.operation == "companion":
        companion(args.part, args.output)
        return 0
    if args.operation == "benchmark-native":
        benchmark_native(args.part, args.output)
        return 0
    targets = json.loads(args.targets)
    require(isinstance(targets, list) and len(set(targets)) == len(targets), "Invalid trusted matrix targets")
    return replay(args.archive, args.output, args.part, set(targets))


if __name__ == "__main__":
    raise SystemExit(main())
