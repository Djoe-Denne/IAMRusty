# Calico local overlay

The unmodified `base/calico.yaml` is Calico **3.29.7**, fetched from the official
repository at commit `82fd18c202ac5400cd7760597a510de9d028b035` (tag `v3.29.7`).
`provenance.json` records its byte count, HTTPS source and locally computed
SHA256; this is not a claim of an upstream-signed checksum. Preserve raw bytes.

`kubectl kustomize ops/deploy/third-party/calico` renders offline. The only local
spec changes are `CALICO_IPV4POOL_CIDR=10.244.0.0/16` and
`FELIX_IGNORELOOSERPF=true` on `kube-system/calico-node`. No remote Kustomize
resource, network fetch, operator or new version is involved. Bootstrap verifies
the vendored checksum before rendering and applies through the existing lease
authority only after its topology/CNI guards. Existing CNI reuse remains no-op.
