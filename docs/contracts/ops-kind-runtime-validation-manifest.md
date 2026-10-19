# Manifeste ops — séparation runtime/validation

Statut Proposed, base 80918ca, auteur D/pool-4. Contrat : [séparation ops](ops-kind-runtime-validation-separation.md). Inventaire Git vérifié : **150 fichiers**, répartis ci-dessous sans recouvrement ; familles ignorées et consommateurs externes en fin de document.

## Format normalisé et preuve

**Chaque chemin énuméré est un enregistrement individuel**, héritant de tous les attributs du profil de sa section. Chemins relatifs à ops/ ; destination « identique » = même chemin complet sous ops/. Un profil n’autorise aucun glob de suppression. Les extensions précisent le rôle individuel : README = guide humain ; kustomization = composition ; YAML/TOML/JSON/FGA = entrée de configuration nommée ; Dockerfile = recette image ; script = opération nommée. Producteur par défaut : source versionnée maintenue par auteurs du dépôt, pas sortie d’une campagne ; exceptions explicites ci-dessous. Les guides sont consommés par l’opérateur, ne sont pas appliqués comme manifestes. Chaque section fournit rôle, producteur/consommateurs, dépendances, effets, statut de preuve, destination, décision, justification et owner exclusif.

Preuve commune **SOURCE**, jamais LIVE : audit A (graphe E1–E7), B (drivers/C1–C6), C (consommateurs/inventaire) consolidés dans le ledger du 2026-10-19 ; chemins vérifiés par git ls-files ops au hash de base. Une référence ou un parse ne prouve pas un déploiement fonctionnel. « Pas de consommateur identifié » reste hypothèse documentée, jamais « inutilisé ». Pour un script, les effets décrits sont ceux de son exécution potentielle, non exécutée par D. L’archivage demeure suspendu à l’accord utilisateur.

## R1 — Sources local-full (19)

- Rôle : configuration/bootstrap nominal et guide ; producteur : sources maintenues ; consommateurs : renderer/deploy, Kustomize, Jobs FGA/OpenBao ; guide : opérateur.
- Dépendances : R2, R5–R9, S1, configs/keys applicatives déjà référencées par platform/kustomization.yaml:9–53 ; aucune injection d’une fixture Rust dans ops.
- Effets : rendu offline sans API ; apply/Jobs init DB/FGA/Transit/config, NP/PSS et workloads sous bail.
- Preuve : A E1 items6–8, platform/kustomization.yaml:3–5,46 ; helpers/fga-bootstrap.py:13,50–64 ; SOURCE seulement.
- Destination identique ; KEEP-RUNTIME ; justification : fermeture effective du profil choisi, pas suppression par vocabulaire gold/bootstrap ; owner I-runtime.

- deploy/apps/overlays/local-full/README.md
- deploy/apps/overlays/local-full/apparatus/kustomization.yaml
- deploy/apps/overlays/local-full/apparatus/patches.yaml
- deploy/apps/overlays/local-full/apparatus/resources.yaml
- deploy/apps/overlays/local-full/config/iam.toml
- deploy/apps/overlays/local-full/config/lazaret.toml
- deploy/apps/overlays/local-full/config/sentinel.toml
- deploy/apps/overlays/local-full/helpers/fga-bootstrap.py
- deploy/apps/overlays/local-full/helpers/seed-openbao.sh
- deploy/apps/overlays/local-full/kustomization.yaml
- deploy/apps/overlays/local-full/namespaces.yaml
- deploy/apps/overlays/local-full/platform/bootstrap.yaml
- deploy/apps/overlays/local-full/platform/budgets.yaml
- deploy/apps/overlays/local-full/platform/dependencies.yaml
- deploy/apps/overlays/local-full/platform/kustomization.yaml
- deploy/apps/overlays/local-full/platform/lazaret-sentinel.yaml
- deploy/apps/overlays/local-full/platform/patches.yaml
- deploy/apps/overlays/local-full/platform/policies.yaml
- deploy/apps/overlays/local-full/plugins.yaml

## R2 — kind-mesh réutilisé par full (6)

- Rôle : base mesh/standalones, pas copie locale-full ; producteur source ; consommateurs deploy-mesh, full platform/kustomization.yaml:3–5 et rendu CI ; README humain.
- Dépendances R8, configs/services, images ; effets apply workloads/envoy/ext-authz/NP, pas de runtime pendant rendu.
- Preuve A E5 et C couplage cross-overlay ; SOURCE ; destination identique ; KEEP-RUNTIME : déplacer cet overlay casserait le nominal ; owner I-runtime.

- deploy/apps/overlays/kind-mesh/README.md
- deploy/apps/overlays/kind-mesh/envoy-mesh.yaml
- deploy/apps/overlays/kind-mesh/ext-authz.yaml
- deploy/apps/overlays/kind-mesh/kustomization.yaml
- deploy/apps/overlays/kind-mesh/networkpolicy.yaml
- deploy/apps/overlays/kind-mesh/platform-services.yaml

## R3 — Base stubs/legacy (7)

- Rôle : base historique de déploiement ; producteur source ; consommateurs overlays kind/J2/J3 et rendu CI, pas preuve de quatre services réels.
- Dépendances R4/O2/R6 ; effets déclaratifs namespaces/SA/NP/stubs si appliqués ; preuve A E2–E4, C inventaire ; SOURCE.
- Destination identique ; KEEP-RUNTIME (compatibilité non nominale), justification : capacité conservée sans la rebaptiser full ; owner I-runtime.

- deploy/apps/base/kustomization.yaml
- deploy/apps/base/lazaret.yaml
- deploy/apps/base/namespaces.yaml
- deploy/apps/base/networkpolicies.yaml
- deploy/apps/base/platform-workloads.yaml
- deploy/apps/base/plugin-stub.yaml
- deploy/apps/base/serviceaccounts.yaml

## R4 — Overlay kind legacy hors payload de sonde (5)

- Rôle : composition/configuration legacy ; producteur source ; consommateurs verify-m*/J2/rendu CI. helmrelease-envoy-gateway.yaml est une déclaration hors resources : pas de contrôleur Flux livré ; le lint M3 utilise O6/values.yaml.
- Dépendances R3, O6, V2 actuellement ; effet apply configuration stub. I-runtime retire seulement le lien V2 du kustomization, I-validation déplace le payload.
- Preuve A E2, C helmrelease hors resources ; SOURCE. Destination identique ; KEEP-RUNTIME ; justification : préserver usages historiques, désolidariser la sonde ; owner I-runtime.

- deploy/apps/overlays/kind/helmrelease-envoy-gateway.yaml
- deploy/apps/overlays/kind/kustomization.yaml
- deploy/apps/overlays/kind/lazaret-invoke-configmap.yaml
- deploy/apps/overlays/kind/patch-lazaret-http.yaml
- deploy/apps/overlays/kind/patch-plugin-http.yaml

## S1 — Modèles OpenFGA canoniques (2)

- Rôle : contrat AuthZ partagé ; producteur model.fga : auteurs ; model.json : just regenerate-openfga-model-json depuis le DSL, **sortie canonique à committer**, pas généré jetable.
- Consommateurs : cinq include_str services/Telegraph/tests/common.rs:96, Manifesto/tests/common.rs:90 et sqs_event_routing_tests.rs:95, Hive/tests/common.rs:94 et sqs_event_routing_tests.rs:80 ; six lectures DSL IT Manifesto (p1_t5_acl:28,p2_t3_persist:88–92,p3_t1_absence:209–213,p3_t4_gateway:384,p3_t5_consent:394,p3_t6_kv:54) ; ensure-host-store:8,72 ; full platform/kustomization:46 ; mesh-authn-kind-e2e.sh:93.
- Dépendances CLI fga ou fallback Docker explicite ; effets régénération JSON, uploads par consommateurs ; TestOpenFga reçoit JSON en paramètre (pin41a86dba, openfga_testcontainer.rs:499), aucun chemin framework à migrer.
- Preuve A+B+C recoupée ; SOURCE. Destination identique ; SHARED-CONTRACT ; justification : runtime ET tests, copie unique ; owner I-runtime pour modèles, I-références seulement pour justfile:156.

- openfga/model.fga
- openfga/model.json

## R5 — Images auxiliaires runtime (2)

- Rôle recettes images ; producteur source, images produites par docker build explicite ; consommateurs préparation full/J2 et local-full-images.py:9–11.
- Dépendances Dockerfile.build/local-build-artifacts pour worker, image Zot pour Dockerfile.zot ; effets build/tag images Linux, registre IDs ; pas tests de développement en Docker.
- Preuve A E1/images, C builds ; SOURCE. Destination identique ; KEEP-RUNTIME : fermeture de livraison ; owner I-runtime.

- deploy/build/Dockerfile.worker
- deploy/p4/Dockerfile.zot

## R6 — P4 runtime/ressources déclarées (11)

- Rôle manifests P4/admission/NP/RBAC ; producteur source ; consommateurs Kustomize P4, J2/J3, full apparatus/kustomization:8–16 (sous-ensemble référencé), CI rendu. admit-job reste un outil de préparation non nominal : pas exécution automatique par full.
- Dépendances R5/R7, images/config/identités admission ; effets apply CRD/RBAC/operator/Zot et Job/signature si explicitement demandé ; pas garantie que le matériel/signing hors mission existe.
- Preuve A E1/E3/E4, C effets transitifs ; SOURCE. Destination identique ; KEEP-RUNTIME : contrats/ressources réutilisés, ne pas confondre assets et fixture Rust ; owner I-runtime.

- deploy/p4/admit-job.yaml
- deploy/p4/allow-admit-zot-np.yaml
- deploy/p4/crd-admissionrecord.yaml
- deploy/p4/kustomization.yaml
- deploy/p4/namespace.yaml
- deploy/p4/operator.yaml
- deploy/p4/rbac-admit-sign.yaml
- deploy/p4/rbac-plugins.yaml
- deploy/p4/rbac-watch.yaml
- deploy/p4/serviceaccounts.yaml
- deploy/p4/zot.yaml

## R7 — Topologies et marqueur racine (2)

- Rôle configs Kind ; producteur source ; consommateurs common.ps1:6–7,83–123 et bootstrap, validate-deploy-static:20–21.
- Dépendances image Kind/pins/CNI et bail ; effets création cluster seulement sur demande/autorisation. cluster.yaml est aussi marqueur de découverte racine : le déplacer casserait toutes les entrées.
- Preuve A E1, C contrat caché ; SOURCE. Destination identique ; KEEP-RUNTIME ; justification : topologie et compatibilité discovery ; owner I-runtime.

- deploy/kind/cluster-local-full.yaml
- deploy/kind/cluster.yaml

## R8 — Certificats et seed à double usage (3)

- Rôle bootstrap runtime ; producteur scripts source ; consommateurs generate-platform-mesh : Compose/deploy-mesh:34–43,106 ; prepare-full : opérateur + cert_inputs:91–105 ; openbao-gold : service Compose openbao-seed + prove-gold:219–231.
- Dépendances OpenSSL/Bash, répertoires privés neufs, OpenBao/Transit ; effets écritures clés/certificats ou secrets/config seed ; protection/logs/cleanup obligatoires.
- Preuve A E1, B bootstrap double usage ; SOURCE. Destination identique ; KEEP-RUNTIME ; justification : consommateur test ne rend pas test-only, prepare-full n’est pas une validation ; owner I-runtime.

- scripts/generate-platform-mesh-certs.sh
- scripts/openbao-gold-seed.sh
- scripts/prepare-local-full-certs.sh

## R9 — Entrées/helpers et autorité de cible (9)

- Rôle préparation/rendu/deploy/inventaire/stockage/recovery et guide ; producteur sources ; consommateurs justfile:189–192, bootstrap/common, validation full et tests via imports dynamiques. Guide : opérateur.
- Dépendances R1/R2/R5/R7/R8/S1, Python/PyYAML/PowerShell ; effets render offline ; sous bail endpoint/UID : bootstrap, load images, apply, scale, backups/restores vers cible neuve, interruption/reprise nœud explicitement confirmées, fichiers privés/locks.
- Preuve A deploy_locked:136–250, B C1, C common.ps1:26 ; SOURCE. Destination identique ; KEEP-RUNTIME ; justification : retirer couplage au harness, pas l’autorité de sécurité. Documenter bail opérateur minimal, sans writer applicatif nouveau ; owner I-runtime.

- deploy/README.md
- deploy/bootstrap-local-full.ps1
- deploy/common.ps1
- deploy/deploy-local-full.py
- deploy/local-full-images.py
- deploy/local-full-node-recovery.py
- deploy/local-full-storage.py
- deploy/local_full_target.py
- deploy/render-local-full.py

## O1 — Entrées historiques mixtes (6)

- Rôle jalons/smoke + déploiements non nominaux ; producteur source ; consommateurs recettes just deploy-m1/m2/m3/j2/j3/mesh et documentation.
- Dépendances R3–R8/O2/O3/O6, certaines infra hôte ; effets build/start/apply/Jobs/lint selon entrypoint, DELETE Jobs transitifs et seeds possibles : ce ne sont pas des consommateurs read-only de l’environnement.
- Preuve A E2–E5, B smoke/couplages, C transitif ; SOURCE, pas succès live. Destination identique ; OTHER-OPS ; justification préserver capacités legacy avec cibles explicites sans les inclure au nominal ; owner I-runtime.

- deploy/deploy-j2.ps1
- deploy/deploy-j3.ps1
- deploy/deploy-mesh.ps1
- deploy/verify-m1.ps1
- deploy/verify-m2.ps1
- deploy/verify-m3.ps1

## O2 — Démo monolithe et schedule manuel (15)

- Rôle assets/dette J3/0604, non nominal ; producteur source ; consommateurs deploy-j3, prove-gold, debt-schedule-reference-kv et rendu CI (suivant ressources réellement référencées), README humain.
- Dépendances infra hôte PG/OpenBao/OpenFGA/hostAliases, R3/R6, images Dockerfiles curl/enroll ; effets build/Jobs/seed DiskStore/AdmissionRecord/enroll, réseau/scale lors d’exécution. schedule-reference-kv:67–106 crée aussi un projet temporaire cargo ; hors nominal, ne pas importer cette chaîne dans full.
- Preuve A E4/E6, B gold, C usages/dynamiques ; SOURCE. Destination identique ; OTHER-OPS ; justification aucune suppression de capacité approuvée, pas « tous les fichiers gold sont fixtures » ; owner I-runtime (y compris README et kustomization).

- deploy/apps/overlays/kind-demo-monolith/Dockerfile.curl
- deploy/apps/overlays/kind-demo-monolith/Dockerfile.enroll
- deploy/apps/overlays/kind-demo-monolith/README.md
- deploy/apps/overlays/kind-demo-monolith/allow-apparatus-plugins-np.yaml
- deploy/apps/overlays/kind-demo-monolith/allow-oodhive-compose-egress-np.yaml
- deploy/apps/overlays/kind-demo-monolith/allow-oodhive-plugin-hop-egress-np.yaml
- deploy/apps/overlays/kind-demo-monolith/apparatus-plugins-np.yaml
- deploy/apps/overlays/kind-demo-monolith/kustomization.yaml
- deploy/apps/overlays/kind-demo-monolith/oodhive-monolith.yaml
- deploy/apps/overlays/kind-demo-monolith/patch-gateway-np.yaml
- deploy/apps/overlays/kind-demo-monolith/patch-lazaret-scale.yaml
- deploy/apps/overlays/kind-demo-monolith/patch-lazaret-service.yaml
- deploy/apps/overlays/kind-demo-monolith/patch-plugins-egress-https.yaml
- deploy/apps/overlays/kind-demo-monolith/patch-plugins-np.yaml
- deploy/apps/overlays/kind-demo-monolith/schedule-reference-kv.ps1

## O3 — Autre mode Compose mesh (3)

- Rôle override/proxy/routes ; producteur source ; consommateurs docker-compose.yml:453–455 et mesh-authn-e2e.sh ; public-routes.md humain.
- Dépendances Compose/services/certs ; effets up/config réseau si invoqué, aucun Kind nominal ; preuve A/B/C ; SOURCE. Destination identique ; OTHER-OPS : mode légitime hors Kind ; owner I-runtime.

- deploy/mesh/compose.yaml
- deploy/mesh/envoy.yaml
- deploy/mesh/public-routes.md

## O4 — FGA hôte (3)

- Rôle guide/config/bootstrap hôte ; producteur source ; consommateurs justfile:95, runtime/monolith/run-host.ps1:12, deploy-j3:161–162 ; ensure-host-store:8,72 lit S1 puis POST/export model IDs.
- Dépendances S1, OpenFGA hôte/.env.openfga ; effets seed distant local/export d’état ; preuve C S6 ; SOURCE. Destination identique ; OTHER-OPS (contrat partagé pour S1 seulement) ; justification preserve host hors nominal ; owner I-runtime.

- openfga/README.md
- openfga/docker-compose.openfga.yml
- openfga/ensure-host-store.ps1

## O5 — Cloud/OpenTofu (5)

- Rôle squelette/contrat autre mode, non livraison cloud ; producteur sources ; consommateurs docs0600/0603 et platform-local-v1-implementation-contract:102 (tofu validate), pas runtime full.
- Dépendances OpenTofu/variables de module ; effets validate/config et état potentiel si opérateur l’étend, jamais implicitement ; preuve C ; SOURCE. Destination identique ; OTHER-OPS, pas suppression par absence de consumer full ; owner I-références/historique, aucun changement cloud autorisé.

- cloud/opentofu/.gitignore
- cloud/opentofu/README.md
- cloud/opentofu/modules/cluster/outputs.tf
- cloud/opentofu/modules/cluster/variables.tf
- cloud/opentofu/modules/cluster/versions.tf

## O6 — Shim Helm tiers (3)

- Rôle chart/values/notes pour M3 ; producteur source ; consommateur verify-m3 (lint), chart Helm ; dépendances Helm/O1.
- Effets lint local, pas installation Flux/Gateway Controller livrée ; preuve A E2, C helmrelease hors resources ; SOURCE. Destination identique ; OTHER-OPS : préserver jalon sans l’appeler runtime full ; owner I-runtime.

- deploy/third-party/envoy-gateway/Chart.yaml
- deploy/third-party/envoy-gateway/templates/NOTES.txt
- deploy/third-party/envoy-gateway/values.yaml

## V1 — Drivers/observateur/tests/checkers (10)

- Rôle validation explicite ou test d’outil ; producteur sources ; consommateurs opérateur, CI pour infra-static.sh (ci.yml:131–140), unittest/import source pour deux tests, wrappers mesh pour leurs cases ; mesh-public-route-cases.json consommé par validation route-matrix.
- Dépendances R9 (autorité/inventaire/stockage), configs/CA/cas explicites, Bash/Python/PS/Kustomize ; pas de fixture Rust déplacée dans ops.
- Effets actuels à séparer : kind-e2e seed FGA Pod:96–143 + set-env/rollout Hive:152–155, CTX legacy:76–82, pas cleanup ; Compose e2e build/up dans driver ; cases modifient state métier/SQL provider_tokens. Acceptance observe sans deploy/subprocess ; infra-static/checker/tests sont offline. Compatibilité de ces anciens modes préservée et déclarée, pas prétendue full.
- Preuve B C1–C6/R1–R5, A E7, C CI/common ; SOURCE. KEEP-VALIDATION ; destination identique SAUF deploy/test_local_full_target.py → ops/tests/test_local_full_target.py. Justification interface décrite au contrat, neutraliser startup/config cachés dans les nouveaux modes smoke/curl/E2E full ; owner I-validation, **aussi source+destination du test déplacé et validate-deploy-static.ps1**.

- deploy/test_local_full_target.py
- scripts/mesh-authn-e2e-cases.sh
- scripts/mesh-authn-e2e.sh
- scripts/mesh-authn-kind-e2e-cases.sh
- scripts/mesh-authn-kind-e2e.sh
- scripts/validate-deploy-static.ps1
- tests/infra-static.sh
- tests/local-full-acceptance.py
- tests/mesh-public-route-cases.json
- tests/test_local_full_acceptance_source.py

## V2 — Sonde Job extraite (1)

- Rôle payload de validation ; producteur source ; consommateur actuel kind/kustomization.yaml, verify-m2 ; futur validation explicite.
- Dépendances R3/R4, images et cible de probe ; effets création Job/réseau si lancé ; preuve B C4/R1 ; SOURCE.
- Destination ops/tests/manifests/invoke-probe-job.yaml ; KEEP-VALIDATION ; justification préserver sonde sans l’exécuter au lancement ; owner I-validation pour source+destination, **I-runtime seul pour le retrait du lien dans R4/kustomization.yaml**. Coupler les deux changements à l’intégration.

- deploy/apps/overlays/kind/invoke-probe-job.yaml

## V3 — Preuve gold historique (1)

- Rôle driver historique mutateur, pas curl read-only ni nominal full ; producteur source ; consommateur just prove-gold:202–203/documentation0605.
- Dépendances R8/O2/R6, hôte/registry/digest historique, .env.openfga ; effets reconfigure, Jobs/NP/env et fichiers TEMP, cleanup incomplet dans l’audit.
- Preuve B (881 lignes, digest pin historique) et C DELETE Jobs/seed/build ; SOURCE, résultat passé non transférable.
- Destination ops/tests/legacy/prove-gold-path.ps1 ; KEEP-VALIDATION ; justification identifier la validation sans retirer le scénario. Préserver contrat legacy/PSScriptRoot/sibling paths via résolution racine explicite ; owner I-validation source+destination. I-références change uniquement recette/docs externes ; I-runtime README O2.

- deploy/apps/overlays/kind-demo-monolith/prove-gold-path.ps1

## V4 — Payload de probe J3 (1)

- Rôle sonde legacy ; producteur source ; consommateurs overlay/procédure J3/prove-gold selon graphe A/B ; dépendances O2/R6, cible HTTP J3.
- Effets Job/traffic si appliqué ; SOURCE, pas succès métier full. Destination identique dans cette phase (couplage legacy clairement annoncé) ; KEEP-VALIDATION ; justification aucune migration J3 large nécessaire pour séparer le nominal full ; owner I-validation pour le payload, I-runtime pour son kustomization consommateur.

- deploy/apps/overlays/kind-demo-monolith/invoke-probe-j3-job.yaml

## T1 — Helper exclusivement test Rust (1)

- Rôle clé de test RSA8192 ; producteur script source, clé produite OpenSSL ; consommateur cfg(test) signing/probe.rs:256,583–588,657,716 et secret CI IAMRUSTY_TEST_RSA8192_PEM_B64.
- Dépendances OpenSSL/base64/Bash, injection env des tests ; effets actuels mktemp/clé privée puis append .env repo:22–25 ; ni runtime ni autorité issuer/JWKS.
- Preuve B C6 + spot-check D sur cfg(test)/script ; SOURCE. Destination services/IAMRusty/infra/tests/fixtures/generate-probe-keys.sh après accord humain ; MOVE-TEST-INTERNAL : asset test-only, sortie privée explicite, pas append global. Sinon rester en place étiqueté test-only, jamais prérequis runtime. Owner I-validation source+destination/commentaires test-only, pas code de production/framework.

- scripts/generate-test-keys.sh

## H1 — Checkpoint historique (1)

- Rôle photographie D ; producteur mission passée ; consommateurs README/ADR0606/validate doc references (C), opérateur historique.
- Dépendances anciennes preuves/source, aucune au runtime ; effets lecture seulement ; preuve SOURCE documentaire, 111→112 expliqué, pas preuve live actuelle.
- Destination docs/history/ops/PACKAGE-D-CHECKPOINT.md **si accord** ; ARCHIVE-HISTORY ; justification préserver trace et distinguer guide nominal ; owner I-références, liens README runtime transmis à I-runtime.

- deploy/PACKAGE-D-CHECKPOINT.md

## H2 — Rewriter de layout et fixtures (21)

- Rôle outil/harness de migration passé ; producteur source et auteurs de la mini-repo ; consommateurs Rewrite-Layout.ps1/self-check et son mapping/README ; les fichiers mini-repo sont des données du test du rewriter, **pas** des assets runtime ni fixtures IT Rust de plateforme.
- Dépendances chemins simulés/mapping, PowerShell ; effets transformation/écritures de l’arbre explicitement ciblé si exécuté ; self-check sur copie jetable, jamais vraie réorganisation implicite.
- Preuve C inventaire/dynamiques + B test de script ; SOURCE. Destination docs/history/ops/layout-rewrite/ avec même structure interne, **si accord** ; ARCHIVE-HISTORY ; justification reproduction historique sans encombrer l’interface nominale ; owner I-références. Revalider self-check/chemins après déplacement, ne pas supprimer assertions pour satisfaire CI.

- scripts/layout-rewrite/README.md
- scripts/layout-rewrite/Rewrite-Layout.ps1
- scripts/layout-rewrite/fixtures/mini-repo/Cargo.toml
- scripts/layout-rewrite/fixtures/mini-repo/Hive/domain/Cargo.toml
- scripts/layout-rewrite/fixtures/mini-repo/Hive/tests/common.rs
- scripts/layout-rewrite/fixtures/mini-repo/IAMRusty/Cargo.toml
- scripts/layout-rewrite/fixtures/mini-repo/IAMRusty/Dockerfile
- scripts/layout-rewrite/fixtures/mini-repo/IAMRusty/docker-compose.yml
- scripts/layout-rewrite/fixtures/mini-repo/IAMRusty/scripts/foo.sh
- scripts/layout-rewrite/fixtures/mini-repo/Manifesto/http/handler
- scripts/layout-rewrite/fixtures/mini-repo/deploy/mesh/.keep
- scripts/layout-rewrite/fixtures/mini-repo/docs/already-moved.md
- scripts/layout-rewrite/fixtures/mini-repo/docs/prose.md
- scripts/layout-rewrite/fixtures/mini-repo/justfile
- scripts/layout-rewrite/fixtures/mini-repo/monolith/Cargo.toml
- scripts/layout-rewrite/fixtures/mini-repo/obsidian/note.md
- scripts/layout-rewrite/fixtures/mini-repo/ops/deploy/mesh.yaml
- scripts/layout-rewrite/fixtures/mini-repo/prefix.md
- scripts/layout-rewrite/fixtures/mini-repo/services/IAMRusty/src/main.rs
- scripts/layout-rewrite/fixtures/mini-repo/services/Lazaret/README.md
- scripts/layout-rewrite/mapping.json

## H3 — Renamer historique (1)

- Rôle outil de transformation passé ; producteur source ; consommateurs rapport de réagencement, aucun autre consommateur trouvé par C (recherches multiples), usage humain non exclu.
- Dépendances layout crate source ; effets rename/écriture si invoqué ; statut SOURCE + hypothèse d’usage historique, pas inutilisé.
- Destination docs/history/ops/rename-crates.sh si accord ; ARCHIVE-HISTORY ; justification préserver outil sans le présenter comme commande nominale ; owner I-références.

- scripts/rename-crates.sh

## H4 — Reliquats 0604 candidats à arbitrage (3)

- Rôle assets de préparation/enroll/Service historique ; producteur source ; consommateurs historiques0604/usage humain possibles ; C n’a trouvé aucun consommateur script actuel après recherches multiples. **Pas preuve d’inutilité.**
- Dépendances O2/DB/Lazaret/identité ancienne ; effets SQL/grants, Job enroll, Service si opérateur les applique.
- Preuve SOURCE/hypothèse d’archivabilité ; destination docs/history/ops/kind-demo-monolith/<basename> uniquement après confirmation utilisateur de capacité/références.
- ARCHIVE-HISTORY proposée, garder chemin actuel jusque-là. Justification ni poubelle ni retrait silencieux de capacité ; owner I-références sur ces trois fichiers ; tout consommateur O2 reste écrit par I-runtime.

- deploy/apps/overlays/kind-demo-monolith/enroll-job.yaml
- deploy/apps/overlays/kind-demo-monolith/reference-kv-plugin-service.yaml
- deploy/apps/overlays/kind-demo-monolith/seed-reference-kv-grants.sql

## G1 — Bytecode Python suivi (7)

- Rôle cache interpréteur ; producteur CPython3.13 lors d’anciens imports ; consommateurs Python possibles mais appels actuels python -B et aucune référence texte selon A/C.
- Dépendances sources .py correspondantes, ABI interpréteur ; effets aucun état nécessaire au runtime, régénérable ; preuve Git commit080b5f4 + audits A/C et absence de règle ignore, pas seule recherche vide.
- Destination aucune (retrait du suivi), REMOVE-GENERATED de routine, ajouter __pycache__/ et *.py[cod] à .gitignore ; justification artefacts reproductibles non canoniques, non des capacités ; owner I-références. D n’a supprimé aucun fichier.

- deploy/__pycache__/deploy-local-full.cpython-313.pyc
- deploy/__pycache__/local-full-images.cpython-313.pyc
- deploy/__pycache__/local-full-node-recovery.cpython-313.pyc
- deploy/__pycache__/local-full-storage.cpython-313.pyc
- deploy/__pycache__/local_full_target.cpython-313.pyc
- deploy/__pycache__/render-local-full.cpython-313.pyc
- deploy/__pycache__/test_local_full_target.cpython-313.pyc

## U1 — Démarrage historique non clos (2)

- Rôle lanceurs humains présumés ; producteur source ; consommateurs aucun identifié après deux recherches C, usage humain non exclu ; dépendances programmes et chemins locaux à revalider avant changement.
- Effets démarrage de services/processus si invoqués, pas exécuté ; SOURCE et usage actuel UNKNOWN.
- Destination identique ; DEFER-UNKNOWN ; justification absence de référence automatisée insuffisante ; owner I-références pour investigation documentaire, aucun changement de comportement autorisé.

- scripts/start-all.bat
- scripts/start-all.sh

## Familles ignorées pertinentes et sorties privées (non-Git)

Les copies ignorées du dépôt canonique ne sont pas héritées par le worktree. Audit C a observé zéro untracked et trois entrées ignorées (certs + deux sous les fixtures layout). Le manifeste couvre **les familles**, pas une autorisation de nettoyer des fichiers privés non inventoriés.

**N1 — ops/certs/** : matériel TLS produit par generate-platform-mesh-certs/Compose/deploy-mesh ; consommateurs montages Compose ×9 et deploy-mesh:36,106. Dépendances OpenSSL/CA et permissions ; effets écritures privées/montages runtime ; preuve C et .gitignore:150–161. Destination privée/identique ; KEEP-RUNTIME **non suivi**, aucune clé à committer ou copier vers une preuve. Owner I-runtime pour références, opérateur pour ressources/cleanup attesté. Pas de suppression par nom ou régénération destructive du matériel existant.

**N2 — ops/scripts/layout-rewrite/fixtures/** ignorés : deux entrées privées observées par C, noms individuels non transmis au ledger ; producteur présumé self-check/copie de mini-repo, consommateur outil H2 ; dépendances règle ignore et rewriter ; effets données de test de transformation. Preuve présence familiale confirmée, contenu/propriété non prouvés. DEFER-UNKNOWN pour les copies privées : conserver en place, pas embarquer dans archive sans inspection/accord. Les 21 sources suivies H2 sont inventoriées séparément, elles ne deviennent pas des fixtures IT plateforme. Owner I-références ; gap N2 à rouvrir uniquement si déplacement affecte ces copies (aucune suppression prévue).

**N3 — caches Python futurs ops/** : producteur imports Python ; consommateurs interpréteur ; dépendances sources/ABI ; effets cache reproductible. REMOVE-GENERATED/non suivi, règle ignore à ajouter avec G1 ; pas de suppression des copies inconnues en dehors des sept suivies sans propriété vérifiée. Owner I-références.

**N4 — sorties de session opérateur** (répertoire privé, bail/images/render/certs/probes, y compris approved_root/cluster-anchor-<task_id>) : producteur opérateur/bootstrap/helpers ; consommateurs runtime/validation via chemins explicites ; dépendances clés publiques/ACL/protocole bail. Effets fichiers privés/locks/ancrage ; SOURCE contractuel, pas contenu préexistant prouvé. KEEP-RUNTIME non suivi pour bail/certs/images ; KEEP-VALIDATION non suivi pour captures/cas privés. Destination neuve task-owned hors Git ; aucun import d’une sortie passée. Runtime possède son protocole, validation ses rapports ; opérateur seul gère la session/cleanup attesté.

**N5 — .env/.env.openfga hôte et ancien TEMP/.tmp/staging** : produits par outils historiques/hôte, consommés O1/O2/O4/V3 ou ancien helper T1 ; dépendances credentials/DB/modèles ; effets exports/seed/build/source staging potentiels. OTHER-OPS non suivi pour état hôte, DEFER-UNKNOWN pour résidus ; garder hors nominal, ne pas migrer/effacer implicitement. Aucune dépendance runtime ops à .tmp n’est prouvée (C), ni autorisation de recycler ces sorties. Owner I-références pour documentation, opérateur pour arbitrage ressources. T1 doit cesser l’append implicite .env après accord de sa migration.

## Contrats externes de migration (hors 150)

**X1 justfile** : producteur source, consommateurs humains/automates ; dépendances scripts ci-dessus et CLI FGA/Docker ; effets selon recette. KEEP-VALIDATION pour recettes prove/smoke/curl/e2e/it-native ; KEEP-RUNTIME pour artefacts/deploy ; SHARED-CONTRACT pour génération FGA. Destination identique, owner exclusif I-références. **Bug avéré justfile:156** : fallback monte actuellement PWD/openfga alors que DSL est ops/openfga/model.fga ; remplacer par montage PWD/ops/openfga, contrôler code de sortie/transformation et ne pas écrire un JSON partiel. Garder nom de recette et deux branches CLI/fallback.

**X2 .github/workflows/ci.yml:131–140** : consommateur infra-static, producteur CI ; dépendances Python/PyYAML/kubectl/Bash ; effets parsing/rendus offline, pas runtime ni Cargo dans ce job. KEEP-VALIDATION, destination identique, owner I-références. Ne pas retirer un overlay de la fermeture de rendu pour rendre CI verte ; ajouter un inventaire attendu.

**X3 consommateurs OpenFGA services/tests + framework** : cinq include_str et six lectures DSL listés S1, TestOpenFga au pin41a86dba. SHARED-CONTRACT, chemins inchangés, **zéro écriture prévue** dans ces consommateurs ou le framework, aucun déplacement S1. Toute demande contraire redevient migration structurelle séparée, pas correction implicite gitlink.

**X4 services/IAMRusty/infra/src/signing/probe.rs** : seuls commentaires/messages cfg(test) pointant T1 peuvent être modifiés par I-validation après accord ; variable d’injection/secret CI et comportement crypto inchangés. MOVE-TEST-INTERNAL ne signifie pas déplacement du code de production probe.rs.

**X5 docs/skills/références historiques** : consommateurs humains, aucune entrée d’exécution nominale ; dépendances chemins migrés ; effets documentation. ARCHIVE-HISTORY pour liens historiques, KEEP-RUNTIME/VALIDATION pour guides actifs selon rôle. I-références possède docs/skills externes ; les README de profils R1/R2/R9/O2/O3/O4 restent I-runtime, sans second écrivain. ADR historiques non réécrites, liens pointant archives explicitement étiquetés historique.

## Bilan et gates du manifeste

Comptage normalisé : R1–R9=64 ; S1=2 ; O1–O6=35 ; V1–V4=13 ; T1=1 ; H1–H4=26 ; G1=7 ; U1=2 ; total **150**, tous explicitement nommés. Vérification D : git ls-files ops et partition en mémoire via Context Mode, zéro MISSING et zéro recouvrement ; aucun script exécuté, aucun runtime/Cargo, aucun fichier modifié.

Chaque owner possède contenu ET chemin de ses fichiers ; déplacement d’un payload ne transfère pas la propriété de son kustomization/recipe. Intégration atomique coordonnée par lead : R4↔V2, R9↔test déplacé, O2↔V3/H4, T1↔commentaires cfg(test)/docs, checkers V1↔références X1/X2. Les fichiers d’archives H* appartiennent exclusivement à I-références ; accord avant migration, pas d’archive appliquée par D.

Gaps nommés : N2 copies ignorées non inspectées, U1 usage humain, H3/H4 archivabilité à ratifier, validation full nouvelle interface non implémentée, live full non prouvé, Bash Windows non prouvé, dépendances applicatives A1–A6 hors lot. Réouverture : chemin/consommateur affecté ou décision humaine explicite. Aucun de ces gaps n’est maquillé en succès, inutilité ou permission de supprimer.

--- Fin manifeste ---
