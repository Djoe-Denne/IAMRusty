# Revue indépendante — conducteur privé Windows/Linux

```yaml
reviewer: security
written_at: 2026-10-30T14:28Z
branch: master
head_sha: 4560b18b1c36702a16792aa36200751237c63251
dirty: true
scope_slug: k6-private-parent
verdict: GO_WITH_COMMENTS
security_decision: REVIEW_REQUIRED
related_paths:
  - ops/tests/k6/local-private-parent.ps1
  - ops/tests/k6/local-private-parent.py
  - ops/tests/k6/tests/test_local_private_parent.py
  - docs/contracts/k6-chromium-causal-observation.md
```

Rapport READONLY reçu du reviewer, persisté par le lead avant correction.
Candidat `result_mvv2kfa4_0035_7zbzwj`, session `ses_e72d3280fffelBiYRwaxmXJwey`,
tâche `task_mvv24mb0_0033_i8mynp`. Aucun site réel fourni ou collecté.

## Verdict

**Sécurité : REVIEW REQUIRED — deux MEDIUM, aucun CRITICAL/HIGH démontré.**
Ne pas lancer le conducteur sur une chaîne de chemins Windows non attestée sûre ;
avant collecte, rendre également explicite l'échec de rétention. Aucun override
demandé, aucune admission de collecte ou de préflight réel par cette revue.

Source examinée : `195059a`, base `cbab06e`. HEAD a avancé vers `4560b18` pour des
documents et preuves de fixtures ; sources conducteur/consommateur/workflow inchangées.

## S-1 — Les feuilles ne prouvent pas la sûreté des ancêtres Windows avant lancement

- Severity : MEDIUM ; Confidence : MEDIUM ; Category : race ; Scope : local.
- Localisation : `local-private-parent.ps1:75–80,193–213,367–400`.
- Propriété : intégrité de l'instance effectivement démarrée et de son stockage privé.
- Surface : résolution de BasePath et ouverture du VHDX au lancement WSL.
- Frontière : stockage Windows → invocation WSL sous l'identité Windows attendue.
- Capacité adverse nécessaire : autre principal Windows ayant droit de renommage/
  substitution sur un ancêtre, notamment DELETE_CHILD. **Précondition non observée
  sur l'hôte réel pendant cette revue.**

### Scénario et preuve

Les feuilles attendues ont leurs bonnes identités et ACL, mais un ancêtre reste
substituable. Le snapshot initial réussit. Instance arrêtée/VHDX non encore ouvert,
l'attaquant substitue le chemin avant Process.Start ; WSL peut commencer à ouvrir
une autre image. Le snapshot postérieur pourra refuser, mais après activation de
cette capacité. SID et handle de wsl.exe restent ceux du bon processus Windows,
sans prouver le stockage qu'il a commencé à ouvrir.

FileIdentity ouvre uniquement le chemin fourni et contrôle le reparse point final.
Get-WindowsSnapshot examine dossier final et ext4.vhdx, pas les identités, reparse
points ou permissions des ancêtres. DriveType.Fixed ne remplace pas ces contrôles.
Le prochain snapshot suit Process.Start.

Impact : une chaîne substituable peut être admise comme privée jusqu'au lancement.
L'exploitabilité de l'installation actuelle n'est pas établie ; **aucun constat de
compromission de l'hôte**. Composants : conducteur Windows/sélection de l'instance.

### Correction et test de régression T-1

Vérifier avant tout lancement la chaîne d'ancêtres des chemins sensibles : absence
de redirection non admise, identités stables, absence de droits de substitution
pour les principaux non fiables. Réutiliser cette observation dans les contrôles
renouvelés. Refuser l'indétermination ; ne pas modifier automatiquement les ACL des
parents existants.

Fixtures : feuilles identiques, mais ancêtre modifiable, reparse intermédiaire ou
identité d'ancêtre changée ; refus **avant Process.Start**, positif d'une chaîne
sûre. Aucun WSL requis. CWE-367.

## S-2 — Un retrait impossible devient une simple entrée invalide

- Severity : MEDIUM ; Confidence : HIGH ; Category : other ; Scope : cross-file.
- Localisation : `local-private-parent.py:360–375,401–406` et
  `local-private-parent.ps1:443–448,470–472`.
- Propriété : rétention confidentielle et signal exploitable d'un relevé non retiré.
- Surface : erreurs de retrait après création d'un relevé valide.
- Frontière : état de rétention Linux → protocole fermé → opérateur Windows.
- Aucun attaquant nécessaire ; erreur de filesystem empêchant unlink suffisante,
  sans supposer de processus hostile du même UID.

### Scénario et preuve exécutée

Une collecte admissible crée un relevé et atteint READY. Le retrait échoue, y
compris au cleanup du contexte. Le module conserve correctement RETENTION_UNPROVEN
et lève une erreur. Le compagnon transforme toute erreur en ABORT/FAIL/INPUT_INVALID.
Windows affiche PARENT_REFUSED sans avertissement spécifique de rétention.

Contre-exemple exécuté avec le filesystem entièrement synthétique existant, sans
site réel ni processus natif : exit compagnon2, frame finale ABORT/INPUT_INVALID,
relevé synthétique encore présent, descripteurs de fixture fermés. La constante
RETENTION_UNPROVEN existe au protocole mais n'est pas utilisée par ce parcours.

Le non-zéro est conservé : **aucun faux PASS démontré**. L'opérateur perd cependant
l'information nécessaire pour traiter une donnée privée potentiellement conservée
au-delà de sa rétention. Composants : compagnon et traitement des abandons Windows.

### Correction et test de régression T-2

Préserver un diagnostic fermé de rétention à travers l'échec du contexte et main,
puis l'avertissement explicite côté Windows. Garder le non-zéro et zéro publication,
sans exception libre, chemin ou contenu.

Parcours synthétique complet avec retrait impossible après ACK et à échéance :
RETENTION_UNPROVEN final, exit non nul, absence de clôture normale et de données
natives dans toutes les sorties. Vérifier l'avertissement Windows réellement câblé.

## Passes et modèle de menace

### Défensive

Contrôles observés cohérents : CI avant création de processus ; préflight sans
session/main ; GUID/compte fixes et environnement réduit ; handle retenu et
PID/création/image/SID ; HOME ouvert/ext4/UID/chaînes/intégrité renouvelés ; frames
fermées/bornées/corrélées, budget monotone de vérifications et aller-retour ;
contexte vivant pendant lecture, sorties natives non relayées et non-zéro conservé.

### Adversariale

Deux chemins distincts : substitution avant vérification post-lancement, puis
perte du diagnostic spécifique après échec réel de retrait. Hypothèse de DACL
nulle investiguée puis **réfutée** : la représentation .NET testée expose une ACE
Everyone, rejetée par le contrôle existant ; aucun finding sur cette hypothèse.

Actifs : instance/autorité du lancement et relevé à rétention limitée. Entrées :
chemins Windows sous principal distinct substituant un ancêtre ; état inattendu
du filesystem empêchant le retrait. Mesures existantes : feuilles et handle ;
cleanup tenté/non-zéro. Mesures manquantes : chaîne d'ancêtres attestée sûre avant
lancement et propagation fermée du diagnostic de rétention.

## Vérifications réellement exécutées par le reviewer

- git diff --check normal et delta `cbab06e..195059a` : exit0.
- Cinq tests Python purs ciblés : 5/5, exit0 ; frames, corrélation, fraîcheur,
  CI et renouvellement après réponse Windows.
- Contre-exemple synthétique de retrait impossible : confirmé, sans site réel.
- Fixture PowerShell de DACL nulle : refus confirmé, hypothèse réfutée.

Preuves du lead réutilisées, non réexécutées : Windows189/exit0/skips8 ;
Linux189/exit0/skips11/2.013s au rapport60. Ce sont preuves de fixtures, pas d'une
invocation réelle. Non exécutés : versions k6/Chromium, inspections, validate/run/
browser/selftest, préflight réel ou collecte. Aucun HTTP/Chromium/cleanup parent
réel qualifié. Aucun fichier, branche, worktree ou ressource runtime créé/modifié.

## À ne pas refaire et limites

Ne pas rouvrir le consommateur gelé ou son ancien S-1. Absence de CI n'est pas
permission humaine ; fin WSL n'est pas cleanup Linux ; DACL nulle réfutée ne doit
pas revenir comme finding. Pas de modification de Child/pins/scénarios/CI/services,
nouveau harness/watchdog/collecteur ou pipeline obligatoire de reviewers.

La sûreté effective des ancêtres Windows reste à établir sous bail/permissions
applicables. Sources inchangées de `195059a` à `4560b18`, arbre sale pour WIP
étrangers non examinés ; actualiser les pointeurs après correction. Le lead
persiste puis libère les seules corrections nécessaires avant admission future.
