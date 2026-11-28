"""Run-owned ephemeral TLS material for the closed CI fixture names.

The caller owns the private output directory and pinned OpenSSL executor.
No operator PKI, trust-store mutation, network access or Kubernetes deployment.
"""
from pathlib import Path
import re
import secrets

COMPANIONS = ("hive", "manifesto", "telegraph")
ORGANIZATION_SLUG = "k6-scope"


def mesh_hostname(run_id, service):
    return f"k6-{run_id}-mesh-{service}.aiforall-local-full.svc.cluster.local"


def identities(run_id, *, companion_mesh=False):
    if not isinstance(run_id, str) or not re.fullmatch(r"[a-f0-9]{32}", run_id):
        raise ValueError("PKI_RUN_ID")
    suffix = ".aiforall-local-full.svc.cluster.local"
    name = "k6-" + run_id
    names = {
        "iam-service": ["DNS:iam" + suffix, "DNS:" + name + "-iam" + suffix],
        # Same CA, deliberately no IAM SAN, for the causal negative phase.
        "hive-service": ["DNS:hive-service"],
        "envoy-mesh": ["DNS:localhost", "IP:127.0.0.1", "DNS:envoy-mesh",
                       "DNS:" + name + "-envoy" + suffix],
        "oauth": ["DNS:" + name + "-oauth" + suffix],
        "idp": ["DNS:" + name + "-idp" + suffix, "DNS:localhost", "IP:127.0.0.1"],
        "ext-authz": ["DNS:ext-authz", "DNS:ext-authz" + suffix],
        "late-check": ["DNS:late-check" + suffix],
        "postgres": ["DNS:postgres", "DNS:postgres" + suffix],
        # v1.8.5's HTTP gateway dials the configured loopback gRPC listener
        # with hostname verification, using the same server certificate.
        "openfga": ["DNS:openfga", "DNS:openfga" + suffix, "IP:127.0.0.1"],
        "localstack": ["DNS:localstack", "DNS:localstack" + suffix, "DNS:localhost", "IP:127.0.0.1"],
    }
    if companion_mesh:
        names.update({f"mesh-{service}": ["DNS:" + mesh_hostname(run_id, service)] for service in COMPANIONS})
    return names


def generate(directory, run_id, execute, *, companion_mesh=False):
    """Create fresh material only; execute accepts an OpenSSL argument list.

    Refuse existing paths rather than overwrite a key. All private files are
    covered by the caller's umask/private root; permission errors stay fatal.
    """
    names = identities(run_id, companion_mesh=companion_mesh)
    root = Path(directory)
    if not root.is_absolute() or root.is_symlink() or not root.is_dir():
        raise ValueError("PKI_PRIVATE_DIRECTORY")
    signers = ["platform"]
    if companion_mesh:
        signers.append("organization")
    paths = [root / name for name in ("ca.key", "ca.crt")]
    paths += [root / f"{signer}-signing.{extension}" for signer in signers for extension in ("pem", "pub")]
    paths += [root / (name + extension) for name in names for extension in (".key", ".csr", ".crt", ".ext")]
    if any(path.exists() or path.is_symlink() for path in paths):
        raise ValueError("PKI_OUTPUT_COLLISION")
    # The canonical path check also rejects symlinked parents.
    if root != root.resolve():
        raise ValueError("PKI_PRIVATE_DIRECTORY")
    execute(["req", "-x509", "-newkey", "rsa:3072", "-noenc", "-sha256", "-days", "1",
             "-subj", "/CN=k6-ci-" + run_id, "-addext", "basicConstraints=critical,CA:TRUE,pathlen:0",
             "-addext", "keyUsage=critical,keyCertSign,cRLSign",
             "-keyout", str(root / "ca.key"), "-out", str(root / "ca.crt")])
    for name, sans in names.items():
        extension = root / (name + ".ext")
        extension.write_text("basicConstraints=critical,CA:FALSE\n"
                             "keyUsage=critical,digitalSignature,keyEncipherment\n"
                             "extendedKeyUsage=serverAuth,clientAuth\n"
                             "subjectAltName=" + ",".join(sans) + "\n", encoding="ascii")
        extension.chmod(0o600)
        execute(["req", "-new", "-newkey", "rsa:2048", "-noenc", "-sha256", "-subj", "/CN=" + name,
                 "-keyout", str(root / (name + ".key")), "-out", str(root / (name + ".csr"))])
        execute(["x509", "-req", "-sha256", "-days", "1", "-in", str(root / (name + ".csr")),
                 "-CA", str(root / "ca.crt"), "-CAkey", str(root / "ca.key"),
                 "-set_serial", "0x" + secrets.token_hex(16), "-extfile", str(extension),
                 "-out", str(root / (name + ".crt"))])
    for signer in signers:
        private, public = root / f"{signer}-signing.pem", root / f"{signer}-signing.pub"
        execute(["genpkey", "-algorithm", "RSA", "-pkeyopt", "rsa_keygen_bits:3072", "-out", str(private)])
        execute(["pkey", "-in", str(private), "-pubout", "-out", str(public)])
    for path in paths:
        if not path.is_file() or path.is_symlink():
            raise ValueError("PKI_OUTPUT_MISSING")
        path.chmod(0o600)
