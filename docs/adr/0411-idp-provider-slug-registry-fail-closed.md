# ADR-0411 : Le provider IdP est un slug typé ; le registry boot est le catalogue ouvert ; l’admission est fail-closed

- Statut : Proposed
- Réalité : Unimplemented
- Date : 2026-09-22
- Décideurs : (à remplir à l’acceptation)
- Jalon concerné : IAM-IdP (hors Apparatus P0–P6)
- SuperSède : aucune — gradue le leftover 0407 §8 et le « long terme » 0408 sans réécrire 0407–0410
- SuperSédée par : —

`Accepted` ratifiera la cible. `Réalité` restera `Unimplemented` tant que l’enum fermé, le skip boot et `validate` incomplet sont dans le dépôt.

## Contexte

0407 §8 autorisait l’enum `Provider { GitHub, GitLab }` comme *adapter v1* des routes `/api/auth/{provider}` et du schéma SQL. 0408 promettait : IdP N+1 = ligne `[[idp.connectors]]` + service HTTP, **sans recompiler** le domaine IAM. 0410 a conservé les routes et annoncé **422** si le slug est hors registry.

Le code n’honore pas encore cette cible :

- enum fermé + `FromStr` à deux bras (`IAMRusty/domain/src/entity/provider.rs`) ;
- HTTP : `PROVIDER_REGEX` = `^[a-zA-Z]+$` **et** liste hardcodée `github`/`gitlab` (`IAMRusty/http/src/validation.rs` `validate_provider_name`) ;
- boot : slugs hors GH/GL **silencieusement ignorés** (`IAMRusty/setup/src/app.rs` `setup_http_idp_clients`, `_ => continue`) ;
- `IdpConfig::validate` n’exige ni `base_url` ni les deux `redirect_uris` (`#[serde(default)]` dans `IAMRusty/configuration/src/idp.rs`) ;
- lecture tokens : `Provider::from_str(&model.provider).unwrap_or(Provider::GitHub)` (`IAMRusty/infra/src/repository/token.rs`, `token_read.rs`).

SQL persisté = déjà le slug minuscule (`as_str`, colonne `String`). Les DTO OAuth HTTP n’exposent pas l’enum serde PascalCase. Le leftover était assumé au closeout S5/S6.

Hors sujet de cette ADR : wasm/plugin, SDK vendor in-process, marketplace, nest monolith, Apparatus/cloud, livrer Hugging Face / Atlassian / Google.

## Décision

1. **`Provider` devient un newtype slug**, nom inchangé. Canon : `^[a-z]+$`, longueur 1–50. Parse HTTP/config : `^[a-zA-Z]+$` puis **case-fold** (`GitHub` → `github`). Stockage, HashMap, serde, SQL : canon minuscule. Pas de chiffres ni tirets en v1 (`huggingface`, `atlassian` tiennent).
2. **Catalogue = `[[idp.connectors]]` au boot.** `setup` construit `HashMap<Provider, Arc<dyn FederatedOAuthClient>>` (pas `HashMap<String, _>`). Toute **ligne complète** est wirée. Un 3ᵉ IdP **ne recompile pas** `iam-domain`.
3. **Ligne complète** = `id` charset-ok, `base_url` non vide, `hmac_secret` ≥ 16 octets, `redirect_uris` résolvant **Callback et Relink**. Registry vide, id dupliqué (case-fold), ligne incomplète ou id illégal = **échec de boot**. Pas de skip silencieux. Pas de flag `enabled`. Pas d’auto-déclaration Connect.
4. **Admission requête.** Syntaxe invalide → **400** `invalid_provider`. Slug syntaxiquement ok absent du registry → **422** `connector_not_configured` (0410). Connect down → échec de **requête**, pas de désinscription du slug.
5. **Onboarding N+1** = copier le gabarit `GitHubConnect/` + service compose + ligne registry + HMAC partagé + dual `redirect_uris` (IAM choisit, Connect refuse si `!=`). Secrets vendor sur Connect. IAM garde routes, CSRF `OAuthState`, linking, JWT `iss=iamrusty`. **Hot-load : non.** Restart IAM pour une ligne registry ; Connect-only si secret vendor et url/hmac IAM inchangés ; rotation HMAC ou `redirect_uris` = les deux process.
6. **Inchangé :** forme des routes (0410), colonne SQL string, 0409 (HMAC `METHOD\nPATH\nTIMESTAMP\nBODY`, allowlist exacte, pas de JWT user vers Connect, pas d’OpenFGA au login OAuth).

Hors décision : nest monolith (reste 0408) ; OIDC Id Token (0409) ; Hive org-sync ; élargissement charset.

## Conséquences

- Plus d’enum fermé ni de `validate_provider_name` GH/GL. Plus de `unwrap_or(GitHub)`.
- Serde `Provider` : slug minuscule (aligné `as_str`). Pas un breaking des URLs OAuth. `Provider` n’est plus `Copy` (`Clone` suffit).
- `PROVIDER_FACTORY_GUIDE` : onboarding = gabarit Connect + registry, pas une variante domaine.
- Travail : domain / configuration / setup / http / infra tokens + IT start/callback/error_mapping + test boot fail-closed. Pas de migration SQL de type.

## Alternatives rejetées

| Option | Pourquoi pas (maintenant) |
|---|---|
| Hot-load / watch config | Coût (courses OAuth in-flight, HMAC, admission hors `validate` boot). Restart IAM/Connect est acceptable |
| `^[a-z][a-z0-9-]{0,31}$` et parse `GitHub` KO | Sort de la regex HTTP actuelle ; HF/Atlassian tiennent en lettres ; IT case-insensitive à conserver |
| `HashMap<String, _>` | Le newtype est la porte unique de parse une fois l’enum retiré |
| Skip silencieux des slugs « inconnus » | Contredit le catalogue ouvert ; masque une ligne mal formée |
| Flag `enabled` | Double vérité avec la présence au registry |
| wasm / plugin / SDK in-process / marketplace / mega-connector / nest / HF maintenant | Rejetés par le jalon ; 0408 tient N déploiements |

## Non décidé ici

- Date d’un 3ᵉ Connect concret (Hugging Face, Atlassian, Google).
- Charset élargi si un slug vendor réel exige un tiret ou un chiffre.
- Nest des Connect dans `oodhive-monolith` (0408).
- mTLS mesh-wide, chiffrement at-rest `provider_tokens` (0409).

## Contrat d’implémentation (slices, pas de code)

0407–0410 restent Accepted / Implemented. 0411 est **Proposed / Unimplemented** jusqu’à Accept humain puis code.

### Hors scope

- Wasm, plugin loader, SDK vendor in-process, marketplace, mega-connector.
- Nest monolith, Apparatus, cloud 0600+.
- Créer `HuggingFaceConnect` / Atlassian / Google.
- Affaiblir 0409.

### Slices ordonnés

1. **Identité** — Newtype `Provider` : parse/fold/charset ; serde slug ; tuer l’enum et `unwrap_or(GitHub)` (erreur de mapping, jamais GitHub).
2. **Completitude boot** — `IdpConfig::validate` : `base_url` non vide + les deux `redirect_uris` résolues ; id charset ; unique case-fold ; HMAC ≥ 16 ; registry non vide.
3. **Wiring** — `setup_http_idp_clients` parse chaque `id` → `Provider` ; wirer toutes les lignes ; **zéro** `continue` silencieux.
4. **HTTP** — `ProviderPath` syntaxe seulement ; drop liste `github`/`gitlab` ; match handlers → parse + lookup map ; **400** charset ; **422** absent.
5. **Docs** — `PROVIDER_FACTORY_GUIDE` = copie gabarit + compose + `[[idp.connectors]]` (exceptions 0408). Plus « unknown slugs skipped ».

### Validation (pas « tout cargo »)

- `iam-configuration` : `validate` refuse `redirect_uris` partiel / `base_url` vide.
- `iam-setup` : ligne `id=huggingface` complète **boote** ; id illégal / HMAC court / registry vide **refusent**.
- `iam-http` `error_mapping` + `validation` : syntaxe 400 ; slug hors registry 422.
- `auth_oauth_start` : case-insensitive `GitHub` ; facebook/google hors registry → 422.
- Infra tokens : row `gitlab` / `bitbucket` **≠** GitHub.
- IT existants github/gitlab restent verts. HMAC 0409 inchangé.

## Références

- ADR : [0407](0407-contrat-authn-federee-vendor-neutral.md) §8, [0408](0408-connecteurs-idp-services-http.md) (registry + long terme), [0409](0409-confiance-callback-oauth-idp-connect.md), [0410](0410-migration-iam-connecteurs-idp.md) (routes + 422)
- Code : `IAMRusty/domain/src/entity/provider.rs`, `IAMRusty/configuration/src/idp.rs`, `IAMRusty/setup/src/app.rs` (`setup_http_idp_clients`), `IAMRusty/http/src/validation.rs`, `IAMRusty/http/src/handlers/auth.rs`, `IAMRusty/http/src/idp_registry.rs`, `IAMRusty/infra/src/repository/token.rs`, `token_read.rs`, gabarit `GitHubConnect/`
- Guide : `IAMRusty/docs/PROVIDER_FACTORY_GUIDE.md`
- Preuve d’implémentation : `aucune`
