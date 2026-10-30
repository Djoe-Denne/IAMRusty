# Observation causale Chromium — proposition minimale

- Statut : **Proposed pour les observations causales** ; Réalité : **Unimplemented**.
  L'addendum d'hôte WSL dédié ci-dessous est **Accepted pour son provisionnement**,
  sans qualification de l'hôte ou admission du relevé privé.
- Date : 2026-10-30 ; jalon : mission1 k6, hors Apparatus P0–P6.
- Canon lu : [contrat V1](k6-functional-linux.md), checkout principal au commit
  `7aaaeafb54c532b19afb5cde165a2aad1b8eee6f`, et [ADR0504](../adr/0504-validation-fonctionnelle-linux-k6.md)
  **Accepted / Partial**. Ne pas utiliser l'ancien candidat123lignes du worktree auteur.
- Entrées : briefing `20261030-k6-causal-anchor-architecture.md`, suivi expert
  `20261030-k6-signal-expert-result.md`, rapports33–34/37 et ledger du lot
  `.cursor/handoffs/20261029-k6-01-socle.md`, tous lus dans le checkout principal.

## Faits et décision proposée

Source783fa80 publiée `6b5810844f51af1e20e38797288c2440468d6d4f`, run40982576736 :
129tests Linux OK sans skip sur22/24 ; tools/versions/inspect PASS ; k6 avant cas,
SCRIPT_ABORTED. Direct22 : SIGABRT observé avant cleanup, privacy/enfants cleanup
PASS, cause inconnue ; direct24 : NoUsableSandbox/privacy FAIL. HTTP, Chromium et
cleanup parent restent INCONCLUSIVE. Le propre cleanup Child n'explique pas le
signal primaire22 ; cela ne désigne ni son émetteur ni une instruction fautive.

Suivi effectif, source `1c0036f`, run40985953377 :140tests Linux OK sans skip22/24,
tools/inspect PASS ; direct22 corrélé stdout EMPTY/stderr PRESENT,
CHROMIUM_FATAL_REPORTED et SIGABRT avant cleanup, privacy/enfants cleanup PASS.
Site/cause inconnus : **STOP reconnaisseurs et répétitions identiques**. Le dépôt
est PUBLIC/isPrivatefalse selon le contrôle du lead : aucun log Actions n'est un
canal privé. Aucun artefact natif fourni à cet arbitrage, aucune collecte nouvelle.

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
avec URL reste soumis au refus privacy. La classification des familles a maintenant
livré FATAL sans site : ne pas la prolonger par une liste spéculative.

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

Les familles fermées ont épuisé leur objectif : FATAL est établi, pas le site.
Elles restent des preuves compatibles, pas une voie pour découvrir la cause par
répétition. Le protocole d'ancre publique reste conditionnel à une entrée utile.

Le mécanisme proposé est préférable à un export libre si un site public pertinent
est établi. **Ce n'est pas encore le cas pour22.** Il manque un candidat précis
`projet/build + fichier public + ligne`, et éventuellement la condition statique,
vérifiable dans une source versionnée. Les sorties publiques disponibles ne
permettent pas de le reconstruire. Une admission abstraite du schéma ne comble
pas ce manque ; l'implémentation/essai spécifique d'ancre reste non admis sans entrée.
Aucun buffer natif du run n'est fourni à cet arbitrage. Si la capture originale
n'est plus disponible, il faudrait une nouvelle observation expressément bornée ;
aucun site découvert ensuite ne serait rétroattribué au run40982576736.

### Option privée minimale concrète — Proposed, non autorisée

**Recommandation : un relevé local structuré pour un opérateur humain désigné**,
sur son hôte Linux amd64 privé autorisé ; aucune exécution hébergée dans Actions.
Ce relevé local d'un site encore inconnu est une frontière distincte de l'ancre
publique ci-dessus, à ratifier explicitement par le lead avant consommateur.

1. **Préflight requis, pas présumé acquis :** hôte/session et opérateur réels,
   UID non-root, espace privé sans synchronisation/sauvegarde automatique ni autre
   workload non fiable sous le même UID. Distro conforme au contrat :24.04 nominal
   ou22.04 diagnostique seulement ; autre distro exige un arbitrage de plateforme.
   Dépendances déjà présentes, pins/archives/ELF/version vérifiés, sandbox et mêmes
   arguments conservés. Le boot Linux proposé par l'utilisateur n'est pas un hôte
   prêt prouvé ; WSL avait échoué sur libnspr4. Aucune installation n'est autorisée
   par ce document. Hôte différent = expérience distincte, sans équivalence CI.
2. **Collecte minimale :** réutiliser les buffers du direct et le format FATAL déjà
   reconnu, après toutes les gardes privacy/overflow/identité/cleanup existantes.
   Un seul enregistrement complet ; sinon aucun relevé. Extraire seulement un
   chemin source relatif littéral ASCII de1–256octets : composants non vides en
   `[A-Za-z0-9_.-]` séparés par `/`, sans composant `.`/`..`, préfixe absolu, drive, URL, backslash
   ou encodage, suffixe `.cc`/`.h`/`.c`,
   et une ligne entière1–1000000. Ne pas normaliser un chemin refusé ni le tronquer.
   Jamais corps, assertion, fonction, PID/date natifs, adresse, errno ou extrait.
   Le chemin reste **confidentiel et non vérifié public** : la grammaire ne prouve
   pas son innocuité ; l'outil ne l'ouvre jamais comme chemin local. Aucun fichier
   contenant stdout/stderr n'est créé.
3. **Canal local exact :** un JSON au plus1KiB, clés fermées `version`, `run_id`
   opaque existant, `browser_version` épinglée, `source_relative`, `source_line`,
   `delete_by_utc`. Création exclusive dans un sous-répertoire neuf0700 de
   `$HOME/.cache/opencode/k6-causal/<run_id>/`, hors Git et sans lien, appartenant à
   l'UID de l'opérateur ; fichier0600 pendant écriture puis0400 avant lecture.
   Réutiliser les contrôles de chemin/identité existants, jamais chmod/adoption
   d'un répertoire étranger. Ni stdout/stderr parent, publisher, artefact Actions,
   chat/MCP/Read/cat enregistré, clipboard ou transfert de ce contenu inconnu.
4. **Accès et rétention :** lecture locale par ce seul opérateur autorisé, puis
   retrait du seul fichier possédé après lecture ou sous60minutes au maximum.
   Le parent conserve la session surveillée jusqu'au retrait vérifié ; sinon ne
   pas ouvrir ce canal. Aucun timer/daemon créé ni expiration magique promise en
   cas de crash. Privacy FAIL/interruption/collecte incomplète => aucun relevé
   conservé ; SIGABRT diagnostique attendu ne change ni son exit ni cette échéance.
   Échec du retrait = rétention/cleanup non prouvé à signaler, pas PASS. Supprimer
   une entrée de fichier n'est pas garantir l'effacement des caches/blocs/swap ;
   administrateur/compromission hôte et capture d'écran restent des risques.
   La preuve de rétention ne rapporte que création/retrait attestés ou inconnus
   et le run opaque, jamais le chemin ni le contenu du relevé.
5. **Retour au public :** l'opérateur vérifie le candidat par navigation dans les
   liens déjà publics de l'arborescence officielle versionnée, sans envoyer le
   chemin inconnu à une recherche/API/URL. Après reconnaissance d'un site public
   exact et examen de sa condition statique, seule l'entrée publique prérevue peut
   être proposée au lead pour ratification. Sinon aucun contenu ne sort ; cause
   inconnue. Un site local ne sera pas rétroattribué au run40985953377, et le site
   rapporté ne prouve toujours pas l'émetteur du signal ou la cause racine.

**Autre voie compatible, conditionnelle : affichage structuré local éphémère à
l'opérateur sur terminal privé non enregistré**, sans fichier. Même sélection,
préflight et interdiction de transfert ; moins de rétention disque, mais scrollback,
enregistrement/partage d'écran plus difficiles à borner et moindre traçabilité de
lecture. Préférer le fichier privé borné réutilisant les gardes existantes ; si le
terminal ne peut être garanti privé, cette alternative est refusée. Aucun service,
tunnel, secret CI ou nouveau stockage distant n'est une alternative admise.

**Manque initial, avant réponse utilisateur :** aucun hôte privé prêt, accès opérateur ni destinataire autorisé
de ce seul couple inconnu n'est établi. Architecture, schéma et choix de rétention
sont dans la délégation du lead ; ils ne fournissent ni cet accès ni l'accord de
divulgation locale au destinataire. Question minimale, si nécessaire : « Votre
boot Linux proposé est-il la session privée utilisable, sur quelle distribution,
et qui est l'opérateur autorisé à lire localement ce seul emplacement non vérifié,
sans nous le recopier ? » Aucune clé/credential ni GO générique n'est demandé.
Pas d'implémentation, collecte ou essai tant que ces prérequis et l'admission ne
sont pas établis. Une éventuelle observation locale aurait ce canal discriminant
explicite, jamais une répétition identique destinée à obtenir du vert. L'exit du
direct, les verdicts HTTP/Chromium/cleanup et les protections restent inchangés.
À défaut : cause non identifiable avec les observations admises.

**Actualisation du lead après choix utilisateur (2026-10-30).** L'utilisateur a
choisi « WSL local, lecture par moi ». Le lecteur est donc désigné ; le préflight
en lecture seule a identifié ARTIST-Ubuntu24.04/x86_64, artist UID1000, et le binaire
Chromium historique conforme au pin. Les dépendances libnspr4/libnss3/libasound2t64
et libasound2-data ne sont pas installées ; simulation0upgrade/4ajouts/0suppression.
Ce choix n'autorise pas leur installation implicite. Le contrôle de l'espace de
stockage privé et toutes les gardes d'activation restent à vérifier ; aucun relevé,
consommateur ou essai n'est admis par cette photographie. La question immédiate
porte sur ces quatre dépendances précises, plus sur un GO générique ou sur
l'identité du lecteur déjà fournie. Voir rapport39 et ledger du lot.

### Addendum hôte WSL dédié sur I: — Accepted pour le provisionnement, 2026-10-30

Base canonique `fdbb29160cc91ed699afdba6323f07955831bd69`, rapport40, ledger et
briefing `20261030-k6-wsl-dedicated-instance-architecture.md`. **La demande
utilisateur de créer l'instance et d'y installer les outils nécessaires est
acquise** ; elle remplace le plan d'installation dans ARTIST-Ubuntu ci-dessus,
conservé comme photographie historique. Le lecteur reste l'utilisateur déjà
désigné (« lecture par moi »). Le lead ratifie ce rattachement avant création,
sans redemander ces autorisations. Le lead ratifie **ce seul provisionnement avant
création**, sous la demande explicite utilisateur, après récupération byte-identique
et sauvegarde du candidat architecte `result_mvuqe1yi_0002_ar0a9l`. Le relevé privé
et sa QA restent Proposed/non admis ; aucun résultat runtime n'est déduit de cet accord.

Réalité après la première tentative : image officielle acquise/hash conforme,
mais WSL a rejeté le nom demandé `AI for All` avec
`Wsl/Service/RegisterDistro/E_INVALIDARG`. Aucun GUID de distribution nouveau,
destination absente et défaut WSL inchangé. L'utilisateur a ensuite explicitement
choisi **`AIForAll`** comme nom technique. Le lead actualise ce seul paramètre avant
la reprise ; libellé humain « AI for All », emplacement et autorisation d'installation
restent inchangés. Aucune création n'est déduite de ce choix.

Réalité ensuite vérifiée : enregistrement `AIForAll` réussi, GUID
`{22ce910b-45a3-42a4-bbab-dc56a1766035}`, BasePath I:\WSL\AIForAll et ext4.vhdx
sur I:, défaut global inchangé. Compte aiforall/defaultUID1000 observé non-root,
HOME0700 natif ext4 ; stockage Windows limité au propriétaire/SYSTEM/Administrators.
Les28paquets demandés sont installés (refresh/simulate/install0, dépôts Ubuntu
authentifiés), sans k6/Chromium exécuté. Le [guide d'hôte](../wsl-aiforall.md)
et rapport43 décrivent cette preuve ; la vérification indépendante est à venir,
les qualifications et le relevé privé ne sont pas admis par ce provisionnement.

- **Hôte exact :** nouvelle instance WSL2 Ubuntu24.04 amd64, nom d'enregistrement
  **`AIForAll`** (choix utilisateur après refus de `AI for All`), emplacement
  **`I:\WSL\AIForAll`**. Préflight lead : I: NTFS sain,
  1240483057664octets libres, parent I:\WSL existant, destination absente ; WSL2.6.1.0
  expose les options nécessaires. Ce sont des constats avant création, pas une
  preuve d'instance livrée. Revalider absence de collision avant mutation ; aucun
  dossier occupé, compte existant ou ancienne distro remplacé.
- **Acquisition :** image officielle versionnée Ubuntu24.04.5 amd64, recoupée avec
  le catalogue WSL `Ubuntu-24.04`, plutôt qu'une acquisition flottante :
  `https://releases.ubuntu.com/24.04.5/ubuntu-24.04.5-wsl-amd64.wsl`, SHA256 attendu
  `bb415d824822c4b878125729af451a5d18fb13d1cf5cbed9a7393ad64ac6039e`, relu dans
  les SHA256SUMS Ubuntu officiels. Contrôler les octets acquis contre ce pin avant
  installation. Image ni téléchargée ni exécutée par cet arbitrage ; métadonnées
  seules vérifiées. La procédure native d'installation locale reste au lead,
  avec nom/emplacement explicites et non-lancement automatique ; pas `Ubuntu`
  flottant vers26.04. Tracer canal/éditeur, identité/version réellement acquise et
  provenance ; un nom ou un HEAD HTTP seul ne prouve pas l'intégrité des octets.
  Le support du nom avec espaces est à confirmer lors de la création : l'aide
  locale est une capacité annoncée, pas un succès. Refus réel => arrêt et retour
  précis, jamais autre nom implicite. L'import d'une image officielle vérifiée
  est une alternative si choisie/tracée par le lead avant création, pas un fallback.
- **Compte :** créer dans l'hôte neuf le compte de travail `aiforall`, UID effectif
  non nul observé et HOME privé. Administration de cette seule instance autorisée
  pour comptes et installations nécessaires ; k6/Chromium/validations non-root,
  invocation explicitement rattachée à cette distro et ce compte. Aucun Cargo/Rust
  de dev dans WSL, toolchain Rust ou Docker installé spontanément ; Cargo Windows
  et son slot unique restent inchangés. Ni réglage noyau/sandbox, fonctionnalité
  Windows globale ni mise à jour WSL implicite sous couvert d'installation.
- **Propriété et bail :** parent responsable de l'inventaire avant/après et du
  registre contemporain de création : GUID d'enregistrement Lxss, BasePath/VHDX
  observés, distribution/version/architecture, compte/UID/HOME et états runtime.
  Corréler ces identités au delta créé, pas au seul nom ; bail propre aux sessions
  actives de cette instance, aucun ancien bail ARTIST adopté. Vérifier distro par
  défaut avant/après, sans la changer. ARTIST-Ubuntu arrêté, docker-desktop/default
  et autres ressources étrangères restent hors propriété et inchangés.
- **Données :** outils, profils et futurs relevés privés sur le FS Linux natif
  du VHDX de cette instance, HOME de `aiforall`, hors Git et sans liens ; jamais
  `/mnt/i` ou `/mnt/c` pour simuler les garanties POSIX. Vérifier aussi les accès
  Windows au nouveau dossier/VHDX ; ne pas modifier les ACL du parent étranger.
  La distro, ses outils et données de projet persistent car demandés. Cela ne
  prolonge pas la rétention du couple source inconnu, toujours bornée séparément.
- **Runtime à la demande :** baux/consommateurs actifs puis jonction des enfants
  possédés ; lorsqu'inutile, revalider GUID/nom/BasePath et arrêter uniquement cette
  distro après libération du bail. Persister les états avant/après. Le stop ne
  détruit pas le disque ; aucun unregister/reset/prune/suppression ou shutdown WSL
  global. Aucun arrêt Docker/ancienne distro ; la RAM du backend partagé n'est
  pas revendiquée entièrement libérée par cet arrêt ciblé.

**Admission distincte :** provisionnement dédié demandé ≠ qualification de l'hôte
≠ admission du consommateur/relevé privé. Pins/arguments, identité, sandbox,
confidentialité, rétention et QA du canal restent à satisfaire/ratifier avant ce
relevé ; l'autorisation d'installer n'autorise aucun export natif inconnu. Les140
tests historiques ne qualifient pas l'instance neuve. Aucun essai, installation,
commande WSL, code ou nouvelle ADR dans cet arbitrage documentaire.

Sources relues : [commandes WSL Microsoft](https://raw.githubusercontent.com/MicrosoftDocs/WSL/main/WSL/basic-commands.md),
[installation Ubuntu WSL officielle](https://ubuntu.com/wsl/docs/stable/howto/install-ubuntu-wsl2/),
[SHA256SUMS Ubuntu24.04.5](https://releases.ubuntu.com/24.04.5/SHA256SUMS),
`.cursor/rules/local-runtime-lifecycle.mdc`. L'aide locale2.6.1.0 observée par le
lead documente le nom à l'installation ; ne pas confondre avec `--mount --name`.

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
Pour le canal privé : vérifier, avec les fixtures existantes, création exclusive,
propriétaire/modes/horsGit/liens, schéma1KiB et refus des chemins interdits sans
troncature ; sentinelle/secret/UTF8/overflow ou plusieurs FATAL => aucun fichier.
Vérifier absence du couple inconnu dans toutes les sorties/publishers, lecture puis
retrait/échéance, interruption et refus sans hôte/opérateur validés. Pas de mock
serveur ni nouvelle infrastructure. Les140tests historiques ne valident pas ce
delta. Revue ciblée avant consommateur ; aucun nouvel essai depuis40985953377 ici.

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
- [Python3.12.10 os.rst, open/unlink](https://raw.githubusercontent.com/python/cpython/v3.12.10/Doc/library/os.rst) :
  modes de création et suppression de fichier, pas garantie d'effacement physique.
  Référence API, pas nouveau pin Python ni permission d'installation.
