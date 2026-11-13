# Campagnes de correction : Clippy prioritaire

Workflow courant pour les services AIForAll. Il est appelé par le skill
[`aiforall-sonar-policy`](../../.agents/skills/aiforall-sonar-policy/SKILL.md)
et remplace le dispatch fondé sur les lignes Sonar des campagnes historiques.
Il ne lance pas une campagne automatiquement et ne crée pas de gate bloquant.
Le sous-module RustyCog reste un lot distinct, sur autorisation et avec sa
politique propre ; ne pas lui appliquer implicitement ce dispatch services.

## 1. Produire une photographie locale fraîche

Le lead réserve l'unique slot Cargo et attend la fin des éditions avant de
lancer **`just clippy-ci`**, nativement sous Windows, depuis la racine intégrée.
Les paramètres sont ceux de [Clippy local](clippy-local.md) : deux passes
production/tests, workspace locked all-features, toolchain épinglée, 24 jobs.
Ne pas exécuter de services ou de tests d'intégration pour ce simple inventaire.

Sources de l'inventaire :
- `target/sonar/clippy-production.jsonl` ;
- `target/sonar/clippy-tests.jsonl` ;
- `target/sonar/clippy.json`, concaténation des deux précédents.

Les deux passes doivent être terminées avec succès ; le lanceur vérifie leurs
codes retour et événements `build-finished`. Une passe échouée ou incomplète
donne un **inventaire partiel**, pas zéro diagnostic ni une preuve d'absence.
Si une passe échoue, ne pas réutiliser un ancien fichier de l'autre passe.

Avant modification, conserver sous `.tmp/<campagne>/<run>/` les rapports utiles
et un petit manifeste : date, HEAD **et état des sources non committées**,
racine/worktree, toolchain, options et résultat de chaque passe. HEAD seul ne
décrit pas un arbre dirty. Après toute modification des sources analysées,
l'inventaire n'est plus une photographie du code actuel. Ne pas comparer les
chiffres de deux configurations différentes comme une progression de dette.

## 2. Lire les diagnostics, pas les anciennes lignes Sonar

Filtrer et agréger les JSON dans Context Mode. Les messages utiles ont
`reason = "compiler-message"` :
- règle : `message.code.code` (`clippy::...` pour Clippy) ;
- niveau et texte : `message.level`, `message.message` ;
- emplacements : `message.spans` où `is_primary = true`, avec `file_name`,
  `line_start`, `column_start`, `byte_start` et `byte_end` ;
- provenance : passe production/tests, `package_id` et `target`.

Conserver les avertissements rustc séparément et remonter les erreurs bloquantes.
Les autres lignes JSON (artefacts, fin de build) ne sont pas des diagnostics.
Un diagnostic sans emplacement primaire reste dans une file à examiner ; ne
pas lui inventer une ligne ou un propriétaire de fichier.

Dans **une photographie**, dédupliquer par niveau + règle + message + ensemble
des emplacements primaires (chemins normalisés par rapport à la racine du run).
Conserver toutes les appartenances production/tests/packages/targets derrière
un finding fusionné. Ne pas additionner naïvement les deux passes ; des tests
peuvent réanalyser le même code de bibliothèque.

Les positions ne sont **pas** des identifiants persistants. Pour un rapprochement
entre runs, utiliser fichier/règle/symbole/contexte après lecture des sources,
et signaler les cas ambigus. Ne pas fusionner des occurrences distinctes au seul
motif qu'elles ont la même règle et le même texte. Ne pas convertir une ligne
Sonar en ligne actuelle par ajout/soustraction d'un décalage.

## 3. Classer et attribuer des lots exclusifs

Conserver deux axes distincts :
- **Dispersion de la règle** : locale si une seule aire du dépôt, transverse si
  plusieurs aires (`services/<nom>`, `workers/<nom>`, `crates/<nom>`,
  `runtime/<nom>`). Ce comptage n'établit pas l'impact d'un correctif.
- **Impact du changement**, confirmé par le code et ses consommateurs :
  local mécanique ; structure privée ; comportement ; API/signatures/appelants.
  Une règle présente partout peut se corriger fichier par fichier ; un seul
  diagnostic sur une API publique peut nécessiter un lot transverse.

Le lead consulte le [registre des décisions](clippy-decisions/README.md) et les
tables du skill, puis constitue des lots de fichiers disjoints. L'unité de
conflit reste le fichier ; respecter aussi le mutex package × src/tests.
Une signature et ses consommateurs restent dans le même lot, même si plusieurs
règles sont concernées. Ne pas scinder un gros fichier commun entre auteurs.
Faire valider les décisions structurelles selon les contrats du dépôt.

Chaque claim consigne : propriétaire, photographie source, fichiers autorisés,
règles et symboles/contexte concernés, invariants, précédents applicables,
validation attendue et dépendances sur les autres lots. La ligne du run est une
aide de navigation, **pas l'identité du lot**. Aucun nombre fixe de sous-agents :
choisir selon les lots indépendants et charger `opencode-ensemble` si délégation.

Dans un worktree divergent, les positions canoniques ne sont qu'une piste :
vérifier le code local et les différences avant correction. Ne pas faire croire
que l'index ou le rapport canonique contient les changements non intégrés.
Un worker ne lance pas Cargo pendant les éditions parallèles ; en cas de
position obsolète ou de consommateur hors claim, il relit le symbole ou rend
le blocage au lead, sans étendre son périmètre ni éditer chez un autre auteur.

## 4. Corriger, intégrer, relancer, redistribuer

1. Les auteurs corrigent leurs lots en appliquant les décisions et validations
   pertinentes. Pas de `--fix` global, de nouveau `allow` ou de changement
   d'exclusions pour faire artificiellement disparaître la dette.
2. Le lead arrête les éditions, intègre les deltas autorisés et vérifie les
   consommateurs, tests et décisions nouvelles. Les tests comportementaux
   nécessaires ne sont pas remplacés par Clippy.
3. Une fois le slot Cargo disponible, relancer **`just clippy-ci`** sur l'arbre
   intégré ; reconstruire l'inventaire à partir de ces nouveaux rapports.
4. Comparer les findings à périmètre constant, examiner les régressions et
   préparer de nouveaux claims sur les sources actuelles. Ne pas « refill »
   une vieille liste à partir de ses lignes. Une disparition par feature
   désactivée, analyse partielle, `allow` ou exclusion n'est pas une correction.

Cette boucle s'effectue après chaque lot intégré, ou après une vague de lots
disjoints intégrés ensemble ; pas un Cargo concurrent par auteur. Entre deux
photographies, les auteurs relisent le code actuel au lieu de recalculer des
numéros de ligne. Un Clippy ciblé peut aider, mais n'atteste pas le compteur
global des deux passes.

## 5. Garder Sonar comme contrôle complémentaire

Les règles Sonar propres (notamment un bug `rust:S...` sans diagnostic Clippy
équivalent) restent une file séparée, à traiter selon leur risque. Clippy-first
ne veut pas dire ignorer une vulnérabilité ou un bug critique jusqu'au zéro
Clippy. Relocaliser ces cas dans le code actuel par règle, symbole et contexte,
pas par un calcul d'offset. Une égalité des compteurs n'est pas attendue.

Le scan Sonar suivant confirme la fermeture de ses issues et les écarts liés
à Linux/Windows ou aux règles différentes. Ne pas clôturer une issue Sonar/GitHub
sur la seule absence d'un warning local, ni changer manuellement son statut
pour masquer un problème. L'ancienne `.cursor/sonar-queue/` et les inventaires
des handoffs d'août/octobre sont des archives, pas le backlog courant.
