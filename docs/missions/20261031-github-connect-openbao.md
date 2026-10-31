# Mission — Secrets OpenBao pour GitHubConnect, réutilisables via RustyCog

## Objectif

Faire résoudre par GitHubConnect ses secrets d'amorçage depuis OpenBao, au lieu de dépendre de valeurs sensibles directement fournies dans sa configuration. Le mécanisme doit être utilisable à l'identique en local, en CI Linux et dans le cloud ; endpoints, identités et références changent selon l'environnement, pas le code métier.

Fiche de mission à exécuter, **pas une ADR Accepted**. Base d'observation : AIForAll `master@8698d5a`, SDK au gitlink `02e5db2` ; vérifier les versions au démarrage et préserver le travail préexistant.

## Périmètre

- GitHubConnect : configuration, composition root, consommation des secrets et tests ciblés.
- Secrets concernés : `github.client_secret` OAuth et `github.hmac_secret` partagé avec IAM. `client_id`, URLs et redirects restent des paramètres non secrets. Le HMAC doit conserver exactement le contrat attendu par IAM ; ne pas modifier IAM dans ce lot.
- Examiner une capacité générique du SDK RustyCog pour éviter de dupliquer des résolveurs par service. Décider sa place et son API avant l'implémentation transverse, selon les règles d'architecture du dépôt. Ne pas imposer une nouvelle crate ou abstraction si l'existant suffit.
- Hors périmètre : installation d'ESO, manifests/Helm/Compose, scripts `ops/`, workflow E2E, seed du coffre, données métier, certificats, rotation à chaud, migration IAM/Lazaret et ajout du flux GitHub App installation/API. Une autre mission prend en charge ces raccords de déploiement.

## Comportement attendu

1. La configuration décrit le coffre, son mode d'authentification et des **références de secrets** (mount/path/champ KV v2), sans contenir les valeurs applicatives en clair dans le parcours nominal OpenBao.
2. Le `setup` résout les secrets au démarrage, puis construit le client OAuth et le vérificateur HMAC existants. Pas d'appel au coffre par requête métier, pas de lecture réseau cachée dans le chargement TOML générique.
3. Coffre inaccessible, accès refusé, référence absente, réponse invalide ou secret vide/invalide : démarrage refusé explicitement. Aucun retour silencieux à une valeur `.env`, à une fixture ou aux defaults du loader. Si un ancien mode direct est conservé, il doit être explicite et mutuellement exclusif, jamais un secours automatique du mode OpenBao.
4. L'authentification au coffre est un contrat explicite, pas un token root distribué à l'application : identité de workload lorsque disponible, ou credential de bootstrap borné à la lecture des chemins nécessaires. Une identité doit toujours être fournie pour accéder au coffre ; ne pas prétendre supprimer ce bootstrap.
5. Aucun secret dans les logs, `Debug`, erreurs, URLs ou arguments de processus. Transport HTTPS vérifié hors exception locale explicite. Timeout et taille de réponse bornés ; aucune boucle de retry infinie.
6. Les secrets résolus restent seulement dans les composants qui les utilisent. Aucun nouveau fichier persistant ou stockage métier. Ne pas promettre une absence totale/courte de résidence en RAM : le client OAuth et le vérificateur HMAC en ont besoin pendant leur utilisation.
7. Le premier lot charge les secrets au démarrage : une mise à jour du coffre exige un redémarrage contrôlé du service. Pas de moteur de cache/rotation/rechargement supplémentaire.

## Volet SDK RustyCog

- Rechercher d'abord les ports/adaptateurs de secrets et d'identité existants. La capacité mutualisée éventuelle est agnostique GitHub : résolution asynchrone d'une référence, erreurs expurgées, adaptateur OpenBao KV v2 ; aucune logique OAuth, aucun client GitHub et aucune dépendance vers un service AIForAll dans le SDK.
- Garder le chargement de configuration déclaratif ; ne pas rendre `load_config_fresh()` dépendant d'OpenBao pour tous les services. Dépendance/feature optionnelle si un nouveau composant partagé est retenu.
- Étudier la réutilisation du comportement de Lazaret sans le déplacer ou le migrer de force. OpenBao **KV** pour lire des secrets est distinct de **Transit** pour signer : ne pas réutiliser les clés ou permissions de signature comme magasin d'API keys.
- Si le SDK change : livrer/publier son commit, puis la consommation et le gitlink AIForAll correspondant, conformément au workflow sous-module. Aucune copie locale divergente ni modification étrangère embarquée.

## Sources vérifiées à reprendre, sans réexploration générale

- `services/GitHubConnect/configuration/src/lib.rs` : `GitHubConfig`, préfixe `GITHUB_CONNECT`, `load_config()`.
- `services/GitHubConnect/setup/src/app.rs` : composition du client et du vérificateur HMAC ; `services/GitHubConnect/infra/src/github.rs` : `GitHubConnectClient`.
- `rustycog/rustycog-config/src/lib.rs` : `load_config_fresh()` / `build_config_with_env_prefix()` chargent `.env`, TOML et variables ; le loader peut revenir aux defaults sur erreur de désérialisation. Ce comportement ne doit pas neutraliser le refus du mode OpenBao.
- `services/Lazaret/infra/src/vault.rs` : `VaultHttpSecretResolver::resolve()` ; `services/Lazaret/setup/src/app.rs` : `build_secret_resolver()` ; `services/Lazaret/domain/` pour le contrat associé.
- `services/IAMRusty/infra/src/signing/transit.rs` : signature distante, **autre usage** ; ne pas confondre avec les anciens bras `Vault`/`GcpSecretManager` non implémentés dans `SecretStorage::resolve()` d'IAM.
- GitHubConnect dispose déjà de fixtures HTTP sortantes dans `tests/fixtures/` ; appliquer ADR0200–0202 aux frontières touchées et les procédures MCP du dépôt.

## Validation proportionnée

Quelques contrôles ciblés du comportement applicatif : lecture réussie des deux références et utilisation effective par OAuth/HMAC ; refus d'un accès refusé, secret manquant/invalide ou coffre indisponible ; absence de fallback et de fuite dans erreurs/Debug. Réutiliser les fixtures adaptées, sans nouvelle campagne de robustesse du framework ni matrice exhaustive.

Compiler/tester nativement sous Windows et conserver la compatibilité CI Linux. Pas de démarrage Kind, E2E réel GitHub ou lecture de secrets personnels nécessaire pour ce lot. Respecter le slot Cargo et les règles de publication du dépôt.

## Retour attendu pour raccorder le pipeline

- Commits publiés du service et, si modifié, du SDK avec gitlink consommé.
- Contrat exact de configuration : champs/env, schéma des références, authentification, transport et comportement d'échec. Ne pas retourner de valeurs sensibles.
- Liste des entrées à créer dans OpenBao et droits minimaux requis ; exemple de configuration **sans secrets réels**, utilisable par la mission de déploiement.
- Vérifications réellement exécutées et limites ; statut explicite des anciens modes directs et du renouvellement des secrets.

Les futurs usages OAuth utilisateur et GitHub App installation/API restent deux jeux d'identifiants logiques, éventuellement rattachés à une même App ou à deux Apps. Ce choix n'est pas un prérequis pour livrer le résolveur ; ne pas ajouter ce second flux dans cette mission.
