# Images Linux : itération locale et release

`ops/deploy/build/images.py` est l'entrée Python commune Windows/Linux pour
**planifier et construire** les images. Aucun déploiement, E2E, démarrage de
Kind, génération de certificat ou provisioning OpenBao n'est implicite.
Le build produit des **IDs Docker locaux** ; `local_registry.py` publie ensuite
dans Zot et produit des digests OCI vérifiés, dans un reçu séparé.
Le Zot P4 et ses données ne sont pas modifiés.

## Développement

Plan sans Docker :

```sh
python ops/deploy/build/images.py plan --github-openbao
```

Construction lorsque le daemon Linux est disponible, avec le builder courant
de type `docker` (aucun changement global de builder/trust n'est effectué) :

```sh
python ops/deploy/build/images.py build --github-openbao --output image-build.json
```

- Les sources sont les **octets actuels** : modifications non committées,
  nouveaux modules/assets publics, même Git-ignorés, et SDK réellement patché
  sous `rustycog/`. L'inventaire est une allowlist de sources/recettes/assets,
  pas une copie récursive de tous les fichiers ignorés (bases/logs/caches exclus).
- Le contexte est une copie possédée et bornée aux sources/recettes publiques.
  `.env`, clés, dossiers privés et état agent sont exclus avant lecture/hash.
  Le `.dockerignore` de l'opérateur n'est jamais remplacé. Ne pas mettre de
  vrais secrets dans les fichiers de source/configuration publics.
- La compilation utilise le snapshot, jamais un mélange de fichiers vivants.
  Les différences restantes à la comparaison finale font échouer l'opération.
  Arbitrage utilisateur du 2026-11-01 : A→B→A est admissible si le snapshot
  compilé et toutes les entrées finales sont identiques A, SDK compris. Ces
  comparaisons ne prétendent pas détecter chaque écriture transitoire.
  Le reçu et les labels nomment cette politique `snapshot-final-v1`.
- Une image déjà conforme est réutilisée ; sinon un seul Cargo reçoit la
  sélection complète pour conserver une unification stable des features. Cargo
  réutilise les artefacts inchangés ; seules les images périmées sont emballées.
  La sélection et le profil sont inclus dans l'identité de construction.
  Un changement SDK invalide **tous** ses consommateurs
  sélectionnés, pas uniquement GitHub/Lazaret. Le déploiement refuse les autres
  images devenues périmées : un build partiel n'autorise pas leur réemploi.
- Les caches Linux registry/git/target restent persistants. Le cache target
  historique est conservé, protégé par `sharing=locked`. Les mtimes des sources
  sont gérées par empreinte pour détecter aussi un rollback de contenu. Le
  journal de synchronisation est remplacé atomiquement **avant Cargo**, même
  si la compilation échoue ensuite ; il n'atteste jamais un build réussi.
  Horloge en retrait ou précision insuffisante ⇒ refus, sans purge du cache.
- Le profil `dev-image` privilégie l'itération fonctionnelle. Pour les mesures
  de performances, utiliser `--profile release`. Les 24 jobs Cargo restent
  configurés dans `.cargo/config.toml` ; le target Windows E: n'est pas partagé.
- `--targets hive` borne explicitement le build. Les profils complets ajoutent
  GitHub avec `--github-openbao` ou GitLab avec `--oauth-proof` (distincts).

Compatibilité avec le chemin existant **kind load**, après obtention du bail :

```sh
python ops/deploy/local-full-images.py --lease LEASE --github-openbao --build-manifest image-build.json --output images.json
```

Le manifeste doit couvrir tous les consommateurs Rust du profil. Le Zot P4
préexistant reste une image séparée ; il n'est ni reconstruit ni adopté par ce
build. Les anciennes images attestées seulement par HEAD doivent être
reconstruites avant utilisation : HEAD seul ne prouve pas l'arbre courant.

## Release

**Non implémentée dans la tranche A.** `--policy release`, `--revision` et
`--cold` sont refusés avant tout build : export automatique du commit exact de
`master` et de son gitlink SDK, référence distante et pins complets des outils
restent à réaliser. Un checkout propre n'est pas un substitut à ce contrat.
L'export futur ne devra ni modifier ni exiger de nettoyer le workspace courant.

`--policy local --profile release` reste disponible pour des **mesures locales**,
pas pour déclarer une livraison cloud. Le namespace target neuf de compilation
froide reste une politique future ; aucune purge du cache dev n'est autorisée.

Les manifestes sont créés sans écrasement, distinguent snapshot courant et
révision d'origine d'une image réutilisée, et conservent les IDs vérifiés.
Les images créées sont enregistrées dans le registre Docker append-only du
projet ; ni images ni caches ne sont nettoyés automatiquement. Les contextes
temporaires possédés sont retirés en fin de commande.

## Limites de qualification

Preuves réelles acquises le 2026-11-01 : build Linux `ext-authz` profil release,
réemploi inchangé en **3,885 s** (même ID Docker), publication OCI et relecture
manifeste/config/filesystem/binaire. Smoke registre **1/1, 33,920 s** : lecture
mTLS, écriture reader refusée `403`, même digest après redémarrage du même Zot.
Ce ne sont pas des preuves de pull CRI Kind, de rollout, ni d'OpenBao/OAuth.
Publication, raccord et refresh restent séparés ; ID Docker ≠ digest OCI.

## Registre applicatif et publication

Le générateur partagé fournit un bundle additionnel explicite, hors Git :

```sh
python ops/scripts/prepare-local-full-certs.py --output-dir PRIVATE_PKI_DIRECTORY --registry
```

`extensions/registry/` contient une leaf serveur SAN
`aiforall-apps-registry`/`127.0.0.1`, plus les clients `registry-publisher` et
`registry-reader` (EKU clientAuth uniquement). La CA mesh existante est réutilisée
sans modification de ses octets/compteurs ; la CA workload reste séparée. Un
bundle existant incomplet ou périmé est refusé, jamais réparé silencieusement.
Les clients doivent présenter leur **leaf seule**, avec la CA séparée du client.

`ops/deploy/registry/transport.lock.json` épingle les manifestes Linux/amd64 de
Zot **2.1.22** et Skopeo 1.22.3, résolus et vérifiés par leur contenu le 2026-11-01.
Ce lock est limité au transport : il ne complète pas les pins de build cloud.
Le lancement impose `10001:10001`, volumes propres persistants, bind hôte loopback,
mTLS et ACL publisher/reader distinctes. État et PKI exigent des chemins absolus
privés **hors dépôt** ; remplacer les paramètres ci-dessous par ces chemins.

```sh
python ops/deploy/local_registry.py registry-up --state-dir REGISTRY_STATE --cert-directory PKI_DIRECTORY
python ops/deploy/build/images.py build --targets ext-authz --profile release --output image-build.json
python ops/deploy/local_registry.py publish --state-dir REGISTRY_STATE --build-manifest image-build.json --output image-release.json
python ops/deploy/local_registry.py status --state-dir REGISTRY_STATE
```

Le reçu `aiforall.image-release/v1` porte référence/digests OCI, entrées source/SDK,
profil et hashes des binaires exportés. Publication vérifie aussi la fraîcheur
finale. Une sortie existante n'est jamais écrasée. Aucun Kind, coffre, build ou
déploiement n'est déclenché par `registry-up`/`publish`.

Zot 2.1.8 a effectivement refusé les ACL avec mTLS seul : [issue officielle
3085](https://github.com/project-zot/zot/issues/3085). La correction concerne
**uniquement le registre applicatif** ; P4 reste épinglé en 2.1.8, stockage intact.

## Consommation Kind — bail explicite

```sh
python ops/deploy/local_registry.py connect-kind --state-dir REGISTRY_STATE --lease LEASE
python ops/deploy/local_registry.py pull --state-dir REGISTRY_STATE --lease LEASE --image aiforall-apps-registry:5000/apps/ext-authz@sha256:DIGEST
```

L'autorité vérifie endpoint/UID/IDs avant les opérations node ; elle connecte
le seul registre possédé au réseau existant et installe seulement son reader
dans `certs.d/aiforall-apps-registry:5000`. Pas de restart/reconfiguration globale
containerd, pas de clé CA transférée, pas de repli vers `kind load`.
Ces opérations sont codées/testées hors ligne mais **pas encore éprouvées sur Kind**.

Pour le profil complet, construire/publier **toutes** ses cibles avec les mêmes
flags de profil, puis produire le mapping attendu par le deploy existant :

```sh
python ops/deploy/local-full-images.py --lease LEASE --github-openbao --release-manifest image-release.json --registry-state REGISTRY_STATE --output images.json
```

Le full `deploy` accepte ce mapping avec `--registry-state`, mais reste une
opération de provisioning/quiesce : **ce n'est pas le refresh rapide**.

## Refresh ciblé sans provisioning

Conserver le reçu publié correspondant à l'image effectivement déployée avant
de modifier le code. Après build/publication du code modifié :

```sh
python ops/deploy/deploy-local-full.py refresh --lease LEASE --registry-state REGISTRY_STATE --previous-release-manifest previous-release.json --release-manifest image-release.json --targets ext-authz
```

- Sélection explicite : `ext-authz`, `sentinel`, `github`, `gitlab`, `controller`.
  Les services qui lancent automatiquement leurs migrations au démarrage sont
  **refusés**, même si la migration semble inchangée ; plan séparé requis.
- Deux reçus de publication récents avec `refresh_inputs_id` sont nécessaires.
  Recettes, assets/config, manifests, build scripts et migrations doivent être
  inchangés ; seul le code Rust ordinaire peut varier. Garde conservative globale.
- Vérifie image précédente et UID namespace/deployment possédés, puis patch JSON
  conditionnel UID/resourceVersion/image. Seuls image et `imagePullPolicy` changent ;
  replicas, certificats, Secrets, bases, OpenBao, Jobs et policies restent intacts.
- Pull par digest et readiness ciblée. Identique ⇒ aucun rollout. Échec/stale ⇒
  arrêt explicite, sans rollback destructeur ; plusieurs cibles ne sont pas atomiques.
  **Refresh réel NOT RUN** tant qu'aucun bail valide n'est sélectionné.

En fin d'utilisation : arrêt réversible des seuls conteneurs possédés ; données,
certificats, images et caches sont conservés :

```sh
python ops/deploy/local_registry.py registry-stop --state-dir REGISTRY_STATE
```

Historique de préparation : [fiche B — PKI/pins](../missions/20261101-app-registry-pki-preparation.md).

Référence du contrôle de builder : [Docker buildx inspect](https://docs.docker.com/reference/cli/docker/buildx/inspect/).
