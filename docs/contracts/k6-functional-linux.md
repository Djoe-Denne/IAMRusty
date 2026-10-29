# Contrat V1 — scénarios fonctionnels k6 sous Linux

2026-10-29 — **Accepted / Unimplemented pour la V1 mission1**, décision structurelle
architecte ratifiée par le lead sous délégation utilisateur. Lié à
[0504](../adr/0504-validation-fonctionnelle-linux-k6.md). Ratification avant code ;
aucune qualification ou permission runtime n'est déduite de ce document.

## Admission de la révision V1 mission1

Révision architecte du 2026-10-29, base
`989614096110b9813ce67c47a23fad8457c2025a`, session
`ses_e76a83e39ffevzY3AXlh4V9l72`. Le lead a relu les documents complets et vérifié
leur récupération octet pour octet dans le checkout principal. Il **ratifie la
V1 mission1 le 2026-10-29** sous délégation long run de la policy révision1
approuvée, puis consigne le HEAD intégré dans
`.cursor/handoffs/20261029-k6-01-socle.md` avant de débloquer l'auteur outillage.
**Accepted / Unimplemented pour cette V1**, sans admission des missions2/3.
Aucune promotion de Réalité ne découle de l'accord sur la cible ; le blocage de
raccordement Linux ci-dessous reste à résoudre sans contourner les gardes.

Plan d'implémentation après cette admission : (1) un auteur pour lanceur, modules,
tests locaux et workflow couplés ; (2) intégration et revue ciblée des gardes,
sorties et permissions ; (3) qualification Linux des outils/Chromium ; (4) HTTP
et selftest seulement après preuve de cible contemporaine du parent ;
(5) inventaires/cleanup et verdicts séparés. Une étape empêchée ne bloque pas les
lectures/corrections indépendantes, mais ne peut être remplacée par un PASS.

Mission1 ne livre que le socle, le smoke IAM et le prérequis Chromium. Les parties
OAuth/authz ci-dessous bornent les missions futures, **ne les lancent pas**.

## Structure V1 et responsabilités

Placement V1 : `ops/tests/k6/` avec `README.md`, `run-linux.py`, `config/`, `lib/`,
`tests/`, `scenarios/smoke/iam-health.js` et `browser-prerequisite.js` dans ce même
dossier smoke ; puis, dans d'autres missions, `scenarios/auth/` et `scenarios/authz/`.
Créer seulement les dossiers/fichiers utiles à la mission courante. Données de
campagne privées hors Git ; exemples de configuration sans credentials.

Parent ops → k6 : configuration validée, identité de cible observée, identifiant
de run, paramètres de phase et références privées aux credentials. Ce manifeste
ne prouve pas à lui seul la propriété de la cible.
k6 → parent : assertions identifiées, compteurs et résultat de phase non secrets.
Parent seul : préflight, préparation de données possédées, pause/restart ciblés,
stimulus de duplication/replay, certificat mauvais nom puis restauration, collecte
des observations et cleanup. Réutiliser les mécanismes existants ; une entrée
Linux mince est retenue, pas un second framework ni une API admin. Ces stimuli
et credentials sont **hors mission1** : son enfant k6 ne reçoit que l'origine
publique validée, un identifiant opaque de run et les chemins de sortie privés
strictement nécessaires, jamais le bail/kubeconfig ou un secret applicatif.

Le lanceur utilise Python3 et sa bibliothèque standard, importe l'autorité
`ops/deploy/local_full_target.py` sans la recopier ; PyYAML reste son prérequis
existant à fournir explicitement, sans installation implicite au lancement.
Aucun bundler, npm, extension xk6, framework de tests ou nouvelle dépendance
structurante. Les imports JavaScript sont locaux ou intégrés à k6.

## Exécution et réseau

Runner Linux hors cluster en V1 ; endpoints de services Linux effectivement
déployés et cible ancrée avant appels. Pas de compilation/tests Rust via Docker.
Pour la parité OAuth **future** : navigateur `http://127.0.0.1:18080`, issuer
`http://127.0.0.1:8080/iam`, callbacks18080 exacts et IdP sur origine enregistrée.
Ces loopbacks doivent désigner les bons endpoints depuis runner ET Chromium.
Sinon STOP de préparation ; pas de changement implicite des identités/allowlists.
Pas de port public nouveau ni d'accès aux contextes protégés. Kind n'est pas
l'environnement des IT Rust ; les E2E consomment un laboratoire déployé séparément.

### Voie locale mission1 et autorité de cible

Voie locale étudiée en V1 : WSL/Linux natif non-root, hors cluster, vers
`http://127.0.0.1:18080/iam/health` sur local-full. Cette origine reste fermée,
mais **n'est pas activable par défaut comme cible publique** au vu du constat
statique ci-dessous. Le parent doit démontrer son identité et sa conformité
publique depuis Linux ; aucune assimilation au loopback Windows, au pod ou au
conteneur et aucune URL libre ne remplace ces preuves.
Pas de modification issuer/callback, proxy, port-forward, tunnel, port public,
NAT ou service de substitution par le lanceur. Docker non privilégié n'est qu'une
alternative à qualifier sous bail du parent, pas un repli automatique ; son
loopback ne satisfait pas ce contrat par défaut.

Avant chaque enfant HTTP, puis après sa fin, le parent/lanceur appelle réellement
`LocalClusterTargetAnchor.check()` sur le bail courant fourni par le parent,
avec `expected_cluster="aiforall-local-full"`, sans injecter `root` ou `privacy`,
sans faux runner ni retour mis en cache. Cela conserve les vérifications :

- `approved_root()` POSIX = `$HOME/.cache/opencode` ; kubeconfig canonique exact
  `<approved_root>/cluster-anchor-<task_id>/kubeconfig.yaml`, sans lien admis par
  la garde. Utiliser le filesystem Linux privé, pas une copie DrvFS du bail Windows.
- `private_file()` : fichier et parent appartenant à l'UID effectif, aucun droit
  groupe/autres, fichier non inscriptible ; `secure()` fournit respectivement
  `0400` et `0700`. Le JSON de bail et la configuration de campagne restent eux
  aussi privés ; pas de chmod/retarget d'une ressource étrangère pour les accepter.
- Bailv2 ancré, `task_id`, delta contemporain créé par cette tâche, roster Docker
  exact d'IDs complets et métadonnées/topologie attendues. Un bail historique,
  même valide syntaxiquement, n'établit pas le bail actif de cette mission.
- Publication API du control-plane en loopback littéral/port exact, hash SHA256
  du kubeconfig, un seul contexte/cluster/user, certificats inline, aucune auth
  externe/TLS dégradé/override ambiant. `KUBECONFIG` et `KUBERNETES_MASTER` ne
  peuvent détourner l'autorité ; environnement explicitement contrôlé.
- `_uid_check()` vérifie localement **avant et après** le GET metadata de
  `kube-system`, compare son UID au précédent et n'actualise jamais une ancre
  devenue périmée. Refus local = aucune requête applicative et aucun GET UID si
  le refus précède ce GET ; UID périmé = au plus le GET nécessaire, sans adoption.

L'ancre du cluster **ne lie pas à elle seule l'origine HTTP**. Le parent relie
en plus, par observations actuelles bornées via les gardes existantes, le mapping
`127.0.0.1:18080 → control-plane:30080`, le Service Envoy et ses endpoints/UID,
le routage IAM et le workload Linux IAM (namespace/UID, propriétaire, image et
endpoint). Les déclarations de manifests et preuves anciennes sont des pistes,
pas une observation live. Ne pas lancer une capture réseau/admin ou ajouter une
API pour cette vérification. Pas de pré-sonde HTTP ajoutée au cas nominal : les
lectures d'identité du parent sont distinctes de son unique GET applicatif.
Les contrôles pré/post bornent une course, ils ne rendent pas HTTP atomique par UID.

**Incompatibilité publique du trajet étudié — addendum architecte ratifié :**
la route IAM publique ne rend pas automatiquement le trajet Envoy public.
`ops/deploy/apps/overlays/kind-mesh/envoy-mesh.yaml:167–170` conserve le préfixe
`/iam/` sans exemption health ; le filtre ext_authz est fail-closed
(lignes201–205). `workers/ext-authz/src/check.rs:82–84` refuse l'absence de Bearer
et `deny:128–130` retourne403. Le test source
`denies_envoy_client_path_without_bearer` corrobore le refus, sans avoir été
exécuté pour cette mission. `render-local-full.py:139–152` adapte le transport
downstream et filtre les callbacks, sans ajouter d'exemption health.

Ces faits statiques sont inchangés entre la base architecte `9896140` et le contrat
intégré `c9b537c` ; **aucun 403 live n'a été mesuré**. Le lead ratifie le 2026-10-29 ce
seul delta utile de la proposition architecte
`ses_e7697c371ffe5Dl4RX4kPhuJDt`, sans réintroduire ses anciens statuts Proposed.
Ne pas ajouter token, exemption, route, tunnel, port-forward ou oracle403 à la
place du 200/OK pour fermer mission1. Aucun appel HTTP n'est activé par défaut
sur 18080. Une exposition publique existante autorisée doit être identifiée,
observée et figée par amendement architecte ratifié avant activation ; pas de
paramètre d'URL arbitraire. D'ici là, le lanceur refuse HTTP avec verdict
INCONCLUSIVE/non-zéro ; l'outillage, Chromium et les tests locaux restent séparés.

**Blocage identifié, pas levé par la préparation :** une branche POSIX existe,
mais aucun bail Linux contemporain, endpoint joignable depuis ce runner ni
qualification opérationnelle de ces gardes n'est fourni ici. Les trois tests
discriminants cités plus bas injectent une racine et neutralisent `privacy`.
Copier un bail/kubeconfig Windows puis recalculer chemin/hash, réutiliser le bail
de clôture0412, ou accepter seulement un manifeste serait une nouvelle adoption.
Le lanceur ne crée ni bail ni cluster. La reprise HTTP exige que le parent
établisse une cible existante et un bail conforme **avec l'autorité runtime
existante**, puis qualifie ce raccordement POSIX/réseau et le lie à cette mission.
Si cela exige un nouveau producteur/transfert de bail, déploiement, tunnel ou
élargissement des gardes, revenir à l'architecte et à l'arbitrage de périmètre ;
ne pas le coder sous couvert d'une entrée Linux mince. D'ici là : **HTTP
INCONCLUSIVE**, outils et Chromium restent qualifiables indépendamment.

### Versions, distribution et provenance avant exécution

Cible initiale outillage/navigateur : Ubuntu24.04 Linux x86_64/amd64, utilisateur
non-root ; CI hébergée `runs-on: ubuntu-24.04`. Enregistrer `/etc/os-release`,
architecture, version du noyau/WSL et image du runner réellement observées dans
une projection non secrète. Ce label CI versionne la distribution, pas une image
immuable ; conserver sa version d'image effective. Autre distribution/architecture
= qualification distincte, jamais réutilisation tacite du pin amd64.

Un lock public sous `ops/tests/k6/config/` doit figer, avant tout lancement de
binaire, nom/version exacte, plateforme, URL officielle exacte d'acquisition,
source de l'empreinte attendue et SHA256 des archives/paquets puis des exécutables
réellement utilisés. Acquisition/provisionnement explicites séparés de `run`,
`browser`, `selftest` et `validate` ; aucun téléchargement à leur initiative.
Comparer les octets au pin revu **avant même `--version`**, puis comparer la
version annoncée par le chemin canonique de ce binaire au lock. Échec ou pin
absent = non-exécution, pas fallback vers le PATH ou navigateur de l'hôte.

- k6 **v2.3.0**, archive officielle `k6-v2.3.0-linux-amd64.tar.gz` de
  `https://github.com/grafana/k6/releases/download/v2.3.0/`.
  SHA256 attendu de l'archive :
  `39c3117b6af817592dcd0ce4242105c0a7af10948c2a425306f0be8f7a8a8ab1`.
  Source : `k6-v2.3.0-checksums.txt`, SHA256
  `8ebbfcd5a13d68ac67d488c2b5f28bf901ea22d2fbbfb15f5345d65c39163be2`,
  recoupé avec le digest d'asset de l'API officielle de la release le2026-10-29.
  Le texte de checksums a été lu/haché ; **l'archive/binaire n'a pas été acquis
  ni exécuté par cette révision**. Après extraction sûre dans un répertoire privé
  neuf, relever le SHA256 du binaire et `k6 version` au chemin contrôlé.
- Chromium : relever une **version complète**, build/distribution et architecture
  depuis une distribution officielle traçable ; figer URL/paquet exact et SHA256
  attendu dans le même lock. Pour un paquet signé de distribution, vérifier les
  métadonnées signées et le SHA256 du paquet, puis celui de l'exécutable réel
  (pas seulement celui d'un wrapper). Pour une archive officielle sans SHA256
  publié, acquérir sans exécuter, documenter son canal d'authenticité et soumettre
  son empreinte à revue dans le lock **avant** qualification ; le hash recalculé
  sur chaque téléchargement ne devient pas automatiquement le pin de confiance.
  Refuser provenance non vérifiable ou pin non revu. Revalider l'exécutable et ses
  éventuels wrappers avant usage via `K6_BROWSER_EXECUTABLE_PATH`, puis `--version`.
  Aucun numéro Chromium n'est fixé/proclamé testé ici ; c'est un prérequis restant
  du lot outillage, pas une permission de prendre `latest`, Chrome149 ou le binaire
  préinstallé d'un runner sans vérifier sa provenance.

Pour les références de **qualification**, pas de tag `latest`/`master`, import
distant flottant, mise à jour automatique ou installation implicite.
L'autorisation utilisateur2026-10-29 de pull d'une image Grafana avec navigateur
(exemple `master-with-browser`) autorise cette acquisition distincte, pas sa
qualification : figer une image compatible k6v2.3.0 par digest, vérifier le vrai
Chromium et conserver la sandbox avant toute utilisation probante. Le pull ne
prouve ni lancement, ni propriété des ressources acquises par l'utilisateur ;
préflight, baux et registre restent au parent. Aucun pull n'est effectué ici.
Pas de dépendance Node supposée (`Buffer`, filesystem/
process, modules npm Playwright). Une incompatibilité k6/Chromium reste un échec
de prérequis ; ne pas changer silencieusement de version ou désactiver la sandbox.

### Interface parent → lanceur → k6

`K6_CONFIG` désigne explicitement un fichier privé POSIX, hors Git, sans valeurs
implicites ni substitution shell. Son schéma V1 fermé décrit : version1, profil
`local-full`, cluster/contexte exacts ci-dessus, chemin absolu canonique du bail
courant, identifiant de cible opaque, origine HTTP fixe ci-dessus et références
privées aux observations contemporaines du parent. Champs inconnus, doublons JSON,
userinfo/query/fragment, autre origine/route, credentials ou commande à exécuter
= refus avant enfant et requête. Ce schéma décrit une cible : **il ne l'autorise
pas**. Le lock d'outillage est distinct de ce fichier et ne contient aucun secret.

L'origine est exactement `http://127.0.0.1:18080` **sans chemin** ; la route
`/iam/health` est ajoutée par le lanceur, jamais choisie par l'entrée de campagne.

L'implémentation choisit des noms de champs privés simples, documentés dans le
README/exemple et testés ensemble ; pas de nouveau format de bail ou protocole
d'administration. Le parent corrèle `task_id`, bail actif, run et observations ;
un JSON auto-attesté n'est pas suffisant. `run-linux.py` fixe la route et la méthode,
ne reçoit ni URL libre en CLI, ni scénario/arguments arbitraires.

- `validate --config "$K6_CONFIG"` : syntaxe, schéma, chemins et cohérence locale
  seulement. **Zéro Docker/kubectl, runtime, requête HTTP ou installation** ; son
  exit0 signifie configuration admissible offline, pas cible qualifiée.
- `run --config "$K6_CONFIG" --output "$K6_RUN_DIR"` : gardes réelles, puis un
  enfant k6 HTTP ; capture privée bornée, validation du résumé, contrôles post-run
  et cleanup. Seul ce parcours autorise le GET nominal après admission live.
- `browser --output "$K6_BROWSER_RUN_DIR"` : qualification autonome du lock et du
  Chromium Linux ; ne lit pas de cible applicative ni bail de cluster.
- `selftest --config "$K6_CONFIG" --output "$K6_SELFTEST_RUN_DIR"` : enfants négatifs
  bornés ci-dessous, sous les mêmes gardes ; aucun service de mocks ajouté.

Chaque `--output` est un chemin privé absolu hors Git, neuf/exclusivement créé
pour le run, sans lien ni écrasement d'une sortie antérieure. Le parent fournit
ces chemins et contrôle propriétaire/droits ; pas de sortie choisie par une URL,
une exception ou un champ arbitraire du scénario. Identifiant opaque neuf, HEAD,
empreintes du contrat/config/scénarios/outils et heures bornent la corrélation ;
une date seule ne prouve pas la fraîcheur. `K6_CONFIG` et ces variables de chemins
ne doivent jamais être interprétés comme des commandes déjà exécutées.

### GitHub Actions : prérequis, pas laboratoire applicatif

Seuls `.github/workflows/k6-functional-smoke.yml` et `ops/tests/k6/` sont autorisés
pour cette CI. Ne pas modifier `.github/workflows/ci.yml`. Actions épinglées par
SHA complet, `permissions: contents: read`, checkout sans credentials persistés,
durée bornée et aucune permission write/OIDC. Pas de `pull_request_target`, secret
applicatif, kubeconfig, bail local, SDK publisher, déployeur ou tunnel. Les PR non
fiables peuvent tester le socle/Chromium isolés, jamais une cible applicative
protégée ; aucun code de PR non revu dans un job de confiance doté de cette cible.

La CI peut exécuter les tests locaux, versions, `k6 inspect` et `browser` sans
application. Nommer ses résultats **prérequis** ; leur vert ne signifie ni HTTP
exécuté ni mission1 qualifiée. Aucun « smoke réussi » obtenu avec un serveur
factice ou `if`/skip. HTTP/selftest réels requièrent une cible Linux existante,
autorisée, ancrée et joignable depuis **ce** runner. Aucun adaptateur de cible
distante n'est introduit par V1. Sans cible : verdict HTTP INCONCLUSIVE avec
cause/reprise explicites ; si le job HTTP est déclaré requis, exit non nul.
Aucun dispatch GitHub n'est lancé par cette préparation.

## Verdict fonctionnel, pas charge

Une VU, une itération explicite par scénario, exécution séquentielle quand les
données sont partagées. Pas de seuil de performance. Délais bornés servent à
éviter un run infini et à vérifier le contrat métier, pas à inventer un SLO.

Mission1 fixe `executor: 'per-vu-iterations'`, `vus:1`, `iterations:1`, `maxDuration`
et timeout HTTP bornés explicites. Pas de `--once`, `--vus`, `--iterations`, stages,
retry ou sélection de scénario qui remplaceraient cette déclaration. Un seul cas
nominal `iam-health`, un seul GET, statut200 et chaîne **exacte** `OK`, pas trim ni
simple socket ouverte. `redirects:0` : toute redirection échoue, sans second appel.
`/health` est une liveness ; ce contrat ne la renomme pas readiness.

Succès = processus k6 réussi ET tous les checks obligatoires réussis ET cardinalité
exacte des cas démarrés/achevés/attendus ET observations/restauration/cleanup
obligatoires présents. `checks: ['rate==1']` seul ne prouve pas la complétude.
Setup échoué, cas absent, skip, abort ou preuve manquante interdisent PASS global.
Les refus403 attendus sont des résultats métier ; ne pas imposer aveuglément
`http_req_failed==0` qui les classerait comme pannes. Adapter explicitement les
statuts attendus et vérifier leur refus discriminant avec témoins positifs.
### Résumé courant et verdicts séparés

`handleSummary()` projette un petit schéma versionné, pas `JSON.stringify(data)`
sur toutes les métriques. Il porte l'identifiant courant et les identifiants de
cas/checks attendus. Le parent vérifie existence, taille, JSON complet et strict,
absence de doublons, corrélation à ce run/scénario et types/valeurs finis. Au nominal,
attendus=démarrés=achevés=**1**, `iterations=1`, `http_reqs=1`, et chacun des deux
checks obligatoires statut/corps exécuté exactement une fois, réussi une fois,
échoué zéro fois. L'ensemble des checks/cas ne peut omettre un attendu ou en ajouter
silencieusement. Une métrique absente n'est **jamais** convertie en zéro. Le résumé
ne peut attester ses propres obligations de cleanup, réservées au parent.

Rapporter au minimum `HTTP`, `Chromium`, `cleanup` et confidentialité, avec causes
non secrètes et périmètres, sans calculer un vert par défaut :

- **PASS de phase** : exit k6 zéro, checks/cardinalités exacts, résumé courant
  valide, garde post-run et cleanup obligatoires valides, confidentialité vérifiée.
- **FAIL** : assertion, cardinalité, garde, résumé invalide/manquant ou contrôle
  de confidentialité/cleanup échoué ; le lancement testé sort non-zéro.
- **INCONCLUSIVE** : prérequis/cible/permission indisponible, preuve non exécutée ;
  un `run`/`browser` demandé mais non qualifiable sort aussi non-zéro. Un skip,
  setup échoué, interruption ou absence de résumé n'est jamais un succès.

HTTP PASS ne qualifie pas Chromium ; Chromium PASS ne qualifie pas HTTP. `cleanup`
distingue enfants du lanceur et ressources du parent : enfant terminé ne signifie
pas laboratoire fermé. Le parent ne déclare la mission qualifiée qu'avec les
preuves Linux obligatoires et tous les cleanups requis ; rétention sous bail actif
nommée comme telle, jamais nettoyage global supposé.

### Selftest discriminant

Le mode selftest teste le **lancement et son verdict**, pas seulement un parseur :
check volontairement faux contre le vrai endpoint autorisé ; entrée requise
manquante ; cible refusée avant toute requête ; cas attendu absent ; interruption
ou absence de résumé. Chaque enfant doit rendre verdict non réussi **et exit
non nul**, et respecter confidentialité/cleanup. Le refus de cible doit démontrer
zéro requête, pas interroger une URL étrangère pour constater qu'elle échoue.
Pas de modification service, mock HTTP, seed ou POST. Les mécanismes d'injection
du selftest ne sont pas des overrides du run nominal via l'environnement.

L'agrégateur peut sortir zéro seulement si **tous** les échecs attendus ont été
effectivement observés avec leurs oracles et cleanup. Son résultat est « selftest
du dispositif réussi », jamais « HTTP PASS ». Sans vraie cible, le faux check live
n'est pas exercé : selftest réel INCONCLUSIVE/non-zéro, même si ses tests locaux
de parsing ont réussi.

### Prérequis Chromium distinct

`browser-prerequisite.js` utilise `k6/browser`, un contexte neuf, `about:blank`
et une évaluation minimale déterministe, une VU/itération bornée ; fermeture
page/contexte en finally puis contrôle externe de tous les descendants possédés.
Pas de navigation applicative, UI, OAuth, screenshot/HAR, trace ou débogage CDP.
Le transport CDP interne nécessaire à k6 n'est pas une permission de publier un
port/debugger distant ou ses messages. Aucune option `no-sandbox`,
`disable-setuid-sandbox`, `ignore-certificate-errors`, `insecureSkipTLSVerify` ou
assouplissement global des user namespaces/AppArmor/seccomp. Vérifier utilisateur,
arguments effectivement lancés et activation effective de la sandbox ; le simple
succès d'`about:blank` ou l'absence d'un flag interdit ne suffit pas à la prouver.
Conserver une projection bornée des contrôles Linux des processus rendus possédés
(`/proc` : identité, filtre seccomp du renderer et isolation de namespaces ou
mécanisme sandbox équivalent documenté pour ce build), sans exporter sa ligne de
commande brute. Un wrapper qui injecte des flags doit être contrôlé lui aussi.
Sandbox non démontrable dans le runner = prérequis non qualifié, sans désactivation.

## Cookies, secrets et sorties

Contexte Chromium neuf et fermé en finally. Aucun transfert implicite de cookie
jar entre `k6/http` et `k6/browser`. Bearer seulement sur les routes autorisées,
jamais transmis à l'IdP ni aux callbacks publics. Corréler le même run ; ne pas
réutiliser des tokens historiques ou déclarer valide une seule structure JWT.

Pas de JWT, cookie, code/state/verifier, clé ou credentials en console, exceptions,
tags/métriques, rapports, traces, captures ou noms de fichiers. Les URL de callback
contiennent des secrets : désactiver/normaliser leurs tags et sorties avant capture.
Pas de `--http-debug`, debugCDP, HAR, captures écran ou métriques brutes par défaut.
Nommer les requêtes avec labels stables sans query. Vérifier par sentinelles que
stdout/stderr et chaque sortie choisie ne publient pas les secrets. Résultats et
manifestes d'artefacts allowlistés et bornés, avec versions/SHA/pin/cible ; absence
de fuite observée sur ce périmètre, jamais promesse d'un filtre universel.

Mission1 n'a besoin d'aucun credential. Le parent construit un environnement enfant
minimal, sans hériter des `K6_*`, proxy (`HTTP_PROXY`/`HTTPS_PROXY`/`ALL_PROXY` et
variantes), options/sorties Cloud, debug, config globale utilisateur ou contrôles
distants ambiants. Usage reporting explicitement désactivé ; options de scénario,
sorties et chemin du binaire fixés par le lanceur. Les options k6 d'environnement/
CLI priment sur le script : cette isolation est une condition du verdict, pas une
préférence. Ne pas exposer par défaut URL/headers/corps/exceptions, tags `url` ou
métriques brutes. Les diagnostics publics utilisent des codes/labels fermés.

Capturer stdout/stderr **privément** avec limite explicite (au plus64KiB chacun) ;
ne jamais les streamer bruts dans les logs CI puis essayer de les caviarder.
Dépassement = arrêt gracieux de l'enfant possédé et verdict non réussi, pas PASS
sur sortie tronquée. Publier uniquement la projection de résultat versionnée et
la provenance non secrète allowlistées (au plus32KiB par fichier,256KiB au total),
pas un glob du répertoire de run. Les sentinelles de confidentialité couvrent
stdout/stderr **et chaque artefact autorisé**, succès, refus de config/cible,
erreurs k6/Chromium et interruption. Échec = aucune publication de cet artefact
ni PASS ; rapport minimal non secret, quarantaine privée bornée si nécessaire.

## Cycle de vie

`setup`/`teardown` k6 ne constituent pas un cleanup garanti après échec ou interruption.
Le parent assure inventaire avant/après, baux, registre contemporain des ressources
créées, restauration et stop gracieux des seuls IDs possédés même sur échec.
Aucun prune/reset/kill forcé/adoption par nom ni nettoyage de données étrangères.
Les actions perturbatrices sont explicites, bornées et séparées du code k6.
Une pause opérateur/refus natif n'est jamais levé par le lanceur.

Le lanceur suit ses processus par identité de création (PID, UID, début de vie et
descendance/groupe dédié), pas par nom d'exécutable. `finally` et handlers de
signaux demandent l'arrêt gracieux, joignent puis vérifient l'inventaire ; un arrêt
impossible reste FAIL/INCONCLUSIVE avec identité privée à remettre au parent,
sans kill forcé ni toucher un processus préexistant. Les handles k6 seuls ne
prouvent pas la fin de Chromium. Interruption avant résumé : publier seulement
le diagnostic allowlisté non réussi après contrôle de confidentialité/cleanup.

Le parent tient les baux/ressources runtime contemporains et le registre Docker
à la création/redémarrage (IDs complets, statut de rétention), puis inventaires
avant/après sur succès/échec/interruption. Le lanceur ne crée pas de cluster et
ne stoppe pas des nœuds simplement parce que leurs noms figurent dans la config.
Si aucune ressource Docker n'est créée, le rapport le dit ; ne pas créer une ligne
de registre fictive. Démarrage/fin de runtime restent soumis aux skills/règles
lifecycle/evidence du dépôt et au parent, pas à `setup`/`teardown` JavaScript.

## Validation et état des preuves de cette révision

Comparaison statique auteur : `docs/adr/README.md` (0504 alors Proposed/Unimplemented), ordre
mission1 et `AGENTS.md` ; `.github/workflows/ci.yml` Linux existante inchangée.
Autorité et tests lus sur les sources du worktree de la base indiquée ; les
sources canoniques Serena sont identiques après normalisation CRLF/LF.

Tests examinés, **pas exécutés** :
`ops/tests/test_local_full_target.py::AnchorTests.test_valid_fixed_config_and_uid_before_write`
(UID GET avant mutation), `test_stale_uid_one_metadata_get_zero_mutation` (un GET,
aucune mutation, ancre inchangée), `test_kubeconfig_outside_approved_root_zero_commands`
(refus sans commande). Leur fixture injecte racine/privacy : compléter la
qualification POSIX réelle des droits, kubeconfig, roster Docker et UID avant
consommation live ; ne pas les présenter comme cette qualification.

Sources du cas nominal : `services/IAMRusty/http/src/public_routes.rs` autorise
`/iam/health` ; `rustycog-http/src/lib.rs::health_check` retourne `OK` au gitlink
`02e5db23aac6016aab062c3078696d5f7fd464ef` (objet Git vérifié, sans reprendre le WIP
du sous-module). [0405](../adr/0405-readiness-crate-partagee.md) distingue liveness
et `/ready`. L'exposition déployée reste à vérifier.

Après code intégré, contrôles ciblés prévus : `git diff --check` ;
`python3 -B -m unittest discover -s ops/tests/k6/tests -p 'test_*.py'` ; versions
des binaires vérifiés ; `k6 inspect` sur les deux scénarios ; les quatre commandes
`validate`, `run`, `browser`, `selftest` ci-dessus avec chemins privés explicites.
Tests locaux à couvrir : gardes avant requête, droits POSIX, preuve non auto-attestée,
résumé absent/tronqué/périmé/cas manquant, overrides ambiants, sentinelles et cleanup
des enfants sur succès/échec/interruption. Les fakes de commandes sont pour les
tests d'outillage, jamais une preuve HTTP/Chromium ni un nouveau serveur simulé.

La préparation architecte n'a exécuté aucun de ces parcours k6, aucun test de garde,
Docker/Kind/WSL/Chromium ou requête applicative. HTTP, Chromium et cleanup runtime
restent **INCONCLUSIVE/non exécutés** ; ressources runtime créées : aucune.
Le lead consigne les commandes réellement exécutées, résultats distincts,
limites et HEAD intégré dans son ledger/rapport avant levée des dépendances.

## Sources et choix retenus le 2026-10-29

Documentation officielle **v2.3.x** consultée ; pour les détails dépendants du
binaire, recoupement avec le tag **v2.3.0** et la release, pas seulement `latest` :

- [Executor per-VU](https://grafana.com/docs/k6/v2.3.x/using-k6/scenarios/executors/per-vu-iterations/index.md)
  et [thresholds](https://grafana.com/docs/k6/v2.3.x/using-k6/thresholds/index.md) :
  cardinalité explicite et checks obligatoires + exit non nul. Exemples à plusieurs
  VU et seuils de latence écartés ; pas de charge/SLO déduit de k6.
- [Priorité des options](https://grafana.com/docs/k6/v2.3.x/using-k6/k6-options/how-to/index.md)
  et [résumé personnalisé](https://grafana.com/docs/k6/v2.3.x/results-output/end-of-test/custom-summary/index.md) :
  environnement fermé et petite projection stricte retenus, export intégral des
  métriques ou stdout d'erreur non filtré écartés.
- [Options navigateur](https://grafana.com/docs/k6/v2.3.x/using-k6-browser/options/index.md)
  et [lancement Chromium v2.3.0](https://github.com/grafana/k6/blob/v2.3.0/internal/js/modules/k6/browser/chromium/browser_type.go) :
  contexte neuf et sandbox conservée ; exemples Docker `master-with-browser`,
  `no-sandbox`, TLS ignoré, proxies et traces de debug écartés. Le code natif de
  cette version n'impose pas `no-sandbox` ; ce n'est pas une preuve du runner.
- [Release v2.3.0](https://github.com/grafana/k6/releases/tag/v2.3.0),
  [métadonnées d'assets](https://api.github.com/repos/grafana/k6/releases/tags/v2.3.0)
  et [checksums officiels](https://github.com/grafana/k6/releases/download/v2.3.0/k6-v2.3.0-checksums.txt) :
  version et empreintes recoupées ci-dessus ; `--once` est utile en général mais
  non retenu ici afin de conserver l'executor/timing explicites exigés.
- [Blog GitHub Actions,2024-07-16](https://grafana.com/blog/performance-testing-with-grafana-k6-and-github-actions/) :
  démarrage petit retenu ; `ubuntu-latest`, actions par tags, version k6 par défaut
  flottante, Cloud/commentaires PR, permissions write, root et charge écartés.
- [Organisation de suite,2024-05-01](https://grafana.com/blog/organizing-your-grafana-k6-performance-testing-suite-best-practices-to-get-started/) :
  modules locaux simples et responsabilité unique retenus ; fallback automatique
  d'environnement et wrapper d'erreurs exposant `res.url`, données de requête ou
  tags arbitraires écartés. Pas de client générique/bundler/observabilité ajouté
  pour un seul GET. Conseils communautaires recoupés avec le contrat, pas adoptés
  comme autorité d'exécution.

Analogie du dépôt : [séparation ops runtime/validation](ops-kind-runtime-validation-separation.md),
notamment D5 et autorité unique par bail, sans promouvoir ce contrat Proposed ni
réécrire ses parcours historiques Windows en preuve Linux. Les preuves de
[clôture0412](20261029-auth-security-qualified-state.md), Chrome149 et17IT gardent
leur portée ; elles ne donnent ni nouveau bail ni pin/proof k6.

## Conservation des preuves et dettes

Mission2 construit la table de traçabilité ancien oracle → k6, parent ops ou
UT/IT conservée. Les17IT0412 et preuves principal/remote restent à leur niveau.
Aucune suppression des anciens pilotes avant parité démontrée. Ne pas réexécuter
toutes les suites par principe ; une équivalence de sources doit être documentée.

Mission3 vérifie propagation/reprise/duplication/replay de droits réels. Même sub
sous deux issuers reconnus, témoins légitimes pour chacun ; aucun fallback UUID,
ou grant historique ressuscité. Le délai de projection vient de son contrat :
**les60s de fraîcheur JWT ne sont pas un délai de propagation OpenFGA**.
Tout défaut produit reste ROUGE ; correction séparée et revue dans son contrat.

Qualification HSM réelle : dette reconnue/acceptée non bloquante pour ces trois
campagnes, à tracer avec déclencheur et critères de fermeture. ADR0309 demeure
Partial tant que son critère explicite n'a pas été amendé/ratifié ou satisfait.
Ne pas modifier un statut pour transformer la dette en preuve matérielle acquise.

### Dette reconnue : qualification d'un signataire matériel réel

Décision utilisateur2026-10-29 : différée et acceptée, non bloquante pour les
trois campagnes k6. Responsabilité : périmètre IAM/signing, arbitrage architecte
et propriétaire du déploiement lors du choix d'équipement. Déclencheur : sélection
du fournisseur matériel ou projet exigeant explicitement cette capacité.
Fermeture : vrai Sign/GetPublicKey, preuve de possession et binding versionné,
rotation/retrait de confiance et refus sans fallback, sur l'équipement retenu,
avec erreurs et cleanup vérifiés. Aucun vendor choisi, délai/coût inventé ou
capacité HSM annoncée disponible. Cette dette ne change pas le critère ni la
RéalitéPartial actuels de0309 ; un changement de critère nécessite son propre
amendement architecte puis ratification.
