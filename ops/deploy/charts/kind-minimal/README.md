# Profil Kind natif

Entrée opérateur : [guide](../../kind-minimal.md), `just kind-local`.
ADR0610/0611 **Proposed / Partial** ; contrats
[lanceur](../../../../docs/contracts/kind-local-native-launcher.md) et
[OpenBao](../../../../docs/contracts/kind-local-openbao-bootstrap.md).

Le chart est un ensemble de déclarations pour `helm template` puis kubectl,
**pas un release à upgrader/uninstaller globalement**. Le lanceur garde les
dépendances existantes compatibles et ne crée que les objets manquants. Retirer
une branche d'un release Helm historique pourrait au contraire supprimer ses
objets : ne pas employer cette méthode pour la transition.

Tous les flags de création/initialisation et `applications.enabled` sont false
par défaut : le rendu par défaut reste vide. Aucune valeur privée dans Helm.
Les noms de Secrets, ConfigMaps et chemins de montage sont le câblage fixe du
lanceur ; les overrides publics permettent notamment le choix des connecteurs,
des queues et du stockage, pas un renommage implicite de ce câblage.

```powershell
helm template kind-local ops/deploy/charts/kind-minimal --namespace aiforall-local-full
helm template kind-local ops/deploy/charts/kind-minimal --namespace aiforall-local-full --set postgres.create=true
helm template kind-local ops/deploy/charts/kind-minimal --namespace aiforall-local-full -f ops/deploy/build/targets.json --set applications.enabled=true --set buildTag=local-example
```

## Déclarations

- `applications.enabled` : onze Deployments de calcul déclarés, dont trois
  connecteurs désactivés par défaut, et Services des applications actives.
  Images du même inventaire/tag Bake, `Never` pour les images applicatives
  importées ; Envoy tiers épinglé. Les documents JSON compacts sont du YAML
  valide et permettent le parsing hôte natif, sans parseur YAML PowerShell.
  Configurations privées complètes montées RO, Secret TLS par application ;
  seule Lazaret monte la clé d'autorité workload. L'initContainer de confiance
  concatène CA système/locales/Bao sans emporter de clé dans ce bundle public.
- `postgres.create` : image15.12-alpine3.21, PVC `postgres-data`, groupe70,
  Recreate1, HBA TLS≥1.2/SCRAM, `POSTGRES_PASSWORD_FILE` et Secrets0440. Socket
  Unix local trust réservé au lab. La readiness `psql SELECT1` utilise
  `verify-full`, PGHOST DNS et PGHOSTADDR127.0.0.1 pour éviter la dépendance à
  son propre EndpointReady ; le Job distant n'utilise pas PGHOSTADDR.
- `postgres.initialize` : ConfigMap SQL et Job `kind-local-postgres-initialize`,
  DB absentes seulement (`pg_database`, CREATE DATABASE/`\gexec`). Aucun rôle,
  password, DROP/TRUNCATE ou migration applicative inventé. Le lanceur recrée
  ce seul Job et attend Complete ; les migrations restent dans les entrypoints.
- `redis.create` : image7.2.7-alpine, même PVC `redis-data`, groupe999, `/data`,
  AOF/appendfsyncalways, Recreate1. Plaintext `port0`, TLS6379/1.2–1.3, Secrets0440.
  `tls-auth-clients no` et `protected-mode no` : aucun nouvel AUTH imposé ;
  ServiceClusterIP seulement, pas une configuration d'auth client production.
  Le mode protégé+NOPASS refuserait Lazaret non-loopback malgré une probe locale
  verte. La probe PING/PONG TLS contrôle CA/transport mais **SNI≠hostname**.
- `openfga.create` : migration datastore native en initContainer, serveur1.8.5,
  HTTPS8080 et gRPC TLS loopback8081. URI datastore privée (CA `/tls/ca.crt`).
  `openfga.inspect` / `initialize` : CLI officiel0.8.2 sans shell, modèle DSL RO.
  Le caller hôte conserve `local-full-fga`, refuse les références incomplètes
  et ne bootstrappe qu'une installation explicitement vide. Job mutateur et
  publication des IDs create-only ; logs entiers JSON strict après Complete.
- `localstack.create` : SQS3.0.2 et certificat serveur local stable. Tous les
  appels déclarés utilisent HTTPS/CA. Le listener fournisseur reste multiplexé
  HTTP/HTTPS ; `USE_SSL` sélectionne aussi les URLs retournées, pas une promesse
  de suppression du support HTTP. Pas de PVC/persistance fictive.
  `localstack.initialize` crée seulement les queues demandées, sans purge.
- `openbao.create` : **coffre absent seulement**, image2.6.2 non-DEV, Raft
  mono-nœud/PVC `openbao-data`, TLS et ServiceAccount limité à create tokenreviews.
  Aucun root/fragment Shamir monté dans le Pod. Statussealed nonReady mais Live ;
  init/unseal/credential handback par CLI au poste, pas readiness avant init.
  Un coffre DEV existant reste totalement hors reconfiguration/reseed.
- `zot.create` : option explicite, **registre OCI P4/plugins**, PVC `zot-data`,
  configuration/auth privées complètes et TLS. Probe TCP, pas preuve OCI/CA/SAN.
  Ce n'est pas le registre d'images applicatives supprimé du lancement. Un P4
  neuf complet (operator/admission-store) n'est pas installé par ce chart.
- `network.create` : ingress natif ciblé par label, namespace propre et espaces
  Apparatus/plugins existants autorisés ; ne reconfigure pas le coffre préservé.

## Conservation et limites

Une dépendance déjà présente n'est pas convertie automatiquement vers TLS.
Un PVC initialisé exige son credential déjà valide. Un Deployment absent ne
permet ni resize/remplacement d'un PVC survivant, ni overwrite de ses ConfigMaps.
`reset-data` est un choix destructif explicite du cluster dédié, pas une fonction
de ce chart ni une sauvegarde. Voir les limites exactes dans le guide opérateur.

Les rendus/syntaxes ne qualifient pas les images, Jobs/API/TLS cross-Pod, données
après restart/PVC ou démarrage global. Les probes HTTPS kubelet ne vérifient pas
la CA/SAN. La preuve Redis Rustls Windows antérieure reste bornée à son client
testé, pas à l'exécution de ce profil Kind. Aucune campagne runtime n'est lancée
par la documentation ou le rendu.
