# Observation causale Chromium — canal local et proposition d'ancre publique

- Statut : **Accepted pour l'implémentation et les tests du consommateur local
  privé, collecte conditionnelle** ; ancre et projection publiques **Proposed**.
  Réalité du consommateur : **Partial**, code et fixtures intégrés, revue ciblée close,
  aucun relevé réel créé. L'ancre publique reste non implémentée et non admise.
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

### Option privée minimale concrète — proposition historique, admission précisée ci-dessous

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

### Addendum admission du consommateur local privé — Accepted, 2026-10-30

Base `b8a62792b7a81e48dcc0307279be5e073d631e83`, briefing
`20261030-k6-private-channel-readiness-architecture.md`, guide WSL, rapports45–46
et ledger relus. Proposition architecte récupérée à l'identique (87lignes),
sauvegardée puis **ratifiée par le lead avant l'auteur unique** sous délégation
mission1 et choix explicites de l'hôte/lecteur. Admission de l'implémentation et
des tests ; toute collecte attend les contrôles ci-dessous, la revue indépendante
et le créneau de lecture local effectivement convenu. L'architecte n'a exécuté
aucun code ou runtime. Réponse finale de confirmation `result_mvuwioz0_0029_3xp18p`,
session conservée ; aucune ancre/projection publique ratifiée.

Photographie après implémentation et correction : quatre fichiers initialement
intégrés dans `968592f`, veto privé corrigé dans `cbab06e`. Tests Windows : 165,
exit0, huit skips ; Linux : 165, exit0, aucun skip, source propre19b1cfe.
Les fixtures n'utilisent que des sites synthétiques connus ; elles ne fournissent
pas le parent_check réel ni le rendez-vous de lecture. Le finding S-1 est fermé
par revue indépendante, sans admission de capture ; [preuves rapprochées](../../.opencode/missions/team_mvtz6p47_000q_4wmht0/reports/53-defaut-de-capture-ferme-conditions-operationnelles-restantes.md).

**Acquis, à ne pas redemander :** WSL local/lecture par l'utilisateur, installation
autorisée, AIForAll livré et provisionnement vérifié indépendamment. GUID
`22ce910b-45a3-42a4-bbab-dc56a1766035`, aiforall UID/EUID1000, HOME0700/ext4,
ACL Windows propriétaire/SYSTEM/admins ; copie native05bd/23hashes conforme et
propre,140tests sans skip, archives/ELF/versions2.3.0 et156.0.8078.73/deuxinspections
PASS. Le refus local pré-cas et le direct unique FATAL/SIGABRT avant cleanup sont
établis, pas leur site ou cause. AppArmor DISABLED ne désigne aucune cause.
30identités possédées/0actives puis AIForAll Stopped ; aucun runtime pour ces docs.

**Activation restante, propre à chaque collecte :**
- Préparer code, fixtures et revue n'exige pas la présence immédiate du lecteur.
  Créer le relevé exige un créneau de lecture locale réellement convenu avec cet
  utilisateur et un parent disponible jusqu'au retrait, au plus60minutes. Ce
  rendez-vous n'est pas un nouveau GO ni un nouveau choix d'hôte/opérateur.
- Parent : revalider l'identité GUID/nom/BasePath, compte, FS privé et bail actif
  par les contrôles existants, avant/après ; pas de vieux bail ou manifeste seul
  pris comme preuve. Recontrôler pins/intégrité et conditions privées d'activation,
  sans prétendre exclure absolument sauvegarde/administrateur Windows.
- Un opt-in CLI explicite, uniquement sur le diagnostic direct local24.04 existant,
  désactivé par défaut ; refus dans les autres modes et en comparaison22. Pas
  d'activation ambiante, fallback ou nouveau mode/harness. Refuser CI/Actions et
  leurs marqueurs présents **avant** nettoyage d'environnement et avant tout enfant.
  Leur absence ne prouve pas un hôte privé : l'admission locale du parent reste
  requise, portée par son appel sous bail vérifié, jamais une nouvelle autorité
  ou un accord permanent. Aucun chemin CI/workflow/publication ne transporte cet opt-in.
- Corréler au refus navigateur local déjà observé, ses pins/identités et à un
  nouveau run direct distinct ; raccord local24 minimal, pas adoption du reçu22
  comme s'il était équivalent. Aucun nouveau browser requis pour fabriquer cette
  corrélation. Le seul futur direct armé de ce canal serait une observation
  discriminante supplémentaire, pas un retry vert ni une preuve de cause GitHub.

**Consommateur et sortie :** réutiliser les captures/Child/environnement/identités
et cleanup existants, sans argument Chromium, outil, lecture système ou requête
ajoutés. Examiner les deux flux complets64KiB chacun, UTF-8 strict, secrets/URLs/
sentinelles et overflow sticky, même si cleanup devient prioritaire. Un unique
enregistrement FATAL complet seulement, primaire observé avant cleanup et enfants
cleanup PASS ; ambiguïté, timeout/interruption, identité non attestée, secret,
overflow ou échec de cleanup => **aucun relevé**, pas de meilleur préfixe exploité.

Seules données natives : le couple chemin relatif/ligne de l'option privée
ci-dessus ; mêmes limites/grammaire, aucun corps/assertion/valeur/PID en plus.
Conserver son JSON fermé et métadonnées prévues, au plus1KiB, emplacement fixe
`$HOME/.cache/opencode/k6-causal/<run_id>/site.json`, Linux natif hors Git/run neuf,
création exclusive0700/0600 puis0400. Valider schéma,
modes, liens et propriété avant de déclarer le relevé prêt ; retirer tout écrit partiel
possédé sur erreur. Un ancien fichier, chemin absolu ou fichier non privé est
refusé, jamais corrigé/adopté. Le chemin source n'est jamais ouvert par l'outil.

**Interdiction de sortie effective :** le couple reste uniquement dans ce relevé
local, le chemin du fichier dans la gestion privée du parent. Aucun champ/exception/repr/extrait/hash
dans stdout/stderr parent, reçu k6, projection publique, logger ou publisher ;
branche privée exclue de publication sans exception « artefact d'échec ». Les
sorties fermées existantes et exits non-zéro restent inchangés. Un relevé valide
n'est ni Chromium PASS ni cleanup parent PASS : l'échec natif/SANDBOX_UNPROVEN
n'empêche pas seul un relevé éligible, sans jamais devenir un succès. **Aucune ancre publique connue
n'est exigée avant cette lecture privée** ; le catalogue/public consommateur
reste Proposed/non admis, avec une admission distincte après vérification publique.

**Lecture/rétention :** l'utilisateur lit localement, jamais un agent Read/catMCP.
Après confirmation de lecture, ou à l'échéance60min même sans confirmation, le
parent retire le seul relevé temporaire dont l'identité/propriété sont attestées,
puis vérifie son absence. Ce retrait strict est prévu par cette admission future
de rétention ; ni outils, distro, image ni autres données ne sont supprimés.
Sans rendez-vous réel, pas de création ; échec de retrait ou crash = rétention
non prouvée signalée, aucun watchdog/effacement magique promis. Pas de prolongation
silencieuse ni reprise identique ; libérer le bail et arrêter seulement le runtime
possédé devenu inutile, en conservant disque/outils selon le contrat WSL.

**Avant collecte :** tests locaux ciblés du futur delta, puis revue indépendante
du canal/permissions/refus. Positif : FATAL unique complet => seul couple attendu,
JSON1KiB privé, jamais PASS fonctionnel. Négatifs : absence/opt-in dans mauvais
mode, CI même avant scrub, parent non admis, faux format/site ambigu, chemin/ligne
invalides, clés/types libres, fichier existant/liens/modes/UID/FS refusés, secret/
URL/sentinelle/UTF8/overflow tardif puis cleanup prioritaire, interruption/cleanup
échoué => zéro relevé. Vérifier zéro couple dans toutes sorties/publishers, lecture
puis retrait, absence de lecture à l'échéance et retrait impossible. Fixtures
existantes, aucun serveur/mock ou runtime nécessaire à cet arbitrage ; les140tests
antérieurs ne valident pas ce delta. Sources officielles156/k62.3 déjà étudiées
réutilisées ; aucun changement d'ADR, pin, sandbox, politique AppArmor ou mission.

### Précision opérationnelle du parent réel — Accepted pour préparation et vérification, 2026-10-30

Base `ce52ae8332ddd08065c3c6913d6032fd7891ed09`, briefing
`20261030-k6-private-parent-operation-architecture.md`, guide, API
`lib/private_site.py` et revue `20261030T1243Z` lus. Proposition architecte de
70 lignes récupérée byte-identique puis sauvegardée. Le lead ratifie la recette
avant préparation du programme parent, sous délégation mission1 ; candidat
`result_mvuzazoo_0002_qfx9jl`, même session architecte. La procédure réelle reste
**Partial** (code et fixtures livrés, préflight réel non vérifié) et son exécution conditionnelle à la vérification de sa composition
et au rendez-vous. Ce parent compose les gardes et APIs existantes : pas nouveau
harness, exécuteur Chromium, service ou autorité. La fermeture distincte du veto
S-1 est désormais intégrée, mais ne fournit pas le contrôle parent réel.

#### Amendement de composition pipes — Accepted pour implémentation et vérification, 2026-10-30

Base `df2a5ab033684c9aebf3de030d17185463d69872`, briefing
`20261030-k6-real-parent-pipes-architecture.md`, plan auteur et rapport55 lus.
Le lead ratifie avant code l'alternative Windows/pipes déjà prévue, après lecture
et récupération byte-identique du delta architecte `result_mvv0mhui_0002_p982l8`
(+55/-27 lignes) et sauvegarde vérifiée. Les précisions suivantes remplacent la
préférence interop. Aucun probe interop ni impossibilité de cette voie n'est
établi ou recherché ici. Le conducteur est **Partial** (code/tests, pas de préflight réel), capture conditionnelle
aux preuves/revue applicables et au rendez-vous réel, non accordé par cet amendement.

Photographie après code : trois fichiers intégrés dans `195059a`, source gelée ;
tests Windows : 189, exit0, huit skips POSIX ; Linux : 189, exit0, onze skips
réservés à PowerShell, sur snapshot propre887b6b0. Les suites exercent respectivement 181 et 178 tests,
pas une collecte réelle. Revue indépendante en cours, profils et préflight des
observateurs réels non acquis, aucun site inconnu créé. [Preuves et limites](../../.opencode/missions/team_mvtz6p47_000q_4wmht0/reports/60-fixtures-du-conducteur-verifiees-sur-les-deux-systemes.md).

1. **Windows, avant l'invocation :** veto CI avant création de processus ; le
   parent sous bail actif relit les contrôles
   existants Lxss/GUID/BasePath/DefaultUid, SID effectif et identité/ACL du dossier/VHDX. Il cible
   explicitement AIForAll/aiforall, vérifie l'association nom→GUID juste avant/après
   lancement et garde le handle natif de sa **seule invocation WSL créée**, avec
   PID/création/image/SID et fin attestés ; pas de récupération par nom ou PID seul,
   ni de défaut WSL. Il crée les pipes privés de ce processus, sans auxiliaire
   Windows interop. Le profil d'installation privé sert à comparer les observations,
   jamais de preuve seul ni de GUID personnel par défaut dans le code partagé.
2. **Dans cette invocation Linux non-root :** le compagnon ponctuel du parent
   établit l'identité du HOME réellement ouvert, UID/modes et FS ext4 avec les
   gardes existantes, vérifie provenance/refus24 conservé et intégrité des outils.
   Il entre `PrivateSiteSession`, appelle `main(..., private_site=session)` et
   **garde le contexte ouvert après le retour non-zéro**, jusqu'au retrait vérifié.
   Le callback combine ses vérifications Linux réelles et la réponse du contrôleur
   Windows lié à ces pipes, jamais un callback de fixture, booléen CLI, token ou
   JSON déclaratif chargé comme preuve. Aucun site ou corps natif ne va au contrôleur.
3. **AUTHORIZE puis WRITE :** comparer phase et séquence attendues, IDs opaques
   navigateur/direct liés à cette invocation, deadline convenue et tuple
   `home_identity=(dev,ino,uid)` à l'observation réelle du parent. À chaque phase,
   revalider compte, descripteur/chemins et bail/créneau encore actifs, plus les
   identités de provenance déjà hachées. Le compagnon demande un verdict Windows
   sur le pipe de cette invocation ; le contrôleur relit réellement Lxss, SID,
   BasePath/DefaultUid, identités/ACL et bail, et vérifie son handle encore lié au
   processus attendu. Le callback ne rend True qu'après les deux volets validés.
   Requêtes/réponses fermées et cadrées, au plus4KiB par message, corrélées aux IDs
   invocation/navigateur/direct, à la phase, au HOME et à la même deadline. Rejet
   des messages étrangers, répétés, hors ordre, tronqués, trop grands, inattendus,
   absents ou tardifs : aucun défaut True ni réponse AUTHORIZE réutilisée à WRITE.
   Aucun dump de message refusé ni fallback de stderr pour déboguer le protocole.
   Le stdin du compagnon est réservé à ce contrôle, jamais au scénario ou au LU
   humain. Ni sorties libres de main/descendants ni stdout/stderr natifs ne sont
   relayés sur les pipes ; seulement les métadonnées fermées utiles et états.
4. **Fraîcheur :** une observation Windows par callback, non le résultat AUTHORIZE
   réutilisé à WRITE ; au plus30secondes entre début des contrôles et retour True,
   sinon veto. Durées monotones mesurées localement, y compris l'aller-retour du
   compagnon ; ne pas comparer des timestamps monotones Windows/Linux. Pas d'attente
   humaine ni autre étape longue entre WRITE et écriture.
   Le lien ext4/provenance peut rester attaché à l'invocation et au HOME ouvert
   tant que descripteurs, device, UID et identités des fichiers restent identiques ;
   bail, ACL Windows, mapping GUID et disponibilité de lecture sont recontrôlés.
   Les gardes du module restent actives avant création/retrait. Ces contrôles
   bornent une course, sans transaction atomique Windows/Linux : administrateur,
   processus hostile du même UID ou substitution hors menace ne sont pas exclus
   par un GUID/tuple. Changement/indétermination constaté => abandon, pas adoption.
5. **Lecture réelle :** rendez-vous déjà convenu avant capture, terminal local
   privé disponible pour l'utilisateur. Sans état READY après main : cleanup,
   aucune lecture autorisée ni relance. Sinon il consulte lui-même le fichier dans son
   terminal privé et confirme uniquement `LU <run_id>` dans le terminal de contrôle
   Windows local, distinct du stdin du compagnon, sans couple, chemin natif, capture
   d'écran ou contenu dans chat/MCP. Le contrôleur transmet seulement l'acquittement
   fermé lié au run courant, jamais le texte libre saisi ou un acquittement de façade.
   L'absence de ce canal interactif prêt interdit la collecte, pas sa préparation.
   Le parent ne lit pas le JSON ; un acquittement d'une autre session est refusé.
6. **Retrait dans le même contexte :** après cet acquittement réel seulement,
   le compagnon appelle `retire(read_confirmed=True)` ; sans acquittement, son
   échéance UTC/monotone conduit à `retire(read_confirmed=False)`, même si le
   contrôleur attend encore le lecteur. Ne pas fabriquer True pour
   forcer le retrait. Vérifier retour True/état REMOVED : le module vérifie
   l'absence par stat/identité, sans ouvrir le site. Perte de bail, interruption
   ou contrôle postérieur refusé, pipe fermé/contrôleur perdu : sortir du contexte pour son cleanup ; échec
   = RETENTION_UNPROVEN, pas suppression aveugle. Garder le parent surveillant
   jusqu'au retrait sous60min, sans watchdog garanti après crash. Ensuite joindre
   ses enfants, attester le cleanup Linux et joindre l'unique processus WSL par
   son handle Windows : sa fin seule ne prouve pas le cleanup Linux. Puis libérer
   le bail et arrêter uniquement AIForAll devenu inutile, sans autre consommateur.

Composition choisie, pas un fallback : contrôleur `local-private-parent.ps1`,
compagnon `local-private-parent.py` et `tests/test_local_private_parent.py` proposés
sous `ops/tests/k6/`, paramètres d'installation/bail privés hors Git. Détails locaux
à l'auteur, sans nouvelle boucle de plan ; consommateur cbab/Child/scénarios/CI
inchangés. Aucun RPC réseau, service, daemon, harness ou collecteur nouveau.
Après intégration et revue applicables, préflight des seuls observateurs possible
sous bail distinct, sans PrivateSiteSession/main, k6/Chromium ou création de site ; il ne
devient pas une admission de collecte ni une preuve fraîche mise en cache pour elle.
Avant opération : contrôler refus sur état Windows/HOME/bail modifié, réponse
absente/lente, IDs/deadline différents, acquittement étranger/absent et retrait
impossible, sans lire de site. Preuves conservées : phases, IDs, fraîcheur et états
de retrait seulement. Sources : API `PrivateSiteSession.authorize/record/retire`,
[commandes WSL officielles](https://raw.githubusercontent.com/MicrosoftDocs/WSL/main/WSL/basic-commands.md),
gardes et preuves parent du guide/ledger ; aucune procédure ni collecte exécutée.

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
