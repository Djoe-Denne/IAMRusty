# Registre applicatif local — déclarations Compose

`compose.yaml` déclare Zot **2.1.22** au digest de `transport.lock.json`, son
réseau dédié, deux volumes persistants et la préparation des permissions.
`config.json` déclare TLS/mTLS, le publisher read/create/update et le reader
read-only sur `apps/**`, sans accès anonyme. Le Zot P4 n'est pas modifié.

`publish.compose.yaml`, profil `transport`, déclare l'auxiliaire Skopeo épinglé,
sans socket Docker ni volume Zot. Archive/destination sont dans `/inputs` RO,
scripts/policy/certificats RO ; seuls `/outputs` (digest et `tmp/`) sont RW.
`prepare.sh`, `publish.sh` et `policy.json` sont les seules
commandes/politiques de ces auxiliaires. Aucun document Compose n'est fabriqué
en Python. `restart: "no"` est intentionnel : seule l'autorité de cycle de vie
peut redémarrer les IDs ACKés, jamais un restart automatique hors bail.

## Frontière privée et création

L'entrée publique reste `local_registry.py registry-up --state-dir ...
--cert-directory ...` (port historique 15443). Les inputs sont des références
de fichiers privés vérifiées ; les clés ne sont ni des variables Compose, ni
des arguments, ni une sortie publique. Le répertoire PKI reste indépendant du
coffre volatile. Les commandes Compose ont un projet unique par reçu/opération,
un `-f` explicite et `--env-file` vide : aucun `.env` ambiant.

Le wrapper utilise le renderer standard, vérifie noms/projet/port/pins avant
création, puis `compose create --no-recreate --no-build --pull never`.
Il n'emploie ni `up` (qui pourrait adopter/démarrer sans ACK), ni `down`,
`rm`, prune ou recréation. Les dépendances déclarées incluent la préparation
`service_completed_successfully`. La frontière d'ACK exige de séparer création
et démarrage : après preuve, le wrapper attend effectivement la réussite du
service permissions avant de démarrer **l'ID** Zot ; Compose ne reçoit jamais
une permission générale de démarrage.

La création opaque est journalisée `UNKNOWN/ask` avant l'appel. Les événements
`create` du daemon, entre deux mesures de son horloge, sont recoupés avec les
noms préalablement absents, les labels owner/projet, les inspect et les pins.
Les IDs constatés sont journalisés même lors d'un échec partiel, puis liés à
des reçus privés fsync avant toute utilisation. Un label ou nom seul ne prouve
jamais la propriété. Un événement manquant conserve l'incertitude et bloque
la reprise automatique ; une erreur/initialisation incomplète ne vaut pas ACK.

Les reçus contrôlent les préimages immuables de containers/volumes/réseau,
les montages, permissions, images, commande, ports et membres. Les seuls alias
de chemins Docker Desktop normalisés sont `C:/...`, `/host_mnt/c/...` et
`/run/desktop/mnt/host/c/...`, jamais une comparaison de suffixe. Les sources
déclaratives sont scellées dans le reçu ; leur changement est refusé, sans
rotation ou reconfiguration implicite.

## États antérieurs et publication

Un état v1 **sans champ `compose`** reste externe : lecture/publication OCI,
vérification, status, reprise et stop des seuls IDs historiques ACKés. Aucun label ajouté,
import/adoption Compose, nouveau volume/writer, changement de port/CA ou
reconstruction d'identité. Un ID perdu ou un setup incomplet bloque.

La publication utilise la même déclaration et la même garde de création que
le transport du registre neuf, avec un projet/helper neuf à chaque opération
et le réseau vérifié référencé `external: true`. Le champ privé `transport`
porte ses propres sources, reçus et helpers : il n'ajoute **pas** `compose`
à l'ancien registre et ne lui invente aucun reçu de création. Aucun appel
à la préparation de ses volumes, aucune création de réseau ou writer Zot.

Les pré/postimages scellent les identités/specs du registre, des volumes et
du réseau ; les seules écritures de contenu permises sont les publications
OCI normales. La PKI est revérifiée et les membres sont bornés aux IDs ACKés.
Un helper actif, un reçu perdu, une dérive ou une opération pending bloque.
Création, exécution et stop passent par un lifecycle commun : même sur échec
partiel/timeout, seuls les nouveaux helpers ACKés sont arrêtés. Échec de stop
= événement `stop-failed/ask` et pending conservé, pas erreur masquée ni
suppression/rejeu. Les helpers terminés restent inventoriés sans suppression.

## Python restant et preuves

Python conserve uniquement l'autorité/état privé, le verrou exclusif, les
pré/postconditions, les ACKs et journaux de création, les identités/pins,
le TLS client et la vérification de provenance/digests/archives OCI. Ces
contrôles ne peuvent être délégués à un simple label Compose. La création
Zot/préparation et le transport historique/neuf partagent un seul protocole
de preuve et de lifecycle. Les inventaires sont groupés par type ; le contrôle
owner/identité est centralisé ; les hashes/ACL/validations PKI ne sont plus
dupliqués dans `up` ; un seul indicateur pending remplace le doublon
pending/initialized. Aucun code n'a été déplacé dans un module caché.
Restent irréductibles sans retirer une garantie : préimage absente + événements
daemon + ACK avant start, comparaison inspect/Compose puis contrôle de dérive,
PKI/ACL, postconditions réseau et stop vérifié même lors d'un échec partiel.

Tests offline : `python -m unittest discover -s ops/tests -p test_local_registry.py`.
Validation standard : `docker compose --env-file <fichier-vide> -p <synthétique>
-f ops/deploy/registry/compose.yaml --profile transport config --quiet`, et
idem `publish.compose.yaml`, avec références **synthétiques** requises.
Ne pas fournir de secrets ou imprimer une configuration réelle résolue.

Le smoke opt-in `ops/tests/test_registry_runtime.py` conserve les oracles
push/read, refus du reader et persistance après stop/start. **NOT RUN** dans
ce lot statique : aucune preuve live Compose/mTLS/permissions/OCI/persistance
n'est revendiquée. Aucun Docker/Kind/WSL, build ou lecture d'inputs réels
n'est autorisé implicitement par ces fichiers.

Références : [Compose create](https://docs.docker.com/reference/cli/docker/compose/create/)
(`--no-recreate`, `--no-build`, `--pull never`) ;
[dépendances Compose](https://docs.docker.com/reference/compose-file/services/#depends_on)
(`service_completed_successfully`). Validation client locale : Compose 5.5.1.
