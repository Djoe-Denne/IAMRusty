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
  - Lazaret/tests/apparatus_p3_t11b_session_mtls.rs
  - Lazaret/tests/apparatus_p3_t12_openbao.rs
  - Lazaret/tests/apparatus_p3_t13_ca_persist.rs
summary: >-
  BC P3 Implemented T1–T14b (A-DEC 2026-09-20). Hors-jalon encore ouverts :
  APP-05, 0006 G/E, pas K8s, pas de 2e protocole.
provenance:
  extracted: 0.84
  inferred: 0.12
  ambiguous: 0.04
created: 2026-09-17T10:55:00Z
updated: 2026-09-20T12:14:00Z
---

# Lazaret

Slice hexagonale RustyCog introduite dans `e978cd0` (2026-09-16). Nom Accepted : dossier `Lazaret`, package `lazaret-service` — analogie de quarantaine, pas un template générique. Canon : [[projects/manifesto/decisions/0007-apparatus-p3-lazaret]]. Hub plateforme : [[projects/aiforall/aiforall]]. Alias historique de la plateforme : [[entities/paravretius]] (zéro hit code). Mesh inter-services : [[projects/aiforall/concepts/https-platform-mesh]].

## Indexes

- [[projects/lazaret/concepts/index]] — identité, grants, secrets, proxy, CA, OpenBao
- [[projects/lazaret/skills/index]] — tests P3

## Rôle

Lazaret est la **frontière serveur** visée par ADR-0004 : autorisation à l’appel, identité de **workload**, KV namespacé, secrets opaques, connecteurs nommés, `invoke` HTTP. Ce n’est **pas** Manifesto (propriétaire du binding) et **pas** IAM (émetteur utilisateur). Composition : standalone **et** monolithe, comme Hive / Manifesto, sans fusionner les domaines.

Préfixe HTTP : `/lazaret`. Compose : hôte **8084** (`8084:8080`). TLS compose : `tls_port` 8080, volume `./Lazaret/certs:/app/certs`, HEALTHCHECK `curl -fk https://localhost:8080/lazaret/health` (T13). Base : `lazaret_dev` (Postgres propre au BC). OpenFGA : aucun type ce slice (`InMemoryPermissionChecker`).

## Ce qui est livré (Implemented)

- Santé : `GET /lazaret/health`, `GET /lazaret/ready`.
- Identité hybride T3 : CSR → certificat client (CA `platform-internal-ca`), puis jeton de session `iss`/`aud`=`lazaret` (EdDSA). Routes `POST /lazaret/enroll`, `POST /lazaret/session`. JWT IAM `[auth.jwt]` = consommateur rustycog pour `AppState` seulement — **pas** l’identité workload. T10 persist Postgres. T11b session sous mTLS optionnel. T13 CA persist + leaf serveur. Détail : [[projects/lazaret/concepts/workload-identity]].
- Grants, consentement, KV, secrets-by-ref, proxy nommé, invoke : [[projects/lazaret/concepts/grants-secrets-and-named-proxy]]. T12 OpenBao **produit** ; T6 IT reste wiremock.
- Tests T3–T13 sous `Lazaret/tests/apparatus_p3_t*.rs`. T8 `kv_purge` landé `f0cf1d2`. T10 landé `9e85edd`. T11b `d605377`. T12 `4e645d5`. T13 `311e0ab`. Skill : [[projects/lazaret/skills/running-apparatus-p3-tests]].
- T14b mesh HTTPS Hive–IAM–Telegraph : [[projects/aiforall/concepts/https-platform-mesh]] (`a27ea5b`).

## Ce qui reste hors-jalon

ADR-0007 est **Accepted / Implemented** (A-DEC 2026-09-20). Holes **fermés** : mTLS `/session` (T11b), OpenBao produit (T12), persistance CA + TLS Lazaret compose (T13), mTLS Hive–IAM–Telegraph comme HTTPS + CA optionnelle (T14b). Hors-jalon **encore ouverts** : APP-05 ; **0006 G et E** (pas d’`invoke` sur `ApparatusRuntime` ; pas de 202 / nouvelle registration sur les 5 routes `/components` gelées) ; pas K8s ; pas de second protocole. Factory, host UI : P4+.

## Related

- [[projects/manifesto/manifesto]] — snapshot de grants / consentement, ticker P2
- [[projects/manifesto/concepts/apparatus-p2-reconciliation]]
- [[projects/manifesto/concepts/apparatus-platform]]
- [[entities/paravretius]] — nom historique, pas dans le code
- [[projects/iamrusty/iamrusty]] — JWT utilisateur, distinct de la session Lazaret
- [[projects/aiforall/concepts/orchestrator-agent-harness]]
- [[journal/2026-09-20]]
