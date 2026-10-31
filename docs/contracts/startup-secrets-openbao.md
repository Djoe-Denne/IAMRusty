# Contrat — Secrets de démarrage OpenBao / SDK RustyCog

- Canon primaire : [ADR-0505](../adr/0505-startup-secrets-sdk-openbao.md), **Proposed / Implemented**, actualisé le 2026-10-31. `Implemented` décrit l’arbre validé, pas une acceptation ni une publication finale du service.
- Mandat : [mission GitHubConnect](../missions/20261031-github-connect-openbao.md).
- Base vérifiée : AIForAll `4d1a137`, SDK `02e5db2`. Contrat destiné à l’implementer ; pas une acceptation ni une preuve de livraison.
- Continuité communiquée par le parent après inspection : SDK frère `main@54d435e`, intégrant le travail JWT préexistant, `origin/main@9e9ef27` et le pin `02e5db2`, sans code secrets. L’implémentation part de cet état intégré, pas d’un retour au vieux pin ; le parent possède validation et publication.
- État livré depuis cette base : SDK publié `main@93b890ff7b821e9cb2e7d0aeb81957d098778a2f`, checkout SDK consommé et adoption GitHubConnect validée sous Windows ; publication du service/gitlink AIForAll encore en cours. [Rapport de résultats du parent](../missions/20261031-github-connect-openbao-results.md), sans modification de la mission utilisateur initiale.

## 1. Place et API SDK

**Un seul package `rustycog-framework`.** Ajouter dans `rustycog-config/src/secrets/` un module exporté `rustycog::config::secrets` sous `cfg(feature = "secrets")`. La feature `secrets` implique `config` et les dépendances techniques existantes nécessaires (`async-trait`, `reqwest`, `serde_json`, `thiserror`, `tokio` avec I/O fichier async au besoin). Ni `http`, ni DB, ni events, ni service AIForAll requis. Garder `default`/`full` inchangés ; ne pas créer de sous-crate/Cargo.toml historique. Pas de nouveau client Vault tiers obligatoire.

Surface publique V1 (signatures normatives ; organisation privée libre) :

```rust
pub struct SecretReference { /* mount, path, field privés ; serde */ }
impl SecretReference {
    pub fn new(mount: String, path: String, field: String)
        -> Result<Self, SecretError>;
}

pub struct SecretValue { /* String privée */ }
impl SecretValue {
    pub fn new(value: String) -> Result<Self, SecretError>;
    pub fn expose_secret(&self) -> &str;
    pub fn into_inner(self) -> String;
}

#[async_trait::async_trait]
pub trait SecretResolver: Send + Sync {
    async fn resolve(&self, reference: &SecretReference)
        -> Result<SecretValue, SecretError>;
}

pub struct OpenBaoConfig {
    pub endpoint: String,
    pub auth: OpenBaoAuthConfig,
    pub allow_loopback_http: bool,
    pub ca_cert_file: Option<std::path::PathBuf>,
}
pub enum OpenBaoAuthConfig { TokenFile { path: std::path::PathBuf } }
pub struct OpenBaoKv2Resolver { /* client + credential privés */ }
impl OpenBaoKv2Resolver {
    pub async fn connect(config: &OpenBaoConfig) -> Result<Self, SecretError>;
}
// OpenBaoKv2Resolver implémente SecretResolver.

pub fn load_config_fresh_strict<T>() -> Result<T, ConfigError>
where T: ConfigLoader<T>;
// Cette dernière fonction est dans rustycog::config, sous la feature config.
```

- `SecretReference` : `Deserialize` à champs exacts `mount`, `path`, `field`, tous obligatoires, `deny_unknown_fields`. La désérialisation applique la validation de `new` sans erreur contenant la valeur. Des accesseurs non secrets sont permis pour construire l’URL, pas de `Display` de référence.
- Les types déclaratifs `SecretReference`, `OpenBaoConfig`, `OpenBaoAuthConfig` peuvent être `Clone`/`Serialize` pour composer le config existant : ils ne portent aucune valeur de secret/token. Cette permission ne s’étend pas à `SecretValue`, au résolveur ou aux DTO de réponse sensibles.
- Mount/path : relatifs, non vides, segments séparés par `/`, chaque segment `[A-Za-z0-9_.-]+`, sauf `.`/`..`. Pas de slash initial/final, segment vide, `%`, backslash, query, fragment ou URL. Field : une clé littérale non vide `[A-Za-z0-9_.-]+`, **pas** JSONPath ni parcours d’objet. Chaque composant limité à 1 024 octets. Aucun format `secret:...#...` ou `${...}` à interpréter.
- `SecretValue::new` refuse chaîne vide/blanche ou >16 KiB UTF-8 ; ne modifie pas les octets. `Debug` constant expurgé ; pas de `Display`, `Serialize`, `Deref<Target=str>` ou `Clone` implicite du type. `into_inner` est le transfert explicite vers le consommateur. Pas de promesse de zeroization complète des buffers HTTP/JSON/OAuth.
- `SecretError` : variants sans payload `InvalidConfig`, `InvalidReference`, `AuthenticationFailed`, `TransportFailed`, `NotFound`, `InvalidResponse`, `InvalidValue`, `LimitExceeded`, `Timeout`. `Debug`/`Display` fixes, `source() = None`. Ne jamais y attacher URL, référence, token, corps HTTP, erreur serde/reqwest/I/O ou valeur résolue. `OpenBaoConfig`, auth, référence, résolveur et éventuels DTO contenant un token ont un `Debug` expurgé, pas un derive exposant les données brutes.
- `load_config_fresh_strict` réutilise `build_config_with_env_prefix(T::config_prefix())` et les mêmes sources/priorités (`RUN_ENV`, fichier existant, `.env`, env préfixé). Aucune acquisition réseau. Toute erreur de build **ou** désérialisation devient une `ConfigError` à message constant expurgé, sans log/source brute. Aucun `T::create_default()` dans ce parcours. Le loader legacy reste inchangé pour les autres services.

## 2. Configuration GitHubConnect, sans ambiguïté

Champs ajoutés :

- `github.secrets_mode` : `openbao` ou `direct`. Absence = état interne `Unconfigured`, refusé au boot ; inconnu = erreur stricte. Aucun choix utilisable dans `Default`.
- `github.client_secret_ref`, `github.hmac_secret_ref` : `Option<SecretReference>`.
- `openbao` : `Option<OpenBaoConfig>`, avec `endpoint: String`, `auth: OpenBaoAuthConfig` obligatoires ; `allow_loopback_http: bool = false`, `ca_cert_file: Option<PathBuf> = None`.
- `OpenBaoAuthConfig` : enum serde taggé `method`, `snake_case`, seule variante V1 `token_file` avec `path` obligatoire. Pas de variante implicite/inline, pas de valeur par défaut, champs inconnus rejetés.
- Les valeurs directes historiques `github.client_secret` / `github.hmac_secret` restent admises **seulement** en mode `direct`, expurgées en `Debug` et ignorées en sérialisation (`skip_serializing`, pas `skip_deserializing`). Ne pas utiliser la sérialisation du config comme moyen de diffuser des secrets.

Validation avant tout I/O :

- `openbao` : deux références et config OpenBao complètes ; les deux champs directs doivent être absents/strictement vides, même si une valeur provient du TOML ancien. Toute valeur directe non vide, y compris blanche, est un conflit, pas un fallback.
- `direct` : deux valeurs applicatives valides ; **aucune** référence et aucune section `openbao`. Zéro I/O coffre/token-file. Ne pas inférer ce mode de la présence des valeurs.
- En cas d’absence de mode, référence partielle, mauvais type, provider auth inconnu, fichier absent ou échec distant : refus de démarrage. Les defaults de champs non sensibles ne donnent jamais un mode utilisable.

Exemple nominal, valeurs et chemins **illustratifs**, aucun endpoint déployé implicite :

```toml
[github]
secrets_mode = "openbao"
client_id = "PUBLIC_OAUTH_CLIENT_ID"
# client_secret et hmac_secret absents ; conserver URLs/redirects/PKCE existants.

[github.client_secret_ref]
mount = "secret"
path = "github-connect/oauth"
field = "client_secret"

[github.hmac_secret_ref]
mount = "secret"
path = "github-connect/iam"
field = "hmac_secret"

[openbao]
endpoint = "https://bao.example.invalid:8200"
allow_loopback_http = false
# ca_cert_file = "C:/secrets/bao-ca.pem" # ou /run/... sous Linux

[openbao.auth]
method = "token_file"
path = "C:/secrets/github-connect.bao-token" # ou /run/... sous Linux
```

Préfixe existant `GITHUB_CONNECT`, séparateur préfixe `_`, niveaux `__` :

- `GITHUB_CONNECT_GITHUB__SECRETS_MODE=openbao`
- `GITHUB_CONNECT_GITHUB__CLIENT_SECRET_REF__MOUNT`, `__PATH`, `__FIELD` ; mêmes suffixes pour `HMAC_SECRET_REF`.
- `GITHUB_CONNECT_OPENBAO__ENDPOINT`, `GITHUB_CONNECT_OPENBAO__ALLOW_LOOPBACK_HTTP`, `GITHUB_CONNECT_OPENBAO__CA_CERT_FILE`.
- `GITHUB_CONNECT_OPENBAO__AUTH__METHOD=token_file`, `GITHUB_CONNECT_OPENBAO__AUTH__PATH`.
- Mode direct explicite : `...GITHUB__SECRETS_MODE=direct`, `...GITHUB__CLIENT_SECRET`, `...GITHUB__HMAC_SECRET`, sans autres entrées coffre/références. Ne pas recopier les valeurs dans un exemple, un log ou des arguments de processus.

Les fichiers locaux de configuration et helpers doivent être rendus explicites : fixtures directes `direct` ; exemple nominal OpenBao sans placeholders de secrets directs en conflit. Ne pas modifier Compose/ops pour compenser un ancien fichier : documenter le raccord requis.

## 3. Authentification et droits minimaux

`connect` valide d’abord la configuration puis lit **une fois** le fichier indiqué, fichier régulier, lecture bornée à 16 KiB + un octet de détection. UTF-8, un token non vide sur une ligne, accepter seulement la fin de ligne terminale LF/CRLF puis la retirer. Pas de trim silencieux des autres espaces ; refuser contrôles/whitespace interne. Pas de lecture `VAULT_TOKEN`, `BAO_TOKEN`, home/CLI token helper ou credential par défaut. Le bearer est placé dans `X-Vault-Token` marqué sensible, jamais dans une URL. Aucun fichier créé, modifié ou supprimé par le SDK.

Premier appel : `GET /v1/auth/token/lookup-self`. Exiger HTTP 200, `data.policies` tableau de strings sans `root`, `data.ttl` entier `1..=3600`. Si `identity_policies` existe, même validation et rejet de `root`. Token opaque : **aucune** heuristique par préfixe/nom. Ne pas conserver ou exposer le champ `id` retourné. TTL nulle, trop longue, champs invalides, accès refusé ou lookup indisponible => `AuthenticationFailed` (limites/timeout gardent leur catégorie).

Le fournisseur externe doit émettre un token applicatif non-root, à TTL initiale/maximale ≤1 h, accordant seulement :

```hcl
path "auth/token/lookup-self" { capabilities = ["read"] }
path "secret/data/github-connect/oauth" { capabilities = ["read"] }
path "secret/data/github-connect/iam"   { capabilities = ["read"] }
```

Adapter les chemins à mount/path configurés. Pas de `list`, écriture, `sys/*`, Transit ou administration nécessaire. **KV autorise au niveau du chemin, pas du champ** : les autres champs du même document seraient lisibles aussi ; ne pas y mélanger d’autres secrets. Le SDK ne prouve pas qu’une policy personnalisée n’accorde aucun autre droit : cette restriction et les ACL POSIX/Windows du fichier sont des préconditions externes, pas une attestation produite par `lookup-self`.

Le fichier peut être fourni par un agent ayant authentifié le workload ou par un bootstrap dédié borné. Le file sink OpenBao documente ce transport, mais son déploiement n’est pas dans ce lot. Fichier plain token seulement : pas de response wrapping/chiffrement de sink pris en charge. Pas de renouvellement, re-login, révocation ou effacement de fichier par le service. Identité/credential **toujours fourni**, jamais anonyme ; les providers workload natifs restent futurs.

## 4. Transport, lecture KV et bornes

- Endpoint = origine `https://host[:port]` (slash terminal permis), pas de userinfo, query, fragment ou base path. Refuser toute autre forme avant réseau, avec erreur expurgée. Aucun namespace OpenBao ni proxy applicatif V1.
- HTTPS : chaîne de confiance et nom d’hôte vérifiés, racines usuelles ; `ca_cert_file` ajoute un certificat/bundle PEM de confiance (fichier régulier, max 1 MiB), jamais un contournement de validation. Pas de flag insecure.
- HTTP : refus par défaut. Avec `allow_loopback_http=true`, seulement IP littérale loopback IPv4/IPv6 ; `localhost`, nom DNS résolu localement, IP privée/non-loopback, `0.0.0.0`, suffixes trompeurs interdits. L’opt-in vendor `IsolatedTest` existant n’autorise **pas** le transport OpenBao.
- Client dédié, proxies ambiants désactivés ; `redirect(Policy::none())`, tous les 3xx refusés (y compris HA). Aucun retry applicatif, découverte ou fallback d’endpoint.
- Timeout total **5 s par requête**, connexion **3 s**, incluant lecture du corps ; ne pas laisser la lecture chunked dépasser le délai. Snapshot GitHubConnect entier (fichier/auth/deux résolutions) sous un budget async **20 s**, pas de tâche de résolution détachée.
- Taille réponse **64 KiB** : contrôler `Content-Length` si présent et limiter **incrémentalement** les octets réellement lus, arrêt dès dépassement, y compris sans en-tête/chunked. Ne pas appeler `.bytes()` sans borne avant de vérifier. Même règle pour lookup et erreurs ; ne pas lire un corps d’erreur inutile. Refuser les contenus compressés non pris en charge, ne pas activer une décompression non bornée.
- `resolve` émet `GET /v1/{mount}/data/{path}` ; construire/encoder par segments validés, pas `Url::join` sur une entrée pouvant remplacer origine/chemin. Aucun paramètre `field` ni token dans l’URL ; pas de version explicite V1 : lecture de la dernière version disponible.
- Exiger HTTP 200 et JSON objet `data.data`, puis champ exact de type string respectant `SecretValue::new`. Pas de coercition nombre/bool/null/objet, ni decode base64, ni fallback KV v1. Supprimé/détruit/absent => refus ; ne jamais réutiliser une valeur antérieure.
- Mapping erreurs KV : 401/403 `AuthenticationFailed`, 404 `NotFound`, autres non-200 `TransportFailed` ; transport/TLS `TransportFailed`, timeout `Timeout`, forme JSON `InvalidResponse`, champ absent `NotFound`, mauvais type `InvalidResponse`, valeur invalide `InvalidValue`, dépassement `LimitExceeded`. Aucun corps/en-tête/URL/source brute dans la chaîne d’erreur ou le tracing.

## 5. Composition et durée de vie GitHubConnect

1. `configuration::load_config()` utilise toujours le loader strict, sans regarder le mode pour choisir un loader.
2. `Application::new(AppConfig)` et `AppBuilder::build` deviennent **async**, sans `block_on` ou runtime imbriqué. Propager `.await` à `src/main.rs`, `tests/common.rs` et aux tests appelants. Hors GitHubConnect, aucune migration de runtime/monolithe : 0408 reste applicable.
3. Le setup valide les modes et entrées avant toute acquisition, résout les deux références (séquentiel suffit), puis valide les consommateurs. OAuth secret non vide/non blanc sans transformation ; HMAC conserve **exactement** le `trim()` puis minimum **16 octets** existants. Pas de nouvelle canonicalisation HMAC, réencodage, hash ou changement de headers/fenêtre/PKCE.
4. Ajouter `GitHubConnectClient::from_config_with_resolved_secret(&GitHubConfig, SecretValue) -> Result<Self, DomainError>` : lit URLs/client_id/redirects/policy/PKCE non sensibles depuis le config ; déplace `into_inner()` dans `oauth2::ClientSecret`, sans lire ou remplir `config.client_secret`. Les constructeurs directs historiques peuvent rester comme wrappers **direct-only** ; jamais autoriser des références non résolues par ces chemins. Les tests infra utilisant ces wrappers déclarent `direct` et une valeur synthétique valide.
5. `HmacKey` reçoit le secret validé au setup. Les temporaires/credential/résolveur sont abandonnés ensuite ; pas de secret dans un `AppState` générique, une readiness, un cache ou une nouvelle propriété de config. En mode direct, retirer les deux chaînes de la config détenue (ex. `mem::take`) avant de conserver celle-ci dans `Application` ; en OpenBao, elles restent vides. Ne pas conserver un sac `ResolvedSecrets` dans `Application` en plus des consommateurs.
6. Toute erreur empêche retour d’une `Application`, bind/listener/readiness positive ou appel vendor. Après boot, aucune requête OpenBao par route, tâche de refresh ou vérification de token. Les deux lectures ne garantissent pas une version atomique commune ; une mise à jour opérationnelle exige un redémarrage coordonné et la même valeur HMAC côté IAM, **sans modifier IAM dans ce lot**.

## 6. Modules touchables, invariants et migration

- SDK : `Cargo.toml`, `rustycog-config/src/lib.rs`, nouveau `rustycog-config/src/secrets/`, tests ciblés et documentation SDK. L’export existant `src/lib.rs` peut rester identique : le nouveau module est sous `config`.
- Service : `services/GitHubConnect/{configuration,setup,infra}/`, `src/main.rs`, configurations TOML propres au service, tests/fixtures et `docs/services/github-connect.md`. Mettre à jour les dépendances **des consommateurs effectifs** de la nouvelle feature, pas l’ensemble des services.
- Aucun schéma DB/migration, route, contrat `idp-connect-contract`, FGA, IAM, GitLabConnect, Lazaret, compose, ops, E2E ou GitHub App installation/API touché.
- Commit SDK publié puis gitlink/pin/lock AIForAll correspondant. La consigne initiale de préserver sans embarquer les changements préexistants (`.serena/project.yml`, fichiers JWT du SDK frère notamment) est contextualisée par l’autorisation utilisateur ultérieure d’intégrer/publier **tout durable préexistant**, sur `AIForAll/master` et `RustyCog/main` existantes, sans renommage, en commits séparés. Préexistant `9a0a794`, merges en `54d435e`, fixtures policy `2bdbc2a`, puis SDK secrets `93b890f` ; aucun secret, log, cache ou donnée d’exécution à intégrer. La photographie 0500–0502 et les ADR Accepted ne sont pas amendées, et cette autorisation ne vaut pas acceptation de 0505.

## 7. Validation ciblée à remettre au parent

Pas de « tout cargo », pas de runtime Kind/Docker requis pour ces preuves. WireMock est autorisé à cette frontière HTTP sortante (ADR0201) ; pas de mock d’une logique interne de route. Conserver les contrats HMAC existants.

- SDK : référence/URL/auth invalides refusées avant réseau, token-file absent/vide/surdimensionné, lookup root/TTL nulle/excessive/refusé ; succès KV exact ; 403/404/JSON/type/valeur vide ; timeout, 3xx avec **zéro appel** à la cible de redirection, surdimensionné avec/sans `Content-Length`. Vérifier HTTPS requis / opt-in IP loopback strict ; cert non fiable refusé et CA explicite acceptée via fixture TLS ciblée si disponible, sinon déclarer ce dernier oracle non exécuté, jamais PASS implicite.
- Loader : mauvais type/mode inconnu/référence partielle malgré defaults et valeurs directes valides ailleurs => erreur, pas `Default`, pas réseau. Sentinelles sensibles dans erreur TOML/serde : absentes des erreurs et du tracing capturé. Tests env sérialisés/restaurés, y compris une surcharge imbriquée `GITHUB_CONNECT_...` réelle.
- Service : boot OpenBao réussi ; la requête OAuth sortante porte **le secret résolu**, et le HMAC accepte la clé résolue/rejette une autre. Un échec de l’un des deux secrets empêche application/route/appel vendor. Mode direct explicite passe sans lire de fichier ni contacter le coffre ; absence de mode/conflit direct-référence échoue. Capturer `Debug`, erreurs/chaînes/tracing et sérialisation config avec des sentinelles : aucune valeur/token. Après plusieurs appels métier et une modification de la fixture KV, compteur OpenBao inchangé et anciens secrets effectifs jusqu’au restart.
- SDK sans option réseau : `cargo check --manifest-path rustycog/Cargo.toml --no-default-features --features config`.
- SDK ciblé : `cargo test --manifest-path rustycog/Cargo.toml --no-default-features --features secrets --lib config::secrets` puis filtre dédié aux nouveaux tests `load_config_fresh_strict` (nommer les tests en conséquence).
- Service : `cargo test -p github-connect-configuration -p github-connect-infra -p github-connect-setup -p github-connect-service` ; un seul Cargo Windows à la fois, jobs=24 de la config conservés. Qualifier séparément ce qui est exécuté sous Windows et la vérification Linux disponible, sans inventer une campagne CI.

## 8. État de validation constaté — 2026-10-31

- SDK `93b890ff7b821e9cb2e7d0aeb81957d098778a2f` publié sur `main`. Selon les résultats communiqués par le parent, sous Windows : **11 tests config avec feature `secrets`** verts (ensemble incluant tests préexistants ; ne pas les présenter comme 11 nouveaux tests secrets), `check` avec `config` seule et Clippy `secrets` / `-D warnings` verts.
- Arbre GitHubConnect consommant ce checkout : packages `github-connect-{configuration,infra,setup,service}`, **17 tests verts** (`1 + 2 + 4 + 7 + 3`), dont 3 nouveaux startup. Résumés lus dans `.tmp/githubconnect-openbao/service-final.log`, artefact local non versionné. Usage OAuth effectif, HMAC `trim()` / 16 octets et absence de refresh couverts par la campagne ; aucune nouvelle exécution par l’architecte.
- **Pas de CI Linux, fixture TLS live, coffre réel ou déploiement qualifiés**. Aucun runtime/conteneur lancé pour cette mission. Les résultats ne constituent pas un PASS automatique de tous les oracles du §7 ; les limites restent explicites.
- Publication service/gitlink AIForAll encore en cours à cette actualisation ; preuves finales et références distantes sous responsabilité du parent dans le [rapport de résultats](../missions/20261031-github-connect-openbao-results.md). Ne pas confondre checkout SDK testé et gitlink committé/publié.

Architecture : sources/consommateurs/tests existants inspectés et preuves parent ci-dessus reprises ; **aucune commande Cargo, exécution runtime, commit ou push par l’architecte**. Statut **Proposed** conservé ; l’éventuel `Accepted` exige toujours un accord humain explicite.
