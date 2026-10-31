# ADR-0505 : Les secrets de démarrage sont résolus explicitement par le SDK, hors du loader déclaratif

- Statut : Proposed
- Réalité : Implemented
- Date : 2026-10-31
- Décideurs : à renseigner après accord humain sur cette proposition
- Jalon concerné : GitHubConnect / secrets de démarrage et SDK (hors Apparatus P0–P6)
- SuperSède : aucune
- SuperSédée par : —
- Contrat d’implémentation : [startup-secrets-openbao.md](../contracts/startup-secrets-openbao.md)

`Accepted` ratifie une cible ; `Réalité` décrit séparément le code. Le mandat d’implémentation ne constitue pas une acceptation implicite de cette ADR.

## Contexte

La [mission utilisateur](../missions/20261031-github-connect-openbao.md) demande de sortir les valeurs OAuth/HMAC de la configuration nominale GitHubConnect, sans changer les protocoles OAuth ou IAM↔Connect. La base vérifiée est AIForAll `4d1a137944e38c855291bee2edaf5ed4e7d56b85`, SDK/gitlink `02e5db23aac6016aab062c3078696d5f7fd464ef` ; les chemins GitHubConnect n’ont pas changé depuis la base de mission `8698d5a` selon le handoff du parent.

Les constats suivants décrivent cette **base initiale, avant implémentation** ; l’état actualisé et ses limites figurent dans « Réalité et preuves » ci-dessous.

- `GitHubConfig` contient deux `String` sensibles ; `Application::new` est synchrone, construit `HmacKey` et `GitHubConnectClient`, puis conserve `AppConfig`. Le client clone actuellement `client_secret` depuis cette config.
- `rustycog::config::load_config_fresh` revient à `T::create_default()` et logue l’erreur brute si la désérialisation échoue. Ce comportement est incompatible avec un sélecteur OpenBao fail-closed. Le handbook 0500 ne spécifie pas de résolution distante.
- Recherche hybride SDK `local-qwen3-8b`, puis matching exact des sources Rust/manifeste : pas de `SecretResolver`, `WorkloadIdentity` ou adapter KV réutilisable dans ce pin. `UserIdExtractor::from_resolved_secret` consomme un secret HS256 déjà résolu : ce n’est pas un résolveur.
- Lazaret possède `SecretResolver::resolve(&str) -> Vec<u8>` et `VaultHttpSecretResolver` KV v2, mais pour une opération autorisée Apparatus (`secret:path#field`, mount global). Son client garde le token, ne désactive pas les redirections et vérifie la taille finale après `bytes()`. Réutiliser l’idée et l’enveloppe KV, **pas** déplacer ce contrat métier ni copier ces limites de sécurité.
- 0307 place les identités workload dans IAM/Hive ; les importer ferait dépendre le SDK des services. Le SDK observé est un **package feature-gated unique** : les répertoires historiques `rustycog-*` sont des modules, pas des crates à recréer.

## Décision

**La résolution est une étape async explicite de composition, derrière une petite API SDK optionnelle ; le loader reste purement déclaratif.**

1. Ajouter `rustycog::config::secrets`, activé uniquement par une nouvelle feature `secrets` du package `rustycog-framework`, impliquant `config` et les seules dépendances techniques nécessaires. Ni nouvelle crate, ni feature `http`/`full` requise, ni dépendance de service. Pas d’activation par défaut ni dans `full` pour ce lot.
2. API générique : référence structurée `SecretReference { mount, path, field }`, valeur opaque `SecretValue`, port async object-safe `SecretResolver::resolve(&SecretReference) -> Result<SecretValue, SecretError>`. Premier adapter : `OpenBaoKv2Resolver`. Les types/erreurs et leurs règles d’exposition sont fixés par le contrat lié.
3. Première authentification supportée : **`token_file` explicite**, sans recherche automatique de credential. Le fichier contient un bearer OpenBao émis pour cette application, soit par une authentification workload externe, soit par un bootstrap à durée/droits bornés. Aucun login Kubernetes/cloud/SPIFFE/AppRole n’est prétendu livré. Le client vérifie via `lookup-self` l’absence de policy `root` et une TTL positive d’au plus une heure ; les ACL précises restent une précondition du provisionnement, pas une propriété déduite du nom du token.
4. HTTPS vérifie chaîne et nom d’hôte ; CA additionnelle configurable. HTTP exige un opt-in **et une adresse IP littérale loopback**. Redirections/proxies implicites interdits ; timeout, lecture incrémentale et tailles bornés ; pas de retry applicatif ni de failover. KV v2 uniquement, lecture du champ string `data.data[field]`, jamais Transit.
5. Ajouter `load_config_fresh_strict<T>()` : mêmes sources/priorités que le loader courant, mais aucun fallback ni erreur brute exposée. GitHubConnect l’utilise toujours. Les callers legacy conservent leur comportement ; pas de réseau ou d’authentification dans les loaders.
6. GitHubConnect choisit explicitement `openbao` ou `direct`, sans mode par défaut utilisable. Références et valeurs directes sont mutuellement exclusives. Le setup async résout et valide **les deux** secrets avant de construire/exposer l’application. Un échec ne déclenche jamais le mode direct.
7. Les valeurs vont aux seuls consommateurs OAuth/HMAC ; elles ne sont pas réinjectées dans `AppConfig`, sérialisées, persistées ou conservées dans un cache de résolution. Le credential/résolveur est abandonné après le boot. Snapshot de démarrage par secret, **pas** transaction atomique multi-références ; modification effective après redémarrage seulement.

## Conséquences

- API/contrat transverse SDK↔GitHubConnect : chargement strict additif, nouveaux types et feature, setup `Application::new` / `AppBuilder::build` async, constructeur OAuth consommant explicitement le secret résolu. Les helpers/tests et configurations directes doivent déclarer leur mode.
- Le bootstrap n’est pas supprimé : il est explicite et portable par chemin de fichier (`PathBuf`, Windows/Linux). Distribution, protection ACL du fichier et émission de l’identité restent externes. Le SDK n’efface pas un fichier qu’il ne possède pas.
- `lookup-self` ajoute une requête et exige son droit `read`. L’absence de policy `root` ne prouve pas le moindre privilège d’une policy personnalisée ; le contrat fournit les seuls chemins nécessaires pour le raccord ops ultérieur.
- Une redirection HA est un refus de démarrage : l’endpoint configuré doit desservir les lectures directement. Disponibilité du coffre nécessaire au boot, pas aux requêtes métier.
- OAuth, PKCE, HMAC (octets/canonicalisation), routes et IAM restent inchangés. Les secrets demeurent en RAM chez leurs consommateurs ; aucune promesse d’effacement total ou de rotation à chaud.
- Livraison SDK séparée puis gitlink/pin consommé ; validation ciblée Windows, compatibilité Linux à qualifier. La consigne initiale de ne pas embarquer le préexistant a été précisée par une autorisation utilisateur ultérieure : intégrer/publier tout travail durable préexistant sur les principales existantes `AIForAll/master` et `RustyCog/main`, sans renommage, en commits séparés. Cela n’autorise ni secrets, ni logs/caches/données d’exécution, ni adoption des branches d’autres missions. Aucune migration DB ni donnée métier.

## Réalité et preuves — actualisation 2026-10-31

**Implemented** décrit la capacité SDK et son adoption GitHubConnect dans l’arbre validé ; **Statut reste Proposed**. Ni acceptation humaine ni publication finale du service ne sont déduites de ce champ.

- SDK implémenté et publié sur `main` : `93b890ff7b821e9cb2e7d0aeb81957d098778a2f`, checkout `rustycog/` consommé par l’arbre AIForAll. Continuité préalable : préexistant `9a0a794`, merge du distant `9e9ef27` puis du pin `02e5db2` en `54d435e`, fixtures principal policy `2bdbc2a`. Cette intégration autorisée ne modifie pas la décision secrets ni ne ratifie l’ADR.
- Validation SDK **Windows**, rapportée par le parent : **11 tests config avec feature `secrets`** verts (pas 11 nouveaux tests secrets), `check` avec `config` seule et Clippy avec `secrets` / `-D warnings` verts.
- Adoption service dans l’arbre : loader strict, configuration exclusive, setup async, consommation explicite OAuth/HMAC et snapshot sans refresh. Campagne Windows des packages `github-connect-{configuration,infra,setup,service}` : **17 tests verts** (`1 + 2 + 4 + 7 + 3`), dont **3 nouveaux tests startup**. Résumés lus dans `.tmp/githubconnect-openbao/service-final.log` ; cet artefact local n’est pas destiné à être committé. Usage effectif du secret OAuth résolu et HMAC conservant `trim()` / minimum 16 octets vérifiés par cette campagne.
- Rapport durable du parent : [résultats GitHubConnect OpenBao](../missions/20261031-github-connect-openbao-results.md), propriétaire de la référence finale de publication AIForAll, **encore en cours** lors de cette actualisation. Le checkout SDK validé ne vaut pas preuve de gitlink/service publiés dans AIForAll.
- Limites : **pas de CI Linux exécutée, de fixture TLS live, de coffre réel ou de déploiement qualifiés** ; aucun runtime Docker/Kind/WSL ni conteneur créé pour ce lot. Les résultats ciblés ne constituent pas une validation implicite de tous les oracles du contrat ni une qualification sécurité/production globale. L’architecte n’a exécuté aucun code, test, runtime, commit ou push.

## Alternatives rejetées

- **Réutiliser le SDK tel quel** : aucune capacité KV/identité générique trouvée ; le helper HS256 ne fait aucune acquisition.
- **Extraire/migrer Lazaret maintenant** : blast radius inutile sur grants/invoke/IT Apparatus, contrat opaque différent ; Lazaret reste inchangé.
- **Résoudre dans `load_config_fresh` ou via interpolation de chaînes** : réseau caché, dépendance globale et risque de fallback silencieux.
- **Importer `WorkloadIdentity` d’IAM/Hive ou implémenter tous les logins** : dépendance inversée / surcharge hors besoin. Le token-file est une première frontière portable, pas une identité anonyme.
- **Token inline/`VAULT_TOKEN` implicite, root dev réutilisé** : credential ambiant, portée non bornée, exposition excessive. Même les tests utilisent un bearer synthétique non-root.
- **Nouvelle crate/service de secrets ou bibliothèque Vault complète** : aucun besoin actuel de discovery, lease, écriture, renouvellement ou Transit ; réutiliser `reqwest` déjà présent suffit.

## Non décidé ici

Déploiement/Compose/ESO/ops/E2E/seed, émission et renouvellement des identités, providers workload natifs dans le SDK, rotation/rechargement, GitHub App installation/API, migration IAM/GitLabConnect/Lazaret et modifications des ADR Accepted.

## Références et écarts

- [0500](0500-config-typee-et-compose-local.md), [0502](0502-rustycog-framework-feature-gated.md), [0102](0102-setup-composition-root.md) ; la photographie rétroactive n’est pas réécrite.
- [0408](0408-connecteurs-idp-services-http.md), [0409](0409-confiance-callback-oauth-idp-connect.md), [0307](0307-workload-identity-port.md) : invariants préservés, aucune supersession.
- Code : `services/GitHubConnect/configuration/src/lib.rs` (`GitHubConfig`, `load_config`), `setup/src/app.rs` (`Application::new`, `AppBuilder::build`), `infra/src/github.rs` (`GitHubConnectClient`), `src/main.rs`, `tests/common.rs`, `tests/hmac_contract.rs` ; `rustycog/{Cargo.toml,src/lib.rs,rustycog-config/src/lib.rs}`.
- Analogie inter-service : `services/Lazaret/domain/src/secrets.rs`, `infra/src/vault.rs`, `setup/src/app.rs::build_secret_resolver`, `tests/apparatus_p3_t12_openbao.rs`. Identité existante : chemins IAM/Hive de 0307, non mutualisés par ce lot.
- Docs : à l’inspection initiale, `docs/services/github-connect.md` décrivait les valeurs en config et mentionnait à tort `client_id` parmi les secrets ; le handbook actualisé distingue désormais `client_id` public et secrets acquis au setup. Le handoff partagé contient des photographies initiales de préservation du préexistant, à lire avec l’autorisation ultérieure et le rapport de résultats, pas comme une interdiction toujours active de cette intégration autorisée.
- Wiki consulté via QMD à l’inspection initiale : `projects/iamrusty/decisions/index.md` (`#9c52e9`) conservait une qualification 0412 `Partial` périmée face au canon `Implemented` ; écart indépendant, pas corrigé par ce lot. Cette page n’est pas une preuve d’acceptation de 0505.
- Sources primaires OpenBao consultées le 2026-10-31, documentation affichée **2.7.x**, pas qualification d’une instance déployée : [KV v2 / Read secret version](https://openbao.org/docs/api/secret/kv/kv-v2/#read-secret-version), [authentification](https://openbao.org/docs/concepts/auth/), [lookup-self](https://openbao.org/docs/api/auth/token/#lookup-a-token-self), [root tokens](https://openbao.org/docs/concepts/tokens/#root-tokens), [file sink](https://openbao.org/docs/agent-and-proxy/autoauth/sinks/file/). Le sink n’est pas déployé par cette décision ; fichier simple non wrapped/non chiffré uniquement dans cette V1.
- Preuves d’implémentation : commit SDK et campagne Windows ci-dessus ; [rapport parent](../missions/20261031-github-connect-openbao-results.md) pour la consommation/publication finale. Aucune preuve Linux, TLS live ou coffre réel implicite.
