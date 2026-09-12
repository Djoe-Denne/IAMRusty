# Prompt — Implémentation Apparatus P2

Tu travailles dans le dépôt `C:\Users\djden\source\repos\AIForAll`. Ne commite rien.

## Contrainte modèle

Décision utilisateur **2026-09-12** : **seul Grok 4.6 Extra High** pour toute délégation, résolveur, agent, contre-agent et review de cette exécution P2.

- Slug Cursor obligatoire : `cursor-grok-4.6-xhigh` (paramètre `model:` de **chaque** Task / sous-agent).
- **Interdit nommément** : Muse Spark, `muse-spark-1.3-max`, Composer, tout autre modèle, et `inherit` s’il peut dévier vers un autre modèle.
- Lance **tous** les sous-agents de cette exécution P2 avec `model: cursor-grok-4.6-xhigh`. Un agent lancé sans ce slug = non conforme ; arrête et relance.

## Mission

Implémente **P2 — réconciliation sans infrastructure réelle** en TDD strict RED-GREEN-REFACTOR, **après** une ADR P2 **Accepted** pour tout code génération / lease / fencing / worker / desired-observed **actif**. Livre code, migration additive, worker in-process, tests et mises à jour documentaires **factuelles**. Ne dépasse pas P2. **P2 n’est pas implémenté** : table P1 `apparatus_bindings` n’a ni génération, ni lease, ni desired/observed.

Conduis la tâche de bout en bout : Phase 0 ADR, exploration du code P1, implémentation tranche par tranche, tests ciblés, retarget du gate T7, formatage, diagnostics, compte rendu. Ne demande une clarification que si une décision matérielle absente des ADR **Accepted** bloque réellement l’implémentation. N’invente **aucune** décision `Accepted`.

## Accepted vs Réalité

`Accepted` = cible ratifiée. `Réalité` = code présent. Ne les confonds pas. Ne réécris pas les décisions ADR Accepted.

Les 5 ADR Vague 1 (`docs/adr/0001` … `0005`) sont **Accepted** et **Réalité Partial**. Il n’existe **aucune** ADR `0006+` dans `docs/adr/` : seulement `0001`–`0005`, `README.md`, `template.md`. **Aucune ADR Accepted ne fige aujourd’hui** génération, lease, fencing, desired/observed du contrôleur, opérations durables ni polling.

Table `docs/adr/README.md` :

- Desired/observed, génération, lease et fencing → **pas d’ADR** (« ADR avant P2 »).
- Hors vague 1 : « Génération, lease, fencing du contrôleur | Avant P2 ».
- Gateway réelle = **P3** (ADR-0004). Factory / host = **P4 / P5**. `APP-01` … `APP-07` restent ouverts.

ADR-0001 : « Le détail desired/observed, lease et fencing reste dans la note bindings ; une ADR dédiée devra le figer avant P2. » Preuve P1 : consentement / génération **non ajoutés**.

Le wiki (bindings, plan) est **conception** (`^[inferred]`), **pas** une ADR. Interdit de traiter le wiki comme Accepted, et d’inventer des colonnes SQL « parce que le wiki les nomme ».

## Sources de vérité à lire avant de coder

1. `AGENTS.md` et les règles du workspace.
2. `docs/adr/README.md`.
3. ADR `0001`–`0005` dans `docs/adr/` (ne pas réécrire `Accepted`).
4. `docs/apparatus-p0-implementation-prompt.md` et `docs/apparatus-p1-implementation-prompt.md` (esprit TDD).
5. Canvas lecture seule : `C:\Users\djden\.cursor\projects\c-Users-djden-source-repos-AIForAll\canvases\apparatus-P0-audit-P1-TDD.canvas.tsx` — T7 gate P1, pyramide. L’E2E canvas « Contrôleur, workload, gateway, Factory » est hors P1. **P2** = réconciliation **in-process** (adaptateur de test). Workload K8s, gateway réseau réelle et Factory = **P3–P6**, pas P2.
6. QMD CLI, collection `aiforall-wiki` si besoin de précision :
   - `projects/manifesto/references/apparatus-implementation-plan.md` (section **P2 — Réconciliation sans infrastructure réelle**)
   - `projects/manifesto/concepts/apparatus-bindings-and-lifecycle.md`
   - `projects/manifesto/concepts/apparatus-capabilities-and-isolation.md`
   - `projects/manifesto/concepts/apparatus-platform.md`
7. Code P1 réel (existant, à étendre, pas à réécrire) :
   - `Manifesto/migration/src/m20260912_000012_create_apparatus_bindings_table.rs` (table `apparatus_bindings` ; enregistrée dans `Manifesto/migration/src/lib.rs`)
   - `Manifesto/infra/src/apparatus_backfill.rs`
   - `Manifesto/infra/src/apparatus_mapping.rs`
   - `Manifesto/infra/src/apparatus_outbox.rs` (`persist_binding_atomically`)
   - `Manifesto/infra/src/transaction.rs` (`ProjectAuthorizationUnitOfWorkImpl`)
   - `Manifesto/infra/src/repository/component_repository.rs`
   - `Manifesto/http/src/handlers/components.rs` (5 registrations `/components` dans `Manifesto/http/src/lib.rs`)
   - `Manifesto/infra/src/event/consumer.rs` (`ApparatusEventConsumer` / `ApparatusEventHandler` — chemin **legacy** `project_id + component_type`)
   - `Manifesto/infra/src/event/processors/component_processor.rs` (`ComponentStatusProcessor`)
   - `Manifesto/tests/apparatus_p1_t1_migration.rs` … `apparatus_p1_t7_gate.rs`
   - `Manifesto/tests/common.rs` ; `openfga/model.fga`
   - Contrats P0 : `apparatus-contracts/src/protocol.rs` (`/bind` `/configure` `/unbind` `/invoke`) ; `apparatus-contracts/src/ports.rs` (`KvStore` seulement — **pas** encore de port runtime)

Si tu touches une API RustyCog (queue, readiness, startup worker), lis `.agents/skills/rustycog/SKILL.md` et uniquement ses références pertinentes. P2 n’est **pas** un nouveau service RustyCog. Pour les fixtures DB : `.agents/skills/creating-testcontainer-fixtures/SKILL.md`. HTTP live seulement si l’ADR P2 l’exige : `.agents/skills/creating-wiremock-fixtures/SKILL.md`.

## Périmètre P2 obligatoire (TDD, une tranche après l’autre)

Invariants P1 **toujours vrais** (sauf ADR Accepted qui les lève) : pas de second UUID public ; identité = `project_components.id` ; tuples `component:{id}` ; routes `/components` (5 registrations) ; events `ComponentAdded` / `ComponentRemoved` inchangés ; mapping injectif ; 1:1 ; `source` ∈ `legacy|managed` ; pas de nouveau type FGA sans escalade ; `component_type` non réécrit ; pas de `VALID` / `VERIFIED` persistants ; harness P0 in-process ≠ runtime de production.

Table réelle P1 `apparatus_bindings` : surrogate `id`, `component_id` UNIQUE FK `fk_apparatus_bindings_component` CASCADE, `digest` nullable, `source` legacy|managed. **Pas** de colonnes génération / lease / desired / observed.

Wiki plan P2 (conception à **trancher** dans l’ADR, pas un contrat RED) : réconciliation sans infrastructure réelle — worker, polling, lease, opérations durables, fencing, adaptateur de test, outbox lifecycle **sans** ACL à chaque phase, cleanup au-delà du CASCADE, isolation `apparatus-events` des bindings managed.

« Ports runtime/gateway » en P2 = **ports + double de test**, **pas** la gateway réseau réelle (P3, ADR-0004). Les noms wiki (`ensure_instance`, `desired_generation`, HTTP `202`, payload d’events) sont des **exemples de conception**. Ne les copie ni dans un RED, ni dans une ADR auto-Acceptée. HTTP `202` + URL de suivi : **défaut = pas de nouvelles routes** tant que l’ADR P2 Accepted ne les exige pas.

Nomme les tests `Manifesto/tests/apparatus_p2_t*.rs` (même esprit que P1 : `t1`…`t7`).

### Phase 0 — ADR P2 (bloquant avant T2–T7 code génération / lease / fencing / worker / desired-observed actif)

- RED : `docs/adr/` n’a **aucune** ADR `0006+` Accepted qui fige génération, lease, fencing, desired/observed du contrôleur, opérations durables, polling. (État actuel : **aucune ADR P2**.)
- GREEN : ADR rédigée depuis `docs/adr/template.md` en **Proposed**. L’implementer **n’accepte pas** lui-même. **STOP T2–T7** jusqu’à Accept **explicite** (PR / utilisateur). Puis ligne `docs/adr/README.md` + index wiki. Wiki reste conception.
- L’ADR doit **trancher** (checklist, sans préempter les réponses) : sémantique génération / lease / fencing ; colonnes SQL **nommées seulement à l’Accept** ; HTTP `202` / URL de suivi (**défaut NON**) ; pas d’adaptateur Kubernetes ; ports de test vs gateway P3 ; événements managed (**défaut : pas de nouveau type d’event** sauf exigence explicite).
- Sortie : ID d’ADR **Accepted** ; T1 peut avancer sans attendre. T2–T7 code **interdit** tant que non Accepted.
- Interdit : promouvoir le wiki en `Accepted` ; inventer des colonnes SQL « parce que le wiki les nomme ».

### Tranche 1 — Isolation événements managed (sans attendre l’ADR P2)

Autorisé **uniquement** par ADR-0001 Accepted : un événement dépourvu de `binding_id` **et** de génération n’affecte **jamais** un binding managed. (La clause wiki « un ancien `active` ne réactive pas un binding supprimé » est conception : pas de tombstone P1, CASCADE actuel ; ne l’implémente pas en T1.)

- RED : tests (unit + si besoin DB) sur `Manifesto/infra/src/event/consumer.rs` et `Manifesto/infra/src/event/processors/component_processor.rs` : si `source=managed` et message **sans** `binding_id` (champ événement, pas une colonne SQL P1), **ignorer** ; chemin legacy `project_id + component_type` inchangé pour `source=legacy`.
- GREEN : filtre minimal, pas de nouveau type FGA, pas de worker, pas de colonne génération.
- Sortie : preuve test (`apparatus_p2_t1_*.rs`) ; legacy intact.

### Tranche 2 — Persistence desired/observed + génération + opérations (APRÈS Phase 0)

- RED (intégration DB, Postgres testcontainer via `Manifesto/tests/common.rs`) : migration additive réversible selon **colonnes figées par l’ADR P2 Accepted** (pas d’invention) ; 1:1 et absence de second UUID public conservés ; `source` legacy|managed intact.
- **Gate T7 dès T2** : retargeter `Manifesto/tests/apparatus_p1_t7_gate.rs` **avant** d’introduire des tokens P2 (`poll`, `worker`, `lease`, `fencing`, `controller`, `desired_state`) dans `Manifesto/migration/src` ou le prod. Allowlist **uniquement** les modules/migration listés par l’ADR. P3+ reste interdit (`kubernetes`/`k8s`, `wasm`/`wasi`/`wasmtime`, `iframe`, `messagechannel`, `apparatus_host`, `ui_host`, `gateway` dans `Manifesto/*/src`, Factory hors allowlist `ManifestoCommandRegistryFactory`).
- GREEN : migration minimale, `down` propre ; T7 P1 ne casse plus T2.
- Sortie : up/down verts ; preuve qu’on n’a pas recopié le wiki au-delà de l’ADR.

### Tranche 3 — Outbox lifecycle sans ACL

- RED : desired + opération + outbox atomiques ; échec → rollback ; **zéro** nouveau type FGA ; pas de tuple lifecycle ; ne pas confondre `grant_revision` AuthZ et génération runtime.
- GREEN : UoW sans ACL pour observations ; `ProjectAuthorizationUnitOfWork` seulement pour add/remove ownership (déjà P1, `persist_binding_atomically`).
- Sortie : rollback vert ; `grep apparatus openfga/model.fga` toujours 0 ; events ownership conservés.
- Événements desired/observed versionnés : **seulement** si l’ADR P2 Accepted les exige. Sinon **aucun** nouvel event managed (P1 les interdisait encore). Ne pas copier un payload wiki.

### Tranche 4 — Ports runtime + adaptateur de test déterministe

- RED : ports et adaptateur **figés par l’ADR P2** (les noms wiki `ensure_instance` / `observe_instance` / `delete_instance` sont des exemples). Étendre `apparatus-contracts/src/ports.rs` (aujourd’hui `KvStore` seulement). Réutiliser DTO P0 `apparatus-contracts/src/protocol.rs` (pas un second protocole wire). Adaptateur **in-process déterministe**. **Zéro** identifiant `gateway` sous `Manifesto/*/src`.
- GREEN : pas de Kubernetes, pas de gateway réseau, pas de mTLS, pas de secret manager, pas de nouveau microservice, pas de broker réel.
- Sortie : tests unit/contract in-process verts (cycle bind/configure/unbind/retries **selon l’ADR**). `invoke` réel via gateway = **P3**, hors P2.

### Tranche 5 — Worker polling + lease + fencing

- RED : worker relit la DB ; scan périodique **in-process** (ticker + DB, **pas** de broker réel ni nouveau service RustyCog sauf si l’ADR P2 l’exige) ; lease par binding ; fencing ; deux workers concurrents ; une queue absente **n’est pas** un contrôleur opérationnel (readiness explicite). Startup/shutdown contrôlés standalone et monolithe.
- GREEN : implémentation minimale **selon l’ADR P2**.
- Sortie : tests concurrence verts ; affiner l’allowlist T7 (commencée en T2) sur les modules contrôleur/worker réellement ajoutés.
- HTTP `202` + URL de suivi : **seulement si** l’ADR P2 Accepted les exige. Sinon **pas** de nouvelles routes. Si l’ADR les exige : tests HTTP live + skill wiremock ; alias ≠ seconde ressource ; `/components` gelé.

### Tranche 6 — Preuves de sortie réconciliation

- RED : preuves **exigées par l’ADR P2** (le plan wiki propose, sans faire foi : crash après ressource simulée puis reprise sans doublon ; événement perdu/doublé/désordonné ; upgrade périmé refusé ; suppression pendant provisioning ; retries bind/configure/unbind).
- GREEN : adaptateur de test seulement.
- Sortie : preuves d’acceptation **figées par l’ADR P2** (crash/reprise, workers concurrents, events désordonnés, génération périmée, suppression pendant provisioning). Le plan wiki informe, il ne fait pas foi. Cleanup = T7.

### Tranche 7 — Cleanup projet relançable + Gate P3

- RED : suppression projet conserve l’intention de nettoyage au-delà du CASCADE SQL actuel (`fk_apparatus_bindings_component`) ; cleanup relançable ; gate négatif P3+ (pas de gateway réseau réelle, Factory, K8s, WASM/WASI, host UI, iframe, MessageChannel, VALID/VERIFIED persistants, `APP-01`..`07` préemptés, pas de nouveau microservice).
- GREEN : opérations de cleanup durables ; gate `Manifesto/tests/apparatus_p1_t7_gate.rs` retargeté P3+ (ne pas laisser les tokens P2 interdits à jamais).
- Sortie : cleanup relançable + gate P3 vert (`apparatus_p2_t7_*.rs`).

## Stratégie de tests (couches pertinentes P2 seulement)

- **Unitaires** : CAS génération, expiry lease, fencing, clés d’idempotence, filtre events managed.
- **Contract** : DTO P0 bind/configure/unbind ; pas de second protocole ; pas de `trusted_skip_gateway`.
- **Intégration DB** (Postgres testcontainer, `Manifesto/tests/common.rs`, skill testcontainers) : T2, T3, crash/reprise, cleanup.
- **Intégration AuthZ** (OpenFGA réel) : **seulement** si on touche tuples/grants. P2 vise zéro changement ACL → preuve d’absence (`grep apparatus openfga/model.fga` = 0) suffit ; ne pas construire une suite FGA « pour faire joli ».
- **Intégration HTTP** : seulement si ADR P2 ajoute 202/suivi. Skill wiremock.
- **Worker in-process** : T4–T6, adaptateur déterministe. **Pas** de cluster K8s. Mocks NetworkPolicy ≠ isolation.
- **E2E gateway / Factory / host / K8s** : hors P2 (P3–P6).
- Réutiliser `Manifesto/tests/common.rs`. Ne crée aucun harness parallèle. Les tests P0 in-process restent des tests Cargo, pas un runtime de production.

## Hors périmètre strict

Ne pas ajouter : Factory, CLI, proc-macro, Git checkout, OCI, registry, signature, admission ; **gateway réseau réelle**, mTLS, identité workload réelle, secret manager, proxy egress, KV plateforme persistant ; Kubernetes, Docker runtime, WASM/WASI, nouveau microservice ; host UI, iframe, CSP, MessageChannel, serveur de bundles ; modification modèle OpenFGA, sentinel-sync ; **nouveaux événements managed** sauf si l’ADR P2 Accepted les exige ; statut `VALID`/`VERIFIED` persistant ; second UUID public ; réécriture `component_type` ; N instances du même Apparatus par projet ; `shared` / `organization` ; `APP-01`..`APP-07` ; broker réel / binaire de contrôleur séparé (ticker in-process + DB).

Préempter P3+ = interdit. Si un besoin P3+ apparaît, note-le en suivi.

Consentement administrateur / passage legacy→managed : P1 l’a différé faute d’ADR. Ne l’invente pas en P2 **sauf** si l’ADR P2 l’inclut explicitement.

## Contraintes de qualité

- Respecter les lints workspace (`unsafe_code = forbid`, Clippy pedantic/nursery/cargo).
- Éviter `unwrap`/`expect` dans le code de production ; documenter types/erreurs publics (`# Errors`, `# Panics` si requis).
- Dépendances minimales, versions du workspace ; migration additive, réversible, sans perte.
- Préserver les modifications utilisateur ; aucune commande Git destructive.

## Vérification

Exécuter au minimum :

1. `cargo fmt --all -- --check`
2. `cargo check` ciblé sur crates/migration/worker modifiés
3. `cargo test` ciblé (P0/P1 non régressés + tranches P2, distinguées unit/contract/DB/worker/HTTP)
4. `cargo clippy` niveaux workspace si le temps reste raisonnable
5. Diagnostics IDE sur les fichiers modifiés

Corriger toute régression introduite. Ne pas élargir aux défauts préexistants sans rapport.

## Documentation de sortie

Mettre à jour **uniquement les faits** : champs `Réalité` des ADR concernées (`Partial`/`Implemented` selon preuves réelles) **sans** modifier `Accepted` ; preuve P2 dans `apparatus-implementation-plan.md` ; **ne pas réécrire** les décisions. L’ADR P2 nouvelle, une fois Accepted, a son propre `Réalité`. Canvas = lecture seule.

## Compte rendu final

Fournir : Phase 0 (ADR ID ou **blocage** si non Accepted) ; architecture worker / ports / adaptateur de test ; fichiers / migrations / tests ; critères par tranche TDD ; commandes + résultats (distinguer unit/contract/DB/worker/HTTP) ; éléments volontairement différés à P3–P6 ; risques ou questions réellement bloquantes.
