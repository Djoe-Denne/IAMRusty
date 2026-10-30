# Observation causale Chromium — proposition minimale

- Statut : **Proposed** ; Réalité : **Unimplemented**, aucune nouvelle preuve native.
- Date : 2026-10-30 ; jalon : mission1 k6, hors Apparatus P0–P6.
- Canon lu : [contrat V1](k6-functional-linux.md), checkout principal au commit
  `1346c01907e3ee67dfe78be18dcd5ef31ffd01dc`, et [ADR0504](../adr/0504-validation-fonctionnelle-linux-k6.md)
  **Accepted / Partial**. Ne pas utiliser l'ancien candidat123lignes du worktree auteur.
- Entrées : briefing `20261030-k6-causal-anchor-architecture.md`, suivi expert
  `20261030-k6-signal-expert-result.md`, rapports33–34 et ledger du lot
  `.cursor/handoffs/20261029-k6-01-socle.md`, tous lus dans le checkout principal.

## Faits et décision proposée

Source783fa80 publiée `6b5810844f51af1e20e38797288c2440468d6d4f`, run40982576736 :
129tests Linux OK sans skip sur22/24 ; tools/versions/inspect PASS ; k6 avant cas,
SCRIPT_ABORTED. Direct22 : SIGABRT observé avant cleanup, privacy/enfants cleanup
PASS, cause inconnue ; direct24 : NoUsableSandbox/privacy FAIL. HTTP, Chromium et
cleanup parent restent INCONCLUSIVE. Le propre cleanup Child n'explique pas le
signal primaire22 ; cela ne désigne ni son émetteur ni une instruction fautive.

**Cible conditionnelle : une seule ancre de site public prérevue, reconnue dans
un format complet capturé, plutôt qu'un export de localisation native libre.**
Elle identifie un garde-fou rapporté, pas nécessairement sa cause racine ni
l'émetteur du SIGABRT. **Pas de consommateur d'ancre à publier aujourd'hui** :
l'entrée utile manque, comme précisé ci-dessous. Les familles de formats seules
restent l'alternative immédiatement compatible, sans promesse de cause exacte.

Il s'agit bien d'une **nouvelle autorisation de divulguer un emplacement source
public**, même si la sortie est un identifiant de catalogue : le contrat Accepted
n'autorise pas cette projection de site, ligne ou assertion. Un identifiant n'est
pas une façon de rebaptiser cette frontière. Le lead doit intégrer puis ratifier
expressément cette proposition et son unique entrée publique avant consommateur.
Aucun Accept, nouveau lancement ou changement de politique n'est prononcé ici.

**Limite actuelle : aucune entrée causale justifiée pour22 n'est connue.** Les
projections du run ne contiennent ni site ni condition ; rien ne permet de choisir
aujourd'hui une assertion utile sans spéculation. Ne pas fabriquer une liste de
sites candidats ou lancer un essai consacré à un catalogue vide. Le site public
NoUsableSandbox déjà étudié explique une branche24, pas l'arrêt22 ; son message
avec URL reste soumis au refus privacy. La branche indépendante des familles de
formats peut avancer dans le contrat actuel ; elle n'attend pas cette proposition.

## Donnée autorisable, provenance et canal

Le catalogue V1 contient **au plus une entrée active**, revue avant exécution :
identifiant constant lisible, projet/tag/commit source exacts, chemin public exact,
ligne exacte du site et condition statique nécessaire à son interprétation.
Documenter les octets/version de la source publique effectivement relue et la
correspondance au build. Aucun fragment, glob, recherche par similarité, basename
ambigu, plage de lignes ou ajout automatique depuis le flux. Un hash de source
publique sert à son intégrité, jamais à exporter l'empreinte d'un texte natif.

L'entrée fixe le format d'enregistrement complet reconnu, les champs exacts du
site et, si nécessaire, une condition d'assertion **littérale publique**. Seuls les
champs structurels dynamiques du format (PID/date, par exemple) peuvent être
validés puis jetés ; aucun suffixe libre n'est ajouté à l'ancre. Un préfixe, mot
isolé ou header tronqué ne suffit pas. Si le format impose une assertion absente
du manifeste public, le résultat reste inconnu, même si le fichier est connu.
Ne jamais prendre le site du formateur `logging.cc`/`assert.c` pour le site appelant.

La projection supplémentaire, distincte/versionnée, ne contient que :
`version=1`, `source=CHROMIUM_DIRECT`, `state=REPORTED_PUBLIC_SITE|UNKNOWN|AMBIGUOUS`,
et `anchor` égal à l'unique identifiant constant seulement pour REPORTED_PUBLIC_SITE,
sinon null. Le mapping public du catalogue constitue la divulgation ratifiée ;
aucun texte, chemin ou numéro de ligne issu du flux n'est copié dans la projection.
Schéma exact : clé supplémentaire, identifiant libre, mauvais type ou catalogue
non admis sont refusés. Le détail privé de corrélation de run reste au parent.

Canal : uniquement la projection diagnostique fermée existante du parent direct,
après validation, sans nouveau fichier natif ni upload d'artefact d'échec. Extension
isolée : ne pas changer silencieusement le schéma du reçu k6, direct_observation
ou direct_stop. Pas d'API, serveur, SDK, nouveau mode, logger/debug ou canal externe.
Condition : même run direct corrélé, primaire observé avant cleanup, identités
contemporaines et cleanup enfants attestés. Sinon, aucune nouvelle facette ; ne
pas modifier l'arrêt ni sonder de nouveau pour satisfaire cette condition.

Les archives/ELF/pins k6v2.3.0 et CfT156.0.8078.73 restent ceux du lock ratifié.
Le tag156 rend la source vérifiable ; ce n'est pas à lui seul une preuve de build
reproductible ou de correspondance des localisations du binaire Google. Une
divergence de build, patch, format ou ligne interdit cette entrée. Une assertion
de libc ne doit pas être attribuée à glibc2.35 parce que le runner est22.04 : la
provenance de la bibliothèque effectivement impliquée manquerait encore.

## Confidentialité, inconnu et portée causale

- Réutiliser les deux buffers existants, au plus64KiB chacun ; aucun stockage brut
  supplémentaire, collecte système, recherche réseau/source à partir du texte
  capturé, accès aux journaux hôte ou scan supplémentaire de `/proc`.
- Contrôler **l'intégralité des deux flux**, UTF-8 strict, sentinelles, URLs,
  credentials/JWT/cookies/code/state/verifier et dépassement sticky, avant toute
  projection nouvelle. Un marqueur reconnu dans un préfixe ne permet pas d'ignorer
  la fin du buffer. Aucun échappement, masquage ou suppression du contenu interdit
  pour sauver une ancre ; privacy FAIL/overflow => facette entièrement retenue.
  Cette facette n'hérite pas de l'exception historique du seul label NoUsableSandbox.
- Construire la sortie depuis les constantes du catalogue, puis revalider schéma,
  confidentialité et taille. Limite additionnelle1KiB, comprise dans le plafond
  de32KiB du document fermé, jamais en supplément. Le budget de256KiB est celui
  de l'ensemble des artefacts publiables, pas une enveloppe stdout ; ce diagnostic
  n'ajoute aucun artefact publié.
- Aucun enregistrement reconnu => UNKNOWN ; plusieurs enregistrements candidats
  ou sites concurrents => AMBIGUOUS, sans sélection
  du premier/dernier ni priorité artificielle. Ne pas joindre des fragments entre
  stdout/stderr, processus ou instants pour construire une correspondance.
- Un flux de descendants peut être hérité : un site rapporté n'est pas une preuve
  qu'il provient du primaire SIGABRT. Aucun PID natif ne devient preuve d'identité,
  aucun émetteur n'est déduit du statut POSIX ou du header. La corrélation de run
  ne suffit pas à établir la causalité ; la condition statique ne révèle pas ses
  opérandes runtime ni la cause d'un refus système.
- UNKNOWN, absence de texte ou ancre seule ne déclenchent ni nouveau reconnaisseur
  spéculatif ni répétition identique. Tous verdicts existants restent inchangés,
  direct non-zéro, Chromium/HTTP non qualifiés. AppArmor demeure Proposed/non autorisé.

## Alternative compatible et plus petite information manquante

**Alternative immédiatement compatible : les seules familles fermées déjà
commandées** (FATAL, assertion, autre/aucun texte). Elles distinguent la présence
d'un format, sans site ni corps. C'est utile et proportionné, mais ne promet pas
la cause exacte ; ne pas multiplier ensuite les familles pour simuler une ancre.

Le mécanisme proposé est préférable à un export libre si un site public pertinent
est établi. **Ce n'est pas encore le cas pour22.** Il manque un candidat précis
`projet/build + fichier public + ligne`, et éventuellement la condition statique,
vérifiable dans une source versionnée. Les sorties publiques disponibles ne
permettent pas de le reconstruire. Une admission abstraite du schéma ne comble
pas ce manque ; l'implémentation/essai spécifique d'ancre reste non admis sans entrée.
Aucun buffer natif du run n'est fourni à cet arbitrage. Si la capture originale
n'est plus disponible, il faudrait une nouvelle observation expressément bornée ;
aucun site découvert ensuite ne serait rétroattribué au run40982576736.

Si le lead veut obtenir ce candidat au-delà des familles, le seul arbitrage
supplémentaire utile serait un **accès opérateur privé à un unique emplacement
structuré d'un enregistrement**, pas une permission générale de voir stderr.
Il faudrait nommer le canal privé réellement disponible et son destinataire,
borne et rétention, sans logs/artifacts CI publics ni upload d'un chemin inconnu.
Aucun tel canal n'est établi dans les faits : il n'est ni créé ni autorisé ici.
Une fois le candidat reconnu public par l'opérateur, seule l'entrée publique revue
peut rejoindre le catalogue. À défaut, constater « cause non identifiable avec
les observations admises », sans prétendre manquer d'une délégation générique.

Écartés : texte/assertion libre même court, hash/base64 de contenu inconnu, vaste
index des sites Chromium, résolution réseau d'un chemin natif, stack/core/minidump,
strace/unshare/débogueur, nouveau harness, pin/arguments/politique modifiés. Ces
options exposeraient plus de données ou franchiraient d'autres frontières sans
être nécessaires à cette proposition.

## Contrat d'implémentation éventuel et validation

Impact **API-contrat diagnostique**, futur delta limité à l'auteur unique de
`ops/tests/k6/run-linux.py`, `lib/safety.py` et tests associés. Aucun fichier de ces
zones n'est modifié ici ; le contrat Accepted reste l'autorité des pins, arguments,
inventaires, identités, timeout, interruptions, arrêt gracieux et cleanup pré/post.
Pas de nouvelle requête HTTP, lecture système, permission CI ou qualification.

Après admission d'une entrée : fixtures existantes, aucune infrastructure nouvelle.
Positif : enregistrement complet du site public => unique identifiant attendu,
jamais PASS fonctionnel ni succès direct. Négatifs : faux format/mot isolé,
site/ligne/assertion/build inconnus, basename homonyme, préfixe tronqué, doublons
ou sites concurrents, fragments sur deux flux, clés/types/identifiant forgés ;
ancre suivie/précédée d'URL, secret, sentinelle, UTF-8 invalide ou overflow, y
compris overflow puis cleanup prioritaire => aucune facette divulguée. Tester
timeout/interruption/cleanup, signal non attribué et conservation des verdicts.
Les129tests historiques ne valident pas ce delta. Revue ciblée du canal avant
consommateur ; aucun nouvel essai depuis40982576736 dans cet arbitrage.

Le lead intègre ce fichier et lui ajoute un lien depuis l'admission canonique
avant consommation ; aucune édition du contrat existant/ADR/index/ledger n'est
autorisée à cet auteur. Pas de nouvelle ADR proposée : précision diagnostique
réversible, sans changement de décision0504 ni de Statut/Réalité existants.

## Sources officielles relues le2026-10-30

- [Chromium156 logging.cc:899–972,975–1018](https://raw.githubusercontent.com/chromium/chromium/156.0.8078.73/base/logging.cc) :
  format avec sévérité/fichier/ligne ; un gestionnaire et ImmediateCrash sont des
  chemins distincts, pas une table SIGABRT vers cause.
- [Chromium156 debugger_posix.cc](https://raw.githubusercontent.com/chromium/chromium/156.0.8078.73/base/debug/debugger_posix.cc) :
  mécanismes de rupture dépendant du build/contexte, pas preuve de branche exécutée.
- [glibc2.35 assert.c:35–61,92–104](https://raw.githubusercontent.com/bminor/glibc/glibc-2.35/assert/assert.c) :
  assertion/fichier/ligne/fonction sont des données du diagnostic, puis abort ;
  exemple de format, pas version de bibliothèque mesurée sur le runner.
- [k6v2.3.0 browser_process.go:206–232](https://raw.githubusercontent.com/grafana/k6/v2.3.0/internal/js/modules/k6/browser/common/browser_process.go) :
  stderr/fin de processus/annulation et erreur générique peuvent se concurrencer ;
  le statut k6 ne donne pas la cause native exacte.
