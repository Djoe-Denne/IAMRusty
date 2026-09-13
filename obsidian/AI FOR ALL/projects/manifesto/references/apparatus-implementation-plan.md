---
title: "Apparatus — plan d’implémentation et décisions restantes"
category: references
tags: [components, roadmap, testing, architecture, visibility/internal]
status: proposed
feature_status: future
sources:
  - "C:/Users/djden/.codex/attachments/486d0052-5759-4277-bcc1-9f209ce353d4/pasted-text.txt"
  - Manifesto/infra/src/transaction.rs
  - Manifesto/http/src/lib.rs
  - openfga/model.fga
  - sentinel-sync/src/translator/manifesto.rs
  - docs/adr/0002-apparatus-contract-first.md
  - apparatus-contracts/src/lib.rs
  - apparatus-reference-kv/apparatus.toml
  - docs/apparatus-p1-implementation-prompt.md
  - docs/services/manifesto.md
  - docs/adr/0006-apparatus-p2-reconciliation-in-process.md
  - docs/apparatus-p2-implementation-prompt.md
summary: >-
  Phases Apparatus : P0/P1/P2 Partial dans HEAD 7455ee5. P2.1 = bump, writer,
  /ready. P3+ = gateway/K8s/invoke.
provenance:
  extracted: 0.22
  inferred: 0.76
  ambiguous: 0.02
created: 2026-09-09T17:50:00Z
updated: 2026-09-13T10:25:00Z
---

# Apparatus — plan d’implémentation et décisions restantes

Plan proposé pour [[projects/manifesto/concepts/apparatus-platform]], sans date ni estimation d’effort inventée. La source utilisateur fixe la vision ; les choix ci-dessous préparent une implémentation future. L’existant est référencé dans [[projects/manifesto/references/apparatus-source-reconciliation]].

## Recommandations du 9 septembre — baseline de traçabilité

Ce tableau conserve les recommandations antérieures aux ADR. Il constitue leur baseline de comparaison, pas une acceptation implicite. ^[inferred]

| Sujet | Choix recommandé le 9 septembre | Pourquoi / limite |
|---|---|---|
| Propriété métier | Catalogue, release et binding dans la frontière Manifesto | Cohérence avec le propriétaire actuel des projets/composants |
| Processus privilégiés | Factory, admission/signature et contrôleur runtime séparés des plugins | Le code auteur ne doit jamais obtenir leurs privilèges |
| Migration | Extension 1:1 de `ProjectComponent`, UUID et ACL `component` conservés | Pas de nouveau type FGA ni de migration des grants en V1 |
| Cardinalité | Un Apparatus canonique du même type par projet | Préserve l’unicité SQL actuelle |
| Installation | Release et digest figés, commande asynchrone avec suivi | Réconciliation stable après restart |
| Lifecycle | Desired state et observations distincts, génération, lease et fencing | Évite les doublons et les observations périmées |
| Autorisation | Gateway : ACL utilisateur/projet/instance ∩ consentement ∩ politique | Un plugin ne reçoit aucun bearer IAM interne |
| Runtime | Managed, isolation par binding/projet ; Kubernetes était une première piste | Le moteur et la plateforme restent ouverts dans `APP-01` |
| Frontend | Host à créer ; schema et sandbox statique Vite | Pas de runtime Node/SSR par Apparatus |
| Données | KV plateforme par binding ; secrets par références | Pas d’accès direct aux bases des services |
| Publication | Git résolu en commit → artifacts → conformance → signature/admission | Le runtime consomme uniquement des digests admis |
| Confiance | `VALID` distinct de `VERIFIED` ; aucun bypass officiel | Même protocole pour tous les publishers |

Ces lignes étaient des recommandations nouvelles, pas des décisions déjà acceptées. Les ADR ci-dessous ont été amendées puis ratifiées séparément le 10 septembre. ^[inferred]

## Décisions — ADR Accepted

Canon : `docs/adr/` ; hub wiki : [[projects/manifesto/decisions/index]]. `Accepted` fixe la cible ; le champ `Réalité` de chaque ADR indique séparément l’état du code.

| Sujet | ADR | Réalité | Encore hors ADR |
|---|---|---|---|
| Propriété Manifesto, 1:1 `ProjectComponent`, unicité | [0001](../../../../../docs/adr/0001-apparatus-binding-owned-by-manifesto.md) | Partial | Consentement ; bump génération update/remove (P2.1) |
| Contrats, digest immuable, Apparatus KV de référence | [0002](../../../../../docs/adr/0002-apparatus-contract-first.md) | Partial | CLI / macro ; champs TOML de détail |
| Plugin hostile hors processus privilégiés | [0003](../../../../../docs/adr/0003-apparatus-untrusted-plugin.md) | Partial | Moteur et adaptateur / `APP-01` |
| Gateway, KV, pas de bearer IAM, fermeture DB immédiate | [0004](../../../../../docs/adr/0004-apparatus-capability-gateway.md) | Partial | mTLS concret (P3) |
| Même protocole ; admission, `VALID`, `VERIFIED` et installabilité distincts | [0005](../../../../../docs/adr/0005-apparatus-same-protocol-valid-verified.md) | Partial | Pipeline OCI (P4), drain/destruction |

## Livraison séquencée

### P0 — Contrats et Apparatus de référence

Écrire le schéma canonique de `apparatus.toml`, les contrats release/binding/operation, l’API de capabilities et les protocoles backend/UI. Définir les versions et erreurs, puis créer un Apparatus de référence simple utilisant KV et une UI de lecture, sans dépendance réseau externe. La macro Rust et l’ergonomie de CLI viennent après les contrats wire. ^[inferred]

**Preuve de sortie** : manifeste accepté/refusé de manière déterministe par une bibliothèque unique et son harness, ID/version sans ambiguïté, DTO sans credentials. Les valeurs limites sont testées ; aucun contrat n’emploie `latest` comme identité. La CLI et la Factory n’existent pas encore et réutiliseront ce validateur plus tard. ^[inferred]

**Preuve P0 livrée (2026-09-10, tests élargis 2026-09-11, P0.1 2026-09-12)** : détail dans [[projects/manifesto/concepts/apparatus-p0-contracts]]. 27 tests `contracts_p0` + 10 `kv_p0` = **37** socle, + `apparatus_p01_micro.rs` (3+2) = **42/42** (contracts 27+3=30, ref-kv 10+2=12) avec `cargo test -p apparatus-contracts --features test-harness` et `cargo test -p apparatus-reference-kv`. CI job apparatus-p0 + coverage Apparatus, gates `fmt` / `check` / `test` / `clippy` verts. Reports P1+ : persistance Manifesto (P1), gateway réseau et identité workload (P3), Factory/admission OCI (P4), host UI et CLI (P5).

### P1 — Persistance, catalogue et migration additive

Dans les couches domain/application/infra/migration de Manifesto, ajouter release, extension 1:1 du binding, opérations et consentements. Étendre les use cases, routes et DTO ; conserver `/components`, `component_id`, les grants et les événements ownership actuels. Adapter le client catalogue HTTP derrière une frontière de catalogue métier stable. ^[inferred]

Backfill : les composants existants deviennent `source=legacy`, avec identité conservée, release non résolue et contrôleur désactivé pour ces lignes. Un administrateur choisit un mapping canonique et une release, puis consent avant passage en managed. Détecter les collisions entre deux anciens types mappés vers le même Apparatus ; ne pas fusionner ni supprimer silencieusement. ^[inferred]

**Preuve de sortie** : migration réversible avant activation de workloads ; comparaisons avant/après des IDs, permissions et réponses legacy ; rejet d’un double ajout concurrent ; rollback transactionnel si écriture outbox échoue. Aucun backfill ne démarre du code tiers. ^[inferred]

**Preuve P1 livrée (2026-09-12, commit `7455ee5`)** : T1-T6 28/28, mapping 5/5, T7 3/3 = P1 36 ; P0.1 42/42. Synthèse : [[projects/manifesto/concepts/apparatus-p1-persistence]].
1. Migration réversible — table `apparatus_bindings` (`id` BIGSERIAL interne, `component_id` UUID UNIQUE FK→`project_components.id` CASCADE, `digest` VARCHAR(128) NULL, `source` CHECK legacy|managed), migration `m20260912_000012` additive réversible, up/down/up verts, 8/8. Fichier : `Manifesto/migration/src/m20260912_000012_create_apparatus_bindings_table.rs`.
2. Backfill legacy — `backfill_apparatus_legacy` explicite (`INSERT...SELECT` legacy `ON CONFLICT DO NOTHING`), idempotent 2 runs même état, legacy intact, 5/5. Fichier : `Manifesto/infra/src/apparatus_backfill.rs`.
3. Collisions rejetées — table injective taskboard/wiki, `check_pairs_injective`, erreur `APPARATUS_MAPPING_COLLISION` v1, `is_unique_violation` 23505→409, `map_unique_conflict` 409, concurrence [201,409] stable, 4/4 + mapping 5/5. Fichiers : `Manifesto/infra/src/apparatus_mapping.rs`, `Manifesto/infra/src/repository/component_repository.rs`.
4. Rollback atomique — `persist_binding_atomically` (BEGIN→INSERT managed→publish→COMMIT/ROLLBACK), échec→0 ligne, succès→1 ligne managed, faux broker in-memory, 0 polling/worker, 3/3. Fichier : `Manifesto/infra/src/apparatus_outbox.rs`.
5. ACL/routes/events inchangés — INSERT managed même txn que composant+ACL+outbox (`Manifesto/infra/src/transaction.rs`), grants projet inchangés, revoke→403 TTL0, ownership conservés, `grep apparatus model.fga` 0, zéro nouveau type FGA, 4/4 ; alias `?binding` même `component_id` ou 404, POST 7 clés gelées, GET==POST, list `{data}`, DTO inchangé, catalogue wiremock, 4/4, 5 routes `/components`, FGA 5 types. Fichier : `Manifesto/http/src/handlers/components.rs`.
6. Zéro workload — T7 : 0 token P2 dans le prod Manifesto scanné par T7 (7 crates src), 0 `VALID`/`VERIFIED` quotés, 5 routes ; T5 : FGA 5 types ; T3+T6 : 0 second UUID ; pas de seconde ressource, pas de renommage, gate 3/3 vert.
Consentement/génération non ajoutés (sans spec ADR, ADR dédiée avant P2).

### P2 — Réconciliation sans infrastructure réelle

Créer des ports runtime/gateway, un adaptateur de test déterministe et un worker avec polling, lease, opérations durables et fencing. Tester bind/configure/unbind, retries, suppressions et générations. Ajouter l’outbox de lifecycle sans modifier les ACL à chaque phase. ^[inferred]

Modifier dès ce stade la suppression du projet pour conserver l’intention de nettoyage au-delà des cascades SQL. La projection legacy ne reçoit plus d’événements `apparatus-events` pour des bindings managed. ^[inferred]

**Preuve de sortie** : crash après création de ressource simulée puis reprise sans doublon ; deux workers concurrents ; événement perdu/doublé/désordonné ; upgrade périmé refusé ; suppression pendant provisioning ; cleanup relançable. ^[inferred]

**État 2026-09-13 (ADR-0006 Accepted, Réalité Partial, HEAD `7455ee5`)** : T1–T7 existent (t2 12, t4 4, t5 5, t7 cleanup 2). Pas de gateway / K8s / 202 / nouvel event. Écarts A/D/I : pas de bump update/remove, pas de writer retry, `/ready` hors ticker. Mineurs (IF NOT EXISTS index/table cleanup, isolation poison, log fencing) **dans** ce commit. `ADD COLUMN` des 9 colonnes sans IF NOT EXISTS. Create ne pose pas `digest` → apply no-op hors tests. Synthèse : [[projects/manifesto/concepts/apparatus-p2-reconciliation]].

**P2.1 (prochain jalon, pas P3)** : CAS `desired_generation + 1` sur update/remove ; writer `retry_count`/`last_error_code` + backoff ; brancher `/ready` sur `is_live()` ; poser `digest` sur le chemin commande. P3+ (`invoke`, gateway, K8s) reste interdit par T7.

### P3 — Frontière de capacités et données

Implémenter identité de workload, certificat/rotation, gateway, grants interactifs et de fond, consentement/revocation, stockage KV, secrets et proxy réseau. Les refus sont contrôlés côté serveur et liés au binding courant. Les règles du projet public ne rendent pas le stockage ni l’invoke public par défaut. ^[inferred]

**Preuve de sortie** : test de deux projets et deux bindings adverses ; plugin incapable de changer son tenant, lire un secret, réutiliser un grant révoqué, contacter l’infrastructure interne ou invoquer une opération non accordée. Tester suspension immédiate du membre avec une projection FGA encore ancienne. ^[inferred]

### P4 — Factory et runtime de production

Construire le pipeline Git/build/package/conformance/admission et le registry ; implémenter KubernetesAdapter sur une infrastructure explicitement provisionnée. Séparer worker de build, test runner de conformance, signataire et contrôleur. P2 et P3 fournissent le protocole et les refus à tester ; une image de test plateforme peut être utilisée avant ouverture aux soumissions tierces. ^[inferred]

**Preuve de sortie** : soumission Git → release par digest → workload ; worker malveillant sans clé ni accès plateforme ; refus de digest altéré, manifeste non conforme, signature inattendue et politique runtime manquante. Tests réels du plugin réseau, quotas, sandbox et suppression des orphelins. ^[inferred]

### P5 — Host UI et parcours complet

Créer le shell produit minimal, l’UI de catalogue/consentement/suivi, les slots, le renderer schema, le SDK bridge et le serveur de bundles. Brancher les contrôles d’autorisation existants et nouveaux. Développer `check/dev/publish` sur les mêmes contrats. ^[inferred]

**Preuve de sortie** : un utilisateur crée un projet, choisit l’Apparatus, consent, attend ready et l’utilise. Refaire le parcours en microservices et en monolithe, avec mêmes préfixes API. Tester navigateurs ciblés, origine hostile, reload et changement de projet. ^[inferred]

### P6 — Qualification avant catalogue communautaire

Tester upgrades/rollback de données, rétention, révocation globale d’une release, reconstruction après perte du contrôleur, restauration de backups et distribution self-hosted/offline. Fixer des objectifs mesurés pour latence gateway, installation/cold start, délai de révocation, coût par binding et quotas Factory. Ne pas annoncer d’autoscaling ou de garantie de disponibilité non mesurée. ^[inferred]

L’Apparatus officiel de référence utilise exactement le chemin de publication et d’exécution communautaire. L’ouverture du catalogue aux tiers attend les preuves de confinement ; `VERIFIED` n’est pas une solution de remplacement. ^[inferred]

## Matrice de tests d’acceptation

La matrice gateway/Factory/K8s reste **future** (P3+). Les preuves worker P2 (crash, lease, fencing, cleanup) sont exercées par `apparatus_p2_t4`–`t7` (Partial). ^[inferred]

| Frontière | Tests décisifs |
|---|---|
| DB / ACL / outbox | rollback atomique, uniqueness race, mêmes IDs/grants après backfill, absence de tuple lifecycle |
| Worker / runtime | crash entre ensure et commit, lease expirée, worker périmé, génération ABA, suppression pendant upgrade |
| Gateway | utilisateur suspendu, droits d’instance manquants, scope organisation changé, grant de fond révoqué, accès anonyme |
| Stockage / secrets | accès namespace voisin, dépassement quota concurrent, CAS conflict, export/purge, secret absent des réponses/logs |
| Egress | redirect vers réseau interne, IPv6/link-local, DNS rebinding, méthodes/chemins non permis, taille et durée excessives |
| Factory | build.rs/config Vite malveillants, dépendance lock modifiée, cache empoisonné, source démesurée, symlink/path escape |
| Admission OCI | artifacts différents de ceux testés, signature non autorisée, blob manquant, release révoquée, import offline |
| Bridge UI | origin/source erronés, requête forgée, payload inconnu/trop gros, replay, navigation/reload, deux projets simultanés |
| Exploitation | archive/suspension, nettoyage orphelin, sauvegarde et rollback incompatible, restart sans broker notification |

Conventions de test : ports/adaptateurs simulés pour logique de réconciliation ; fixtures HTTP typées pour API collaboratrices ; conteneurs réels pour PostgreSQL, broker, OpenFGA et registry quand le protocole compte ; cluster de test réel pour réseau et isolation Kubernetes. Des mocks de NetworkPolicy ne prouvent pas l’isolement réseau. ^[inferred]

## Questions encore ouvertes

| ID | Arbitrage | Recommandation par défaut / moment où il bloque |
|---|---|---|
| APP-01 | Plateforme cible, budget, architecture CPU, moteur d’isolation, registry et autorité de signature | Profil opérateur unique et vérifié ; à choisir avant P4. Docker Compose ne suffit pas à répondre |
| APP-02 | Qui peut soumettre/publier/installer ? Portée du catalogue privé/public et ownership publisher | Administrateur projet pour installer, publisher authentifié pour soumettre ; politique finale avant ouverture |
| APP-03 | Rétention, purge, export et restore après désinstallation/archive | Suspendre à l’archive, suppression logique puis purge différée ; durée et confirmation de purge avant P6 |
| APP-04 | Premier Apparatus métier et ses besoins réels de données/opérations | Référence KV simple pour qualifier, puis Git ou wiki à choisir ; évite de généraliser une API réseau sans cas concret |
| APP-05 | UI publique sur projet public et tâches de fond autorisées | Invoke privé par défaut et grants de service explicites ; décision avant exposition de données réelles |
| APP-06 | Quotas, limites de taille, délais, capacité et coût | Limites conservatrices configurables, mesures en P6 ; valeurs économiques et SLO restent à fixer |
| APP-07 | Organisation qui change ou projet déplacé vers une autre organisation | Bloquer/reconsentir les grants dépendant de l’organisation avant reprise ; le transfert du rôle owner déjà livré ne change pas l’identité du composant |

Ces choix ne bloquent pas la documentation ni P0/P1. Ils ne sont pas tranchés artificiellement sans besoin produit ou infrastructure disponible. ^[inferred]

## Hors V1, sans les perdre

- `shared` et `organization` : pool partitionné, isolation multi-tenant démontrée, quotas et upgrade coordonné.
- Plusieurs instances du même Apparatus : migration de l’unicité et de l’adressage legacy.
- Scale-to-zero : activation des requêtes, délais de cold start et traitement des tâches de fond ; `replicas=0` seul ne le réalise pas.
- WASM/WASI : adapter aux capacités et contraintes de runtime ; recompilation et conformance spécifiques.
- SaaS externe : identité et politique de sortie de données distinctes, consentement explicite.
- Builders supplémentaires, contributions organisation/globales, UI intégrée trusted, dépendances inter-Apparatus et marketplace : nouveaux contrats à qualifier.

Ces extensions conservent le même binding métier ; elles ne sont pas des dépendances nécessaires à la première version. ^[inferred]

## Raccordement à la plateforme

Si Factory ou gateway deviennent des services RustyCog séparés, suivre le guide [nouveau service](C:/Users/djden/source/repos/AIForAll/docs/guides/nouveau-service.md) : workspace, configuration, préfixes, composition, shutdown, documentation et tests. Ajouter modèle OpenFGA et traducteur seulement si un nouveau type d’autorisation est réellement créé. Un nouveau binaire de service n’exige pas de nouveaux tuples pour les bindings existants. ^[inferred]

Le contrôleur nécessite une startup/shutdown contrôlée dans les modes standalone et monolithe. Une queue absente n’est pas silencieusement assimilée à un contrôleur opérationnel : le polling et la readiness rendent explicite la capacité réellement disponible. ^[inferred]

## Notes associées

- [[projects/aiforall/roadmap]] — priorités plateforme.
- [[projects/manifesto/references/manifesto-testing-and-fixtures]] — tests existants.
- [[projects/sentinel-sync/concepts/db-to-openfga-reconcile]] — reconstruction des droits.
- [[projects/manifesto/concepts/apparatus-bindings-and-lifecycle]] — contrat détaillé de réconciliation.

