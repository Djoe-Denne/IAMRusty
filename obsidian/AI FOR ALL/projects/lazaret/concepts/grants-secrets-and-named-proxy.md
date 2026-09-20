---
title: >-
  Grants, secrets opaques et proxy nommé
category: concepts
tags: [architecture, components, visibility/internal]
sources:
  - docs/adr/0007-apparatus-p3-capability-boundary-after-accept.md
  - Lazaret/domain/src/grants.rs
  - Lazaret/domain/src/secrets.rs
  - Lazaret/domain/src/connectors.rs
  - Lazaret/domain/src/kv.rs
  - Lazaret/infra/src/secrets_deny.rs
  - Lazaret/infra/src/vault.rs
  - Lazaret/tests/apparatus_p3_t12_openbao.rs
summary: >-
  Grant = intersection live Manifesto. T12 : OpenBao produit pin 2.6.2 +
  testcontainer ; T6 IT reste wiremock (preuve protocole ≠ produit).
provenance:
  extracted: 0.86
  inferred: 0.12
  ambiguous: 0.02
created: 2026-09-17T10:55:00Z
updated: 2026-09-20T10:35:00Z
---

# Grants, secrets opaques et proxy nommé

Hub : [[projects/lazaret/lazaret]]. Identité : [[projects/lazaret/concepts/workload-identity]]. Canon : [[projects/manifesto/decisions/0007-apparatus-p3-lazaret]].

## Grants à l’appel

`evaluate_grant` est pur : origine d’appel + snapshot live Manifesto (`BindingGrantSnapshot`, consentement, membership). Crypto-valid session ≠ autorisation. Un fetch Manifesto en échec est un refus (fail-closed).

Consent write / révocation : close-at-commit côté Manifesto (preuve T5). Le plugin ne peut pas réutiliser un grant révoqué si le snapshot est frais. ^[inferred]

## KV plateforme

Port async derrière `apparatus_contracts::KvStore`. Adapters Postgres + Redis (T6). Namespacing par binding. Le plaintext des secrets **n’entre pas** dans le KV plugin.

P3-close livré : `kv_purge` sur Manifesto `component_removed` (file `lazaret-kv-events`, T8, land `f0cf1d2`). 0007 **reste Partial**. ^[extracted]

## Secrets

Forme unique : `secret:{path}#{field}`. Path et field = `^[A-Za-z0-9/_-]+$` ; `.`, `..`, `//`, `://` rejetés. Injection seulement dans une opération **déjà accordée**.

Si `[vault]` est vide : `DeniedSecretResolver` échoue toujours (`ResolveFailed`).

### T12 — OpenBao produit (`4e645d5`)

Trou OpenBao **produit hors compose** **fermé**. Image pin `openbao/openbao:2.6.2` dans docker-compose **et** testcontainer service-local `lazaret_test-openbao`. KV v2 `secret/`. Mode `-dev` écoute `0.0.0.0:8200`. Services compose `openbao` + `openbao-seed`. Preuve : `Lazaret/tests/apparatus_p3_t12_openbao.rs`.

T6 IT **reste wiremock**. Preuve protocole (forme `secret:path#field`, deny, adaptateur HTTP) ≠ preuve produit (image pin + seed + KV v2 réel). Les deux couches doivent rester distinctes. ^[inferred]

Pattern testcontainer : [[concepts/integration-testing-with-real-infrastructure]].

## Proxy nommé

Le plugin passe un **nom** admis par l’opérateur, jamais une URL. `ConnectorRegistry` mappe nom → base URL ; `join_path` refuse de s’échapper du préfixe. Sortie via `ConnectorProxy`.

## Invoke

Handler HTTP Lazaret, DTO P0 et `INVOKE_PATH` (`/invoke`) réutilisés. Ce n’est **pas** une méthode `ApparatusRuntime` (0006 G encore en vigueur dans Manifesto). Les 5 routes `/components` restent gelées (0006 E : pas de 202).

## Related

- [[projects/manifesto/concepts/apparatus-capabilities-and-isolation]]
- [[projects/manifesto/concepts/apparatus-p2-reconciliation]]
- [[journal/2026-09-20]]
