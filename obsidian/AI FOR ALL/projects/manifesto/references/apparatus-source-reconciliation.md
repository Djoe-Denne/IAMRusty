---
title: "Apparatus — confrontation du document au dépôt"
category: references
tags: [components, architecture, reference, roadmap, visibility/internal]
status: proposed
feature_status: future
sources:
  - "C:/Users/djden/.codex/attachments/486d0052-5759-4277-bcc1-9f209ce353d4/pasted-text.txt"
  - Manifesto/domain/src/entity/project_component.rs
  - Manifesto/domain/src/port/service.rs
  - Manifesto/application/src/usecase/component.rs
  - Manifesto/infra/src/transaction.rs
  - apparatus-events/src/component.rs
  - openfga/model.fga
  - sentinel-sync/src/translator/manifesto.rs
  - docker-compose.yml
  - monolith/src/runtime.rs
evidence_commit: bbf236eee7b5be08477de3ed30d34c6ff280e81b
rustycog_commit: cb0cff1d0130a848d781b18ad3def58055e7fabd
source_sha256: 4c1f93d4ec2319babdc4c724ff90c09bdd66db0ff68193cb6374aa06f2385c13
summary: "État vérifié du dépôt au 9 septembre 2026, écarts avec le document Apparatus et provenance des décisions futures proposées."
provenance:
  extracted: 0.85
  inferred: 0.15
  ambiguous: 0.0
created: 2026-09-09T17:50:00Z
updated: 2026-09-09T17:50:00Z
---

# Apparatus — confrontation du document au dépôt

Le document **« Architecture du système d’Apparatus »**, fourni le 9 septembre 2026, décrit une plateforme d’extensions souhaitée. À la demande de l’utilisateur, ses descriptions du contexte ne sont pas prises comme preuve d’une implémentation. Cette note sépare les faits du dépôt des propositions de [[projects/manifesto/concepts/apparatus-platform]].

## Provenance et méthode

- Source originale : [document fourni](C:/Users/djden/.codex/attachments/486d0052-5759-4277-bcc1-9f209ce353d4/pasted-text.txt), 40 912 octets ; SHA-256 dans le frontmatter.
- Photographie locale : commit `bbf236eee7b5be08477de3ed30d34c6ff280e81b`, submodule RustyCog `cb0cff1d0130a848d781b18ad3def58055e7fabd`. L’arbre était propre avant les ajouts documentaires.
- Vérification : Serena sur AIForAll, recherches GrepAI, QMD collection `aiforall-wiki`, lectures ciblées des symboles, migrations, configuration et documentation. QMD est une aide de découverte ; le fichier présent sur disque et le code priment sur son index.
- Destination : `C:/Users/djden/source/repos/AIForAll/obsidian/AI FOR ALL`. La configuration Obsidian globale pointait vers un autre projet et n’a pas été modifiée.
- Deux audits Terra ont couvert backend/AuthZ et frontend/infrastructure ; une lecture Luna a contrôlé l’intégration documentaire. La synthèse et les recommandations ont été relues ensemble.
- Aucun build, test d’intégration ou déploiement n’a été exécuté pour cet audit de documentation. « Présent » signifie trouvé dans le code inspecté, pas validé en production. « Absent » se limite au dépôt audité.

## Matrice source / réalité

| Sujet du document | Réalité constatée | Conséquence pour l’implémentation |
|---|---|---|
| IAM / Organization / Telegraph / Manifesto | Les noms réels sont IAMRusty, **Hive**, Telegraph et Manifesto, avec sentinel-sync et RustyCog | Réutiliser Hive pour le contexte organisationnel |
| Projet extensible | `ProjectComponent` existe, attaché au projet par UUID/type/statut | Évoluer depuis cette ancre ; ne pas créer deux ACL pour une installation |
| Catalogue | `ComponentServiceClient` appelle un catalogue HTTP externe | Le catalogue versionné et l’admission OCI restent à construire |
| Version / endpoint | `ComponentInfo` inclut version et endpoint ; le composant ne les persiste pas | Figer la release et le digest dans le futur binding |
| Provisioning | DTO endpoint/access_token restent `None` | Aucun handoff runtime complet existant |
| Événements Apparatus | Crate et consumer de `ComponentStatusChanged` présents | Ne pas assimiler ce consumer à un reconciler |
| Binding / desired state | Pas de type ou stockage de binding versionné, de génération ou d’opération runtime | Table 1:1 et worker à ajouter |
| Droits | ACL DB et modèle OpenFGA `project/component`, synchro sentinel-sync | Préserver ces identifiants ; ajouter les capacités, pas remplacer l’ACL |
| Atomicité | Ajout/retrait composant, ACL et outbox utilisent une UoW transactionnelle dans le runtime câblé | S’appuyer dessus, sans transaction distribuée avec le runtime |
| Host UI | Aucun frontend produit dans Manifesto ; React limité à une fixture QA IAM | Host, slots, schema renderer, SDK et UI d’installation sont de nouveaux travaux |
| Factory / OCI | Dockerfile du service Manifesto présent ; pas de Factory Apparatus, signature ni admission de releases | Nouveau pipeline isolé nécessaire |
| Kubernetes | Compose local présent ; pas de déploiement Kubernetes Apparatus dans le dépôt | Cluster/profil sécurité/adapter sont des prérequis futurs |
| Monolithe | Mode microservices et `oodhive-monolith` présents | Même API de contrôle dans les deux ; plugins hors processus |
| `shared/organization/project`, scale-to-zero, WASI, SaaS | Aucun de ces runtimes Apparatus démontré par le code | Préserver les interfaces, limiter la première livraison |

Les conséquences de la dernière colonne sont des recommandations de conception, pas des faits déjà livrés. ^[inferred]

## Preuves de code

Les lignes sont indicatives pour le commit de référence ; les symboles constituent les points d’entrée à privilégier si les fichiers évoluent.

| Référence | Ce qu’elle établit |
|---|---|
| [ProjectComponent](C:/Users/djden/source/repos/AIForAll/Manifesto/domain/src/entity/project_component.rs:8) | UUID, projet, type, statut et timestamps ; pas de digest/version/runtime |
| [ComponentStatus](C:/Users/djden/source/repos/AIForAll/Manifesto/domain/src/value_objects/component_status.rs:12) | Modèle métier existant pending/configured/active/disabled |
| [Migration project_components](C:/Users/djden/source/repos/AIForAll/Manifesto/migration/src/m20241015_000002_create_project_components_table.rs:82) | Unicité projet/type ; table avec FK de projet en cascade |
| [ComponentServicePort / ComponentInfo](C:/Users/djden/source/repos/AIForAll/Manifesto/domain/src/port/service.rs:5) | Catalogue externe : type, nom, description, version et endpoint |
| [ComponentServiceClient](C:/Users/djden/source/repos/AIForAll/Manifesto/infra/src/adapters/component_service_client.rs:60) | GET /api/components, timeout, bearer API key et échec fermé |
| [add_component](C:/Users/djden/source/repos/AIForAll/Manifesto/application/src/usecase/component.rs:164) | Validation de type, quota, unicité, création et droits |
| [ProjectAuthorizationUnitOfWork](C:/Users/djden/source/repos/AIForAll/Manifesto/infra/src/transaction.rs:479) | Ajout/retrait avec données, ACL et outbox atomiques |
| [ComponentStatusProcessor](C:/Users/djden/source/repos/AIForAll/Manifesto/infra/src/event/processors/component_processor.rs:33) | Recherche projet/type, comparaison ancien statut, doublons/péremption, date de changement |
| [apparatus-events](C:/Users/djden/source/repos/AIForAll/apparatus-events/src/component.rs) | Événement entrant sans binding_id ni génération |
| [ApparatusEventHandler](C:/Users/djden/source/repos/AIForAll/Manifesto/infra/src/event/consumer.rs:144) | Traitement du transport ; pas de vérification d’identité du publisher dans ce handler |
| [Modèle OpenFGA](C:/Users/djden/source/repos/AIForAll/openfga/model.fga:42) | Type component, parent project et héritages editor/viewer |
| [Traducteur Manifesto](C:/Users/djden/source/repos/AIForAll/sentinel-sync/src/translator/manifesto.rs) | Attachement/détachement vers tuples ; statut sans delta AuthZ |
| [Porte de lecture des composants](C:/Users/djden/source/repos/AIForAll/Manifesto/application/src/usecase/world_read.rs:87) | Gate instance et exception de lecture publique |
| [Routes Manifesto](C:/Users/djden/source/repos/AIForAll/Manifesto/http/src/lib.rs:14) | Préfixe /manifesto et surface API project/components/members |
| [Configuration](C:/Users/djden/source/repos/AIForAll/Manifesto/config/default.toml:8) | Contrat JWT utilisateur issuer/audience |
| [Dockerfile Manifesto](C:/Users/djden/source/repos/AIForAll/Manifesto/Dockerfile:5) | Image du service plateforme, pas builder générique pour code tiers |
| [Compose](C:/Users/djden/source/repos/AIForAll/docker-compose.yml:187) | Assemblage local ; ne prouve pas de cluster Kubernetes |
| [Runtime monolithe](C:/Users/djden/source/repos/AIForAll/monolith/src/runtime.rs) | Composition et tâches de fond des services existants |

## Corrections conceptuelles importantes

1. Le mot « Apparatus » apparaît déjà dans les événements. Cette présence ne signifie pas que la plateforme décrite existe.
2. Le client catalogue fait déjà échouer l’ajout si l’amont ne répond pas correctement. Il ne faut pas réintroduire de fallback catalogue permissif pour rendre les nouveaux tests verts.
3. Les droits par instance ne découlent pas simplement de l’appartenance au projet. En revanche, la lecture publique du projet actif ouvre actuellement ses composants : cela exige une décision explicite pour les futures données/actions d’Apparatus.
4. L’outbox AuthZ et la comparaison de statut existent, mais ne garantissent pas une réconciliation runtime avec générations, opérations durables et identité authentifiée.
5. Les mécanismes d’impersonation/discovery évoqués dans certaines anciennes pages restent des pistes historiques. Le nouveau contrat recommandé utilise un gateway et une identité de workload dédiée, sans fournir de bearer IAM au plugin. ^[inferred]

## Couverture de l’intention originale

| Sections du document | Intention conservée / note de résolution |
|---|---|
| 1–7 | Extension officielle/communautaire, Rust, Git, structure, manifeste, absence de Dockerfile tiers → [[projects/manifesto/references/apparatus-factory-and-distribution]] |
| 8–17 | UI schema/custom, builders statiques, sandbox, slots, SDK sans credentials → [[projects/manifesto/references/apparatus-ui-and-protocol]] |
| 18–23 | Capabilities, réseau, isolation, consentement, build hostile, piste WASI → [[projects/manifesto/concepts/apparatus-capabilities-and-isolation]] |
| 24–31 | Factory, OCI, provenance, conformance, VALID/VERIFIED, Catalogue/Registry → [[projects/manifesto/references/apparatus-factory-and-distribution]] |
| 32–42 | Control plane, binding, réconciliation, isolation, discovery, runtime, managed/external → [[projects/manifesto/concepts/apparatus-bindings-and-lifecycle]] |
| 43–46 | Inspirations, design system, thème et CLI → [[projects/manifesto/concepts/apparatus-platform]], [[projects/manifesto/references/apparatus-ui-and-protocol]] |
| 47–52 | Parcours auteur/utilisateur, métier indépendant du runtime et philosophie → [[projects/manifesto/references/apparatus-implementation-plan]] |

Les exemples de macros, noms de packages et endpoints du document sont traités comme conceptuels. La V1 proposée restreint volontairement les modes d’isolation, builders, slots et dépendances ; ces restrictions sont nouvelles, explicites et restent révisables. ^[inferred]

## Références wiki

- [[projects/manifesto/manifesto]] — état courant du service.
- [[projects/manifesto/references/manifesto-entity-model]] — modèle actuel.
- [[projects/manifesto/references/manifesto-event-model]] — événements actuels.
- [[projects/manifesto/concepts/component-catalog-and-fallback-adapter]] — frontière catalogue.
- [[projects/aiforall/references/modular-monolith-runtime]] — deux modes de composition.

