# Contrat V1 — scénarios fonctionnels k6 sous Linux

2026-10-29 — **Accepted / Partial pour la V1 mission1**, décision structurelle
architecte ratifiée par le lead sous délégation utilisateur. Lié à
[0504](../adr/0504-validation-fonctionnelle-linux-k6.md). Ratification avant code ;
aucune qualification ou permission runtime n'est déduite de ce document.

## Admission de la révision V1 mission1

Photographie vérifiée au 2026-10-30 : source `783fa80`, publiée dans `6b58108`,
[run Linux 40982576736](https://github.com/Djoe-Denne/IAMRusty/actions/runs/40982576736).
129 tests natifs sans skip sur Ubuntu 24.04 et sur la comparaison diagnostique
22.04 ; archives, exécutables, versions et deux inspections vérifiés. Le navigateur
reste non qualifié : k6 refuse avant le cas, direct22 rapporte SIGABRT avant cleanup,
direct24 NoUsableSandbox avec refus de confidentialité. Cleanup des enfants PASS,
runtime parent INCONCLUSIVE ; HTTP et selftests live sans cible conforme autorisée.
La [proposition d'observation causale](k6-chromium-causal-observation.md) reste
**Proposed / Unimplemented** : ni divulgation de site, ni consommateur ou essai
d'ancre admis ; aucune entrée publique pertinente pour22 n'est connue.

Photographie historique antérieure : socle intégré jusqu'à `5d79557`, 50 tests
Linux/WSL réussis sans skip, correctif S-1 revu indépendamment. k6 v2.3.0 annoncé
par le binaire vérifié ; deux inspect réussis. Chromium `--version` échoue sur
WSL (bibliothèque `libnspr4.so` absente), donc outils composites et navigateur
non qualifiés. HTTP/selftest live toujours INCONCLUSIVE faute d'exposition
publique existante conforme. La ratification historique ci-dessous précédait
le code ; elle n'est pas une preuve globale d'exécution. Aucun paquet implicite,
déploiement, tunnel ou affaiblissement de sandbox n'a été ajouté.

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

### Addendum diagnostic parent Chromium — Accepted, 2026-10-30

**Proposition architecte ratifiée par le lead le 2026-10-30 avant code**, sous
délégation mission1 et instruction utilisateur de poursuivre par des essais
raisonnés/recherchés. Candidate `result_mvu5exmt_0002_s0lf6k`, session
`ses_e761c08c9fferiab1yLWGU5Ka6`, récupérée et vérifiée identique avant admission ;
la V1 et ADR0504 restent **Accepted / Partial**. Aucun nouveau harness de preuve,
nouvelle ADR ou qualification ne découle de cet addendum. Base documentaire
`f8df62f`, source outillage `08f6c03` ; rapport18 de la mission
`team_mvtz6p47_000q_4wmht0` et ledger canonique lus dans le checkout principal,
puis intégrés par le lead dans le commit documentaire `b637fd7`.

**Arbitrage : compatible avec mission1 comme diagnostic du parent**, distinct du
cas `k6/browser`. Il précise une interface diagnostique jusque-là non spécifiée,
pas un substitut à k6. Lancer directement le même Chromium épinglé peut conserver
une erreur native que le parseur k6 perd ; ce lancement n'est pas équivalent au
contexte/transport k6 et ne peut ni qualifier Chromium ni prouver seul AppArmor.
Le constat CI3 (résumé courant valide, attendu1/démarré0/achevé0, itération1,
requête0, exit k6 non nul, renderer et lectures namespace non observés) localise
le défaut avant le corps du cas ; il ne mesure pas un refus de namespace.

Contrat minimal de cette expérience, après admission :

- Une invocation explicite et séparée du parent, au plus **un** lancement natif,
  sans fallback automatique dans `browser`, boucle de retries ou nouvel exécuteur.
  Réutiliser `run-linux.py`, `pinned_tools`/vérifications d'identité, `Child` et
  `child_environment`, leurs limites et cleanup ; aucune copie de ces gardes.
  Même archive/ELF, version, UID, runner et environnement contrôlé que l'essai k6
  comparé ; corrélation privée des deux runs et différences d'arguments explicites.
  Un changement d'hôte ou de pin impose de déclarer la comparaison non équivalente.
- Arguments fixes et revus : headless, profil privé neuf, uniquement `about:blank`,
  sortie minimale bornée et prévention du trafic d'arrière-plan. Aucun argument,
  URL ou exécutable libre ; aucun wrapper, port/debug CDP, trace, screenshot/HAR,
  flag affaiblissant sandbox/TLS ou accès applicatif. Pas de nouvel outil, paquet,
  helper SUID, changement AppArmor/sysctl/seccomp, privilège ou déploiement.
- Collecte hôte facultative **en lecture seule**, par fichiers précis : version
  OS/noyau, UID effectif, disponibilité/valeurs des restrictions userns
  (`kernel.unprivileged_userns_clone`, `kernel.apparmor_restrict_unprivileged_userns`,
  `user.max_user_namespaces`), activation AppArmor et état seccomp/NoNewPrivs du
  parent. Aucune énumération globale de `/proc`, profils AppArmor ou journaux hôte
  supplémentaire ; pas de `sudo`, `unshare` ou sonde privilégiée. Absent, refusé,
  illisible et valeur zéro restent distincts ; une restriction configurée n'est
  pas une preuve qu'elle a causé cet échec. Aucun dump d'environnement/commande.
- Captures privées au plus64KiB par flux, projection fermée versionnée au plus32KiB,
  enveloppe totale existante256KiB. Réutiliser les catégories lexicales natives,
  sans publier extraits, suffixes, chemins ou URL. Identifier la source
  `CHROMIUM_DIRECT` ou `K6` ; conserver exit/signal/timeout/interruption distincts.
  Les catégories supplémentaires de codes k6 doivent provenir exactement de
  `errext/exitcodes/codes.go` v2.3.0 ; inconnu reste inconnu. Ne jamais appliquer
  cette table aux exits Chromium ni confondre message rapporté et cause établie.
- Identités contemporaines PID/UID/début de vie et descendants, inventaire pré/post,
  profil privé propre au run et arrêt gracieux/join via `Child`, y compris refus
  précoce/interruption. Ni adoption par nom, arrêt étranger ni kill forcé. Une
  identité non observable interdit d'attester le cleanup ; transmettre la limite
  au parent. Les résultats échec/confidentialité/sandbox du superviseur ne sont
  jamais neutralisés pour obtenir un succès diagnostique.
- Oracles séparés : message natif classé ou inconnu ; métadonnées lues ou non ;
  confidentialité ; cleanup. Aucune observation, exit0 ou renderer aperçu ici ne
  produit `Chromium PASS`, `HTTP PASS` ou un résumé fonctionnel synthétique. Même
  un diagnostic complet laisse la qualification navigateur **INCONCLUSIVE** ;
  conserver intact l'échec k6 source. Une erreur native peut discriminer une
  hypothèse de démarrage ; son absence ne réfute pas AppArmor/userns.

Validation ciblée avant nouvel essai : tests locaux des catégories k6 et source
Chromium distinctes, métadonnées absentes/refusées, profil/argv fixes, sorties
inconnues/invalides/sentinelles/dépassement, interruption et cleanup ; cas explicite
« natif exit0 sans qualification ». Revue du seul delta parent/CI s'il est requis,
aucun serveur mock ni essai runtime dans cet arbitrage. Les 70 tests Linux et le
cleanup CI3 rapportés restent historiques, pas validation du futur diagnostic.

Sources réutilisées : [restrictions Chromium156](https://chromium.googlesource.com/chromium/src/+/156.0.8078.73/docs/security/apparmor-userns-restrictions.md),
[parseur k6v2.3.0](https://github.com/grafana/k6/blob/v2.3.0/internal/js/modules/k6/browser/common/browser_process.go),
[codes k6v2.3.0](https://github.com/grafana/k6/blob/v2.3.0/errext/exitcodes/codes.go).
Installer un helper/profil, modifier la politique hôte, créer un laboratoire ou
faire de ce diagnostic un second harness de qualification sortirait de ce contrat
et exige un arbitrage distinct ; aucune permission de ce type n'est proposée.

### Addendum comparaison Linux — Accepted pour l'essai22.04 ; voie durable Proposed, 2026-10-30

**Comparaison22.04 ratifiée par le lead avant code le2026-10-30**, sous délégation
mission1 ; voie durable AppArmor toujours **Proposed et non autorisée**. Candidate
architecte `result_mvu6v2hg_000n_ev67mv`, session `ses_e75fb3daeffeCew1RlUmBKejVR`,
base `efe90e7e13a0e28fe4dc30dfb0a2f7ec8292a00d`, récupérée à l'identique puis sauvegardée.
Elle ne modifie aucun passage Accepted : Ubuntu24.04 reste la cible V1 et
l'interdiction de changer politique/privilèges du diagnostic précédent demeure.
Le lead ratifie seulement l'expérience ci-dessous, sans nouveau GO utilisateur de
routine ni autorisation de l'exception AppArmor évoquée ensuite. Aucun test ou
runtime n'a été exécuté par cet arbitrage ; aucune qualification n'est prononcée.

**Preuve réutilisée, pas nouvel essai :** rapport23 et ledger du lot
`team_mvtz6p47_000q_4wmht0`, run40973768642/source `cf9f5ca`, image Ubuntu24.04
`20261025.353.1` : 96 tests Linux OK sans skip, tools PASS ; k6 avant le cas,
`SANDBOX_UNPROVEN` ; direct `NO_USABLE_SANDBOX_REPORTED`, native SIGNAL et privacy
FAIL, aucun artefact d'échec publié. Cleanup enfants PASS, parent runtime
INCONCLUSIVE. AppArmor activé/restriction userns1, clone1/limite positive et
seccomp/NoNewPrivs parent0 concordent avec le problème documenté, sans prouver
un refus AppArmor corrélé au processus ni sa causalité exclusive.

Recoupement du code Chromium156 : `zygote_host_impl_linux.cc:115–141` situe ce
message dans la sélection namespace sandbox, puis helper SUID, puis FATAL si
aucune voie n'est retenue. `credentials.cc:267–284` tente
`ForkWithFlags(CLONE_NEWUSER | SIGCHLD)` mais peut aussi rendre false via
GetRESIds/TSAN : aucun errno de ce run n'est mesuré. Le FATAL contient des URL de
documentation, cohérent avec privacy FAIL ; leur présence n'autorise aucune
publication du flux brut ni modification du contrôle de confidentialité.

**Prochain essai recommandé : une comparaison native Ubuntu22.04 amd64 non-root,
sans modification de sécurité**, à vocation diagnostique uniquement. C'est un
changement d'environnement **non équivalent**, pas un downgrade de V1 ni une
qualification Ubuntu24.04. Le retrait officiel du runner22.04 le2027-04-17 exclut
d'en faire le choix durable implicite. Après ratification, contrat minimal :

- Réutiliser le lanceur/superviseur, scénarios et contrôles existants ; au plus un
  lancement `k6/browser`, avec outils/versions/inspections préalables. Le direct22
  est facultatif, non nécessaire à la question de dépendance à l'environnement ;
  s'il est retenu, un lancement séparé seulement après échec browser, selon le
  contrat diagnostique Accepted. Aucun retry, nouveau harness, paquet
  implicite, helper SUID, `sudo`, profil/sysctl/seccomp modifié ou cible applicative.
  Même k6v2.3.0 et CfT156.0.8078.73, mêmes archives/ELF/SHA256 revus et arguments
  que la référence24.04 ; revalidation avant exécution, pas de binaire préinstallé.
- L'éventuel delta ultérieur reste dans `ops/tests/k6/` et le workflow dédié :
  activation diagnostique fermée pour22.04, réutilisant les modes existants et
  séparée de leur qualification nominale ; pas trois nouveaux modes ni fallback.
  `run-linux.py:linux_platform()` refuse aujourd'hui22.04 ; ne pas le contourner
  par faux OS, environnement libre, modification silencieuse du lock24.04 ou
  assouplissement du chemin nominal. Garder le job24.04 canonique ; aucun échec
  existant rendu vert par suppression, skip ou `continue-on-error`. Le diagnostic
  direct reste non-zéro et ne devient jamais une qualification.
- Relever OS, architecture, noyau, image effective, UID et mêmes métadonnées hôte
  autorisées, versions/empreintes et delta de configuration. Un label22.04 ne
  prouve ni l'absence d'AppArmor ni l'autorisation effective des user namespaces.
  Le run40973768642 reste la référence historique : ne pas relancer24.04 à
  l'identique uniquement pour fabriquer une paire, ni annoncer un A/B à une variable.
  Un push normal ajoutant le job22 au même workflow déclenchera aussi le job24
  canonique : cette répétition incidente est admise, sans skip ni neutralisation
  de son échec. Ce n'est ni un essai24 dédié à fabriquer une paire ni une comparaison
  mono-variable ; la référence causale conservée reste run40973768642.
- Conserver gardes d'identité, sorties bornées, sentinelles, environnement fermé,
  absence de reporting et inventaires/cleanup pré/post, y compris interruption.
  Privacy FAIL interdit publication/PASS ; ne pas neutraliser le FATAL avec URL.
  Parent runtime, enfants, HTTP et Chromium restent des verdicts distincts.
  Aucun dispatch, secret de PR, nouveau déploiement ou permission CI ajouté.
- Si les prérequis/pins ne passent pas, arrêter avant browser et décrire la limite.
  Si k6 franchit le démarrage sur22.04, cela discrimine une dépendance à
  l'environnement, **pas AppArmor seul**. S'il échoue aussi, cela ne réfute pas
  AppArmor sur24.04. Distinguer démarrage, cas complet, témoin sandbox et cleanup ;
  un direct réussi ne remplace pas k6, ni une qualification24.04 manquante.

**Preuve sandbox inchangée.** `lib/safety.py:sandbox_witness` exige, pour un même
renderer possédé et identifié au même instant : UID effectif attendu, arguments
sans flags interdits, seccomp2, NoNewPrivs1, namespaces user **et** PID lisibles et
distincts du parent, nombre de filtres supérieur à celui du parent. Ne pas assembler
plusieurs processus/instants, confondre seccomp du parent et du renderer ou traiter
un `/proc` refusé comme une preuve. Ce témoin vérifie des frontières observables,
pas le contenu complet des filtres, l'application d'un profil AppArmor ni toutes
les garanties Chromium. Absence de témoin = SANDBOX_UNPROVEN, même après succès
`about:blank` ; pas de nouvelle sonde privilégiée ou de debug pour le rendre lisible.

**Voie durable — Proposed même après ratification de la seule comparaison22.04.**
Aucune exception AppArmor n'est autorisée par cette ratification partielle.

**Recommandation, à arbitrer séparément : rester sur24.04 et conserver
les pins, avec une exception AppArmor userns strictement dédiée au binaire**, si
le lead accepte explicitement ce changement de politique après examen. Ni cette
proposition ni l'admission de l'essai22.04 n'en autorise le provisioning. Avant
code/exécution de cette voie, un amendement distinct doit fixer : chemin canonique
exact et profil revu sans glob/arbre de builds, propriété/intégrité de la chaîne de
chemins, autorité de chargement/retrait, mesure de l'attachement effectif et
retour arrière vérifié sans toucher un profil étranger. Ne pas copier le profil
`flags=(unconfined)` d'exemple comme s'il confinait à lui seul Chromium. AppArmor
autorise par chemin, pas par SHA256 : un chemin remplaçable par le runner peut
autoriser un autre binaire. Les fichiers privés actuels appartiennent à son UID
(`private_path`) ; passer à un emplacement administré non remplaçable demande une
décision explicite sur le contrôle d'intégrité, pas une exception cachée aux gardes.
La mutation de politique ne doit pas s'exécuter à partir d'une PR non fiable.
Sans cet arbitrage et sa preuve opérationnelle, Chromium24.04 reste INCONCLUSIVE.

**Alternatives :** un Linux encore supporté, déjà disponible/autorisé avec userns
utilisables sans changement de politique, est recevable avec les mêmes pins et
gardes, mais exige une qualification de plateforme distincte ; aucun hôte nouveau
n'est présumé disponible. Ubuntu26.04 ne garantit pas la disparition des restrictions.
Chrome fourni par la distribution/profil existant change provenance/pin et ne se
substitue pas silencieusement au CfT privé. Docker non privilégié n'efface pas les
restrictions du noyau/AppArmor/seccomp et n'est pas un remède présumé. Sont écartés :
désactivation globale, `no-sandbox`, helper SUID, capabilities/runner privilégié,
déplacement opportuniste vers un chemin déjà autorisé et migration durable22.04.

Validation avant l'essai futur : tests Linux existants sans skip ; régressions du
sélecteur fermé (nominal22.04 refusé, diagnostic22.04 admis, autre OS refusé), pins
et résultats non qualifiants, confidentialité/cleanup inchangés. Puis versions et
inspections, expérience bornée et projections actuelles corrélées. Aucune commande
runtime ni test k6/Linux n'est exécuté par cet arbitrage documentaire. Aucun delta
Rust/service/identité/OAuth/HTTP/charge/SDK, nouvelle ADR ou changement de Réalité.

Sources officielles relues le2026-10-30 : [Chromium156, options et risque du chemin](https://chromium.googlesource.com/chromium/src/+/156.0.8078.73/docs/security/apparmor-userns-restrictions.md),
[Ubuntu, justification et limites des profils non confinés](https://discourse.ubuntu.com/t/understanding-apparmor-user-namespace-restriction/58007),
[retrait runner22.04, annonce14254](https://github.com/actions/runner-images/issues/14254),
[Puppeteer, recoupement AppArmor/CfT et avertissement SUID obsolète](https://raw.githubusercontent.com/puppeteer/puppeteer/main/docs/troubleshooting.md).
Les trois dernières sources sont du contexte daté, pas des preuves sur notre ELF.
Code Chromium au même tag156.0.8078.73 : [sélection de sandbox](https://raw.githubusercontent.com/chromium/chromium/156.0.8078.73/content/browser/zygote_host/zygote_host_impl_linux.cc),
[test de création userns](https://raw.githubusercontent.com/chromium/chromium/156.0.8078.73/sandbox/linux/services/credentials.cc).
Le ticket runner-images10443 concerne Buildah/unshare, pas notre cause Chromium.

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
