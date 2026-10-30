# Composer le parent Windows et son compagnon Linux — sans capture

## Mandat et entrées

Continuer la session auteur13 et son arbre isolé, sans redécouverte globale.
Le plan reçu est persisté dans `20261030-k6-real-parent-author-plan.md` ; le choix
du lead retient l'alternative pipes déjà écrite par l'architecte. Lire le contrat
causal CANONIQUE du principal, amendement pipes **Accepted avant code**, delta
architecte `result_mvv0mhui_0002_p982l8` récupéré byte-identique et ratifié. ADR0504/V1
Accepted/Partial ; consommateur `cbab06e` gelé, S-1 fermé, 165 tests Linux sans skip.
Les propositions publiques restent Proposed ; aucune capture réelle existante.

Ce travail Windows/Linux est transversal. Le retour `result_mvv0pt4r_0002_ww2z41`
établit que le profil natif de l'auteur interdit les outils d'équipe : il ne peut
pas appeler team_handoff. L'outil exposé au lead ne cible que sa propre session.
Ne pas contourner cette limite par une API brute ou modifier les permissions.

**Disposition du lead :** conserver auteur/session/arbre/modèle OpenAI approuvé,
profil natif implementer, et appliquer localement les obligations spécialisées
de `.cursor/agents/hard-implementer.md` (corps du profil, pas sa sélection de modèle
frontmatter). C'est une substitution explicite de routage, pas un handoff natif
réalisé ni un changement de modèle/politique. Lire ces obligations avant code :
cause/conception avant fix complexe, hypothèses recoupées aux tests, pas de patch
au hasard ; remonter une conception incompatible au lieu de la redéfinir ; leaf,
pas d'élargissement ou refactor opportuniste ; retour fichiers/choix/preuves/limites
et HANDOFF_MARKDOWN sans écrire le ledger partagé. Le contrat normal est conservé.

L'autorisation de code/tests reste acquise. Commencer directement ROUGE puis
implémentation selon ce brief, sans nouveau tour de plan, transition ou GO humain.
Les choix locaux réversibles sont délégués à l'auteur.

## Résultat et périmètre

Un contrôleur Windows et un compagnon Linux minces composent les APIs existantes.
Fichiers préférés : `ops/tests/k6/local-private-parent.ps1`,
`ops/tests/k6/local-private-parent.py`, `tests/test_local_private_parent.py`.
README peut expliquer le lancement privé et ses limites. Un petit fichier de test
supplémentaire est possible si nécessaire aux frontières natives, sans framework
ou bibliothèque générale. Paramètres d'installation, profil attendu, bail, chemins
de campagne et journaux restent privés hors Git, non secrets et bornés.

Ne modifier ni consommateur `lib/private_site.py`, classificateur/veto/runner,
Child, lock, scénario, workflow, autres services ou contrats. Avant édition vérifier
les 22 fichiers source contre le canon `cbab06e` et préserver les anciens deltas/
index b316. Ne pas appliquer le full diff de cet ancien HEAD. Pas de reset/rebase,
commit-tree, hook modifié ou répétition du commit long connu en échec.

## Composition retenue

1. Windows garde le `Process`/handle natif de son seul lancement WSL explicite par
   GUID et compte. Observer PID, création, image et SID par le handle retenu ;
   constater la fin et fermer les handles après jonction. Aucun auxiliaire interop,
   recherche par nom, `Kill`, TerminateProcess ou processus étranger adopté.
2. Contrôles réels Windows avant lancement et renouvelés à AUTHORIZE/WRITE :
   Lxss/GUID/nom/BasePath/DefaultUid/version, SID, identité des dossiers/VHDX sans
   reparse/substitution et ACL conformes, bail actuel tenu et créneau actif.
   Profil attendu = bornes de comparaison, jamais preuve ou autorité par JSON seul.
3. Le compagnon Linux non-root ouvre réellement HOME, lie dev/inode/UID/modes/
   chaîne au montage noyau ext4, vérifie provenance et refus nominal24 historiques
   conservés ainsi que les outils/identités. Composer les gardes existantes et
   observer réellement les faits manquants, pas de lambda True de fixture. Aucun
   nouveau browser ou reçu22 fictif pour fabriquer une preuve.
4. Pipes privés attachés au processus créé : frames fermées, UTF-8 strict,
   tailles (4 KiB par message)/fréquences/délais bornés et séquence exacte. Lier session, séquence,
   phase, IDs navigateur/direct, deadline et HOME ; initialisation distincte des
   callbacks AUTHORIZE puis WRITE. Nouvelle observation Windows à chaque callback,
   budget monotone de 30 s pour l'ensemble des contrôles jusqu'à True, pas seulement
   le parsing. Contrôler les deux extrémités et traiter EOF, retard, doublon, champ
   libre, donnée étrangère ou divergence comme refus. Aucun site, corps natif,
   exception libre ou chemin de relevé ne traverse le protocole. Pas de canal réseau,
   token déclaré, cache AUTHORIZE réutilisé ou fallback interop.
5. stdin/stdout du compagnon sont réservés au protocole, distincts de l'enfant
   Chromium et du terminal de contrôle Windows. Le résultat fermé de runner.main
   ne doit pas polluer les frames ; capturer/valider cette seule sortie sans jamais
   exposer ou lire les flux natifs gardés par Child. Drainer les deux pipes sans
   deadlock, buffers/frames bornés même sans LF ; ne pas utiliser ReadLine non borné.
6. Mode de préflight explicite et **non armant**, utilisable depuis l'agent sous
   bail : mêmes observateurs et liaison de processus, mais pas de session
   PrivateSiteSession, pas runner.main, k6/Chromium ou site écrit/lu. Il vérifie
   l'opérabilité des observateurs, pas un rendez-vous absent ou une qualification.
   Ce n'est pas un nouveau mode de qualification ou un sélectionneur libre d'hôte.
7. La voie armée exige de vrais terminaux locaux prêts et un créneau convenu.
   Environnement CI/Actions refusé avant scrub/processus. Entrée redirigée/MCP ne
   vaut jamais rendez-vous. Après main non nul garder le contexte : READY seul
   autorise la lecture humaine dans son terminal distinct. Windows n'affiche que
   l'ID opaque, l'échéance et des états fermés. LU exact reçu du terminal local de
   cette session est transmis comme acquittement, jamais fabriqué par l'agent.
8. Retrait dans le contexte vivant après LU réel ou échéance UTC/monotone sous
   60 minutes. Sur EOF/interruption/perte de bail, abandon et cleanup du contexte,
   sans faux acquittement. Vérifier retour True/REMOVED/absence via le module sans
   ouvrir le JSON, joindre les enfants et rendre les inventaires. Réponse finale
   ou process non joint = cleanup inconclusif ; aucun succès administratif déduit
   du seul exit WSL. Arrêt du seul runtime devenu inutile coordonné par le lead,
   jamais global, pas de kill forcé ni suppression étrangère.

## Oracles et preuve

Écrire les régressions discriminantes avant la composition lorsque possible.
Unittest existant, frontières OS simulées ; exercer aussi les fonctions PowerShell
réellement livrées sur métadonnées fictives, sans WSL et sans nouveau framework.
Les checks de texte du script ne suffisent pas seuls à prouver son comportement.

Cas : GUID/SID/ACL/VHDX/HOME/device/ext4/bail/provenance changés, identité poignée
ou process non joint, frame absente/tronquée/surdimensionnée sans LF/UTF8 invalide/
clé ou type libre/IDs ou deadline étrangers, WRITE sans AUTHORIZE et doublons,
contrôles >30s, EOF/interruption, LU étranger/absent, échéance monotone, retrait
impossible. Positifs connus : séquence complète simulée et retrait, préflight
n'appelle jamais session/main/k6/Chromium. Sentinelles de confidentialité sur
sorties et toutes erreurs ; zéro ouverture du site et non-zéro du diagnostic
préservé même si retrait réussi. Aucun mock HTTP ou nouveau serveur.

Commands autorisées auteur : tests purs Windows avec temp approuvé, syntaxe AST
Python/PowerShell et diff-check. Pas de runtime WSL, acquisition, Chrome/k6, Cargo,
CI, installation ou capture par l'auteur. Le lead fera relire le delta intégré
indépendamment puis vérifiera le préflight réel sous bail neuf, sans armement.
Deux tentatives substantielles sans progrès vérifiable : rendre l'hypothèse et la
preuve, pas répéter ou augmenter le périmètre.

## Disponibilité et documentation vérifiées par le lead

Hôte WindowsPowerShell **5.1.26100.9711**, CLR4.0.30319.42000, observé sans WSL.
`Process.SafeHandle` disponible, `ProcessStartInfo.ArgumentList` absent. Ne pas
dépendre implicitement de PowerShell7 ou installer une nouvelle version. Si
Arguments est utilisé, quoting exact testé sur argv fixes et paramètres fermés,
jamais une commande shell interpolée. Pas d'ExecutionPolicy Bypass ajouté.

Documentation officielle recoupée (HTTP200) :
- [SafeHandle](https://learn.microsoft.com/en-us/dotnet/api/system.diagnostics.process.safehandle?view=netframework-4.8.1)
  donne le handle natif ; retenir le handle, pas redécouvrir le processus par PID.
- [RedirectStandardOutput](https://learn.microsoft.com/en-us/dotnet/api/system.diagnostics.processstartinfo.redirectstandardoutput?view=netframework-4.8.1)
  décrit les deadlocks si le parent attend ou lit un seul flux bloquant.
- [GetProcessTimes](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getprocesstimes)
  fournit création/fin depuis le handle, accès limité requis.
- [Arguments](https://learn.microsoft.com/en-us/dotnet/api/system.diagnostics.processstartinfo.arguments?view=netframework-4.8.1)
  précise la sémantique des guillemets. Ce sont références API, pas preuve du code.

## Livraison

Rendre le delta ciblé réellement présent, choix locaux, commandes ROUGE/GREEN/
suite Windows avec totaux/skips, ressources (aucun runtime attendu) et limites.
Geler puis laisser le lead intégrer. Pas de source inconnue, site.json réel,
secret ou log brut dans le rapport. Aucun push, ledger partagé, agent enfant ou
qualification du conducteur avant les preuves réelles.
