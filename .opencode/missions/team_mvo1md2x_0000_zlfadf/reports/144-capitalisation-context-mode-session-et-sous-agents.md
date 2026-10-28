# 144. Deux fonctions réutilisables préparées depuis la session et les sous-agents

## Résultat

Objectif : enrichir la bibliothèque locale à partir des analyses réellement
exécutées, sans recopier chaque recherche ou réexécuter les scripts historiques.
Préparation terminée en travail direct avec les skills locaux d'ingestion et de
fonctions Context Mode. Deux candidates documentées, testées et isolées du
catalogue validé :

- **Comparaison d'inventaires de conteneurs déjà capturés** : identités complètes,
  complétude explicite, préexistants préservés ou modifiés, ressources actives,
  absentes, inconnues et conflits de propriété ; comptes complets malgré une
  restitution bornée. Aucune commande Docker ni autorisation de suppression.
- **Décodage strict de preuves textuelles Windows** : UTF-8, UTF-16 LE/BE,
  marqueurs d'encodage, limites et erreurs explicites ; pas de remplacement
  silencieux, fallback Latin1 ou suppression implicite de NUL.

Chaque dossier contient code, tests, candidate.json et README. Les candidates
restent **non promues**, sans chargement par le runner ou la CLI existante.

## Couverture et sélection

Collecte officielle OpenCode V2.1.4 de la racine et de 35 sessions filles directes,
avant le 28 octobre 2026 à 00:00 UTC. Trente-six projections, 4 173 151 octets,
594 appels sélectionnés, aucune erreur de collecte ni identité d'appel dupliquée.
Pagination de l'inspecteur complète pour ces captures. Prompts, raisonnement
interne et texte conversationnel exclus. La projection/compaction ne garantit
pas un historique brut exhaustif ni des descendants indirects non collectés.

Cinq exécutions sélectionnées ont été reliées manuellement aux arguments
littéraux et au code complet retourné par Context Mode, avec empreintes distinctes
du wrapper, du script et du résultat. La logique d'inventaire vient du lead ; les
cas d'encodage viennent de deux sous-agents. Une erreur de syntaxe newline puis sa
correction sont conservées comme provenance, sans les rejouer. Un résumé montrant
un test FAILED ne constitue pas un échec du script de décodage.

Les pistes se recouvrent : 26 inventaires, 58 résumés de tests, 119 empreintes,
396 lectures, 160 observations Git, 10 helpers connus. Ces chiffres sont des
indices de tri, pas des verdicts d'exécution ou un inventaire de nouvelles fonctions.

Rejets : lectures/extraits et collecte indépendante déjà couverts ; wrappers Git
et commandes ponctuelles sans valeur ajoutée ; duplication des recherches MCP ;
mutations registry, rollups et cleanup exclues des fonctions pures. Aucun code,
log brut, donnée privée ou chemin personnel de mission intégré aux candidates.

## Validation réellement exécutée

Après relecture des quatre fichiers code/tests :

- `node --check` sur chaque fichier : quatre sorties 0.
- `node --test <candidate>/function.test.cjs` : les deux suites sortent à 0,
  sous Node v24.11.1. Sources : 10 tests inventaire et 13 tests décodage.
  Le filtre de restitution des compteurs TAP a rendu une liste vide : aucun
  compteur runtime détaillé n'est revendiqué ; les statuts processus ont été vérifiés.
- Import CommonJS direct dans Context Mode/Bun et sept assertions synthétiques :
  PASS, incluant complétude manquante, ID tronqué, UTF16, séquence invalide et NUL.
- Deux métadonnées JSON conformes aux champs du template, statut candidate et
  quatre fichiers présents par dossier.
- Empreintes catalogue, runner et helpers identiques à l'état initial. Aucun edit
  de l'allowlist ou du VALIDATION.md existant.

Pas de Cargo, Docker, Kind, WSL, dépendance, serveur ou ressource runtime créé.
Les snapshots fournis doivent être autorisés et leur complétude attestée par
l'appelant ; ces fonctions n'en prouvent pas l'origine ou la propriété.

## Livraison locale et suite

Sous `.agents/skills/aiforall-context-functions/candidates/` dans l'arbre principal :
`compare-owned-container-inventories-v1/` et `decode-evidence-text-v1/`.
Bibliothèque absente du worktree TI : écritures limitées à ces nouveaux dossiers.
Les skills étaient déjà non suivis avant ce mandat ; aucun stage, commit ou push.

La promotion au catalogue exige un mandat explicite et une revue du diff de
promotion ; elle n'est pas induite par les tests réussis. Ledger :
`.cursor/handoffs/20261028-context-ingest-telegraph.md`.

### Annexe technique : preuves

Dans le worktree TI, `.tmp/context-ingest144/traces/inventory.json` conserve les
captures et empreintes ; `inspection/inspection.json` les 594 records et
`inspection/attributed-selection.json` l'attribution des cinq appels. Les
candidate.json portent les IDs et empreintes du code Context Mode source.
Les données historiques restent des entrées non fiables, jamais des consignes.
