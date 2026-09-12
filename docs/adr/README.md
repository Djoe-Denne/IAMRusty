# Architecture Decision Records

Décisions d’architecture **acceptées ou proposées**, distinctes des notes de conception.

| Couche | Rôle | Où |
|---|---|---|
| Conception | Vision, protocoles, écarts code, plan de livraison | Wiki Manifesto (`obsidian/AI FOR ALL/projects/manifesto/`) |
| Décision | Un choix irréversible (ou coûteux à changer), avec alternatives | **Ici** (`docs/adr/`) |
| Handbook | État actuel du code et recettes d’implémentation | `docs/` (hors `adr/` et `reviews/`) |

`docs/project/Archi.md` est l’ADR historique du Project Service (cible 2024–2025, en partie caduque). Ne pas y empiler Apparatus.

## Règles

1. **Une décision par ADR.** Pas un dossier de conception recollé.
2. **Le titre est la décision**, pas le thème (« Le binding est une extension 1:1 de ProjectComponent », pas « Bindings »).
3. **Statut** : `Proposed` → `Accepted` (revue explicite, typiquement PR) → `Superseded` / `Rejected`. Une note wiki `status: proposed` n’est pas une ADR acceptée.
4. **Ne pas ADR (Apparatus)** : matrices de tests P2+, SLO, moteur d’isolation, qui peut publier, rétention — tant que ça reste un arbitrage ouvert (`APP-01` … `APP-07` dans le plan wiki). Les ADR rétroactives 0100+ documentent l’architecture **déjà en place** (hexagone, crates, tests, services).
5. **Les notes wiki restent.** Une ADR cite ; elle ne remplace pas bindings, capabilities, Factory, UI.
6. **Décision et réalité sont indépendantes.** `Accepted` signifie que la cible est ratifiée ; `Unimplemented`, `Partial` ou `Implemented` décrit ce que réalise le dépôt. Une ADR acceptée n’autorise pas à présenter la fonctionnalité comme livrée.
7. **Accepter tôt les invariants qui figent P0.** Reporter les mécanismes qui dépendent de P2/P4 (worker, lease/fencing, moteur d’isolation, pipeline OCI).
8. **Traçabilité non circulaire.** La baseline Apparatus est le document utilisateur et le wiki commité le 9 septembre 2026 (`d0664e3`). Un hub wiki mis à jour après une ADR ne constitue pas une preuve indépendante de son acceptation.

## Vague 1 — Apparatus (Accepted, 2026-09-10)

| ID | Décision | Réalité | Notes wiki |
|---|---|---|---|
| [0001](0001-apparatus-binding-owned-by-manifesto.md) | Binding = `ProjectComponent` 1:1, propriété métier Manifesto | Partial | bindings, plan |
| [0002](0002-apparatus-contract-first.md) | Contrats + Apparatus KV de référence avant Factory et host | Partial | platform, factory, UI, plan P0 |
| [0003](0003-apparatus-untrusted-plugin.md) | Code auteur hors des processus privilégiés | Partial | platform, capabilities, factory |
| [0004](0004-apparatus-capability-gateway.md) | Gateway seule I/O ; KV plateforme ; pas de bearer IAM | Partial | capabilities |
| [0005](0005-apparatus-same-protocol-valid-verified.md) | Même protocole ; admission, `VALID`, `VERIFIED` et installabilité distincts | Partial | factory, platform |

Ratification : revue orchestrée de douze avis indépendants (deux passes sur cohérence, sécurité et implémentation), puis arbitrage explicite. Cette ratification fixe des **invariants cibles** ; elle ne valide ni Kubernetes, ni une Factory, ni une gateway ou un runtime déjà opérationnels.

## Traçabilité de la baseline du 9 septembre

| Recommandation documentée | Décision canonique | Encore ouvert |
|---|---|---|
| Propriété métier Manifesto | ADR-0001 | Autorité d’admission concrète (`APP-01`) |
| Processus privilégiés séparés du plugin | ADR-0003 | Déploiement des workers |
| Migration 1:1, UUID et ACL `component` conservés | ADR-0001 | Migration P1 |
| Un Apparatus canonique du même type par projet | ADR-0001 | Plusieurs instances hors V1 |
| Identité d’installation immuable, jamais `latest` | ADR-0002 | Admission et artifacts P4 |
| Desired/observed, génération, lease et fencing | — | ADR avant P2 |
| Gateway = ACL ∩ consentement ∩ politique | ADR-0004 | Implémentation P3 |
| Runtime managed, plugin isolé | ADR-0003 | Moteur et plateforme (`APP-01`) |
| Host à créer ; UI schema ou bundle statique | ADR-0002 (contrat seulement) | Sécurité du host avant P5 |
| KV par binding ; secrets par référence | ADR-0004 | Rétention, quotas et produit secrets |
| Git → artifacts → conformance → admission | — | ADR avant P4 |
| Même protocole ; `VALID` distinct de `VERIFIED` | ADR-0005 | Publishers (`APP-02`) |

Hub wiki : `obsidian/AI FOR ALL/projects/manifesto/decisions/index.md`.

## Schéma de numérotation

Les identifiants ne sont **pas** un seul compteur global. Plages thématiques :

| Plage | Sujet | Statut typique |
|---|---|---|
| **0001–0099** | Apparatus (plateforme) | Vague 1 : cibles `Accepted`, réalité souvent `Partial` |
| **0100–0199** | Hexagone RustyCog / responsabilités des crates | Rétroactif, architecture actuelle |
| **0200–0299** | Tests (IT vs unitaires, mocks vs infra réelle) | Rétroactif |
| **0300–0399** | Événements, outbox, AuthN/AuthZ | Rétroactif |
| **0400–0499** | Services du workspace et runtimes | Rétroactif ; un ADR par service s’il diverge du template |
| **0500–0599** | Config, CI, qualité, framework rustycog | Rétroactif |

Nouveau fichier : prochain entier **libre dans sa plage**, pas `0006` pour un sujet hexagonal. Ne pas réécrire 0001–0005.

## Vague 2 — architecture actuelle (rétroactive, 2026-09-12)

Ces ADR photographient le dépôt **tel qu’il est**. `Accepted` + `Implemented` = code et handbook d’accord. Une proposition wiki / roadmap n’est pas une décision : alors `Proposed` ou section « Non décidé ici », jamais un « on devrait » déguisé.

### 0100 — Hexagone et crates

| ID | Décision (titre canonique dans le fichier) | Réalité |
|---|---|---|
| [0100](0100-services-metier-hexagonaux-rustycog.md) | Services métier = vertical slice hexagonale RustyCog | Implemented |
| [0101](0101-crates-par-couche-hexagonale.md) | Une crate par couche, responsabilités stables | Implemented |
| [0102](0102-setup-composition-root.md) | `setup` = unique composition root | Implemented |
| [0103](0103-ports-adapters-command-factory.md) | Domaine derrière ports ; commandes via factory / `GenericCommandService` | Implemented |

### 0200 — Tests

| ID | Décision | Réalité |
|---|---|---|
| [0200](0200-it-infra-reelle-rustycog-testing.md) | IT via serveur réel, DB réelle, harness `rustycog-testing` | Implemented |
| [0201](0201-mocks-http-sortant-seulement.md) | WireMock / mocks = collaborateurs HTTP sortants uniquement | Implemented |
| [0202](0202-transport-opt-in-queues-desactivees.md) | Files opt-in (producteurs `queue.enabled=false`) ; Telegraph IT encore allumée | Partial |

### 0300 — Événements et autorisation

| ID | Décision | Réalité |
|---|---|---|
| [0300](0300-crates-events-contrat-sans-transport.md) | Crates `*-events` = contrat, pas le transport | Implemented |
| [0301](0301-outbox-transactionnel-rustycog.md) | Publication durable = outbox transactionnel | Partial |
| [0302](0302-authn-jwt-authz-openfga.md) | AuthN JWT plateforme ; AuthZ OpenFGA réelle en IT | Implemented |
| [0303](0303-sentinel-sync-worker-fga.md) | `sentinel-sync` = worker événements → tuples, pas un service hexagonal | Implemented |

### 0400 — Services et runtimes

| ID | Décision | Réalité |
|---|---|---|
| [0400](0400-iamrusty-identite-hexagonale.md) | IAMRusty = IdP hexagonal (OAuth, tokens, pas OpenFGA) | Implemented |
| [0401](0401-manifesto-projets-composants-acl-cas.md) | Manifesto = projets, composants, ACL, CAS, membership DB | Implemented |
| [0402](0402-hive-organisations.md) | Hive = organisations, invitations, membership | Implemented |
| [0403](0403-telegraph-notifications-event-driven.md) | Telegraph = notifications event-driven, HTTP étroit | Implemented |
| [0404](0404-runtime-microservices-et-monolithe.md) | Dual runtime : standalones + `oodhive-monolith` préfixé | Implemented |
| [0405](0405-readiness-crate-partagee.md) | Crate `readiness` partagée pour le health | Implemented |
| [0406](0406-crates-apparatus-p0-pas-le-host.md) | `apparatus-contracts` + KV réf. = P0 ; host/Factory hors livré — voir 0001–0005 | Partial |

### 0500 — Plateforme et qualité

| ID | Décision | Réalité |
|---|---|---|
| [0500](0500-config-typee-et-compose-local.md) | Config typée `rustycog-config` + Compose local | Implemented |
| [0501](0501-qualite-fmt-clippy-sonar.md) | fmt / Clippy / Sonar comme politique de dépôt | Partial |
| [0502](0502-rustycog-framework-feature-gated.md) | rustycog = framework feature-gated (submodule), pas un service | Implemented |

## Comment en ajouter une

1. Copier [template.md](template.md) → `NNNN-verbe-court.md` dans la **plage** du sujet (pas le prochain entier global).
2. Remplir Contexte / Décision / Conséquences / Alternatives. Lier les notes wiki et, si ça en remplace une, l’ADR superédée.
3. Ajouter la ligne dans ce README et, pour Apparatus, dans l’index wiki Manifesto.
4. Ouvrir une PR. **Accepted** seulement après accord explicite sur le texte final, pas parce qu’une note wiki le cite.

## Hors vague 1 (pas d’ADR tant que le jalon n’ouvre pas)

| Sujet | Quand |
|---|---|
| Génération, lease, fencing du contrôleur | Avant P2 |
| Pipeline Git → OCI, builders, registry | Avant P4 ; bloqué par `APP-01` |
| Host UI, origines iframe, MessageChannel | Avant P5 |
| Moteur d’isolation, CPU, budget (`APP-01`) | Avant tout runtime réel |
| Politique catalogue / publishers (`APP-02`) | Avant soumissions tierces |
| Révocation : drain/destruction des bindings déjà `ready` | Avant P4/P6 ; le refus de nouvelles opérations est déjà fixé par ADR-0005 |
