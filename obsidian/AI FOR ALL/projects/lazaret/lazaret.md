---
title: >-
  Lazaret
category: project
tags: [platform, rust, components, visibility/internal]
sources:
  - Lazaret/README.md
  - docs/services/lazaret.md
  - docs/adr/0007-apparatus-p3-capability-boundary-after-accept.md
  - Lazaret/src/main.rs
  - Lazaret/http/src/lib.rs
summary: >-
  Bounded context P3 : frontière de capacités et de données, distincte de
  Manifesto. Préfixe /lazaret, port 8084. Réalité Partial (T1–T9).
  T9 prouve POST /lazaret/invoke sur prefixed_router. Hole `kv_purge`
  fermé (`component_removed`) ; autres holes 0007 ouverts.
provenance:
  extracted: 0.82
  inferred: 0.14
  ambiguous: 0.04
created: 2026-09-17T10:55:00Z
updated: 2026-09-17T17:40:00Z
---

# Lazaret

Slice hexagonale RustyCog introduite dans `e978cd0` (2026-09-16). Nom Accepted : dossier `Lazaret`, package `lazaret-service` — analogie de quarantaine, pas un template générique. Canon : [[projects/manifesto/decisions/0007-apparatus-p3-lazaret]]. Hub plateforme : [[projects/aiforall/aiforall]].

## Indexes

- [[projects/lazaret/concepts/index]] — identité, grants, secrets, proxy
- [[projects/lazaret/skills/index]] — tests P3

## Rôle

Lazaret est la **frontière serveur** visée par ADR-0004 : autorisation à l’appel, identité de **workload**, KV namespacé, secrets opaques, connecteurs nommés, `invoke` HTTP. Ce n’est **pas** Manifesto (propriétaire du binding) et **pas** IAM (émetteur utilisateur). Composition : standalone **et** monolithe, comme Hive / Manifesto, sans fusionner les domaines.

Préfixe HTTP : `/lazaret`. Compose : hôte **8084** (`8084:8080`). Base : `lazaret_dev` (Postgres propre au BC). OpenFGA : aucun type ce slice (`InMemoryPermissionChecker`).

## Ce qui est livré (Partial)

- Santé : `GET /lazaret/health`, `GET /lazaret/ready`.
- Identité hybride T3 : CSR → certificat client (CA `platform-internal-ca`), puis jeton de session `iss`/`aud`=`lazaret` (EdDSA). Routes `POST /lazaret/enroll`, `POST /lazaret/session`. JWT IAM `[auth.jwt]` = consommateur rustycog pour `AppState` seulement — **pas** l’identité workload. Détail : [[projects/lazaret/concepts/workload-identity]].
- Grants, consentement, KV, secrets-by-ref, proxy nommé, invoke : [[projects/lazaret/concepts/grants-secrets-and-named-proxy]].
- Tests T3–T9 sous `Lazaret/tests/apparatus_p3_t*.rs`. T9 prouve `POST /lazaret/invoke` sur `prefixed_router`. Skill : [[projects/lazaret/skills/running-apparatus-p3-tests]].

`Lazaret/README.md` dit encore « Slice T2 : health / ready uniquement » — **périmé** vis-à-vis du code et de l’ADR (T1–T9 Partial). ^[ambiguous]

## Ce qui reste hors livré

Holes listés dans le canon ADR-0007 : mTLS complete, OpenBao produit, APP-05, **0006 G et E** (pas d’`invoke` sur `ApparatusRuntime` ; pas de 202 / nouvelle registration sur les 5 routes `/components` gelées). Factory, host UI, gateway K8s : P4+. Hole `kv_purge` **fermé** : `component_removed` sur file dédiée `lazaret-kv-events` (T8). ADR-0007 reste Partial.

## Related

- [[projects/manifesto/manifesto]] — snapshot de grants / consentement, ticker P2
- [[projects/manifesto/concepts/apparatus-p2-reconciliation]]
- [[projects/manifesto/concepts/apparatus-platform]]
- [[projects/iamrusty/iamrusty]] — JWT utilisateur, distinct de la session Lazaret
- [[projects/aiforall/concepts/orchestrator-agent-harness]]
