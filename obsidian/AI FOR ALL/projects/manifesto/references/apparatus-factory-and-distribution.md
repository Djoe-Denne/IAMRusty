---
title: "Apparatus — manifeste, Factory et distribution OCI"
category: references
tags: [components, build, security, architecture, visibility/internal]
status: proposed
feature_status: future
sources:
  - "C:/Users/djden/.codex/attachments/486d0052-5759-4277-bcc1-9f209ce353d4/pasted-text.txt"
  - Manifesto/Dockerfile
  - Manifesto/domain/src/port/service.rs
  - https://doc.rust-lang.org/cargo/reference/build-scripts.html
  - https://github.com/opencontainers/image-spec/blob/main/manifest.md
summary: "Contrat TOML proposé, builders contrôlés, pipeline hostile Git vers OCI, conformance, signatures, provenance et admission de releases immuables."
provenance:
  extracted: 0.35
  inferred: 0.65
  ambiguous: 0.0
created: 2026-09-09T17:50:00Z
updated: 2026-09-09T17:50:00Z
---

# Apparatus — manifeste, Factory et distribution OCI

Le document utilisateur demande une entrée Git et une sortie OCI ; aucun Dockerfile, chart ou script de déploiement tiers dans le parcours normal. Ce pipeline appartient à la cible [[projects/manifesto/concepts/apparatus-platform]] ; le Dockerfile actuel de Manifesto construit le service plateforme, pas des Apparatus.

## Contrat source proposé

Une seule syntaxe canonique remplace les exemples conceptuels TOML/YAML parfois différents du document. Versionner le schéma du manifeste séparément du SDK et du protocole wire. Rejeter les clés inconnues, identifiants invalides et capacités non supportées plutôt que les ignorer. ^[inferred]

```text
apparatus.toml
backend/Cargo.toml
backend/Cargo.lock
backend/src/
frontend/package.json       # seulement pour UI sandbox
frontend/package-lock.json # npm en V1
frontend/src/
ui/settings.schema.json    # si UI schema
tests/                     # tests de l’auteur, optionnels en plus de la conformance
```

Exemple de **contrat cible**, pas fichier reconnu actuellement par Manifesto : ^[inferred]

```toml
manifest_version = 1

[apparatus]
id = "com.example.git"
name = "Git"
version = "1.0.0"
protocol = "manifesto-apparatus/1"

[backend]
sdk = "^1.0"
builder = "rust-native-v1"
package = "example-git"
binary = "example-git"

[runtime]
type = "managed"
supported_isolation = ["project"]

[storage]
profile = "kv-v1"
quota_mb = 100
data_schema_version = 1

[permissions]
required = ["project.read", "storage.kv.read", "storage.kv.write"]
optional = []

[[permissions.network]]
id = "github-api"
connector = "github-api-v1"

[frontend]
type = "sandbox"
builder = "vite-v1"
entry = "index.html"

[[ui.contributions]]
id = "repositories"
slot = "project.tab"
title = "Repositories"
entry = "/repositories"

[[ui.contributions]]
id = "settings"
slot = "project.settings"
mode = "schema"
schema = "ui/settings.schema.json"
```

Le publisher est lié à une identité vérifiée lors de la soumission, pas auto-certifié par une chaîne dans le TOML. `apparatus_id` est réservé dans le catalogue ; les règles de nommage doivent rester compatibles avec la limite actuelle de 100 caractères de `component_type`. `version` suit SemVer, mais l’installation référence toujours le digest résolu. ^[inferred]

`quota_mb` et `supported_isolation` sont des demandes fonctionnelles soumises à politique. Ils ne permettent pas de choisir des volumes host, un service account, une image de base ou un endpoint interne. Le connecteur réseau est une politique plateforme nommée ; supporter des domaines arbitraires déclarés demandera un mécanisme d’admission distinct. ^[inferred]

V1 : frontend absent, schema ou sandbox statique ; backend Rust obligatoire. Un Apparatus avec UI seulement est une extension possible du contrat, pas une exception implicite. Les dépendances entre Apparatus sont refusées en V1 : leur résolution exigerait graphe acyclique, contraintes de versions, droits et ordre de suppression. ^[inferred]

## SDK et compatibilité

Le SDK backend masque serveur HTTP, handshake, santé, observabilité, identité de workload, shutdown et hooks bind/configure/unbind. RustyCog peut servir à construire les composants plateforme de confiance ; le SDK tiers ne doit pas distribuer la configuration privilégiée de ces services. Le protocole wire demeure indépendant des traits Rust pour permettre plusieurs versions du compilateur ou un futur runtime WASM. ^[inferred]

La release indique protocole majeur, fonctionnalités optionnelles et plage compatible de SDK. La conformance rejette un majeur inconnu. Une évolution additive de champs ne doit pas changer le sens d’un hook ; un changement incompatible impose une nouvelle version du protocole. Le pin exact du SDK et de ses dépendances est fourni par `Cargo.lock`, pas seulement par `sdk = "^1.0"`. ^[inferred]

## Étapes de Factory proposées

| Étape | Contrôle et résultat |
|---|---|
| Soumission | Utilisateur/publisher authentifié, quotas et clé d’idempotence ; URL Git et ref demandée |
| Résolution | Transformer branche/tag/release en SHA exact et enregistrer les deux ; identifiant de publisher autorisé |
| Acquisition | Checkout sans hooks, sous-modules/LFS désactivés en V1 ; limites de taille/temps ; URL Git validée contre SSRF |
| Validation | Structure, TOML, lockfiles, chemins contenus dans le repo, tailles, compatibilité, capacités et builder |
| Dépendances | Fetch via miroir/proxy autorisé, cache compartimenté ; aucun secret de publication dans le workspace |
| Build | Toolchain et images de builder pinées par digest ; compilation et frontend dans sandboxes éphémères |
| Tests et scans | Tests Rust/frontend compatibles avec le profil, scans dépendances/secrets, SBOM, politique de licences de l’opérateur |
| Packaging candidat | Image backend non privilégiée, bundle statique, manifeste normalisé et métadonnées ; digests calculés |
| Déploiement éphémère | Installer exactement ces digests dans un environnement de test sans données réelles |
| Conformance | Suite plateforme indépendante : protocole, isolation, reprise, bind, UI et capacités |
| Admission et signature | Service de confiance vérifie les résultats liés aux digests, signe puis publie la release atomiquement dans le catalogue |

Ces étapes sont proposées, entièrement à construire. La conformance intervient sur les artifacts candidats finaux : une recompilation après les tests invaliderait la preuve et demanderait une nouvelle validation. ^[inferred]

Les refs Git mobiles sont acceptables comme saisie, jamais comme identité de build. L’URL doit aussi être normalisée et interdite de transport local/`file:`, de commande SSH arbitraire ou d’accès réseau interne. Revalider résolution DNS et chaque redirection, y compris pour les dépendances Git ; le worker n’a pas de réseau libre. Un dépôt privé utilise un fetcher plateforme avec credential limité ; celui-ci copie le snapshot source nettoyé vers le worker sans lui transmettre le credential. ^[inferred]

## Builders contrôlés, code néanmoins hostile

Rust peut compiler puis exécuter `build.rs` avant la compilation du package. La sélection d’un builder connu ne rend donc pas le build fiable. [Cargo — build scripts](https://doc.rust-lang.org/cargo/reference/build-scripts.html).

Les proc macros, configurations Vite et tests sont eux aussi du code fourni par l’auteur. Le profil V1 ne lance pas de commande libre `build="..."` ou `npm run ...` choisie dans le manifeste : il utilise des commandes plateforme. Pour npm, installer avec lockfile et scripts lifecycle désactivés, puis n’autoriser que les étapes nécessaires définies par le builder ; les packages incompatibles sont refusés jusqu’à extension explicite du profil. La sandbox reste obligatoire pour les commandes autorisées. ^[inferred]

Séparer récupération contrôlée des dépendances et compilation aussi fermée que possible. Un cache partagé doit être immuable ou validé par digest, jamais un répertoire writable commun à deux publishers. Le worker ne reçoit ni socket Docker, ni credentials cloud, ni clé de signature, ni droits de publication OCI. Les sorties de tests/scanners sont non fiables : plafonner, parser et vérifier au niveau de l’orchestrateur. ^[inferred]

`cargo build --release --locked` empêche une mise à jour implicite du lockfile ; il ne prouve pas la reproductibilité bit à bit. Capturer toolchain, cible, images de base, dependencies et environnement. La V1 garantit l’immuabilité de l’artifact admis et la traçabilité du build ; revendiquer un build reproductible exigera des reconstructions indépendantes comparant les digests. ^[inferred]

## Distribution

Un manifeste OCI peut porter un type d’artifact, des blobs et un `subject` reliant des métadonnées à un autre artifact. Ces mécanismes permettent de distribuer autre chose qu’une image exécutable. [Spécification OCI](https://github.com/opencontainers/image-spec/blob/main/manifest.md).

Proposition : un **descripteur de release** OCI immuable référence explicitement le manifeste fonctionnel normalisé, l’image backend et le bundle frontend. SBOM et attestations sont liées au digest ; le catalogue stocke ce descripteur, son résultat d’admission et l’identité de signature. Le runtime vérifie la chaîne de confiance et déploie les digests, jamais un tag. ^[inferred]

La provenance minimale comprend URL source normalisée, ref demandée, commit exact, empreinte source, lockfiles, builder et toolchain, images de base, plateforme cible, digests produits, suite de conformance/politique/scanners utilisés, résultats, date et identité de signature. Une attestation de provenance prouve ce qui a été construit, pas l’innocuité du code. ^[inferred]

Le registry choisi devra supporter les artifacts/attestations retenus et leur export offline ; prouver ce comportement par un test de push/pull/vérification, pas par l’étiquette « compatible OCI ». La distribution self-hosted exporte le descripteur et tous ses blobs/références plus les racines de confiance configurées par l’opérateur. ^[inferred]

## VALID, VERIFIED et révocation

Le document distingue `VALID` (conformité technique automatisée) et `VERIFIED` (confiance supplémentaire). Garder cette distinction : les Apparatus officiels et communautaires ont les mêmes contrats et tests.

`VALID` dépend d’un digest, d’une version de politique et d’un rapport de conformance. Il ne signifie pas « autorisé partout » : installation soumise aux capacités consenties et à la politique locale. Une politique nouvelle ou une vulnérabilité découverte peut suspendre l’admission ; publier une base runtime corrigée produit une nouvelle release/digest, sans modifier silencieusement l’ancienne. ^[inferred]

`VERIFIED` appartient aux métadonnées de confiance du catalogue. Une signature prouve le signataire ; elle ne remplace pas une revue ni un contrôle d’autorisation. Les clés de signature, leur rotation et les listes de révocation appartiennent au service d’admission de confiance. ^[inferred]

## CLI de développement

Les commandes candidates `manifesto apparatus check/dev/publish` utilisent le même validateur versionné que la Factory. `dev` fournit un host local, des API factices et une sandbox, avec des identités de test. `publish` soumet un commit ; il n’envoie pas un binaire local considéré comme autoritaire. Les simulations locales ne délivrent jamais le statut catalogue `VALID`. ^[inferred]

## Liens

- [[projects/manifesto/references/apparatus-ui-and-protocol]] — contrat du bundle et hooks.
- [[projects/manifesto/concepts/apparatus-capabilities-and-isolation]] — frontières des workers et workloads.
- [[projects/manifesto/concepts/component-catalog-and-fallback-adapter]] — client HTTP historique à migrer.
- [[projects/manifesto/references/apparatus-implementation-plan]] — jalons et preuves d’acceptation.
