# WSL identifié ; quatre dépendances manquent avant tout essai

## Objectif et résultat

Vous avez choisi WSL avec lecture privée par vous. Le préflight a identifié
Ubuntu 24.04, architecture x86_64, utilisateur artist non-root. Le binaire Chromium
conservé correspond au SHA256 épinglé. Aucune exécution Chromium ou k6, collecte
privée, installation ou mise à jour de métadonnées n'a été effectuée.

La lecture statique du binaire trouve32bibliothèques requises directement ; cinq
noms sont absents du cache du chargeur. Les paquets correspondants ne sont pas
installés. La simulation du gestionnaire de paquets réussit et propose seulement :

- `libnspr4` ;
- `libnss3` ;
- `libasound2t64` ;
- `libasound2-data`, dépendance ajoutée par la simulation.

Aucune mise à niveau ni suppression n'est proposée : les compteurs de simulation
confirment0upgrade/4nouveaux/0suppression, et les quatre paquets sont absents.
Les origines déclarées en cache sont archive.ubuntu.com/security.ubuntu.com,
sans identifiants dans les URLs ; cela ne remplace pas l'authentification des
paquets effectivement acquis avant installation. Il s'agit d'un prérequis
d'environnement manquant, pas d'un test navigateur échoué ou d'une cause établie
du refus sur GitHub.

## Recommandation et autorisation précise

Autoriser ces quatre ajouts dans **ARTIST-Ubuntu seulement**, après vérification
des sources et du plan effectif. Le choix de WSL n'autorisait pas leur installation
implicite. Aucun paquet recommandé supplémentaire, upgrade, suppression, dépôt
nouveau ou changement de sandbox n'est proposé. Si le plan effectif diffère ou
exige une mise à jour des métadonnées, arrêter et exposer ce changement avant suite.

Cela débloquerait les vérifications des outils, pas une garantie de démarrage ou
une qualification navigateur. Le relevé privé resterait soumis à sa ratification,
aux tests/revue et à votre lecture locale ; aucun journal natif ne serait publié.
Les dépendances ajoutées seraient conservées dans cette distribution : aucun
désinstalleur ou nettoyage destructif implicite n'est prévu.

Autre choix : ne rien installer. L'état est préservé et aucun nouvel essai WSL
n'est lancé ; la recherche de cause reste en attente d'un autre environnement
privé prêt. Aucun délai fiable n'est établi, les candidates du cache pouvant
nécessiter un nouvel arbitrage si elles ne sont plus disponibles.

## Runtime et limites

Avant préflight, ARTIST-Ubuntu était arrêté, Docker Desktop actif et étranger.
Un bail borné a été enregistré avant le démarrage pour les seules lectures.
Après les commandes, ARTIST-Ubuntu est **de nouveau arrêté**, Docker Desktop
reste actif ; aucun arrêt explicite/global, conteneur ou ressource Docker créé.
Le bail est libéré et aucun relevé privé n'existe. La mission n'est pas clôturée.

## Annexe technique

- Distribution ID `{30c42515-80b5-4ca9-bcb1-a1bfcf273bfc}`, WSL2,
  noyau6.6.87.2-microsoft-standard-WSL2, UID/EUID1000.
- `readelf -d` et `ldconfig -p`, exit0 : cache sans libnspr4.so, libnss3.so,
  libnssutil3.so, libsmime3.so, libasound.so.2. Pas d'exécution du binaire inspecté.
- `dpkg-query -s` des quatre paquets : exit1/absents.
- Candidates en cache : libnspr4 `2:4.35-1.1build1`, libnss3
  `2:3.98-1ubuntu0.3`, libasound2t64 `1.2.11-1ubuntu0.3` ; origines en cache Ubuntu
  officielles vérifiées, authentification effective à préserver avant mutation,
  aucune disponibilité réseau présumée.
- `apt-get --simulate --no-install-recommends install libnspr4 libnss3 libasound2t64` :
  exit0, quatre Inst/Conf, zéro Remv et zéro paquet déjà installé modifié.
- États WSL décodés UTF16LE strict par le helper validé, exit0 avant/après.
  Bail `.tmp/k6-wsl-private-preflight-20261030-mvuayb9x.json` mis à jour et libéré.
- Preuves140Linux du run40985953377 réutilisées à leur portée ; aucun nouveau
  résultat fonctionnel WSL, HTTP, Chromium ou cleanup parent ajouté.
