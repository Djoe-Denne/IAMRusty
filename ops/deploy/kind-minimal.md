# Kind local — parcours natif

```powershell
just kind-local                  # rebuild, import, lancement ; données conservées
just kind-local reset-data       # efface UNIQUEMENT le lab dédié, puis le recrée
```

Entrée : `ops/deploy/kind-local.ps1`. Elle utilise Bake, Kind, Helm, kubectl,
OpenSSL et les CLI fournisseurs, **sans Python**. Contrats
[lanceur](../../docs/contracts/kind-local-native-launcher.md) et
[OpenBao](../../docs/contracts/kind-local-openbao-bootstrap.md), ADR0610/0611
**Proposed / Partial**. Un [cycle opérateur réel avec reprises explicites](../../docs/missions/20261110-kind-local-operator-cycle.md)
est documenté ; ce n'est pas une installation sans intervention ou une qualification
métier globale. Les anciens scripts de campagne ne sont pas des prérequis.

## Préparer les entrées une fois

Hôte Windows/PowerShell5.1 ; outils natifs `just`, `docker.exe` avec Buildx,
`kind.exe`, `kubectl.exe`, Helm et `openssl.exe` disponibles. Docker Desktop doit
déjà servir des conteneurs Linux : le lanceur ne le démarre/redémarre pas.
Les builds Linux sont des images de livraison Kind, pas des tests de développement.
Un seul lancement/build Cargo à la fois ; laisser les **24 jobs** du dépôt.

Tous les inputs privés restent **hors checkout/contexte Docker**, par exemple :

```text
C:\prive\aiforall-local-full\
  postgres.password
  openfga.env
  pki\                          # générée/réutilisée par OpenSSL
  openbao\ops-token.json        # données JSON KV pour une première installation
  apps\iam\development.toml
           environment.env
           platform-signing.pem # exemple de fichier annexe privé
           platform-signing.pub
  apps\hive\development.toml
            environment.env
  apps\telegraph\development.toml
                 environment.env
  apps\manifesto\development.toml
                 environment.env
  apps\lazaret\development.toml
               environment.env
  apps\sentinel-sync\sentinel-sync.toml
                     environment.env
  apps\ext-authz\environment.env
  apps\envoy-mesh\envoy.yaml
```

Les répertoires de configuration applicative sont montés **en entier, en lecture
seule** : `/app/config` pour les services et Sentinel, `/etc/envoy` pour Envoy.
Mettre les fichiers annexes à plat dans le même dossier ; les Secrets créés avec
`kubectl --from-file=<dossier>` ne parcourent pas les sous-dossiers. IAM peut ainsi
référencer `/app/config/platform-signing.pem` depuis son TOML. `environment.env`
est importé avec `kubectl --from-env-file` ; valeurs `NOM=valeur`, **sans guillemets
de shell**. Il peut être vide si aucun override n'est requis. Aucun composeur
TOML, secret de démonstration ou téléchargement de CLI n'est ajouté.

Préparer des configurations **complètes**, adaptées au lab, pas des copies
inchangées des exemples Compose qui contiennent encore HTTP, clés de test et
backends différents. Les fichiers TOML sont lus en environnement `development`
(`RUN_ENV`) ; le lanceur ne réécrit pas leur contenu.

Préparation vérifiée dans le [cycle opérateur réel](../../docs/missions/20261110-kind-local-operator-cycle.md) :
descripteur IAM non vide même avec le récepteur GitHub désactivé, `kid` IAM
**UUIDv4 simple lowercase sans tirets**, `api_key` Hive non vide **pour la composition
seulement, pas comme preuve d'authentification**, et regex Envoy trop grande
remplacée par six chemins exacts dans l'input privé. Ce jeu local n'utilise pas
de compte externe et ne qualifie ni OAuth ni l'autorisation organization-signer.

- PostgreSQL : `postgres.aiforall-local-full.svc.cluster.local:5432`, password
  valide du backend dans `postgres.password`. Les cinq services à migrations
  exigent aussi leur `DATABASE_URL` dans `environment.env`. Utiliser TLS vérifié,
  `PGSSLMODE=verify-full` et `PGSSLROOTCERT=/app/certs/ca.crt` pour SQLx/libpq.
- `openfga.env` contient la clé **`uri`**, URI PostgreSQL complète du datastore
  `openfga_dev`, avec `sslmode=verify-full&sslrootcert=/tls/ca.crt`. Encoder les
  caractères réservés du password dans l'URI. Le fichier n'est pas une URL FGA.
- HTTP interne : HTTPS et DNS du Service (`iam`, `hive`, `telegraph`, `manifesto`
  sur8443, `lazaret` sur8080, `ext-authz` sur8090). L'edge Envoy écoute HTTPS10000.
  Les configurations doivent fournir les certificats clients requis par le mTLS.
- OpenFGA : HTTPS `openfga.aiforall-local-full.svc.cluster.local:8080`. Les IDs du
  ConfigMap `local-full-fga` sont injectés pour les services ayant un préfixe FGA
  et pour Sentinel. Aucun nouveau store n'est créé si ces références sont présentes.
- SQS : endpoint explicite **`https://localstack.aiforall-local-full.svc.cluster.local:4566`**,
  région `us-east-1`, compte `000000000000`, credentials locaux `test`/`test`.
  Ne pas laisser le constructeur historique `host=localstack` produire HTTP.
  Les queues par défaut sont `telegraph-events`, `user-events`,
  `lazaret-kv-events`, `sentinel-sync-events` ; compléter les valeurs publiques
  si la configuration complète en consomme d'autres. LocalStack reste volatil.
- Lazaret : conserver **`kv.backend="redis"`**, Redis DNS `redis`/6379,
  `tls_enabled=true`, `tls_ca_cert_path="/app/certs/ca.crt"`. Autorité d'identité
  `/app/identity-ca/ca.crt` et `/app/identity-ca/ca.key` ; certificat serveur
  `/app/certs/server.crt` et clé associée. Ne pas changer silencieusement de backend.
  Le Vault produit utilise HTTPS8443, la CA explicite et un token autorisé valide,
  jamais le root initial révoqué.
- Envoy : fournir `envoy.yaml` complet, edge TLS avec `/app/certs/envoy-mesh.crt`
  et `.key`, routes/issuers/callbacks correspondant à `https://127.0.0.1:18443`,
  upstreams HTTPS/mTLS et chemin `/iam/health` accessible à la readiness.
- ext-authz : fournir notamment `EXT_AUTHZ_AUDIENCE`, `EXT_AUTHZ_JWKS_URL` HTTPS,
  `EXT_AUTHZ_TLS_CERT=/app/certs/ext-authz.crt`, `EXT_AUTHZ_TLS_KEY` associé et
  `EXT_AUTHZ_TLS_CA=/app/certs/ca.crt`. Sa probe vérifie le handshake mTLS et la
  réponse du listener, pas une décision d'autorisation ou la fraîcheur JWKS.

Chaque application reçoit sa **propre** feuille dans `/app/certs` (`tls.crt`/
`tls.key` et noms natifs), pas toutes les clés du mesh. Seul Lazaret reçoit la
clé de l'autorité workload. `/app/trust/ca-bundle.crt` combine les racines publiques
de l'image, les CA locales et la CA OpenBao ; `SSL_CERT_FILE` le sélectionne.
La CA Bao est également montée dans `/run/openbao-ca/ca.crt`. La PKI hôte reste
stable ; aucune confiance système ou rotation n'est installée implicitement.

## Lancer et réutiliser

```powershell
$env:KIND_LOCAL_PRIVATE_DIR = 'C:\prive\aiforall-local-full'
$env:KIND_LOCAL_OPENBAO_CA_FILE = "$env:KIND_LOCAL_PRIVATE_DIR\pki\mesh\ca.crt"
just kind-local
```

Pour un coffre existant signé par une autre CA, fournir **sa vraie CA**. Un
cluster déjà présent exige sa PKI existante ; aucune autorité de remplacement
n'est générée pour ses données. Options natives : `OPENSSL`, `AIFORALL_HELM`,
`KIND_LOCAL_CERT_DIR` (hors Git), `KIND_LOCAL_BUILD_TAG`, `KIND_LOCAL_PUBLIC_VALUES`
(fichier Helm **public**). Les noms de Secrets/ConfigMaps et chemins de câblage du
profil restent ceux déclarés par défaut ; ne pas les renommer via ces valeurs.

Le lanceur reconstruit les onze images, garde/crée Kind et son CNI, importe les
images, arrête les anciens calculs avant migrations et crée seulement les objets
de dépendances absents. Les DB/queues manquantes sont créées par Jobs natifs, sans
purge. Les dépendances existantes compatibles et PVC restent hors reconfiguration,
y compris le coffre DEV mémoire : il reste volatil. Un ancien service plaintext
nécessite une transition explicite ; ce lancement ne le convertit pas en cachette.
Une erreur laisse l'état partiel visible, sans rollback ni sauvegarde automatique.

Accès HTTPS au premier plan après le lancement :

```powershell
kubectl --context kind-aiforall-local-full -n aiforall-local-full port-forward --address 127.0.0.1 service/envoy-mesh 18443:10000
```

`local-up` appelle cette même entrée. `local-stop` arrête uniquement les calculs
natifs étiquetés, **pas les données**. `local-status` affiche Pods/PVC/Jobs.
Build/import séparés : `just kind-local-build <tag>`, `just kind-local-import <tag>`.

## Première installation / coffre neuf

Avant le premier lancement, sélectionner explicitement :

```powershell
$env:KIND_LOCAL_INITIALIZE_EMPTY = '1'
$env:KIND_LOCAL_BAO = 'C:\outils\openbao-2.6.2\bao.exe'
$env:KIND_LOCAL_OPENBAO_INIT_OUT = 'C:\prive\aiforall-bao-first\init.json'
$env:KIND_LOCAL_OPENBAO_PRODUCT_OUT_DIR = 'C:\prive\aiforall-bao-first\products'
just kind-local
```

Les deux sorties doivent être **neuves**, hors Git ; `openbao/ops-token.json` est
un objet de données KV fourni par l'opérateur. Le coffre neuf est Raft/PVC, Shamir
**1part/seuil1**. Un port-forward temporaire vers le Pod running permet le CLI
HTTPS, même avant readiness/unseal. Init/unseal/bootstrap utilisent fichiers et
stdin privés ; le root est révoqué et son refus vérifié avant remise opérateur.
La sortie `products/lazaret.json` contient un token produit orphan, borné à8h.

Ce premier passage **s'arrête avant les applications**. Reporter le token produit
dans le champ chaîne `VaultConfig.token` du TOML Lazaret complet et compléter les
autres inputs, puis relancer. Le bundle d'init contient une part secrète et le
root historique désormais révoqué : ne pas le publier ni l'utiliser comme token
produit. Un unseal ultérieur requiert explicitement
`KIND_LOCAL_OPENBAO_UNSEAL_INPUT` ; pas de recherche automatique du dernier bundle.
Ni les TTL ni les credentials ne sont renouvelés automatiquement.

FGA neuf : inspection native readonly vide, puis `store create --model` depuis
`ops/openfga/model.fga`, publication create-only des vrais IDs. Références absentes
sur des stores existants ou Job mutateur déjà présent : arrêt pour reprise
opérateur, pas de choix par nom ni deuxième store automatique.

## Reset et options séparées

`just kind-local reset-data` détruit les **trois nœuds du seul cluster
`aiforall-local-full`** et toutes ses données node-local : DB, cache, PVC, tuples,
coffre, Secrets et éventuel stockage P4. Les fichiers hôtes/PKI, images/caches du
builder, registres externes et autres clusters restent hors périmètre. Aucun
backup implicite. Choisir de nouvelles sorties Bao et recréer les credentials,
même si les anciens fichiers hôtes ont été préservés. Ne pas l'utiliser sur des
données historiques ou ressources étrangères sans leur disposition explicite.

Connecteurs désactivés par défaut : activer `applications.services.<nom>.enabled`
dans les valeurs publiques et fournir `apps/<nom>/{development.toml,environment.env}`
et fichiers annexes. OAuth réel demande les callbacks et credentials fournisseur
correspondants, préparés hors lanceur.

Le registre OCI **P4/plugins est distinct** : les ressources Zot/admission-store
existantes ne sont jamais retirées par le rebuild normal. Pour créer Zot absent,
`KIND_LOCAL_ENABLE_P4_REGISTRY=1` exige `zot/config.json` et `zot/htpasswd` complets
(TLS `/tls/tls.crt`/`.key`, auth `/etc/zot/htpasswd`, stockage `/var/lib/zot`). Cela
n'installe pas l'operator/admission-store P4 neuf et ne qualifie pas son parcours.

## Portée de livraison

Le [rapport du cycle réel et ses commandes](../../docs/missions/20261110-kind-local-operator-cycle.md)
documente build/import Linux, Kind et 13 déploiements disponibles, HTTPS edge200
avant/après stop/start. Des reprises ont été nécessaires : Calico réparé manuellement
après correctif publié `bd1fb33`, inputs privés corrigés, puis **unseal Bao et
recréation manuelle des quatre queues SQS** après restart (liste vide constatée).
Ce n'est pas un bootstrap sans intervention ou une reprise autonome. Le stop exit0
a laissé les nœuds **Exited137** : pas de propreté d'arrêt revendiquée.

Mêmes PVC UID/PV et références FGA constatés, **pas d'audit des données DB/cache ni
de messages SQS conservés** ; aucun nouveau backend persistant ajouté. Le contrôle
Schannel a nécessité `--ssl-revoke-best-effort` explicite (sinon exit60), avec
CA/SAN vérifiés, sans `-k` ou modification de confiance globale : commande au rapport.
Les trois nœuds sont laissés running sur demande, forwards temporaires fermés.
Reset automatisé et qualification métier globale restent hors preuve. Les probes
HTTPS natives Kubernetes et `redis-cli --sni` ne prouvent pas la CA/SAN des clients.
LocalStack3.0.2 sait aussi accepter HTTP sur son listener multiplexé : tous les
consommateurs de ce profil doivent utiliser explicitement HTTPS, pas un fallback.
`OPS-OPENBAO-262-REDIRECTS` reste ouvert pour Agent auto-auth.

L'ancien framework Python, sa façade legacy, son chart/profils et ses tests
exclusifs sont supprimés. Les seules exceptions dans `ops/deploy` sont
`local_full_target.py` (gardes encore consommées par common.ps1/k6 Windows/Linux)
et `bootstrap_local_kind.py` (wrappers Kind/CNI historiques). Ils ne sont jamais
appelés par cette entrée ; leur migration n'est pas annoncée acquise. La PKI
Python et le registre applicatif sont retirés, pas le registre P4/plugins.
Les déclarations Bao sont désormais dans `ops/deploy/openbao/bootstrap.json`.
Aucun nouveau test n'est écrit ; seuls les imports/chemins des vérifications
conservées sont adaptés au retrait de leurs anciens supports.
