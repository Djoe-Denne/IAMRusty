# Cycle opérateur Kind réellement exécuté — 10 novembre 2026

## Résultat et limites

Les cinq anciens conteneurs autorisés ont été retirés, puis le lab natif a été
installé avec `just kind-local`. Après corrections, **8 applications et
5 dépendances disponibles** ; les trois connecteurs optionnels restent désactivés.
Arrêt complet des trois nouveaux nœuds, redémarrage des **mêmes IDs**, unseal
OpenBao et recréation des queues volatiles : **13/13 disponibles**, HTTPS
`/iam/health` **200 avant et après**. Le lab est laissé en marche sur demande.

Ce n'était **pas** une installation sans intervention :

- Premier appel sans environnement : refus `KIND_LOCAL_PRIVATE_DIR` manquant.
- Premier build/install : 11 images et Kind créés, puis erreur de namespace
  Calico. Correctif `bd1fb33` ; application manuelle de Calico au cluster déjà créé.
- Premier passage complet des dépendances : remise privée Bao, arrêt volontaire
  avant les applications, puis report manuel du token produit dans Lazaret.
- Corrections des entrées privées : descripteur IAM obligatoire même avec
  récepteur désactivé, credential Hive non vide, kid IAM UUIDv4 canonique sans
  tirets, remplacement d'une regex Envoy trop grande par six chemins exacts.
- `docker stop --timeout 60` a rendu exit0 mais les nœuds ont fini **Exited137** :
  aucun arrêt propre de tous les processus revendiqué.
- Après `docker start`, Bao était scellé et les queues SQS avaient disparu.
  Les étapes supplémentaires ci-dessous ont été nécessaires.

Aucun nouveau test écrit. Aucun changement Rust, backend Lazaret toujours Redis
TLS. Images, caches de build, volumes Docker et fichiers privés hôtes conservés.
Les mêmes UID/PV des trois PVC et les mêmes références OpenFGA ont été constatés,
pas une intégrité exhaustive des données ni une conservation des messages SQS.
OAuth, WIF, autorisation organization-signer Hive et P4 ne sont pas qualifiés.

## Commandes de lancement et de listing

Les arguments exécutés sont regroupés ici avec des variables PowerShell. Le
dossier privé neuf réel et les journaux non secrets sont référencés dans le
ledger local `.cursor/handoffs/20261110-kind-local-operator-cycle.md`.
La préparation privée n'est pas un nouveau déployeur livré.

```powershell
$ctx = 'kind-aiforall-local-full'
$ns = 'aiforall-local-full'
$private = '<dossier privé absolu hors Git>'
$env:KIND_LOCAL_PRIVATE_DIR = $private
$env:KIND_LOCAL_OPENBAO_CA_FILE = "$private\pki\mesh\ca.crt"
$env:OPENSSL = 'C:\Program Files\Git\mingw64\bin\openssl.exe'
$env:AIFORALL_HELM = "$PWD\.tmp\ops-iac-tools\helm-v4.3.1\windows-amd64\helm.exe"
$env:KIND_LOCAL_BAO = "$PWD\.tmp\ops-iac-tools\openbao-2.6.2-windows-amd64\bin\bao.exe"
$env:KIND_LOCAL_BUILD_TAG = 'operator-20261110'

# Première installation seulement : sorties Bao neuves, hors Git.
$env:KIND_LOCAL_INITIALIZE_EMPTY = '1'
$env:KIND_LOCAL_OPENBAO_INIT_OUT = "$private\openbao\initialization.json"
$env:KIND_LOCAL_OPENBAO_PRODUCT_OUT_DIR = "$private\openbao\products"
just kind-local

# Le premier succès s'arrête à la remise Bao. Compléter le TOML Lazaret avec
# auth.client_token de products/lazaret.json, sans afficher ni utiliser le root.
just kind-local

just local-status
docker ps -a --no-trunc
kubectl --context $ctx -n $ns get deployments,pods,services,pvc -o wide
```

Réparation Calico réellement effectuée après le premier échec :

```powershell
kubectl --context $ctx apply -f ops/deploy/third-party/calico/base/calico.yaml
kubectl --context $ctx -n kube-system rollout status daemonset/calico-node --timeout=240s
```

Elle ne constitue pas une nouvelle exécution du bootstrap corrigé sur un lab
entièrement neuf. Les reprises suivantes ont conservé le cluster existant.

## Arrêter et redémarrer complètement

IDs de ce cycle uniquement, vérifiés par inspect/labels avant chaque mutation :

```powershell
$nodes = @(
  'feee478f36ca48f5a57be20ee70270977cff44db115fcde4133b97781e6c1434',
  'dbbe19a816ecb7b304d621ab9273c0d40706f807203b788edc8d9a9e34510c80',
  '24206ca4fd5536260f2f018df4075d9e8888206d660a3943e7d5eef59317a0c1'
)
docker stop --timeout 60 $nodes
docker ps                   # vide lors de cet essai
docker ps -a                # les trois nœuds sont arrêtés, pas supprimés
docker start $nodes
kubectl --context $ctx wait --for=condition=Ready nodes --all --timeout=180s
kubectl --context $ctx -n $ns wait '--for=jsonpath={.status.phase}=Running' pods -l app.kubernetes.io/name=openbao --timeout=180s
```

`just local-stop` ne remplace **pas** cet arrêt : il met seulement les applications
à zéro réplique, sans arrêter les dépendances ni les conteneurs Kind.

### 1. Unseal Bao après le redémarrage

Premier terminal (pod observé dans ce cycle ; sélectionner le pod courant) :

```powershell
kubectl --context $ctx -n $ns port-forward --address 127.0.0.1 pod/openbao-7bbc7d94bd-s8gs9 18444:8443
```

Second terminal, avec les mêmes variables d'environnement :

```powershell
Import-Module ./ops/deploy/openbao-cli.psm1 -Force
$connection = @{
  Executable = $env:KIND_LOCAL_BAO
  Address = 'https://127.0.0.1:18444'
  CaFile = $env:KIND_LOCAL_OPENBAO_CA_FILE
  ServerName = 'openbao.aiforall-local-full.svc.cluster.local'
}
Get-KindOpenBaoStatus -Connection $connection
Unlock-KindOpenBao -Connection $connection -InputFile "$private\openbao\initialization.json"
```

Résultat observé : initialized=true, sealed=true → sealed=false. Fermer ensuite
le port-forward temporaire. Aucune initialisation ni nouvelle émission de token.
Le token Lazaret initial est non renouvelable, limité à **8 heures** : ce cycle
ne fournit pas de renouvellement automatique ou d'accès durable après expiration.

### 2. Recréer les queues manquantes du LocalStack volatil

`list-queues` avait rendu `QueueUrls: []`. Les quatre créations suivantes ont
rendu exit0 ; aucune ancienne donnée/message n'a été restaurée :

```powershell
foreach ($queue in @('sentinel-sync-events','telegraph-events','user-events','lazaret-kv-events')) {
  kubectl --context $ctx -n $ns exec deployment/localstack -- aws --endpoint-url=https://localhost:4566 --ca-bundle=/tls/ca.crt sqs create-queue --queue-name $queue
}
kubectl --context $ctx -n $ns exec deployment/localstack -- aws --endpoint-url=https://localhost:4566 --ca-bundle=/tls/ca.crt sqs list-queues --output json
just local-status
```

Ce sont les opérations fournisseur du Job natif SQS, exécutées ici manuellement
pour éviter un nouveau rebuild complet. `docker start` seul ne suffit donc pas
à restaurer la disponibilité du profil actuel. Ne pas substituer silencieusement
une persistance nouvelle à ce comportement.

## Contrôle HTTPS réellement utilisé

Premier terminal :

```powershell
kubectl --context $ctx -n $ns port-forward --address 127.0.0.1 service/envoy-mesh 18443:10000
```

Second terminal :

```powershell
curl.exe --silent --show-error --fail --noproxy '*' --ssl-revoke-best-effort --cacert "$private\pki\mesh\ca.crt" --output NUL --write-out '%{http_code}' --max-time 15 https://127.0.0.1:18443/iam/health
```

**200/exit0 avant et après redémarrage.** L'appel Schannel sans
`--ssl-revoke-best-effort` avait échoué (révocation inconnue pour cette CA privée).
L'option est limitée à cette commande et tolère les points de distribution CRL
absents/hors ligne ; signature, CA et SAN restent vérifiés. Pas de `-k`, de
modification du magasin de confiance ou de rotation implicite.
Sources : [curl TLS](https://curl.se/docs/sslcerts.html),
[option CRL](https://github.com/curl/curl/blob/master/docs/cmdline-opts/ssl-revoke-best-effort.md).

## Preuves et état laissé

Vérification indépendante infra finale **PASS borné** : correctif Calico/publié,
trois nœuds et treize déploiements actifs disponibles, pas de nouveau warning
après récupération. Les preuves de transitions et de TLS ont été réutilisées,
sans nouvelle campagne. Cela ne prouve ni un bootstrap neuf du correctif ni une
reprise autonome : les interventions Bao/SQS restent nécessaires et documentées.

Journaux bornés : `.tmp/kind-minimal-tests/operator-cycle-*` ; sorties privées
complètes dans le dossier opérateur hors Git. Registre append-only :
`.tmp/docker-resources-20261110.log`. Aucun secret dans ce compte rendu.

Trois nouveaux nœuds conservés **running sur demande explicite**, avec leurs
volumes. Observateurs et port-forwards temporaires fermés. Aucune équipe,
branche ou worktree temporaire créé pour ce cycle ; changements Obsidian étrangers
exclus. Ce compte rendu ne transforme pas ces interventions en parcours automatisé
sans étape manuelle ni en qualification métier globale.
