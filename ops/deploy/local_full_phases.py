"""Lease/UID/private-input guards for the graph in local-full.just.

No workflow ordering, service roster, workload spec or replica policy lives here.
Receipts make an interrupted mutating phase non-replayable without arbitration.
"""
import copy
import hashlib
import ipaddress
import json
import os
import re
from pathlib import Path
import shutil
import subprocess
import sys
from types import SimpleNamespace
import uuid

import local_full_helm as helm
from local_full_declarations import identity as declarations_identity
from local_full_secrets import hydrate
from local_full_target import persist, private_file, unique_json


def input_stamp(args):
    paths = [value for key, value in vars(args).items() if isinstance(value, Path) and key not in
             {"lease", "output_dir", "environment_dir", "registry_state"}]
    result = []
    for path in paths:
        files = sorted(path.rglob("*")) if path.is_dir() else [path]
        for file in files:
            if file.is_dir():
                continue
            from local_registry import pki
            pki.checked_path(file)
            info = file.stat()
            result.append([str(file.resolve()), info.st_size, info.st_mtime_ns])
    return result


def validate_flags(args):
    oauth = bool(getattr(args, "oauth_connect", False))
    if getattr(args, "preserve_platform_tls", False) and (not oauth or getattr(args, "isolated_plugins", False)):
        raise ValueError("TLS preservation requires OAuthConnect without isolated plugins")
    if sum(bool(getattr(args, flag, False)) for flag in ("oauth_proof", "github_openbao", "oauth_connect")) > 1:
        raise ValueError("distinct explicit connector profiles required")


def inputs(api, args, run_id, *, validate_pki=True):
    validate_flags(args)
    oauth = bool(getattr(args, "oauth_connect", False))
    if args.github_input_file and not (args.github_openbao or oauth):
        raise ValueError("private GitHub input requires an explicit connector profile")
    if getattr(args, "simulated_input_file", None) and not oauth:
        raise ValueError("private simulated input requires --oauth-connect")
    if args.oauth_proof_input_file and not args.oauth_proof:
        raise ValueError("private proof input requires --oauth-proof")
    images = json.loads(args.image_manifest.read_text(encoding="utf-8"))
    mesh, identity, cert_hash = api.cert_inputs(args.cert_directory)
    proof = api.oauth_inputs(args.oauth_proof_input_file) if args.oauth_proof else None
    if proof and proof["run_id"] != run_id:
        raise ValueError("private proof run identity changed")
    github = api.github_inputs(args.github_input_file) if args.github_openbao or oauth else None
    simulated = api.github_inputs(args.simulated_input_file) if oauth else None
    from local_full_oauthconnect import seed_inputs
    seed = seed_inputs(github, simulated) if oauth else github
    tls = api.openbao_cert_inputs(args.cert_directory, run_id, args.github_openbao, oauth, validate=validate_pki)
    proof_tls, proof_hash = api.oauth_cert_inputs(args.cert_directory, run_id) if proof else ([], "")
    options = {"images": images, "cert_identity": cert_hash + proof_hash,
        "oauth_proof": args.oauth_proof, "github_openbao": args.github_openbao, "oauth_connect": oauth,
        "github_client_id": github["client_id"] if github else "offline-synthetic-client",
        "simulated_client_id": simulated["client_id"] if simulated else "offline-simulated-client",
        "public_ca": mesh["ca.crt"].decode("ascii"), "isolated_plugins": args.isolated_plugins,
        "plugin_ca_sha256": hashlib.sha256(identity["ca.crt"]).hexdigest() if args.isolated_plugins else ""}
    return options, mesh, identity, proof, seed, tls + proof_tls


def graph_predecessor(phase):
    """Read the authority from just itself; do not maintain a second phase graph."""
    matches = re.findall(r"^" + re.escape(phase) + r":(?: ([a-z-]+))?\s*$",
                         (helm.ROOT / "ops/deploy/local-full.just").read_text(encoding="utf-8"), re.MULTILINE)
    if len(matches) != 1:
        raise ValueError("unique declared just phase required")
    return matches[0] or "none"


def wait_workloads(api, objects):
    for obj in objects:
        if obj["kind"] == "Deployment":
            api.k("rollout", "status", "deployment/" + obj["metadata"]["name"], "-n",
                  obj["metadata"]["namespace"], "--timeout=300s")


def network_inputs(api, data):
    from local_full_network import control_plane
    nodes = json.loads(api.k("get", "nodes", "-l", "node-role.kubernetes.io/control-plane", "-o", "json"))["items"]
    node, address = control_plane(nodes)
    network = {"apiCidr": str(address) + "/32", "ingressCidr": ""}
    raw = api.k("get", "networkpolicy", "allow-envoy", "-n", api.CLUSTER, "--ignore-not-found", "-o", "json")
    if raw.strip():
        current = json.loads(raw)
        extra = [peer for rule in current["spec"].get("ingress", []) for peer in rule.get("from", []) if "ipBlock" in peer]
        if extra:
            attestation = data.get("localFullIngress")
            if (not attestation or attestation["nodeUid"] != node["metadata"]["uid"]
                    or attestation["policyUid"] != current["metadata"]["uid"]
                    or attestation["spec"] != current["spec"]
                    or node["metadata"].get("annotations", {}).get("projectcalico.org/IPv4IPIPTunnelAddr") != attestation["source"]):
                raise ValueError("existing ingress exception needs unchanged owned source attestation before foundation apply")
            network["ingressCidr"] = str(ipaddress.IPv4Address(attestation["source"])) + "/32"
        expected = next(o for o in helm.render(phase="foundation", values={"network": network})
                        if o["kind"] == "NetworkPolicy" and o["metadata"]["name"] == "allow-envoy")
        if current["spec"] != expected["spec"]:
            raise ValueError("ingress policy differs from attested declaration; no overwrite")
    return network


def preserved_inputs(api, args, data, options, run_id, tls, mesh):
    from local_full_tls import preserved_platform_tls, reuse_openbao_tls
    enabled = getattr(args, "preserve_platform_tls", False)
    reader = lambda name: json.loads(api.k("get", "deployment", name, "-n", api.CLUSTER, "-o", "json"))
    preserved = preserved_platform_tls(data, enabled=enabled, oauth_connect=args.oauth_connect, read_deployment=reader)
    refs = {}
    if enabled:
        ca = json.loads(api.k("get", "configmap", "openbao-public-ca", "-n", api.CLUSTER, "-o", "json"))
        if ca.get("data", {}).get("ca.crt", "").strip() != mesh["ca.crt"].decode("ascii").strip():
            raise ValueError("TLS preservation requires unchanged OpenBao public CA")
        original = api.renderer.render(run_id, **dict(options, secret_refs={}))
        tls, refs = reuse_openbao_tls(original, tls, data, reader)
    return preserved, tls, refs


def run(api, args, operation=None):
    validate_flags(args)
    if not operation:
        raise ValueError("deploy phase graph requires the original exclusive-operation capability")
    lock = args.lease.with_name(args.lease.name + ".local-full-lock")
    authority = json.loads(lock.read_text(encoding="utf-8"))
    if authority.get("operation") != operation or authority.get("action") != "deploy" or not private_file(args.lease):
        raise ValueError("private original lease and active operation token required")
    proof = api.oauth_inputs(args.oauth_proof_input_file) if args.oauth_proof else None
    run_id = proof["run_id"] if proof else uuid.uuid4().hex[:16]
    invocation = args.lease.with_name(args.lease.name + ".deployment-" + authority["operation"] + ".json")
    if invocation.exists() or invocation.is_symlink():
        raise ValueError("operation invocation already exists; no replay/overwrite")
    values = {key: {"path": str(value.resolve())} if isinstance(value, Path) else value for key, value in vars(args).items()}
    context = {"operation": authority["operation"], "args": values, "runId": run_id,
               "declarationsId": declarations_identity(),
               "graphSha256": hashlib.sha256((api.ROOT / "ops/deploy/local-full.just").read_bytes()).hexdigest(),
               "inputs": input_stamp(args), "completed": [], "state": "prepared"}
    persist(invocation, context)
    command = shutil.which("just")
    if not command:
        raise RuntimeError("just required for the local-full phase graph")
    # Closed, non-secret OS/tooling context. In particular no ambient kubeconfig,
    # Docker override, OAuth bearer, PYTHONPATH or dotenv reaches just/its children.
    public_environment = ("PATH", "PATHEXT", "SystemRoot", "WINDIR", "COMSPEC", "TEMP", "TMP", "HOME", "USERPROFILE",
        "HOMEDRIVE", "HOMEPATH", "APPDATA", "LOCALAPPDATA", "PROGRAMFILES", "PROGRAMFILES(X86)", "PROGRAMDATA",
        "LANG", "LC_ALL", "PYTHONUTF8", "PYTHONIOENCODING", "AIFORALL_HELM")
    env = {key: os.environ[key] for key in public_environment if key in os.environ}
    env.update(AIFORALL_LOCAL_FULL_INVOCATION=str(invocation), AIFORALL_LOCAL_FULL_PYTHON=sys.executable)
    result = subprocess.run([command, "--justfile", str(api.ROOT / "ops/deploy/local-full.just"),
        "--working-directory", str(api.ROOT), "run"], cwd=api.ROOT, env=env, check=False)
    if result.returncode:
        raise RuntimeError("local-full phase failed; incomplete private receipt retained; no replica/DB/KV rollback")
    receipt = json.loads(invocation.read_text(encoding="utf-8"))
    if receipt.get("state") != "complete":
        raise ValueError("local-full graph returned without final ACK")


def guarded_phase(api, invocation, phase, predecessor):
    if not private_file(invocation):
        raise ValueError("private original invocation required")
    context = json.loads(invocation.read_text(encoding="utf-8"), object_pairs_hook=unique_json)
    args = SimpleNamespace(**{key: Path(value["path"]) if isinstance(value, dict) and set(value) == {"path"} else value
                              for key, value in context["args"].items()})
    lock = args.lease.with_name(args.lease.name + ".local-full-lock")
    authority = json.loads(lock.read_text(encoding="utf-8"), object_pairs_hook=unique_json)
    expected = context["completed"][-1] if context["completed"] else "none"
    if (authority.get("operation") != context["operation"] or authority.get("action") != "deploy"
            or context["state"] not in {"prepared", "acknowledged"} or predecessor != expected
            or predecessor != graph_predecessor(phase)
            or context["graphSha256"] != hashlib.sha256((api.ROOT / "ops/deploy/local-full.just").read_bytes()).hexdigest()
            or context["declarationsId"] != declarations_identity()
            or phase in context["completed"] or input_stamp(args) != context["inputs"]):
        raise ValueError("phase context/predecessor/input changed; no mutation permitted")
    options, mesh, identity, proof, seed, tls = inputs(api, args, context["runId"], validate_pki=phase == "plan")
    data = json.loads(args.lease.read_text(encoding="utf-8-sig"))
    if data.get("context") != api.CONTEXT or data.get("cluster") != api.CLUSTER:
        raise ValueError("parent local runtime lease required")
    active = data.get("localFullOperation")
    if active and active["id"] != context["operation"] and active["state"] != "complete":
        raise ValueError("previous local-full mutation is unresolved; explicit parent arbitration required")
    if data.get("anchor_state") == "anchored":
        data = api.runtime.lease(args.lease)
    options.update(context.get("publicOptions", {}))
    objects = api.renderer.render(context["runId"], **options, phase=phase if phase in helm.PHASES else "final")
    context["state"] = "in-progress"
    context["phase"] = phase
    persist(invocation, context)  # Crash after this point is NOT a retryable phase.
    if input_stamp(args) != context["inputs"]:
        raise ValueError("private/public input changed during preflight")
    if phase != "plan":
        data["localFullOperation"] = {"id": context["operation"], "state": "in-progress", "phase": phase}
        api.runtime.save_lease(args.lease, data)
    if phase == "plan":
        # Validate the selected private mount/source in memory; only the declared
        # Secret envelopes are later transferred, and no private hash is public.
        hydrate(copy.deepcopy(objects), proof_inputs=proof, workload_ca=identity["ca.crt"])
        certificate_inputs = [entry for entry in context["inputs"] if Path(entry[0]).is_relative_to(args.cert_directory.resolve())]
        context["certificateInputs"] = certificate_inputs
        if not getattr(args, "preserve_platform_tls", False) and any(data.get("localSecretInputs", {}).get(api.CLUSTER + "/" + name)
                for name in ("platform-mesh-certs", "lazaret-identity-ca")):
            if data.get("localFullCertificateInputs") != certificate_inputs:
                raise ValueError("existing TLS sources are changed or unattested; no implicit certificate rotation/overwrite")
        for obj in objects:
            annotations = obj["metadata"].get("annotations", {})
            if obj["kind"] == "Secret" and annotations.get("aiforall.dev/attempt-bound") == "true":
                key = obj["metadata"]["namespace"] + "/" + obj["metadata"]["name"]
                if data.get("localSecretInputs", {}).get(key):
                    raise ValueError("create-only private attempt identity was already used; no overwrite or adoption")
        _, _, refs = preserved_inputs(api, args, data, options, context["runId"], tls, mesh)
        context.setdefault("publicOptions", {})["secret_refs"] = refs
        if args.isolated_plugins:
            if not args.namespace_wiring_confirmed:
                raise ValueError("isolated plugin namespace source/artifact confirmation required")
            capability_sources = {
                "services/Lazaret/configuration/src/lib.rs": ["pub namespace: String", "fn validate"],
                "services/Lazaret/application/src/invoke.rs": ["NamespacedDigestDnsPluginLocator", "try_new"],
                "services/Lazaret/setup/src/app.rs": ["NamespacedDigestDnsPluginLocator::try_new", "config.plugin_hop.namespace.clone()", "config.plugin_hop.validate()"],
                "crates/apparatus-reference-kv/src/bin/reference-kv-http.rs": ["LAZARET_CA_CERT_PATH", "add_root_certificate"]}
            for source, markers in capability_sources.items():
                if any(marker not in (api.ROOT / source).read_text(encoding="utf-8") for marker in markers):
                    raise ValueError("isolated plugin capability source is missing")
        if any("@sha256:" in image for image in options["images"].values()):
            if not args.registry_state:
                raise ValueError("OCI images require --registry-state")
            from local_registry import Registry
            Registry(args.registry_state).verify_mapping(options["images"])
        else:
            api.image_provenance.verify(options["images"])
    if phase == "target":
        api.bootstrap_local_kind.initialize(args.lease, api.CLUSTER, api.ROOT / "ops/deploy/kind/cluster-local-full.yaml", args.after_final_it)
        data = api.runtime.lease(args.lease)
    if phase in {"plan", "target"} and data.get("anchor_state") == "anchored":
        api.preflight_resources(objects, data)
        context.setdefault("publicOptions", {})["network"] = network_inputs(api, data)
    if phase == "target":
        api.verify_images(options["images"], args.registry_state)
        api.prepare_namespaces(args, data, objects)
        for namespace in (o["metadata"]["name"] for o in objects if o["kind"] == "Namespace"):
            active = api.k("get", "jobs", "-n", namespace, "-o", "jsonpath={.items[*].status.active}")
            if any(value not in {"0", ""} for value in active.split()):
                raise ValueError("active job remains; no implicit cleanup or concurrent migration")
    elif phase == "quiesce":
        existing = []
        preimages = []
        for obj in objects:  # The standard quiesce render contains applications only.
            namespace, name = obj["metadata"]["namespace"], obj["metadata"]["name"]
            raw = api.k("get", "deployment", name, "-n", namespace, "--ignore-not-found", "-o", "json")
            if raw:
                current = json.loads(raw)
                if not any(e.get("kind") == "Deployment" and e.get("name") == name and e.get("namespace") == namespace
                           and e.get("uid") == current["metadata"]["uid"] for e in data.get("ownedResources", [])):
                    raise ValueError("existing application Deployment UID not explicitly owned")
                existing.append((namespace, name))
                preimages.append({"namespace": namespace, "name": name, "uid": current["metadata"]["uid"],
                                  "resourceVersion": current["metadata"]["resourceVersion"], "replicas": current["spec"].get("replicas", 1)})
        if existing and not args.confirm_quiesce:
            raise ValueError("existing owned deployment requires --confirm-quiesce")
        context["quiescePreimages"] = preimages
        persist(invocation, context)
        quiescent = objects
        api.apply_owned([obj for obj in quiescent if (obj["metadata"]["namespace"], obj["metadata"]["name"]) in existing], args, data)
        for namespace, name in existing:
            obj = next(o for o in quiescent if o["metadata"]["name"] == name and o["metadata"]["namespace"] == namespace)
            selector = obj["spec"]["selector"]["matchLabels"]["app.kubernetes.io/name"]
            api.k("wait", "--for=delete", "pod", "-n", namespace, "-l", "app.kubernetes.io/name=" + selector, "--timeout=180s")
    elif phase == "foundation":
        from local_full_tls import omit_preserved_tls
        preserved, tls, refs = preserved_inputs(api, args, data, options, context["runId"], tls, mesh)
        if refs != context.get("publicOptions", {}).get("secret_refs", {}):
            raise ValueError("preserved TLS references changed after preflight")
        foundation = objects
        hydrate(foundation, proof_inputs=proof, workload_ca=identity["ca.crt"])
        foundation += tls + [api.secret("platform-mesh-certs", mesh), api.secret("lazaret-identity-ca", identity)]
        foundation = omit_preserved_tls(foundation, preserved)
        from local_full_network import api_preimages, apply_changes
        from local_full_environment import owned
        target = api.runtime.target()
        changes = api_preimages(target, options["network"]["apiCidr"].removesuffix("/32"), owned, partial=True)
        apply_changes(target, changes)
        for obj in foundation:
            if obj["kind"] == "NetworkPolicy" and obj["metadata"]["name"] == "allow-api" and any(
                    e.get("kind") == "NetworkPolicy" and e.get("name") == obj["metadata"]["name"]
                    and e.get("namespace") == obj["metadata"]["namespace"] for e in data.get("ownedResources", [])):
                continue  # Existing policies were reconciled only by UID/RV/spec CAS.
            if obj["kind"] == "Secret":
                key = obj["metadata"]["namespace"] + "/" + obj["metadata"]["name"]
                if data.get("localSecretInputs", {}).get(key) and obj["metadata"].get("annotations", {}).get("aiforall.dev/attempt-bound") == "true":
                    raise ValueError("private attempt envelope became occupied; no overwrite")
                if not data.get("localSecretInputs", {}).get(key):
                    acknowledged = api.runtime.create(obj, args, data)
                    if not acknowledged["metadata"].get("uid"):
                        raise ValueError("confidential envelope UID ACK missing")
                    data.setdefault("localSecretInputs", {})[key] = True
                    api.runtime.save_lease(args.lease, data)
            else:
                api.apply_owned([obj], args, data)
    elif phase in {"dependencies", "bootstrap", "applications-first", "final"}:
        desired = objects
        if phase == "final":
            data["localFullDeclarationsSha256"] = context["declarationsId"]
            if not getattr(args, "preserve_platform_tls", False):
                data["localFullCertificateInputs"] = context["certificateInputs"]
            desired = [obj for obj in desired if helm.category(obj) == "applications"]
        if phase == "bootstrap":
            # UID ACK precedes confidential input transfer. Complete is awaited below.
            for obj in desired:
                created = api.runtime.create(obj, args, data)
                seed_kind = obj["metadata"].get("annotations", {}).get("aiforall.dev/seed")
                if seed_kind:
                    if seed is None or not created["metadata"].get("uid"):
                        raise ValueError("seed Job/input ACK unavailable")
                    secret_name = next(v["secret"]["secretName"] for v in obj["spec"]["template"]["spec"]["volumes"] if v.get("secret"))
                    child = api.secret(secret_name, {"inputs.json": json.dumps(seed).encode()})
                    child["metadata"]["ownerReferences"] = [{"apiVersion": obj["apiVersion"], "kind": "Job", "name": obj["metadata"]["name"], "uid": created["metadata"]["uid"], "controller": True}]
                    acknowledged = api.runtime.create(child, args, data)
                    if not acknowledged["metadata"].get("uid"):
                        raise ValueError("seed private input UID ACK missing")
            for obj in desired:
                api.k("wait", "--for=condition=complete", "job/" + obj["metadata"]["name"], "-n", obj["metadata"]["namespace"], "--timeout=240s")
        else:
            api.apply_owned(desired, args, data)
            wait_workloads(api, desired)
    elif phase != "plan":
        raise ValueError("unknown guarded phase")
    context["completed"].append(phase)
    context["state"] = "complete" if phase == "final" else "acknowledged"
    persist(invocation, context)
    if data.get("anchor_state") == "anchored":
        data["localFullOperation"] = {"id": context["operation"], "state": context["state"], "phase": phase}
        if phase == "final":
            data["connectorProfile"] = "oauth-connect" if args.oauth_connect else "github-openbao" if args.github_openbao else "gitlab-proof" if args.oauth_proof else "base"
            if args.oauth_connect:
                data["publicEndpoint"] = "https://127.0.0.1:18080"
        api.runtime.save_lease(args.lease, data)


def main(api):
    if len(sys.argv) != 4 or not os.environ.get("AIFORALL_LOCAL_FULL_INVOCATION"):
        raise ValueError("phase requires the original private operation invocation")
    guarded_phase(api, Path(os.environ["AIFORALL_LOCAL_FULL_INVOCATION"]), sys.argv[2], sys.argv[3])
