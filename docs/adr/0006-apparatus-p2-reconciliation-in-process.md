# ADR-0006 : La réconciliation Apparatus P2 est un contrôleur in-process de Manifesto, sans infrastructure réelle

- Statut : Accepted
- Réalité : Partial
- Date : 2026-09-12
- Décideurs : Architecture AIForAll — Accept explicite utilisateur (« Je te fais confiance », 2026-09-12) ; checklist A–M figée par l’orchestrateur après avis expert-engineer
- Jalon concerné : P2
- SuperSède : aucune
- SuperSédée par : —

`Accepted` ratifie la cible ci-dessous. `Réalité : Partial` : T1 (isolation events, ADR-0001) est livré ; T2–T7 se mesurent aux preuves de tests, pas à cette ratification.

## Contexte

P1 a livré `apparatus_bindings` en extension 1:1 : surrogate `id`, `component_id` UUID UNIQUE FK `fk_apparatus_bindings_component` ON DELETE CASCADE, `digest` VARCHAR(128) NULL (désiré non résolu), `source` ∈ `legacy|managed`. Aucune colonne de génération, lease, fencing, observed.

ADR-0001 : une ADR dédiée devait figer desired/observed, lease et fencing avant P2 ; `apparatus-events` (`project_id + component_type`) reste le chemin **legacy** ; un événement dépourvu de `binding_id` et de génération n’affecte jamais un binding managed.

Le wiki (bindings, plan P2) reste **conception**. Cette ADR nomme le schéma et les ports ; elle ne copie pas le wiki.

## Décision

P2 vit **dans le process Manifesto** (standalone et monolithe) : ticker + scan DB, pas de nouveau service, pas de broker réel, pas d’adaptateur Kubernetes, pas de gateway réseau. Invariants P1 inchangés (identité = `project_components.id`, 1:1, pas de second UUID public, tuples `component:{id}`, 5 routes `/components`, events ownership inchangés, `source` legacy|managed, zéro nouveau type FGA).

### A — Génération

Compteur monotone `desired_generation` **par binding**. **Incrémenté seulement par la commande** métier existante (add/update/remove via `/components`, `source=managed`) : CAS `UPDATE … SET desired_generation = desired_generation + 1 WHERE component_id = $id AND desired_generation = $attendu`. Le ticker **n’incrémente jamais**. Scan worker : `source = 'managed'` seulement ; **legacy ne bump jamais**. `component_status_changed` managed sans `binding_id` **ou** sans génération **reste ignoré** (ADR-0001 / T1). P2 **n’ajoute pas** de champ génération sur l’event et ne s’en sert pas comme gâchette.

### B — Lease

Lease **par binding**. TTL constante **30 s** (`APPARATUS_LEASE_TTL`, injectable en tests). À expiry, un autre worker **peut voler** (CAS `lease_epoch = lease_epoch + 1`). **Pas de heartbeat** : renouvellement uniquement en reprenant le travail (`lease_expires_at = now() + TTL`). Tick **2 s** (`APPARATUS_TICK_INTERVAL` ≪ TTL). `lease_owner` = id de process (VARCHAR, **pas** un UUID public).

### C — Fencing

**Les deux** : `desired_generation` (observation périmée refusée) **et** `lease_epoch` (writer périmé, même génération). Écriture observed/retry refusée si `UPDATE … WHERE desired_generation = $g AND lease_epoch = $e AND lease_owner = $me AND lease_expires_at > now()` affecte 0 ligne.

### D — Schéma SQL (figé à l’Accept)

#### `apparatus_bindings` — existant (inchangé)

| Colonne | Type | Null | Défaut | Contrainte |
|---|---|---|---|---|
| `id` | `BIGSERIAL` | NOT NULL | identity | PK interne |
| `component_id` | `UUID` | NOT NULL | — | UNIQUE, FK CASCADE → `project_components.id` |
| `digest` | `VARCHAR(128)` | NULL | NULL | digest **désiré** |
| `source` | `VARCHAR(20)` | NOT NULL | — | CHECK `legacy\|managed` |

#### `apparatus_bindings` — additif P2

| Colonne | Type | Null | Défaut | Contrainte |
|---|---|---|---|---|
| `desired_generation` | `BIGINT` | NOT NULL | `0` | CHECK `>= 0` ; 0 = jamais commandé |
| `observed_generation` | `BIGINT` | NOT NULL | `0` | CHECK `>= 0` **et** `<= desired_generation` |
| `observed_digest` | `VARCHAR(128)` | NULL | NULL | dernier digest appliqué |
| `lease_epoch` | `BIGINT` | NOT NULL | `0` | CHECK `>= 0` ; +1 à chaque claim |
| `lease_owner` | `VARCHAR(64)` | NOT NULL | `''` | `''` = jamais claim |
| `lease_expires_at` | `TIMESTAMPTZ` | NULL | NULL | NULL = unowned |
| `next_retry_at` | `TIMESTAMPTZ` | NULL | NULL | dû si `NULL` ou `<= now()` ; terminal = `NULL` **et** `last_error_code IS NOT NULL` |
| `retry_count` | `INTEGER` | NOT NULL | `0` | CHECK `>= 0` |
| `last_error_code` | `VARCHAR(64)` | NULL | NULL | pas de secrets |

CHECK owned : `(lease_owner = '' AND lease_expires_at IS NULL) OR (lease_owner <> '' AND lease_expires_at IS NOT NULL)`.

Index : `idx_apparatus_bindings_managed_due` **partiel** `(next_retry_at) WHERE source = 'managed' AND desired_generation > observed_generation` ; `idx_apparatus_bindings_lease_expiry` **partiel** `(lease_expires_at) WHERE source = 'managed' AND lease_owner <> ''`.

`operation_id` P0 : **dérivé** (`component_id` + génération + verbe), **pas** de colonne SQL.

#### Table interne `apparatus_cleanup_jobs` (pas un UUID public)

| Colonne | Type | Null | Défaut | Contrainte |
|---|---|---|---|---|
| `id` | `BIGSERIAL` | NOT NULL | identity | PK interne |
| `component_id` | `UUID` | NOT NULL | — | copie, **pas de FK** (survit au CASCADE) |
| `project_id` | `UUID` | NOT NULL | — | copie, **pas de FK** |
| `desired_generation` | `BIGINT` | NOT NULL | — | snapshot (clé d’idempotence) |
| `digest` | `VARCHAR(128)` | NULL | NULL | digest à démonter |
| `next_retry_at` | `TIMESTAMPTZ` | NULL | NULL | même sémantique |
| `retry_count` | `INTEGER` | NOT NULL | `0` | CHECK `>= 0` |
| `last_error_code` | `VARCHAR(64)` | NULL | NULL | |
| `completed_at` | `TIMESTAMPTZ` | NULL | NULL | NULL = ouvert ; CAS de fin |
| `created_at` | `TIMESTAMPTZ` | NOT NULL | `now()` | |

Pas de lease sur cette table : `teardown` idempotent + `UPDATE … SET completed_at = now() WHERE id = $id AND completed_at IS NULL`.

Index : `uq_apparatus_cleanup_jobs_open` UNIQUE partiel `(component_id) WHERE completed_at IS NULL` ; `idx_apparatus_cleanup_jobs_due` partiel `(next_retry_at) WHERE completed_at IS NULL`.

`down` : drop table + drop index + `ALTER TABLE apparatus_bindings DROP COLUMN` les 9 colonnes P2. Pas de trigger SQL.

### E — HTTP 202

**NON.** Pas de nouvelles routes. `/components` gelé (5 registrations).

### F — Kubernetes

**NON.**

### G — Ports de test

Trait sync `ApparatusRuntime` dans `apparatus-contracts/src/ports.rs` (comme `KvStore`) :

1. `bind(&BindRequest) -> BindResponse`
2. `configure(&ConfigureRequest) -> ConfigureResponse`
3. `unbind(&UnbindRequest) -> UnbindResponse`
4. `observe(&BindingId) -> RuntimeObservation`
5. `teardown(&BindingId) -> ()` (idempotent ; **pas** `release`)

Pas d’`invoke` (P3). Pas d’`ensure_instance` / `observe_instance` / `delete_instance`. Double : `InProcessApparatusRuntime` déterministe. `BindingId` wire = `project_components.id`. **Zéro** identifiant `gateway` sous `Manifesto/*/src`.

### H — Events

**Pas** de nouveau type d’event. `ComponentAdded` / `ComponentRemoved` inchangés. `component_status_changed` = legacy seulement. T3 « outbox » = `rustycog_outbox_events` pour les events **ownership déjà existants**, même transaction que le bump desired / insert cleanup ; pas de payload desired/observed versionné.

### I — Worker / readiness

Ticker in-process + scan DB, standalone **et** monolithe. Broker/queue absente **≠** contrôleur ready. Ready = tâche ticker **vivante** + DB joignable, branché sur `/ready` existant.

### J — Outbox lifecycle

Desired + (job cleanup si delete) + outbox ownership dans **la même txn**, **sans** nouvel ACL. `ProjectAuthorizationUnitOfWork` uniquement add/remove (P1). `desired_generation` **≠** `grant_revision`. `grep apparatus openfga/model.fga` = 0.

### K — Cleanup

Insert `apparatus_cleanup_jobs` dans l’UoW **delete** si managed et (`digest IS NOT NULL` ou `desired_generation > 0`). Worker `teardown` puis CAS `completed_at`. Relançable tant que `completed_at IS NULL`. Pas de tombstone P1, pas de trigger.

### L — Gate T7 dès T2

Tokens P2 (`poll`, `worker`, `lease`, `fencing`, `controller`, `desired_state`) **uniquement** si `allow_path` contient :

- `Manifesto/migration/src/m20260912_000013_apparatus_p2_runtime.rs`
- `Manifesto/infra/src/apparatus_runtime/`

P3+ **interdit y compris** dans ces fichiers : `kubernetes`, `k8s`, `wasm`, `wasi`, `wasmtime`, `iframe`, `messagechannel`, `gateway`, `apparatus_host`, `ui_host`. `factory` : allowlist inchangée (`ManifestoCommandRegistryFactory`).

Retargeter `Manifesto/tests/apparatus_p1_t7_gate.rs` **avant** d’introduire ces tokens en prod. En T7, retargeter le gate vers P3+ (ne pas laisser les tokens P2 interdits à jamais) ; le gate P3+ vit aussi dans `Manifesto/tests/apparatus_p2_t7_*.rs`.

### M — Consentement

Hors P2.

## Conséquences

- Migration `m20260912_000013_apparatus_p2_runtime` additive réversible.
- Prod P2 : `Manifesto/infra/src/apparatus_runtime/` ; wiring `start_apparatus_runtime` dans setup (noms hors allowlist **sans** tokens P2).
- Tests `Manifesto/tests/apparatus_p2_t2_*.rs` … `t7_*.rs`. T1 reste vert.
- T7 P1 retargeté dès T2 selon L.

## Alternatives rejetées

| Option | Pourquoi pas |
|---|---|
| Copier le wiki (ports `ensure_*`, table d’opération UUID, HTTP 202) | Conception, pas contrat |
| Fencing = génération seule | Trou « même génération, writer mort » |
| Heartbeat | TTL + steal suffisent |
| Tombstone / trigger DELETE | CASCADE efface le binding ; table interne sans FK |
| Nouveau type d’event / type FGA | ADR-0001 ; H et J |
| Adaptateur K8s / gateway / 202 / `invoke` / consentement | P3+ ou hors P2 |

## Non décidé ici

- `APP-01` … `APP-07`
- Gateway réelle, identité workload, KV plateforme (P3, ADR-0004)
- Factory, host UI, iframe, MessageChannel (P4/P5)
- `VALID` / `VERIFIED` persistants (ADR-0005)
- N instances ; `shared` / `organization`

## Références

- Prompt : `docs/apparatus-p2-implementation-prompt.md`
- ADR-0001 … 0005
- Code P1 : `m20260912_000012_create_apparatus_bindings_table.rs`
- Preuve T1 : `apparatus_p2_t1_events` 4/4, `apparatus_p2_t1_lookup` 1/1
- Preuve T2–T7 (2026-09-12, t5 poison 2026-09-13) : t2 12, t3 5, t4 4, t5 5, t6 3, t7 cleanup 2 + gate 5 ; T7 P1 3. `/ready` n’est pas encore branché sur le ticker (`is_live()` explicite).

### Écarts de réalité P2

Cible Accepted A–M inchangée. Ce jalon n’a pas livré ces exigences (même motif que `/ready`) :

- **§A** : create managed pose `desired_generation = 1` et `next_retry_at = NOW()`. Update/remove **ne bumpent pas**. Reconfiguration non réconciliable dans ce jalon (génération figée à 1). Report conscient, déjà noté dans `.serena/memories/architecture/apparatus-p2-t3-persist.md`.
- **§D** : colonnes `retry_count` / `last_error_code` / prédicat terminal présentes ; **aucun writer** en prod. `next_retry_at` fixé à l’insert. Échec de `bind` = re-scan au tick suivant, sans backoff.
- **§I** : `/ready` reste non branché (déjà écrit ci-dessus).
