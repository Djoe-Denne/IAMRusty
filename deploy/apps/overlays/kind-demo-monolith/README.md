# Overlay démo J3 — `oodhive-monolith` sur kind `aiforall-local`

**Pas le canon cluster.** [ADR-0601](../../../../docs/adr/0601-cluster-trust-namespaces-standalones.md) reste 4+1 standalones. [ADR-0604](../../../../docs/adr/0604-j3-overlay-demo-monolith-kind-invoke.md) (Proposed) enregistre cet écart de séquence.

Distinct de [`../kind`](../kind) (preuve stub 0603 : nginx `ok\n`). Ne pas fusionner. `just deploy-m2` applique encore `../kind`.

Image : `aiforall-oodhive-monolith:j3` (`monolith/Dockerfile`, rustc **1.94**-slim-bookworm — Cargo.lock aws-sdk exige ≥ 1.91.1 ; J2 controller reste 1.89).

`deploy-j3.ps1` pinne `*_DATABASE__HOST` / `*_OPENFGA__HOST` sur l’IP `host.docker.internal` du nœud kind (sqlx/ndots). Job probe : `j3-curl:local` (`Dockerfile.curl`, `kind load`) parce que busybox wget n’écrit pas le corps HTTP 4xx.

## Infra Compose (pas in-kind)

Postgres et OpenFGA restent sur l’hôte : `just up-infra`. Depuis le pod kind (Docker Desktop Windows) :

- DNS : `host.docker.internal`
- `hostAliases` sur le Deployment (IP lue depuis le nœud kind, défaut Docker Desktop `192.168.65.254`)
- Env `*_DATABASE__HOST` / `*_OPENFGA__HOST` = `host.docker.internal` (placeholders J1)

Pas d’extraPortMapping, pas de hostNetwork, pas de preuve via localhost hôte.

## Preuve

Index :
- `just prove-gold` = nominal Kind 0605
- `just prove-j1` / `monolith-prove` = host J1
- `monolith/prove-e2e-curl.ps1` = curl hôte ≠ preuve

Job `invoke-probe-j3` dans `aiforall-plugins` : **POST** `http://lazaret.aiforall-gateway.svc.cluster.local:8080/lazaret/invoke` → **401** `{"error":"unauthorized"}` (preuve réseau, pas le gold path).

Gold path Kind (ADR-0605) : `just prove-gold` — signup → POST components (catalogue pin digest) → T5 consents → VALID Adm-A → Pod+Service `plugin-{32hex}` → enroll workload → session mTLS → `kv.get` **200** in-cluster. Curl hôte `:8080` n'est **pas** la preuve.

`LAZARET_PLUGIN_HOP__ENDPOINT_URL` et `reference-kv-plugin-service.yaml` = **dette** (hors kustomization nominale). Formule : `LAZARET_PLUGIN_HOP__USE_DNS_FORMULA=true`. Job `lazaret-enroll-reference-kv` = secours, pas l'étape G4.

En prod, remplacer le htpasswd zot par le robot du registre cible ; les env `APPARATUS_REGISTRY_*` restent les mêmes.

Recette overlay : `just deploy-j3`.
