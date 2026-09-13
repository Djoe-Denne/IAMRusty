# ADR-0406 : Les crates Apparatus livrées sont P0 (contrats + KV) ; Factory, host et gateway ne sont pas livrés

- Statut : Accepted
- Réalité : Partial
- Date : 2026-09-12
- Décideurs : Architecture AIForAll — photographie rétroactive du dépôt
- Jalon concerné : P0 (crates) — pas le host P5
- SuperSède : aucune — photographie du livré ; les invariants restent **0001–0005**
- SuperSédée par : —

`Accepted` ratifie une cible. Le champ `Réalité` indique séparément ce que le code du dépôt réalise. Ici **Partial** : P0 code présent, plateforme d’exécution absente.

## Contexte

Les ADR 0001–0005 fixent la **cible** Apparatus (binding Manifesto, contract-first, plugin untrusted, gateway, `VALID` ≠ `VERIFIED`). La tentation, en photographiant le workspace, est d’appeler « service Apparatus » tout dossier `apparatus-*`, ou de présenter le KV de référence / l’UI `schema` comme le host P5.

Au 12 septembre 2026 le dépôt livre des **crates de contrat et un KV de référence**, pas un host, pas une Factory, pas une gateway.

## Décision

1. **Livré P0** = `apparatus-contracts` + `apparatus-reference-kv` (harness in-process, `kv-v1`, **UI `schema`**, aucune capacité réseau). Ce n’est **pas** un service hexagonal RustyCog (0100).
2. **Non livrés** : Factory (Git→OCI), **host** UI, **gateway** de capacités, contrôleur / workers, CLI `check/dev/publish`, macro `#[manifesto::apparatus]`.
3. Les décisions de plateforme **ne sont pas re-écrites ici** : **0001** (binding Manifesto), **0002** (contrats avant Factory/host), **0003** (plugin untrusted), **0004** (gateway), **0005** (même protocole ; admission / `VALID` / `VERIFIED` / installabilité).
4. **`apparatus-events` n’est pas un crate « members ».** C’est le chemin **legacy** `component_status_changed` (`project_id` + `component_type`). Sans `binding_id` / génération, il n’affecte pas un binding managed (0001).
5. **UI `schema` ≠ host P5.** Le mode manifeste `schema` (et un bundle `sandbox` statique déclaré) qualifie le contrat ; origine, CSP, iframe et MessageChannel restent à décider avant P5 (0002).

P1 persistance Manifesto (table, backfill) est **0001** / handbook Manifesto, pas une preuve que le host existe.

## Conséquences

- On ne documente pas `apparatus-reference-kv` comme runtime de production ni comme host.
- Un dossier `apparatus-*` supplémentaire n’autorise pas à skip 0003–0005.
- `Accepted` sur 0001–0005 + cette photographie **Partial** : la cible tient ; Factory/host/gateway restent à construire.

## Alternatives rejetées

| Option | Pourquoi pas |
|---|---|
| « Apparatus = service hexagonal du workspace » | Pas de `setup` / préfixe HTTP plateforme ; 0100 les exclut déjà |
| Compter l’UI schema comme host P5 | 0002 borne explicitement l’isolation UI à une ADR avant P5 |
| Fusionner cette ADR avec 0002 | 0002 est la cible contract-first ; 0406 est l’inventaire **livré vs pas livré** |

## Non décidé ici

- `APP-01` … `APP-07` (moteur, publishers, rétention, …).
- Lease / fencing / génération (avant P2).
- Qui publie / installe (`APP-02`).

## Références

- ADR : [0001](0001-apparatus-binding-owned-by-manifesto.md), [0002](0002-apparatus-contract-first.md), [0003](0003-apparatus-untrusted-plugin.md), [0004](0004-apparatus-capability-gateway.md), [0005](0005-apparatus-same-protocol-valid-verified.md)
- Wiki : `projects/manifesto/concepts/apparatus-p0-contracts`, `projects/manifesto/decisions/index.md`
- Code : `apparatus-contracts/`, `apparatus-reference-kv/`, `apparatus-events/src/lib.rs` (`ComponentStatusChanged` seulement)
- Preuve partielle : tests P0.1 `cargo test -p apparatus-contracts --features test-harness` + `cargo test -p apparatus-reference-kv` ; absence de crates Factory/host/gateway dans le workspace. Isolation T1 (2026-09-12) : `component_status_changed` sans `binding_id` n'altère pas un binding managed (`Manifesto/tests/apparatus_p2_t1_events.rs`)
