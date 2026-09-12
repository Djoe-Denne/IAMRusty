# Prompt — Implémentation Apparatus P1 (avec fermeture P0.1)

Tu travailles dans le dépôt `C:\Users\djden\source\repos\AIForAll`. Ne commite rien.

## Mission

Ferme **P0.1** (micro-réserves d'audit, sans les gonfler), puis implémente **P1 — persistance/catalogue Manifesto** en TDD strict RED-GREEN-REFACTOR. Livre code, migration, tests et mises à jour documentaires factuelles. Ne dépasse pas P1.

Conduis la tâche de bout en bout : exploration, migration additive, backfill, tests ciblés, formatage, diagnostics, compte rendu. Ne demande une clarification que si une décision matérielle absente des ADR bloque réellement l'implémentation.

## Sources de vérité à lire avant de coder

1. `AGENTS.md` et règles du workspace.
2. `docs/adr/README.md` puis ADR `0001` à `0005` dans `docs/adr/`.
3. `docs/apparatus-p0-implementation-prompt.md` (structure et esprit à reprendre).
4. Canvas d'audit (lecture seule) : `C:\Users\djden\.cursor\projects\c-Users-djden-source-repos-AIForAll\canvases\apparatus-P0-audit-P1-TDD.canvas.tsx` — verdict P0, réserves P0.1, tranches 0-7, pyramide de tests.
5. Via QMD CLI, collection `aiforall-wiki` (seulement si besoin de précision) :
   - `projects/manifesto/references/apparatus-implementation-plan.md` (P1)
   - `projects/manifesto/concepts/apparatus-bindings-and-lifecycle.md`
   - `projects/manifesto/concepts/apparatus-capabilities-and-isolation.md`
6. Code réel : workspace `Cargo.toml`, `Manifesto/` (entité `project_component`, migration, routes `/components`, `openfga/model.fga`, `apparatus-events/`), crates P0 `apparatus-contracts` / `apparatus-reference-kv`, `Manifesto/tests/common.rs`.

`Accepted` = cible ratifiée. `Réalité` = code présent. Ne prétends jamais que contrôleur, polling, lease/fencing, Factory, gateway, runtime ou host existent. Ne préempte ni `APP-01`..`APP-07`, ni l'ADR future P2 (génération/lease/fencing).

Si tu touches une API RustyCog, lis `.agents/skills/rustycog/SKILL.md` et uniquement ses références pertinentes. Pour les fixtures, lis `.agents/skills/creating-testcontainer-fixtures/SKILL.md` et `.agents/skills/creating-wiremock-fixtures/SKILL.md` avant d'en créer une.

## Phase 0 — P0.1 : fermer l'audit (faire en premier, rester micro)

Objectif : P0 vert en CI, docs synchronisées. Aucune feature.

1. **Docs/compteurs/Réalité.** Remplacer les compteurs obsolètes `16/16` par la réalité auditée (`27` contrats + `10` référence KV = `37/37`, commandes du canvas). Corriger les champs `Réalité` désynchronisés (ADR concernées, preuve P0 dans le plan wiki). RED : `grep` échoue sur `16/16` résiduel dans les preuves P0 ; GREEN : compteurs et `Réalité` factuels.
2. **CI P0.** Ajouter les deux crates Apparatus aux jobs CI de test (et couverture si le job existe) : fmt/clippy workspace seuls ne prouvent pas les 37 tests. RED : job CI ne lance pas `cargo test -p apparatus-contracts --features test-harness` ni `cargo test -p apparatus-reference-kv` ; GREEN : jobs verts et traçables.
3. **Micro-fidélité harness (sans refonte).** Traiter ou assumer explicitement par écrit : `ReadyResponse` peu exercée, `configure` persistant, validation des identités au bind, doublon des adaptateurs KV in-memory. Ajouter uniquement les micro-tests manquants (in-memory, déterministes, sans Docker). Si un point est assumé comme non-bug, le justifier en 2 lignes dans le compte rendu, pas de refactor opportuniste.

Critère de sortie P0.1 : `cargo fmt --check`, `cargo check`, `cargo test` ciblés P0, `cargo clippy` ciblés verts ; CI exécute les 37 tests ; aucun compteur obsolète ; aucune modification de décision `Accepted`.

## Périmètre P1 obligatoire (TDD, une tranche après l'autre)

Invariants globaux P1 (ADR-0001..0005) : extension **1:1 additive et réversible** sur `project_components.id`, **sans second UUID public** ; UUID, tuples OpenFGA `component:{id}`, routes `/components` et événements `ComponentAdded`/`ComponentRemoved` **inchangés** ; `component_type` non réécrit ; backfill `source=legacy`, release/digest **nullable non résolue**, contrôleur **off** ; mapping legacy→Apparatus **injectif** ; `source` ∈ `legacy|managed` ; **aucun workload tiers ne démarre** ; aucun `VALID`/`VERIFIED` produit ; `apparatus-events` (statut `project_id + component_type`) reste le chemin legacy et n'affecte jamais un binding managed sans `binding_id`+génération.

### Tranche 1 — Migration additive réversible

- RED (intégration DB, PostgreSQL testcontainer réel via `Manifesto/tests/common.rs`) : `up` crée l'extension 1:1 (FK vers `project_components.id`, digest nullable, `source`, consentement/génération si prévu), `down` la retire proprement ; unicité existante par projet intacte ; migration idempotente sur base legacy.
- GREEN : migration minimale, réversible, sans réécriture de données existantes.
- Sortie : up/down verts sur Postgres réel, rollback testé.

### Tranche 2 — Backfill legacy

- RED : IDs et statuts inchangés, `source=legacy`, release non résolue, `component_type` inchangé, contrôleur off, zéro workload démarré, `apparatus_id` attribut distinct si présent.
- GREEN : backfill minimal idempotent, rejouable.
- Sortie : backfill rejoué deux fois = même état ; aucune donnée legacy altérée.

### Tranche 3 — Collisions et concurrence

- RED : mapping legacy injectif (deux anciens types vers le même Apparatus = **rejet**, pas de fusion silencieuse) ; double ajout concurrent refusé par la DB (contrainte, pas un `if` applicatif) ; tests unitaires du mapping + intégration DB concurrence réelle.
- GREEN : contrainte DB + erreur versionnée stable.
- Sortie : collision documentée et testée, aucune fusion silencieuse.

### Tranche 4 — Transaction et outbox

- RED : échec outbox/écriture binding+consentement = rollback atomique (rien de partiellement persisté). N'introduire un broker réel que si P1 l'exige vraiment ; sinon, outbox transactionnelle locale testée avec Postgres réel et faux broker in-memory.
- GREEN : atomicité minimale, pas de worker/polling.
- Sortie : test rollback vert, preuve qu'aucun polling n'existe.

### Tranche 5 — ACL et événements

- RED (intégration AuthZ, OpenFGA **réel** en testcontainer, uniquement parce que tuples/ACL sont en jeu) : tuples `component:{id}` et grants inchangés, suspension/révocation fail-closed dès le commit DB, événements ownership conservés, **aucun nouveau type FGA**.
- GREEN : zéro changement de modèle FGA si les tests le confirment ; sinon escalader (ne pas inventer `apparatus_binding`).
- Sortie : suites AuthZ vertes, preuve d'absence de nouveau type.

### Tranche 6 — Compatibilité HTTP

- RED (intégration HTTP : serveur Manifesto **live** + fixture **wiremock typée** pour le catalogue externe, style `MockService` chainable avec `reset()`, matchers ordonnés premier-match-gagne) : `GET/POST /components` inchangés (contrat sérialisé gelé par contract tests), alias binding = même ressource `component_id` (pas une seconde ressource), catalogue vivant toujours via HTTP externe.
- GREEN : alias minimal, pas de seconde ressource, pas de renommage de route.
- Sortie : compatibilité legacy prouvée en live, contract tests sérialisés verts.

### Tranche 7 — Gate P2 (garde-fou)

- RED : test/board négatif — aucune ligne P1 ne peut armer polling, worker, lease, fencing, controller, Factory, gateway réseau, host UI, workload. `grep`/test d'absence ciblé + revue du diff.
- GREEN : supprimer ou ne pas introduire le code fautif.
- Sortie : gate vert, P2 toujours impossible sans la future ADR.

## Stratégie de tests (intégration seulement si pertinent)

- **Unitaires** (chaque tranche, rapides, sans infra) : mapping injectif, validation, conversions, erreurs versionnées.
- **Contract tests** (sans infra) : DTO/manifeste/protocole wire, compatibilité sérialisée `/components` et alias binding gelée.
- **Intégration DB** (PostgreSQL testcontainer réel, via `Manifesto/tests/common.rs` et le skill testcontainers) : uniquement tranches 1-4 (migration, backfill, concurrence, rollback/outbox).
- **Intégration AuthZ** (OpenFGA réel en testcontainer) : uniquement tranche 5, parce que tuples/grants/suspension sont touchés.
- **Intégration HTTP** (serveur live + wiremock typée, skill wiremock) : uniquement tranche 6.
- **E2E contrôleur/workload/gateway/Factory** : **hors P1** (P2-P6). Ne pas les construire.
- Ne crée aucun harness parallèle : réutilise `Manifesto/tests/common.rs`, les fixtures existantes et les skills. Les `tests/` Cargo in-memory restent des tests Cargo, pas des preuves système.

## Hors périmètre strict

Ne pas ajouter : polling, worker, desired/observed actif, génération auto, lease, fencing ; Factory, CLI, proc-macro, Git checkout, OCI, registry, signature, admission ; gateway réseau, mTLS, identité workload réelle, secret manager, proxy egress ; Kubernetes, Docker runtime, WASM/WASI, nouveau microservice ; host UI, iframe, CSP, MessageChannel, serveur de bundles ; modification du modèle OpenFGA, sentinel-sync, nouveaux événements managed ; statut `VALID`/`VERIFIED` persistant ; second UUID public ; réécriture de `component_type` ; N instances du même Apparatus par projet.

Si un besoin P2+ apparaît, note-le comme suivi sans l'implémenter. Ne préempte ni `APP-01`..`APP-07`, ni la future ADR P2.

## Contraintes de qualité

- Respecter les lints workspace (`unsafe_code = forbid`, Clippy pedantic/nursery/cargo).
- Éviter `unwrap`/`expect` dans le code de production ; documenter types/erreurs publics (`# Errors`, `# Panics` si requis).
- Dépendances minimales, versions du workspace ; migration additive, réversible, sans perte.
- Préserver les modifications utilisateur ; aucune commande Git destructive.

## Vérification

Exécuter au minimum :

1. `cargo fmt --all -- --check`
2. `cargo check` ciblé sur crates/migration modifiées
3. `cargo test` ciblé (P0.1 + tranches P1, y compris intégration DB/AuthZ/HTTP réellement ajoutées)
4. `cargo clippy` niveaux workspace si le temps reste raisonnable
5. Diagnostics IDE sur les fichiers modifiés

Corriger toute régression introduite. Ne pas élargir aux défauts préexistants sans rapport.

## Documentation de sortie

Mettre à jour uniquement les faits : champs `Réalité` des ADR concernées (`Partial`/`Implemented` selon preuves réelles) sans toucher aux décisions `Accepted` ; preuve P1 dans `apparatus-implementation-plan.md` (migration réversible, backfill legacy, collisions rejetées, rollback atomique, ACL/routes/events inchangés, zéro workload) ; documentation publique de la migration et des commandes de tests. Ne réécris jamais une décision ratifiée pour l'adapter à l'implémentation.

## Compte rendu final

Fournir : périmètre P0.1 fermé (docs/CI/micro-tests) ; architecture et emplacement de l'extension 1:1 ; fichiers/migrations/fixtures créés ou modifiés ; critères P1 satisfaits par tranche TDD ; tests et commandes exécutés avec résultats (distinguer unitaires/contract/intégration) ; éléments volontairement différés à P2-P6 ; risques ou questions réellement bloquantes.
