# Tests end-to-end

Use [the hosted k6 campaign](k6/ci/README.md) for HTTPS health, browser OAuth,
and Envoy SAN on an isolated stack. An authorized reviewer can request it on an
internal PR with `/k6-e2e`; the run requires that PR's published Linux binaries.
Local Linux/WSL users can [run a standard scenario directly](k6/scenarios/README.md)
against a prepared isolated target. The Windows k6 adapters are retired. The
Python tests here verify the harness; they are not a second application suite.

## Contrôles offline de la CI

`infra-static.sh` vérifie la syntaxe shell et le YAML source, rend les huit
inventaires Kustomize déclarés, puis contrôle le chart Kind minimal avec Helm.
Les répertoires `templates/` et `files/` d'un chart identifié par `Chart.yaml`
sont des entrées du renderer, pas du YAML brut ; ses métadonnées et valeurs
restent vérifiées comme YAML.

`test_ci_render.py` renders the hosted fixture chart with public synthetic
inputs and checks its admission failures. It uses `AIFORALL_HELM` or Helm on
`PATH`; it does not start a cluster or need secrets, kubeconfig, or a private CA.
