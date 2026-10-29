# Authentification JWT

Deux configurations distinctes. Les confondre est la principale source d’erreur.

## Contrat runtime (aujourd’hui)

Access JWT plateforme = **RS256 + `kid` opaque + JWKS**. Principal AuthN = `(iss, sub)`. `aud` = `aiforall`.

### Émetteur (IAMRusty seulement)

Section **`[jwt]`** : comment IAM **signe** les access tokens.

L'exemple PEM ci-dessous est exclusivement dev/test explicitement isolé. En
production, la crypto privée est déléguée aux providers selon0311, jamais chargée
par défaut dans IAM ; ce n'est pas une recette de production ratifiée.

```toml
[jwt]
expiration_seconds = 900
refresh_token_expiration_seconds = 2592000
public_base_url = "http://127.0.0.1:8080"
jwks_url = "http://127.0.0.1:8080/iam/.well-known/jwks.json"
allowed_algorithms = ["RS256"]
audience = "aiforall"
oauth_state_secret = "…"

[jwt.secret]
type = "pem_file"
private_key_path = "config/keys/test-platform.pem"
public_key_path = "config/keys/test-platform.pub"
key_id = "test-rs256-kid-01"
```

- Claims émis : `sub` (UUID), `iss` (`{public_base_url}/iam`), `aud`, `exp`, `iat`, `jti` ; header `typ=aiforall-access+jwt`, `alg=RS256`, `kid`.
- Signature via `SigningProvider` ; IAM conserve claims/header/assemblage et registry publique. PEM réservé dev/test explicites ; production délègue les opérations privées selon0311, sans fallback PEM/autre clé en panne. Registry, version/public/issuer/trust et fence final après attente du signer restent autoritatifs.
- Rotation org = N+1 : clé Pending publiée dans le JWKS avant toute signature ; `can_sign` = Active seulement.
- Probe Transit = challenge `sign_digest` + verify PKCS1v15 (URL Transit = config IAM, refuse fermé sans URL).
- Identity org-managed persistée (RPC interne) sans mint de JWT org ; login/refresh restent platform.
- Hive s2s → IAM via `WorkloadIdentity` / `StaticCredential` (`iam-internal-token`).
- JWKS : `GET /iam/.well-known/jwks.json` depuis `SigningKeyRegistry::list_jwks_keys` (pending/active/retiring admissibles selon leur fenêtre). **Un état vide autoritatif publie `keys:[]` et retire la confiance ; aucun fallback bootstrap ni résurrection de clé révoquée.** Un registry initialisé vide n'est pas un bootstrap jamais initialisé. Jamais HMAC dans JWKS.
- Login / refresh appellent `ensure_platform_identity(user_id, platform_issuer)`.

**Autorité du mint OAuth :** les deux `TokenService` du domaine reçoivent issuer
et audience explicitement depuis la même configuration que le codec. Aucun
DEFAULT legacy en production, Host/redirect/clé comme autorité ou relaxation
RS256. Transaction/cookie/PKCE et consommation writer avant échange restent
inchangés ; échec après consommation exige un nouveau démarrage. En laboratoire,
issuer8080/iam et redirects18080 restent volontairement distincts.

**Révocation et durées :** un refresh opaque révoqué ne peut plus renouveler ;
rotation/révocation ne déconnecte pas instantanément tous les access JWT émis.
Ils restent soumis à exp et aux contrôles de confiance/retrait de clés. **60s
est l'âge maximal de confiance depuis le dernier snapshot autoritatif validé**,
même en panne : pas TTL d'access, délai garanti de logout ou purgeOAuth, ni
prolongation du cache par hit/refresh échoué. Un JWKS vide valide retire les clés
du snapshot reçu ; aucune propagation réseau instantanée promise.

**Signataire distant :** fournisseur HTTP existant supporté ; HTTPS authentifié
obligatoire en `Verified`, exception HTTP seulement `LocalInsecure`/`IsolatedTest`
vers `127.0.0.1` ou `[::1]` littéraux. Zéro redirection/proxy ambiant, réponses
bornées, pas fallback transport/provider/clé. Voir le [contrat transport](../contracts/remote-signer-secure-transport.md).
Tests config/protocole/TLS qualifiés dans leur portée, **pas cloud/HSM/production**.
La preuve SAN d'Envoy utilise ses oracles séparés, pas ce seul client HTTP.

Guide historique (non canon) : [`services/IAMRusty/docs/JWT_CONFIGURATION_GUIDE.md`](../../services/IAMRusty/docs/JWT_CONFIGURATION_GUIDE.md). Recette consommateur : [../guides/jwt-consommateur.md](../guides/jwt-consommateur.md).

### Consommateurs (Hive, Manifesto, Telegraph, extractor IAM)

Section **`[auth.jwt]`** :

```toml
[auth.jwt]
audience = "aiforall"
jwks_url = "http://127.0.0.1:8080/iam/.well-known/jwks.json"
allowed_algorithms = ["RS256"]
```

`UserIdExtractor` ([`rustycog/rustycog-http/src/jwt_handler.rs`](../../rustycog/rustycog-http/src/jwt_handler.rs)) :

- Vérifie **RS256** via JWKS (`kid` + `iss` du JWK). Insert `JwtPrincipal { iss, sub, org }` dans les extensions.
- Runtime prod/dev : `allowed_algorithms=["RS256"]`. HS256 n'est pas le défaut — uniquement via le flag explicite `allowed_algorithms` (fenêtre IT `test.toml`).
- Claims : `sub` UUID, `exp`, `iat`, `jti` ; RS256 exige aussi `typ=aiforall-access+jwt` et `kid`.

Principal vérifié = **(iss,sub)** ; org est contexte de trust, pas permission.
Checker/cache conservent l'autorité exacte selon0314. Un extracteur UUID-only
est une vue plateforme validée, jamais un flattening d'identité organisation.
Un principal incompatible ne devient pas anonyme ; pas bannissement global des
identités organisation.

En test HS256 fenêtre : `create_jwt_token(user_id)` pose encore `iss=iamrusty` (backfill membership Hive). Préférer le `iss` du token (`JwtPrincipal`) plutôt que hardcoder `iamrusty` dans les services.

### Tableau runtime

| Élément | Valeur attendue |
|---|---|
| `iss` plateforme | `{public_base_url}/iam` |
| `aud` | `aiforall` |
| Algorithme access | RS256 |
| JWKS | `/iam/.well-known/jwks.json` |

## Cible (ADR-0304)

Ratifiée : [ADR-0304](../adr/0304-jwt-acces-plateforme-rs256-jwks.md). Identity : [0305](../adr/0305-account-identity-trust-domain.md). Config signer Hive→IAM : [0306](../adr/0306-hive-iam-configuration-signature.md) (transport au composition root : InProcess monolithe / HTTP micro). WorkloadIdentity : [0307](../adr/0307-workload-identity-port.md) (Accepted / Implemented) — adapters OIDC WIF AWS/GCP/Azure sur le chemin HTTP (preuve wiremock `wif_exchanges`) avec fallback `StaticCredential` si provider absent. Mesh AuthN [0308](../adr/0308-mesh-authn-jwt.md) (`workers/ext-authz/` Check HTTP) : profil opt-in (`docker compose --profile mesh`, overlay `kind-mesh/`) ; JWT rustycog in-process reste le défaut ; Accepted / Partial (pas Implemented). Remote signer HTTP [0309](../adr/0309-remote-signer.md) : Accepted / Partial (pas Implemented).

## Suite

Qualification2026-10-29 : voir [état qualifié](../contracts/20261029-auth-security-qualified-state.md).
0412 exact-route est Implemented dans le laboratoire après ratification explicite.
Les autres ADR gardent leurs états propres, notamment0308/0309 ; aucune promotion
transitive ou qualification cloud/HA déduite.

- [../guides/jwt-consommateur.md](../guides/jwt-consommateur.md)
