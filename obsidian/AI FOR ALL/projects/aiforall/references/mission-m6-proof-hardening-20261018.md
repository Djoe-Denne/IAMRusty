---
title: >-
  Apparatus m6 — preuves fiabilisées et enseignements de la campagne du 18 octobre 2026
category: references
tags: [testing, rust, kubernetes, visibility/internal]
sources:
  - _raw/opencode-m6-proof-hardening-20261018/inventory.json
  - docs/missions/20261018-apparatus-m6-proof-hardening-results.md
  - .cursor/handoffs/20261018-m6-proof-hardening-preparation.md
  - .agents/skills/aiforall-test-evidence/SKILL.md
summary: >-
  Oracles21, IT9+1 et m6 réel1 verts sur E:, H1–H4 archivés ; remplacements K8s locaux acceptés et pièges évitables transformés en skill/règles.
provenance:
  extracted: 0.96
  inferred: 0.04
  ambiguous: 0.0
created: 2026-10-18
updated: 2026-10-18
---

# Apparatus m6 — campagne de fiabilisation du 18 octobre 2026

Suite de [[projects/aiforall/references/mission-g17-m6-20261018]], pas un nouveau
résultat attribué rétroactivement à G-17. Code exécuté `fd9c412`, bilan commité
`46d5232`, pin rustycog `41a86dba…` inchangé. Aucun changement de production,
configuration Cargo/lockfile ou Réalité ADR.

## Archive de session et sous-agents

**10 sessions,1 006 messages visibles exportés,73 pièces de preuve** ; inventaire
SHA256 `4b224f7cf12142ed3921111bcbf7976d33d21663c992d0dfda16fa68646ecefb`.
Les73pièces sont des copies identiques en octets, toutes avec empreinte source et
destination ; aucune transformation additionnelle nécessaire après la sélection.

Racine native `ses_eb1108de1ffeGnYdY0GIcFGTw8`, équipe
`mission-ce5af9225f72e1c6`. Sources sous `_raw/opencode-m6-proof-hardening-20261018/` :
textes visibles utilisateur/assistant projetés par export natif V2 authentifié,
métadonnées d'outils, inventaire des omissions/empreintes ; preuves sélectionnées
archivées séparément. Tous les membres, y compris sessions arrêtées/dupliquées et
fork pool9, sont inclus. Les comptes exacts et dates de capture sont dans l'inventaire.

> [!warning] Portée de l'archive
> Snapshot à un instant par session, pas sauvegarde continue des messages futurs.
> Raisonnements internes, instructions système, états fournisseur, compactions et
> payloads d'outils exclus ; données sensibles filtrées. L'export conserve la
> projection native disponible, pas une promesse de journal d'événements intégral.
> Un fork peut répéter des messages hérités ; le total n'est pas un compte de messages uniques.

Le vault projet est déterminé par `.env`, pas par le config global pointant vers
texturetonevariations. Taxonomie `_meta/taxonomy.md` absente : tags existants
réutilisés, aucune nouvelle taxonomie inventée. Obsidian/CLI non disponible dans
ce contexte : fichiers du vault modifiés directement, sans ouvrir l'application.

## Contributions et tentatives conservées

- pool1 : QA initial avant writers ; pool2 : contrat ciblé, sans code de production.
- pool3 : N2 staging/docs, test des trois producteurs et union des features.
- pool4 : N3 fixture/oracles, preuve rouge conservée puis livraison bornée.
- pool5 : doublon arrêté, pas de fusion aveugle ni changement réputé intégré.
- pool6 : correctness indépendante, puis handoff natif infra-verifier confirmé,
  modèle/session conservés ; pas promotion en lead.
- pool7 : revue tests, relecture corrective puis audit des preuves et14empreintes.
- pool8 : sécurité avant captures puis audit confidentialité/portée, scénario
  Endpoints falsifié corrigé et O-LIVE-1 distingué du vert fonctionnel.
- pool9 : fork natif de pool4, S1/S2/T1/T3 test-side,21tests verts ; rougeClippy
  point-virgule avant correctif conservé, gates finales vertes ; patch intégré et
  six hashes identiques, pas réutilisation d'une exception de hook.

Les verdicts initiaux erronés ou incomplets restent des sources, pas des faits
promus : «owned par nom/digest» a été explicitement corrigé par le reviewer.

## Résultats et limites attestés

- Staging :6tests,108mutations internes ; reqwest/rcgen racine et features effectives
  cohérents chez trois producteurs, sans prétendre un run m5/gold path.
- Oracles fixture/preuve :10+11verts, strictClippy des deux cibles vert.
- Windows natif E: :t6 9/9, m1 1/1 ; E2E m6 sur `kind-aiforall-local` :1/1,51.07s,
  zéro failed/ignored/filtered, un seul Cargo, pas rerun redondant.
- H1 catalogue/enveloppe/CRI distincts ; H2 même catalogue VALID/Scheduled ;
  H3 403/200,hops0/1,hostname lié au catalogue ; H4 positif404 puis timeout GNU borné.
- Trois lectures PodAPI indépendantes d'Endpoints : UID/IP/Ready/nonDeletion
  avant positif et avant/après wget. Un EndpointB pointant nominalement PodA est
  rejeté ; un timeout quelconque n'est pas une preuve réseau suffisante.
- Correctness/tests PASS observable, sécurité PASS corpus désigné, infra PASS
  technique. Le pass prouve les assertions runtime, pas un pcap/body/stderr brut,
  DNS/Service dataplane, causalité exclusive Netpol ou ensemble du mesh.
- **StrictClippy m6 production reste ROUGE différé** :27diagnostics historiques,
  plus avertissement rustycog signalé ; pas corrigé par le vert des oracles.

## O-LIVE-1 — comportement local accepté, pas permission de production

Snapshots avant/après : le harness a remplacé le Pod plugin et l'AdmissionRecord
préexistants par nom déterministe. Deux UID changés ; noms/digests ≠ propriété.
Les autres UID inventoriés sont stables, ce qui ne certifie pas specs/labels intacts.

Après explication des interruptions/pertes possibles d'état en mémoire, des
risques aussi en local partagé et de la distinction avec production, l'utilisateur
a décidé **« Alors on accepte ce comportement. »** Réserve close par arbitrage
humain daté, pas par le libtest vert ; pas d'autorisation initiale rétroactivement
inventée. L'accord n'autorise ni nouveau run, restauration ou suppression globale,
ni production/contexte protégé. Réouverture si périmètre/données/concurrence changent.

## Pièges évitables et contre-mesures durables

1. **Préflight incomplet** : lire toute la chaîne de suppressions/reconcile, pas
   seulement `ensure` ; annoncer remplacements, UID et effets avant le run.
2. **Kubeconfig partagé périmé** : port60787 refusé alors que node expose60508 ;
   export privé dédié a prouvé NodeReady, sans réécrire la config partagée.
3. **Registres mixtes** : préflight arrêté avant Cargo sur CP1252 ; lecture stricte
   par ligne UTF8/CP1252 et BOMUTF16,6selftests avant relance, historique inchangé.
4. **Faux compteur H1** : libtest joint nom du test et H1 sur même ligne ;
   extraction au marqueur, unicité puis fullmatch fermé, jamais réécrire le log.
5. **Sécurité d'archive** : allow-list14fichiers, hashes vérifiés ; kubeconfig privé
   exclu puis supprimé après fin du bail. Jamais zip récursif runtime/.tmp.
6. **Worker faussement repris** : message stocké ou candidat retardé ≠ nouveau
   travail. Réveil explicite, vérification ciblée d'admission/activité, pas polling.
7. **Long chemin Windows** : hook `os error206` dans worktree ; exception unique
   user consommée pour N3, pas droit permanent à `--no-verify`. Patch/hash puis
   commit canonique au hook normal pour le correctif.
8. **Scripts Context Mode** : bash et PATH propres au sandbox, PowerShell `$`/JS
   newline à échapper ; préférer exécutables explicites et scripts courts. Une
   erreur d'arguments/échappement n'est pas une panne du MCP.

Ces observations sont promues dans `.agents/skills/aiforall-test-evidence/SKILL.md`,
pointé depuis `AGENTS.md`, et règles `agent-handoff.mdc`/`infra-safety.mdc`.
Le skill est procédural : aucun runner temporaire promu en harness officiel. ^[inferred]
Instructions durables commitées séparément dans **`80918ca`** (hook normal).
L'archive wiki est sauvegardée localement ; aucune publication/push ni commit
global des modifications wiki préexistantes de l'utilisateur n'a été effectué.

### Recette d'export vérifiée pour cette session
- CLI installé : `resources/opencode-cli.exe` du client Desktop, même s'il n'est
  pas sur le PATH du sandbox. Export natif `session export --sanitize <sessionID>`,
  via service authentifié existant ; pas copie de toute la base utilisateur.
- Lister les descendants natifs par API `session.list` filtrée parentID avec
  pagination, **puis faire l'union avec les membres de l'équipe** : pool9 est un
  fork dont parentID est null, donc absent d'un simple parcours des enfants du lead.
- Projeter seulement textes user/assistant et métadonnées d'outils ; exclure
  reasoning/system/compaction/providerstate/payloads, compter les omissions.
- Un seul inventaire du bundle assure le delta/tailles/hashes de chaque session
  et pièce ; `.manifest.json` suit ce bundle, conserve la synchronisation globale
  précédente et date seulement une synchronisation bornée à cette mission.
- Validation :83fichiers d'archive vérifiés,1006records,10sessions,73pièces ; YAML
  de5pages et liens de la nouvelle page valides. Pas de runtime de tests démarré
  pour cette sauvegarde, ni de réindexation globale des autres projets.

## Restitution et ressources

Fixtures Docker finales exited0 par ID, helpers terminés ;119événements mission
dans registre18 avec filiation des remplacements. Six volumes et deux images
nouveaux conservés `ask`, pas adoption/prune. Node/Desktop/WSL préexistants
conservés ; mémoire résiduelle mesurée, pas RAMzéro revendiquée. Pression
control-plane transitoire pendant m6, résolue à l'audit.

Captures et preuves dans l'inventaire : aucune archive de credentials privés.
T2/T4/G-A/G-B et résidus credentials antérieurs gardent leurs accept/defer nommés.
Voir [[projects/aiforall/skills/running-apparatus-p4-tests]] et
[[projects/aiforall/concepts/orchestrator-agent-harness]].
