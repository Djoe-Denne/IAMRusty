# Qualification fonctionnelle k6 Windows — raccord minimal proposé

- Statut : **Accepted** ; Réalité : **Partial**, adaptateur et21tests intégrés ; aucun k6/Chrome Windows réel lancé ici.
- Date : 2026-10-30 ; mission1 uniquement. Base principale915042f, OPS gelé
  `f279e713079159766188fcbe5078d37217ca530d` ; briefing
  `20261030-k6-windows-pivot-preparation.md` et rapport76.
- Canon lié : [ADR0504](../adr/0504-validation-fonctionnelle-linux-k6.md),
  [V1 Linux](k6-functional-linux.md), [sondes historiques](k6-chromium-linux-probes.md).
  Leur portée Linux Accepted/Partial n'est ni réécrite ni qualifiée par Windows.
- Amendement du 2026-10-31 : **Accepted**, ratifié par le lead ; voir
  « Amendement révision3 — simplification Windows » ci-dessous. Le statut Accepted
  ci-dessus et les clauses antérieures sont conservés comme historique de ratification.

## Admission et décision

Ratification du lead le2026-10-30 après approbation native utilisateur de la
révision2, hash `4d651a945ab4f6757f24117ed715c37cf82fc712bf5b22721d593240543cac64`.
Le corps architecte194lignes a été intégré et comparé byte-identique avant
ratification, SHA256 `32bb3c02006988db9d4c35a76750041027dc64716964b035d5d30a65dbd5732a`.
La suite est admise dans les limites ci-dessous, sans nouveauGO à chaquejalon.
Les formulations propositionnelles conservées décrivent la provenance du contrat,
pas un état nonapprouvé courant. Aucun résultat réelWindows n'en est déduit.

La demande utilisateur remplace la poursuite WSL : **qualification WSL INCONCLUSIVE**,
unshare1 réussi et ldd1/countnull conservés à leur portée. Aucun dernier ldd, nouvel
unshare ou travail de conducteur privé/Windows-ended n'est poursuivi. La seconde
mesure ldd ratifiée mais non consommée reste historique, pas un quota réutilisable.

Proposer un **petit adaptateur parent Windows**, mêmes scénarios JS et mêmes oracles,
pas un nouveau framework. ADR0504 prescrit Linux : cette proposition Windows est
une voie distincte, pas une conformité rétroactive ou une supersession silencieuse.
Le lead prépare la révision effective objectif/QA par config, puis **start utilisateur
avant tout code/test Windows nouveau**. Le présent document n'est ni ce start ni
une permission d'exécuter les commandes proposées. Modèle/QA proportionate inchangés.
Le lead référence/ratifie le contrat lors de cette admission ; pas de nouveau GO
à chaque étape ensuite. La mission Linux/VM GitHub Actions sera distincte et future.

## Amendement révision3 — simplification Windows

**Statut : Accepted ; Réalité : non qualifiée par cet amendement documentaire.**
Ratification du lead le 2026-10-31 après lecture et intégration byte-identique des
deux documents de l'architecte, candidat `result_mvw5ppw0_0001_m1d2jm`.
Empreintes avant cette ratification : contrat `c599205c80526164cdf0f2241692caaf226e6e885fcc91769d044d16e3c9b9ba`,
notice d'abandon `4398bc6986618765d7e9d16cfd8372abfb3b35743d267acffd38f4a8bf9f6cd6`.
La consommation de ce delta par l'auteur est admise. La révision3 est
approuvée, hash `82a5aefa6a4dcd7deb05121d85b35da21b567585b615661f568ff45d27a22bb6` ;
aucun nouveau GO utilisateur n'est requis. Sources de décision : briefing
`.cursor/review-briefings/20261031-k6-windows-simplification.md`, rapport94
`94-revision-simplifiee-approuvee-et-reprise.md` et décisions du ledger canonique
`.cursor/handoffs/20261029-k6-01-socle.md`. Cet amendement ne modifie pas ces sources.

Après ratification, seules les exigences suivantes remplacent les clauses historiques
correspondantes de ce contrat ; le reste demeure inchangé :

- **Permissions et intégrité** : utiliser les permissions Windows ordinaires, sans
  les modifier ni ajouter de contrôle ACL personnalisé. Retirer les validations
  ACL/propriétaires/ancêtres de « Surface minimale et invariants » et de la QA,
  ainsi que les verrous personnalisés des exécutables/DLL. Aucune obligation de
  protection persistante du code ne les remplace. Le manifeste
  `config/tools.windows.lock.json` reste un manifeste de versions, provenance et
  SHA256 d'archives/exécutables vérifiés avant usage, pas un verrou système.
- **Navigateur** : conserver les protections natives Chromium, sans `no-sandbox`
  ni contournement TLS. Remplacer intégralement l'exigence « Pour Chromium PASS »
  de la section parent : aucune sonde/attestation renderer, token, niveau
  d'intégrité ou AppContainer n'est requise ni ajoutée. Son absence ne bloque ni
  l'exécution ni un PASS fonctionnel ; ce PASS ne qualifie pas le sandbox.
  Le [raccord browser-version Accepted](k6-windows-browser-version.md) reste ratifié :
  `browser.version()` demeure obligatoire dans le même cas. Son renvoi historique
  au témoin sandbox ne réintroduit pas l'exigence ici remplacée ; retrait/cleanup
  et preuve de version restent exigés, sans nouvelle commande Chrome `--version`.
- **Processus et données** : conserver profil temporaire séparé, contexte neuf,
  timeout borné, suivi et nettoyage raisonnable des seuls processus effectivement
  créés et observés, sur succès, échec et interruption. Pas d'adoption par nom ou
  ancien PID, de processus étranger arrêté, d'élévation ou de kill forcé implicite.
  Ne pas supprimer les données d'un processus observé encore actif. Ce devoir de
  nettoyage ne devient ni conservation persistante du code ni nouvelle protection.
- **QA et ordre** : retirer les tests propres aux ACL, verrous et attestations
  supprimés ; conserver les contrôles ciblés de pins/version, résumé courant non
  vide, assertions/cardinalités, environnement/confidentialité, timeout et cleanup.
  Après correction minimale et contrôle court du delta, premier vrai cas Windows :
  `about:blank`, `6 * 7 === 42`, puis fermetures normales. Aucune nouvelle enquête,
  protection, infrastructure ou revue préalable contre des risques hypothétiques.
  Un échec réel se diagnostique avec logs privés bornés, code retour et processus
  observés ; aucune protection supplémentaire sans défaut reproductible ou exigence
  explicite. Les conditions restantes de PASS ne sont pas relâchées.

La proposition [code-custody](k6-windows-code-custody.md) est **abandonnée, non
ratifiée**, remplacée par ce delta ; elle n'est ni une dépendance ni un gel des runs.
Pas de nouvelle ADR : précision réversible du raccord existant, sans nouvelle
frontière/API. Windows ne qualifie ni WSL ni Linux ; aucune preuve réelle ou
exécution OPS/test/runtime/CI n'est produite par cet amendement.

## Surface minimale et invariants

- Réutiliser sans modification fonctionnelle `scenarios/smoke/iam-health.js`,
  `browser-prerequisite.js` et `lib/summary.js` sous `ops/tests/k6/`. Le délai750ms
  du cas navigateur n'est pas un témoin Windows parce que son commentaire cite /proc.
  Précision ratifiée le 2026-10-30 : le [raccord de version Windows](k6-windows-browser-version.md)
  autorise seulement le check `browser-version` dans ce même cas, obligatoire sous
  Windows. HTTP et summary.js restent inchangés ; aucun nouveau cas ou lancement.
- Entrée proposée `ops/tests/k6/run-windows.ps1`, pins Windows séparés sous config,
  tests ciblés utilisant le harness local existant. Ne pas importer run-linux.py/
  ChildPOSIX comme preuve Windows, copier le conducteur privé ou créer une nouvelle
  autorité de cible. Factoriser seulement des validations réellement neutres et
  vérifiées, pas un port global du harness. Aucun changement OPS dans cette préparation.
- Compte Windows ordinaire, OS/build et architecture observés, pas d'élévation.
  Répertoire privé du run hors Git, ACL/ancêtres/identités/reparse vérifiés avec les
  garanties Windows ; aucun chmod POSIX simulé, confiance NetworkService ajoutée ou
  mutation de I:. Profil navigateur neuf, jamais profil utilisateur existant.
- Aucune modification Rust/service/identité/OAuth, POST, seed, charge ou migration.
  Aucun Docker/Kind, VM, déployeur/tunnel, CI dispatch ou nouveau workflow.
  La révision Windows doit inclure la suspension bornée du workflow Linux existant
  avant publication de sources Windows, pas une nouvelle campagne GitHub Actions.

## Outils Windows, avant toute exécution

Cible proposée Windows x64 natif, pas WSL, ELF Linux ou émulation supposée.
Inventaire en lecture fourni par le lead : `C:/Program Files/k6/k6.exe` existe,
VersionInfo vide donc version inconnue ; Python3.13.1 annoncé ; Chrome installé
`C:/Program Files/Google/Chrome/Application/chrome.exe` annonce149.0.7827.53,
**pas le pin156 et non adopté**. Absence dans PATH ne prouve pas absence du fichier.
Ces métadonnées ne remplacent ni SHA/provenance ni versions réellement éprouvées.
Conserver k6v2.3.0 et CfT156.0.8078.73 si leurs octets Windows sont vérifiables :
- ZIP portable officiel `k6-v2.3.0-windows-amd64.zip`, plutôt qu'installation MSI.
  SHA256 d'archive recoupé entre API release et checksums officiels :
  `112276d495e5741c968e2bc09ea6196099c1275bd6db9ee0875d173c7148ce43`.
- CfT officiel win64 annoncé pour156.0.8078.73, révision1706539 :
  `https://storage.googleapis.com/chrome-for-testing-public/156.0.8078.73/win64/chrome-win64.zip`.
  Le catalogue annonce version/URL, **pas une empreinte Windows acquise ici**.
- Établir avant lancement un lock Windows revu : provenance/URL/version/architecture,
  SHA256 archive et chaque exécutable réellement utilisé. Pour CfT, acquisition
  explicite revue et empreintes figées, sans les présenter comme checksums éditeur
  si elles ne le sont pas. Aucun hash Linux transposé ou hash local d'inventaire
  considéré seul comme preuve indépendante de provenance.
- Acquisition/extraction explicites séparées du runner, pas latest, téléchargement
  au run ou recherche automatique d'un Chrome installé différent. Vérifier octets,
  PE/architecture, chemins/identités, puis seulement après admission les versions
  annoncées par ces chemins exacts. Métadonnées de fichier seules ne qualifient pas
  le binaire. Conformité des assets locaux, SHA CfT/exécutables et versions réelles
  restent à établir ; aucun Chrome149 de remplacement pour obtenir un succès.

## Parent Windows, sandbox et nettoyage

Le parent garde l'environnement allowlisté, sorties bornées et handles de ses
processus créés, PID/création et faits observés contemporains. SID/image observés
pendant la vie ne deviennent pas des relectures après fin. Attente/fin par handle,
protocole/résumé et nettoyage des descendants sont des preuves distinctes.

Appartenance des descendants k6/Chromium à établir sans adoption par nom/PID seul.
Préférer un Job Object Windows propre affecté avant exécution, sans échappement
admis, si sa compatibilité avec les jobs du sandbox Chromium est prouvée. Pas de
KILL_ON_JOB_CLOSE, TerminateJobObject/taskkill ou kill forcé implicite. Ne pas copier
un superviseur POSIX ou ignorer une impossibilité d'affectation/nesting. Arrêt normal
du scénario puis arrêt gracieux borné des seuls processus possédés, jonction et
inventaires pré/post sur succès, timeout, refus et interruption ; échec => cleanup
non prouvé/non réussi, pas vert ni intervention sur un Chrome utilisateur étranger.
Ces primitives et leur compatibilité restent à vérifier, pas livrées par ce contrat.

`K6_BROWSER_EXECUTABLE_PATH` désigne exactement le chrome.exe vérifié ; headless,
contexte neuf, about:blank et évaluation42 seulement, fermetures finally conservées.
Env K6/proxy/loader/sorties contrôlé, reporting désactivé. Pas de no-sandbox, TLS
ignoré, debug/HAR/screenshot ou FATAL brut. Vérifier les arguments réellement
appliqués, pas seulement une variable vide ; pas de port CDP exposé ou logging ajouté
au transport interne normal de k6. Le code k6v2.3.0 relu n'ajoute pas no-sandbox dans
sa table de flags par défaut ; cela ne prouve pas les arguments d'un run futur.

Pour Chromium PASS, exiger en plus des assertions un témoin Windows effectif de
l'isolation du renderer possédé, revu contre la politique Chromium156 : restrictions
du token et intégrité/AppContainer selon la variante documentée, pas seulement
un nom de processus, Job Object de supervision ou absence de flag interdit.
Aucun /proc, userns ou seccomp Linux inventé. Si ce témoin est indisponible ou
insuffisant : Chromium INCONCLUSIVE même avec JS réussi ; conserver cleanup séparé.

## HTTP, résultats et confidentialité

Le JS actuel accepte seulement `http://127.0.0.1:18080`. Il faut une exposition
existante autorisée, ancrée contemporanément et joignable depuis Windows ; le
handler public ne prouve pas l'accès public derrière ext_authz. Pas d'URL arbitraire,
token/exemption, modification JS d'origine, nouveau serveur factice ou démarrage
de stack pour satisfaire le test. Sans cette cible : HTTP INCONCLUSIVE, zéro appel.
Autre origine => amendement explicite avant modification, pas redirection opportuniste.

Nominal : per-vu-iterations, VU1/itération1 ; un cas attendu/démarré/achevé et un
GET /iam/health, HTTP200 et corps exact OK, aucune redirection suivie. Browser est
une qualification distincte, sans requête applicative. Ses requests=0 projetés
signifient absence de k6/http dans le cas, pas mesure de tout trafic système.

PASS de phase exige exit k6 zéro, checks tous exécutés/réussis, cardinalités exactes
non nulles, résumé courant valide lié au run et cleanup attesté. Résumé absent,
périmé, tronqué, setup échoué, skip/interruption ou métrique absente => non positif.
Conserver le schéma JS ; enveloppe parent/provenance explicitement Windows, versions,
hashes et run/source corrélés, jamais résultat Linux/GHA ou globale réussite supposée.

Captures stdout/stderr privées bornées, projection fermée et allowlistée après
contrôle de confidentialité, aucune métrique brute, URL sensible, secret, cookie,
SID/chemin privé ou exception native divulguée. Sentinelles sur positifs/négatifs ;
violation interdit publication et PASS. Retrait des profils/captures/copies de run
et absence des processus possédés à prouver, sans suppression de données étrangères.

## QA proposée et archives WSL

Après approbation de la révision seulement : tests ciblés Windows des pins/ACL/
identités, environnement/argv, résumés non vacants, confidentialité et cycles
normal/non-zéro/timeout/interruption/descendant/handle indisponible. Puis versions
réelles, deux k6 inspect et cas navigateur. HTTP/selftests réels seulement si cible
admissible ; faux check, entrée manquante, cible refusée avant GET, cas absent et
absence de résumé doivent rester non réussis. Pas de nouveau serveur mock ni suite
Linux redondante ; le lead fixe les commandes Windows exactes dans la révision QA.

### Suspension Linux avant publication Windows

Le workflow `.github/workflows/k6-functional-smoke.yml` est déclenché par push/PR
touchant `ops/tests/k6/**` ou son propre fichier. Les jobs `prerequisites` et
`ubuntu2204-comparison` lancent outils/navigateur/diagnostics Linux : ne pas publier
les sources Windows en laissant cette voie active. Inclure dans la révision QA,
après admission et avant cette publication, une condition de job littérale
`if: ${{ false }}` sur **chacun des deux jobs**, commentaire daté et nom de workflow
préfixé SUSPENDED. Préserver IDs/noms des checks, historique, triggers, actions par
SHA et permissions ; ne pas changer les protections de branche ou ajouter dispatch,
variable de réactivation, nouveau job vert ou continue-on-error. Toutes les étapes
y compris diagnostics/publishers sont alors hors exécution, pas seulement le smoke.

Déclarer **CI Linux SUSPENDED / NOT_EXECUTED**, qualification INCONCLUSIVE. GitHub
documente qu'un job skipped peut reporter Success et satisfaire un required check :
ce comportement natif n'est pas un PASS de qualification, ni une preuve Linux/Windows.
Ne pas remplacer le workflow par un path-filter silencieux laissant des checks
requis pendants. Sa réactivation relèvera de la future mission Linux explicite.
La suspension n'arrête pas des runs anciens déjà en file/en cours : leur disposition
reste distincte, sans annulation étrangère implicite. Aucun YAML/CI modifié ici.

### Placement des archives

Archivage minimal proposé `docs/archives/k6-wsl-20261030/` : README de disposition,
manifest des sources/hashes/origines et références aux rapports crédibles ; seulement
sources publiques utiles du script ponctuel, parent et fixtures, en `.source.txt`
(contenu original conservé, nom original dans manifest), hors découverte tests/CI.
Aucune entrée de lancement ni import depuis l'archive ; réactivation jamais implicite.
Ce chemin docs ne correspond pas aux filtres actuels de ce workflow k6, contrairement
à une archive placée sous ops/tests/k6 ; aucune garantie sur d'autres workflows non étudiés.
L'extension ne constitue pas une barrière de sécurité à une exécution manuelle.
Zéro logs/captures/config/profils/baux/cache/binaires ou données privées archivés.
Ne pas déplacer/effacer silencieusement les canons ou l'histoire : dispositions des
propositions abandonnées et preuves à leur portée, WSL INCONCLUSIVE explicite.
AIForAll/VHDX/données/outils sur I: sont conservés. L'archivage et le cleanup réels
relèvent du lead, pas déclarés achevés par cette proposition.

Pas de nouvelle ADR à ce stade : raccord réversible dans les responsabilités
existantes, sans nouvelle API/service/autorité ; ADR0504 reste canon Linux. Aucun
Accepted, ledger, source OPS ou CI modifié ici ; aucune commande de qualification.

## Sources vérifiées, sans qualification implicite

- [Release k6v2.3.0](https://github.com/grafana/k6/releases/tag/v2.3.0) et
  [checksums](https://github.com/grafana/k6/releases/download/v2.3.0/k6-v2.3.0-checksums.txt).
- [k6v2.3.0 browser_type.go](https://github.com/grafana/k6/blob/v2.3.0/internal/js/modules/k6/browser/chromium/browser_type.go) :
  recherche Windows et flags ; chemin explicite préféré à cette découverte automatique.
- [CfT156.0.8078.73](https://googlechromelabs.github.io/chrome-for-testing/156.0.8078.73.json),
  [sandbox Chromium156 Windows](https://github.com/chromium/chromium/blob/156.0.8078.73/docs/design/sandbox.md),
  [Job Objects Microsoft](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects).
- [Conditions de jobs GitHub et skipped/Success](https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/control-jobs-with-conditions) ;
  triggers et deux jobs du workflow local relus, sans déclenchement.
- Sources locales JS/summary relues ; le commentaire /proc, ChildPOSIX et les preuves
  WSL ne constituent pas une validation du futur adaptateur ou sandbox Windows.
