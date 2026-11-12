---
title: >-
  Séparation runtime/validation — le contrat ops Kind
category: concepts
tags: [architecture, ops, kubernetes, contracts, visibility/internal]
sources:
  - docs/guides/ops-declarative-boundaries.md
  - docs/missions/20261110-kind-local-operator-cycle.md
  - docs/history/iac-k6-qa-20261112/README.md
  - docs/contracts/ops-kind-runtime-validation-separation.md
  - docs/contracts/ops-kind-runtime-validation-manifest.md
  - .cursor/handoffs/20261018-ops-kind-runtime-validation-preparation.md
  - docs/missions/20261108-ops-oauthconnect.md
  - ops/deploy/README.md
summary: >-
  Déclarations Ops, cycle opérateur et preuves restent séparés. Kind natif disponible et slice k6 qualifiée ne valent ni déploiement sans intervention ni qualification globale.
provenance:
  extracted: 0.95
  inferred: 0.05
  ambiguous: 0.0
created: 2026-10-19
updated: 2026-11-12
---

# Séparation runtime/validation — contrat ops Kind

## Actualisation — 9 au 12 novembre

L'utilisateur demande que les définitions soient dans les formats standards,
pas dans un second déployeur : Helm pour les workloads, Kustomize pour Calico,
Bake pour les images, Compose pour le registre, OpenSSL pour la PKI. Wrappers
bornés aux entrées privées/protocoles/contrôles indispensables ; déplacer le même
générateur de Python vers PowerShell ne répond pas à ce but. Aucun provider cloud
nouveau n'est livré. Le guide `docs/guides/ops-declarative-boundaries.md` porte
les frontières, les rapports intermédiaires restent datés.

Le lanceur Kind natif du10novembre a un cycle réel avec corrections et interventions :
13/13 actifs disponibles et healthHTTPS200 après reprise des mêmes IDs. Bao doit
être unsealed et les queues LocalStack manquantes recréées ; cela ne restaure pas
les messages volatils. Exited137 observés au premier stop, pas d'arrêt propre global
revendiqué. Le helper de reprise n'est pas une nouvelle installation/initialisation.

L'exception Agent OpenBao2.6.2/redirections n'est pas résolue par HTTPS : pas de
clone auto-auth implicitement sûr, de bump ou contournement. Les inputs opérateur
restent complets et privés ; la capacité produit consommatrice n'est pas effacée
par l'amincissement des ops.

Le PASS k6 du12novembre concerne une **slice fixture**, pas l'edge nominal ni tous
les parcours du lab. Voir [[projects/aiforall/skills/testing-k6-fixture-first]] et
[[projects/aiforall/references/session-iac-k6-qa-20261112]]. Les sections D1–D6
ci-dessous conservent le contrat historique d'octobre, sans les promouvoir en ADR
Accepted ni prétendre que ses lanceurs Python retirés sont les commandes actuelles.

Décisions D1–D6 du contrat `docs/contracts/ops-kind-runtime-validation-separation.md`
(Proposed, lien secondaire vers l'ADR 0606 sans la consommer). Mission d'origine :
[[projects/aiforall/references/mission-ops-kind-separation-20261019]].

## D1 — Un nominal : local-full

Cluster `aiforall-local-full` (1 control-plane + 2 workers), namespaces dédiés.
Séquence : artefacts Linux (compose build **sans** up) → certificats → bail
opérateur neuf → bootstrap → images épinglées → déploiement → validation
séparée. Les 5 autres chemins (M1/M2/M3, J2 hybride, J3/gold, mesh, dette
schedule-reference-kv) restent **non nominaux avec cibles explicites** —
conservés, jamais rebaptisés full, jamais supprimés sans arbitrage.

Cette séquence décrit le contrat d'octobre. Le profil OAuthConnect ajouté
le8novembre utilise aussi les outils explicites `build/images.py` puis
`local-full-images.py` : plan/build, provenance/reçu immuable et déploiement
restent des actions séparées, sans build implicite lors d'un render/up.
Voir [[projects/aiforall/skills/deploying-oauthconnect-local]].

## D6 — Le bail est produit par l'opérateur (premier maillon)

JSON v2 minimal : `version=2`, paire cluster/context **fermée**, `task_id`
(32-hex minuscule), `anchor_state=pending_creation`, `nodeContainerIds=[]`,
`kubeconfig_path` sous `approved_root/cluster-anchor-<task_id>/` — sur Windows
`Path.home()/AppData/Local/Temp/opencode` (`local_full_target.py:128–129`).
Aucun champ préfabriqué (UID, ID, mapping, hash, endpoint) : le bootstrap
produit les attestations, jamais l'inverse. Le writer leasev2 parent n'existe
PAS sous ops — la faille du premier maillon de l'audit A est traitée par la
procédure opérateur documentée au README runtime.

## Interface validation — quatre canaux séparés

- **Smoke** : readiness/get/wait uniquement, zéro mutation (pas de Pod, seed,
  set-env, rollout, NP).
- **Curl métier (route-matrix)** : accès contractualisé `127.0.0.1:18080`,
  positifs/négatifs discriminants, **matrice cohérente avec le profil déployé**
  (full : `connectors=[]` → login OAuth attendu **422** `connector_not_configured`,
  pas 303 ; voir `mesh-public-route-cases-local-full.json`) ; la couverture
  authentifiée est rapportée **NOT RUN**, jamais PASS implicite.
- **E2E** : état métier **déclaré** par `--inputs` privé, prepare-probes/
  cleanup-probes séparés (cleanup = seules ressources enregistrées, jamais par
  nom), BLOCKED explicite tant que le setup full approuvé est absent.
- **IT natives Windows** : `it-native.ps1 -Suite <svc>` → `cargo test -p <pkg>
  --tests`, un package à la fois, testcontainers, jamais Kind.

Code0 seulement si toutes les obligations sont exécutées ; skip ⇒ non-zéro
avec raison non secrète ; rapport LIVE/SIMULÉ/NOT RUN.

## Extension livrée, validation live non acquise

Le profil **`--oauth-connect`** prépare GitHub+simulated, chaîne OpenBao et
entrée **HTTPS** `127.0.0.1:18080`. Le profil de base et les deux flags legacy
restent distincts ; les attentes HTTP/`connectors=[]` ci-dessus ne sont pas les
oracles du nouveau profil. Ni un ancien script HTTP/GitLab ni un nom de port ne
prouvent leur compatibilité avec celui-ci.

La session ops qualifie des tests de configuration/rendu/PKI synthétique et une
revue statique, pas un build Linux, apply Kind ou parcours navigateur. Une source
privée changée ou un UID inconnu n'est pas une invitation à relancer un seed ou
adopter l'objet. Voir
[[projects/oauthconnect/concepts/local-ops-and-secret-distribution]] et
[[projects/aiforall/references/session-ops-oauthconnect-20261108]].

## Manifeste de tri — ownership unique

150 fichiers suivis, chaque chemin = enregistrement individuel (rôle,
producteur/consommateurs, dépendances, effets, preuve SOURCE jamais LIVE,
destination, décision, justification, owner). Catégories : KEEP-RUNTIME,
KEEP-VALIDATION, SHARED-CONTRACT (OpenFGA model.fga/json, copie unique dans
ops/), MOVE-TEST-INTERNAL, ARCHIVE-HISTORY (H1–H4 → `docs/history/ops/`, H4
archivé 2/3 : `enroll-job.yaml` conservé, consommateur vivant), REMOVE-GENERATED
(.pyc), OTHER-OPS/DEFER-UNKNOWN (« pas de consommateur » = hypothèse documentée,
jamais inutilisé ; ≥2 recherches indépendantes avant toute conclusion).
« Moins de fichiers » n'est pas un oracle ; **moins de couplages** en est un.

## Pièges persistants

- `offline-no-kubeconfig` inexistant **par design** — ne pas le créer.
- Marqueur racine `Cargo.toml` + `ops/deploy/kind/cluster.yaml` (le déplacer
  casse toutes les entrées).
- `infra-static.sh` exige `PYTHON` (Windows : shim WSL sur `bash` du PATH —
  Git Bash explicite) et `PYTHONUTF8=1` (BOM).
- Cycle de vie runtime : [[projects/aiforall/concepts/local-runtime-lifecycle]].
