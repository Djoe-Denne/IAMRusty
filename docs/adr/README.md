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
4. **Ne pas ADR (Apparatus)** : matrices de tests P2+, SLO, qui peut publier, rétention — tant que ça reste un arbitrage ouvert (`APP-02` … `APP-07` dans le plan wiki). **`APP-01`** (moteur d’isolation, registry, signature, admission, processus) est **ADRé** : [0008](0008-apparatus-p4-k8s-isolation-outside-manifesto.md). Les ADR rétroactives 0100+ documentent l’architecture **déjà en place** (hexagone, crates, tests, services).
5. **Les notes wiki restent.** Une ADR cite ; elle ne remplace pas bindings, capabilities, Factory, UI.
6. **Décision et réalité sont indépendantes.** `Accepted` signifie que la cible est ratifiée ; `Unimplemented`, `Partial` ou `Implemented` décrit ce que réalise le dépôt. Une ADR acceptée n’autorise pas à présenter la fonctionnalité comme livrée.
7. **Accepter tôt les invariants qui figent P0.** Reporter les mécanismes qui dépendent de P2/P4 (worker, lease/fencing, moteur d’isolation, pipeline OCI).
8. **Traçabilité non circulaire.** La baseline Apparatus est le document utilisateur et le wiki commité le 9 septembre 2026 (`d0664e3`). Un hub wiki mis à jour après une ADR ne constitue pas une preuve indépendante de son acceptation.

## Vague 1 — Apparatus (0001–0008 Accepted)

| ID | Décision | Réalité | Notes wiki |
|---|---|---|---|
| [0001](0001-apparatus-binding-owned-by-manifesto.md) | Binding = `ProjectComponent` 1:1, propriété métier Manifesto | Partial | bindings, plan |
| [0002](0002-apparatus-contract-first.md) | Contrats + Apparatus KV de référence avant Factory et host | Implemented | A-DEC 2026-09-22 ; Kind V1 ; Factory/host = P5/P6 après ; dette D-TRANSIT-TCB / D-ADMB / D-PROD |
| [0003](0003-apparatus-untrusted-plugin.md) | Code auteur hors des processus privilégiés | Implemented | isolation Kind/Calico + 4 SA invoke ; harness test-only OK |
| [0004](0004-apparatus-capability-gateway.md) | Gateway seule I/O ; KV plateforme ; pas de bearer IAM | Implemented | capabilities — gateway réseau = Lazaret invoke (P3 T7) |
| [0005](0005-apparatus-same-protocol-valid-verified.md) | Même protocole ; admission, `VALID`, `VERIFIED` et installabilité distincts | Implemented | Adm-A = source `VALID` ; Adm-B jamais source (`D-ADMB`) ; `VERIFIED` = P6 après |
| [0006](0006-apparatus-p2-reconciliation-in-process.md) | Réconciliation P2 = contrôleur in-process Manifesto, sans infra réelle | Implemented | bindings, plan — Accepted 2026-09-12 |
| [0007](0007-apparatus-p3-capability-boundary-after-accept.md) | Frontière P3 = BC Lazaret, distinct de Manifesto ; 0006 G et E restent | Implemented | Accepted 2026-09-13 ; Réalité Implemented (A-DEC 2026-09-20) ; holes hors-jalon, ne bloquent plus Implemented |
| [0008](0008-apparatus-p4-k8s-isolation-outside-manifesto.md) | Moteur P4 = K8s hors Manifesto ; Cosign + Transit ; admission worker-only ; operator+Jobs ; enveloppe ≠ image CRI ; 4 SA | Implemented | Accepted 2026-09-20 ; Implemented A-DEC 2026-09-22 ; Kind = V1 ; dette D-TRANSIT-TCB / D-ADMB / D-PROD ; [0008-closeout.md](0008-closeout.md) |

Ratification 0001–0005 : revue orchestrée du 2026-09-10. ADR-0002 / 0003 / 0005 : Réalité **Implemented** (A-DEC 2026-09-22 ; chaîne Kind M1–M6 ; Kind = V1) ; Factory/host = P5/P6 **après** ; dette D-TRANSIT-TCB / D-ADMB / D-PROD. ADR-0006 : Accept explicite utilisateur du 2026-09-12 (checklist A–M figée). `Accepted` fixe la cible ; T1–T7 + writer backoff §D sont livrés (Réalité Implemented). ADR-0007 : Accept explicite utilisateur du 2026-09-13 (« accepté ») ; checklist 1–14 ratifiée ; G et E **non levées**. Réalité **Implemented** (A-DEC 2026-09-20 ; T1–T14b livrés) ; holes (`APP-05`, G/E, pas K8s-as-P3, pas de second protocole) **hors-jalon**, ne bloquent plus Implemented. Inventaire de clôture : [0007-closeout.md](0007-closeout.md). ADR-0008 : ratification chat « Je valide tout. Je ratifie tout. » du 2026-09-20 (même force que 0007 « accepté ») ; README L17 / L158 : `Accepted` typiquement après PR — **écart documenté** comme 0006/0007. Réalité **Implemented** (A-DEC 2026-09-22 ; T2–T12 `apparatus-operator` + Kind `apparatus-p4-it` + M1–M6) ; dette TCB **hors-jalon**, ne bloque plus Implemented. Inventaire de clôture : [0008-closeout.md](0008-closeout.md).

## Traçabilité de la baseline du 9 septembre

| Recommandation documentée | Décision canonique | Encore ouvert |
|---|---|---|
| Propriété métier Manifesto | ADR-0001 | Split admit vs signer encore ouvert (0008) |
| Processus privilégiés séparés du plugin | ADR-0003 | Déploiement des workers |
| Migration 1:1, UUID et ACL `component` conservés | ADR-0001 | Migration P1 |
| Un Apparatus canonique du même type par projet | ADR-0001 | Plusieurs instances hors V1 |
| Identité d’installation immuable, jamais `latest` | ADR-0002 | Artifacts P4 = 0008 (Implemented, Kind V1) |
| Desired/observed, génération, lease et fencing | ADR-0006 | Colonnes et sémantique figées ; Réalité Implemented |
| Gateway = ACL ∩ consentement ∩ politique | ADR-0004 | Livrée P3 (Lazaret) ; restes `APP-05` / `APP-03` / `APP-06` |
| Runtime managed, plugin isolé | ADR-0003 | Moteur = ADR-0008 (Implemented) |
| Host à créer ; UI schema ou bundle statique | ADR-0002 (contrat seulement) | Sécurité du host avant P5 |
| KV par binding ; secrets par référence | ADR-0004 | Restes `APP-03` / `APP-06` ; secrets = 0007 T12 |
| Git → artifacts → conformance → admission | ADR-0008 | Réalité Implemented (closeout [0008-closeout.md](0008-closeout.md)) |
| Même protocole ; `VALID` distinct de `VERIFIED` | ADR-0005 | Publishers (`APP-02`) |

Hub wiki : `obsidian/AI FOR ALL/projects/manifesto/decisions/index.md`.

## Schéma de numérotation

Les identifiants ne sont **pas** un seul compteur global. Plages thématiques :

| Plage | Sujet | Statut typique |
|---|---|---|
| **0001–0099** | Apparatus (plateforme) | Vague 1 : 0002–0008 `Implemented` sauf 0001 `Partial` |
| **0100–0199** | Hexagone RustyCog / responsabilités des crates | Rétroactif, architecture actuelle |
| **0200–0299** | Tests (IT vs unitaires, mocks vs infra réelle) | Rétroactif |
| **0300–0399** | Événements, outbox, AuthN/AuthZ | Rétroactif |
| **0400–0499** | Services du workspace et runtimes | 0400–0406 : rétroactif Vague 2 ; **0407+ : vivant IAM-IdP** (`Accepted` / `Implemented`) |
| **0500–0599** | Config, CI, qualité, framework rustycog | Rétroactif |
| **0600–0699** | Cloud / IaC / GitOps / topologie cluster | **Vague 4 living** (`Proposed` / `Unimplemented`) |

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

## Vague 3 — IAM living / IdP fédérés (0407+, hors Apparatus)

Jalon **IAM-IdP** : le template P0–P6 Apparatus ne s’applique pas. IAM reste l’IdP *plateforme* ([0400](0400-iamrusty-identite-hexagonale.md)) ; les Connect sont des *federated authenticators*. **Ne pas** utiliser 0009 (collision sémantique P4). **Ne pas** réécrire 0100–0502. Prochains libres services/runtimes : **0412+**.

Wiki (pointeurs, pas canon) : `obsidian/AI FOR ALL/projects/iamrusty/decisions/`. Index Manifesto 0001–0008 : **hors sujet**.

| ID | Décision | Statut | Réalité |
|---|---|---|---|
| [0407](0407-contrat-authn-federee-vendor-neutral.md) | Contrat fédéré vendor-neutral ; le domaine IAM ne connaît pas GitHub/GitLab | Accepted | Implemented |
| [0408](0408-connecteurs-idp-services-http.md) | GitHub/GitLab Connect = services HTTP + crate contrat ; pas de factory in-process | Accepted | Implemented |
| [0409](0409-confiance-callback-oauth-idp-connect.md) | Callback/CSRF/linking = IAM ; secrets vendor + appel OAuth = connecteur ; HMAC+HTTPS | Accepted | Implemented |
| [0410](0410-migration-iam-connecteurs-idp.md) | Migration incrémentale ; routes `/api/auth/{provider}` conservées | Accepted | Implemented |
| [0411](0411-idp-provider-slug-registry-fail-closed.md) | Provider = slug typé ; catalogue = registry au boot ; admission fail-closed ; pas de hot-load | Proposed | Unimplemented |

## Vague 4 — Cloud portable / cluster (0600+, hors Apparatus)

Jalon **Cloud-portable** : le template P0–P6 Apparatus ne s’applique pas. P4 ([0008](0008-apparatus-p4-k8s-isolation-outside-manifesto.md), Accepted / Implemented ; dette D-TRANSIT-TCB / D-ADMB / D-PROD) **s’insère** dans cette plateforme ; 0600/0601/0602 **ne sont pas** des ADR Apparatus et **ne SuperSèdent pas** 0008. **Ne pas** utiliser 0009 (collision sémantique P4), 0412 (prochain IAM-IdP), 0503 (CI rétro 0500). **Ne pas** réécrire 0001–0008 ni 0100–0502 ni 0407–0411. Prochain libre cloud : **0603+**.

Wiki (pointeurs, pas canon) : `obsidian/AI FOR ALL/projects/aiforall/decisions/0600-cloud-portable.md`, `0601-cluster-topology.md`, `0602-observabilite-portable.md`. Index Manifesto 0001–0008 : **Related seulement** (P4 se loge ici), pas une fusion Apparatus.

Contrat : `docs/platform-cloud-v1-implementation-contract.md`. Canon Vague 4 = **0600 + 0601 + 0602**.

| ID | Décision | Statut | Réalité |
|---|---|---|---|
| [0600](0600-cloud-portable-opentofu-k8s-gitops.md) | Trois couches compose/kind/remote k8s ; OpenTofu jusqu’au cluster ; premier adapter **GKE (`gcp`)** ; **Flux** + Kustomize ; image digest commune | Proposed | Unimplemented |
| [0601](0601-cluster-trust-namespaces-standalones.md) | Unité cluster = 4+1 Deployments ; namespaces `aiforall-*` identiques ; pas ns-per-tenant | Proposed | Unimplemented |
| [0602](0602-observabilite-portable-otlp-lgtm.md) | Câble tracing+OTLP rustycog (plan A) ; LGTM/Tempo premier adaptateur cluster derrière collector/Alloy (plan B) ; W3C `traceparent` | Proposed | Unimplemented |

0601 **ferme** le Non décidé 0404 « 1 vs 4 Deployments » sans éditer 0404. 0602 **ferme** Q3 0600 (câble apps vs premier adaptateur cluster LGTM/Tempo) **sans** SuperSéder 0600. Grafana n’est **pas** dans le SDK Rust. 0500 (liste Compose stale vs `docker-compose.yml` actuel) : écart **cité** dans 0600/0601, **pas corrigé** dans la photographie Accepted.

## Comment en ajouter une

1. Copier [template.md](template.md) → `NNNN-verbe-court.md` dans la **plage** du sujet (pas le prochain entier global).
2. Remplir Contexte / Décision / Conséquences / Alternatives. Lier les notes wiki et, si ça en remplace une, l’ADR superédée.
3. Ajouter la ligne dans ce README et, pour Apparatus, dans l’index wiki Manifesto.
4. Ouvrir une PR. **Accepted** seulement après accord explicite sur le texte final, pas parce qu’une note wiki le cite.

## Hors vague 1 (pas d’ADR tant que le jalon n’ouvre pas)

| Sujet | Quand |
|---|---|
| Génération, lease, fencing du contrôleur | ADR-0006 Accepted (2026-09-12) |
| Pipeline Git → OCI, builders, registry | ADR-0008 Accepted (2026-09-20) |
| Host UI, origines iframe, MessageChannel | Avant P5 |
| Moteur d’isolation (ADR-0008) ; CPU / budget numériques (`APP-06`) | Moteur tranché 2026-09-20 ; CPU/budget numériques encore ouverts |
| Politique catalogue / publishers (`APP-02`) | Avant soumissions tierces |
| Révocation : drain/destruction des bindings déjà `ready` | Avant P4/P6 ; le refus de nouvelles opérations est déjà fixé par ADR-0005 |
