# Contexte partageable : Chrome s'arrête au démarrage sous Linux

## Objectif et conclusion actuelle

Nous voulons qualifier un navigateur Linux sans interface graphique, avec son
sandbox de sécurité conservé, avant de l'utiliser pour des tests k6. Même un
cas local vide ne démarre pas. Un lancement direct du navigateur, sans k6,
échoue également : le problème n'est donc pas limité au scénario JavaScript
ou à l'intégration k6. Cela ne démontre pas un défaut du produit Chromium :
les bibliothèques, le noyau, les permissions et l'environnement de lancement
restent des causes possibles.

Ce rapport prépare une recherche indépendante ; aucune décision utilisateur
n'est demandée. Aucun navigateur ou runtime n'a été relancé pour le rédiger.
Les versions, commandes et résultats ci-dessous sont des observations
historiques, non une nouvelle preuve de qualification.

## Environnement de l'échec local

- Hôte Windows, instance dédiée WSL2 Ubuntu 24.04, architecture Linux amd64.
- Utilisateur Linux non root, UID 1000 ; HOME sur le système de fichiers
  Linux natif ext4, et non sur un répertoire Windows monté.
- Navigateur exact : **Google Chrome for Testing 156.0.8078.73**, moteur
  Chromium, distribution `linux64`, révision **1706539**. Ce n'est pas le
  paquet Chromium d'Ubuntu, ni `chrome-headless-shell`.
- **k6 2.3.0**. Les versions ont été lues depuis les exécutables réellement
  utilisés ; les archives et exécutables ont été contrôlés par SHA256.
- Mode `--headless` : pas d'interface graphique ni de serveur d'affichage
  requis, mais des bibliothèques Linux restent nécessaires.
- AppArmor observé désactivé localement ; seccomp du processus parent à 0 ;
  limite de création d'espaces de noms utilisateur positive. Ces observations
  ne prouvent ni que Chrome peut créer ses propres espaces de noms, ni que
  son sandbox fonctionne.
- La version exacte du noyau WSL et une vérification exhaustive des
  dépendances ne sont pas établies par ce contexte. Aucun contrôle `ldd`
  du binaire n'a encore été exécuté pour répondre à cette question.

## Ce qui échoue exactement

Le scénario k6 ouvre un contexte neuf, charge seulement `about:blank`, évalue
`6 * 7`, attend brièvement puis ferme la page et le contexte. Il n'utilise
ni application métier, ni authentification, ni site externe à visiter.

Résultat local : `SCRIPT_ABORTED`, cas attendu = 1, cas commencé = 0,
cas terminé = 0, itérations = 1, requêtes HTTP = 0. Le scénario n'a pas
commencé et aucun processus renderer n'a été observé.

Un unique essai direct, sans k6, a aussi échoué : sortie standard vide,
format de diagnostic `FATAL` reconnu sur la sortie d'erreur, puis signal
**SIGABRT**, c'est-à-dire un arrêt par abandon, avant le nettoyage du
superviseur. Le site source, le message natif exact et la cause ne sont
pas connus par le canal partageable. Il ne faut donc pas inventer une
signature d'erreur plus précise.

## Commande directe historique

Les deux chemins ci-dessous sont volontairement remplacés par des variables
descriptives. Ce bloc décrit l'essai déjà effectué, pas un nouvel essai.

```sh
"$CHROME" \
  --headless --dump-dom --timeout=5000 \
  --disable-background-networking --disable-component-update \
  --no-first-run --no-default-browser-check \
  --disable-sync --metrics-recording-only \
  --user-data-dir="$PROFIL_PRIVE_NEUF" \
  about:blank
```

Le superviseur borne cet essai à 15 secondes. Le sandbox n'est pas désactivé.
`--dump-dom` sert à faire terminer une navigation locale bornée ; ce n'est
pas une preuve de qualification à lui seul.

L'environnement enfant est reconstruit, pas hérité intégralement :

```text
PATH=/usr/bin:/bin
HOME=<répertoire privé neuf>
XDG_CONFIG_HOME=<même répertoire privé>
TMPDIR=<répertoire temporaire privé neuf>
LANG=C.UTF-8
LC_ALL=C.UTF-8
TZ=UTC
K6_NO_USAGE_REPORT=true
K6_BROWSER_HEADLESS=true
K6_BROWSER_ARGS=
K6_BROWSER_DEBUG=false
K6_BROWSER_TIMEOUT=10s
K6_BROWSER_EXECUTABLE_PATH=<exécutable Chrome vérifié>
```

Pas de `DISPLAY`, de proxy ou de variable de chargement de bibliothèques
hérités. Aucun `--no-sandbox`, contournement de certificat ou débogage
réseau n'a été ajouté. Un profil utilisateur neuf est créé pour chaque essai.

## Pistes de recherche, pas diagnostics acquis

1. **Dépendances Linux de cette distribution précise de Chrome.** Le succès
   de `--version` ne démontre pas que le navigateur complet peut démarrer.
   La documentation Puppeteer recommande notamment de rechercher les
   bibliothèques manquantes avec `ldd` ; ce contrôle reste à faire ici.
2. **Sandbox et espaces de noms sous WSL2/Ubuntu 24.04.** Chercher les
   restrictions effectives, sans déduire leur fonctionnement de la seule
   valeur d'une limite système. L'avertissement AppArmor d'Ubuntu est une
   piste générale, pas une cause démontrée sur notre WSL où il est désactivé.
3. **Compatibilité ou régression du binaire 156.0.8078.73.** Une comparaison
   contrôlée pourrait être informative, mais aucune autre version n'a été
   qualifiée et les versions épinglées n'ont pas été changées.
4. **Contraintes de notre environnement épuré et des répertoires privés.**
   Leurs effets au démarrage ne sont pas exclus. Aucune modification
   spéculative des droits ou des protections n'est justifiée à ce stade.

L'information la plus discriminante reste le diagnostic fatal exact, à
consulter uniquement par le lecteur privé prévu. Aucun enregistrement
source privé réel n'a encore été créé. Les difficultés de sécurisation de
ce canal de lecture ne sont pas une cause établie de l'arrêt de Chrome.

Des essais GitHub distincts existent : Ubuntu 24 a rapporté
`NO_USABLE_SANDBOX_REPORTED` avec un contrôle de confidentialité en échec ;
Ubuntu 22 a rapporté `FATAL`/SIGABRT. Ce ne sont pas des preuves d'une cause
commune avec WSL, ni des qualifications réussies.

## Références publiques pour commencer

- [Chrome Headless : fonctionnement officiel](https://developer.chrome.com/docs/chromium/headless).
  Page relue pour ce rapport ; le mode moderne partage le code du navigateur
  normal, et n'est pas l'ancien `headless-shell`.
- [Puppeteer : Chrome ne démarre pas sous Linux](https://pptr.dev/troubleshooting#chrome-doesnt-launch-on-linux).
  Page consultée via Context Mode, version affichée 25.16.0 : pistes sur les
  dépendances et le sandbox, pas une matrice certifiée pour notre binaire.
- [Puppeteer : restrictions AppArmor sur Ubuntu](https://pptr.dev/troubleshooting#issues-with-apparmor-on-ubuntu).
  Ne pas appliquer automatiquement ses contournements à notre environnement.
- [Archive officielle exacte de Chrome for Testing](https://storage.googleapis.com/chrome-for-testing-public/156.0.8078.73/linux64/chrome-linux64.zip).
- [Exécution GitHub publique déjà observée](https://github.com/Djoe-Denne/IAMRusty/actions/runs/40985953377).
  Les journaux natifs privés ne doivent pas être ajoutés à un ticket public.

Requêtes utiles : `Chrome for Testing 156.0.8078.73 headless SIGABRT Linux`,
`Chrome headless WSL2 Ubuntu 24.04 user namespaces sandbox`, et
`Chrome for Testing Linux missing dependencies ldd`.

## Annexe technique : localisation, impact et limites

- Versions et empreintes : `ops/tests/k6/config/tools.lock.json`.
- Commande directe : `direct_chromium`, `ops/tests/k6/run-linux.py`.
- Environnement : `child_environment`, `ops/tests/k6/lib/safety.py`.
- Cas minimal : `ops/tests/k6/scenarios/smoke/browser-prerequisite.js`.
- Résultats historiques locaux : [WSL éprouvé, cause native inconnue](46-wsl-eprouve-mais-cause-native-encore-inconnue.md).
- Empreinte SHA256 attendue de l'archive Chrome :
  `c51c24f0126534aaf6d21d1f85b8e243934b3951989dbb59a1f8aab8e32b20c1`.
- Empreinte SHA256 attendue de l'exécutable Chrome :
  `8ea0ecbfc1ac0dc0e957de722babf9b72f8e86cb8643308ff17a491d59bdaa80`.
  Ces empreintes reprennent le verrou et les contrôles historiques ; ce
  rapport ne les présente pas comme un manifeste signé publié par Google.
- Impact : qualification navigateur non acquise. L'absence de cible HTTP
  autorisée est un blocage distinct du smoke HTTP, pas un effet prouvé de
  Chrome. Aucune conclusion sur OAuth ou les missions suivantes.
- Validation de ce rapport : relecture ciblée du verrou, de l'environnement
  enfant, du scénario et de la commande historiques ; consultation des deux
  documentations publiques ci-dessus. Aucun nouveau test, changement de
  configuration, relâchement du sandbox ou démarrage de runtime.
