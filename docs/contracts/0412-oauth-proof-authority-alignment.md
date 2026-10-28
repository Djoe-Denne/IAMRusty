# Contrat ratifié — conserver l'autorité IAM 8080 et les redirects navigateur 18080

- Statut : **Accepted / Unimplemented**, ratification explicite du lead le 2026-10-28 sous délégation utilisateur après lecture complète de la proposition architecte v2.
- Auteur structurel : architecte, proposition `729d653cb9506c96ce5a811b1c6b598f41cc5174`, succédant à la proposition v1 `40e6d86`. La décision ci-dessous reprend sa v2 ; sources et proposition complète préservées dans sa branche.
- Canon : ADR0304/0308/0412 Accepted / Partial, ADR0305 Accepted / Implemented, ADR0314 Accepted / Unimplemented, ADR0606 Proposed / Partial. Aucun changement de leurs Statuts par cette ratification.
- Entrées : [alignement navigateur](0412-oauth-local-full-alignment.md), [frontière AuthZ](authz-trust-domain-boundary.md). Le raccordement IAM indépendant est maintenant intégré au `c98b6ea`, SDK publié `72845d9` sélectionné ; ces faits postérieurs à la proposition n'en changent pas la décision.

## 1. Décision et alternatives examinées

**Navigateur réellement sur 18080, mais conserver `jwt.public_base_url =
"http://127.0.0.1:8080"` et l'autorité plateforme exacte
`http://127.0.0.1:8080/iam`.** Les callbacks 18080 sont enregistrés explicitement
dans les allowlists IAM et GitLabConnect. Les policies restent explicites
8080/false. Aucune nouvelle API séparant origine et issuer.

Les sources discriminantes de l'architecte sont :

1. `JwtConfig::platform_issuer()` dérive `{public_base_url}/iam` ; `jwt.issuer` décrit le legacy HS256 et ne remplace pas cette dérivation RS256.
2. `OAuthBrowserPolicy::new` conserve seulement `local_http` et `allow_http_authorization`, pas l'origine ou le port. En local_insecure, 8080 et 18080 sélectionnent la même classe de cookie. Le navigateur reçoit le cookie host-only sur 127.0.0.1 ; les cookies n'ont pas de scope de port.
3. Les handlers sélectionnent les redirects depuis `IdpConfig.redirect_uri(slug, flow)`, pas depuis public_base_url. La chaîne allowlistée est liée à la transaction writer et vérifiée exactement par le connecteur.
4. Envoy route ces chemins avec `domains: ["*"]`, sans exigence de Host8080 ni réécriture à introduire.

L'option coordonnant public_base_url, autorité et toutes policies à 18080 est
cohérente techniquement, mais change inutilement les identités plateforme/org,
les métadonnées JWK et les bindings persistés. Elle n'est pas retenue. L'option
public_base_url18080 avec issuer RS2568080 via jwt.issuer est inopérante.
Alias8080↔18080, double grant, Host/JWKS comme policy, fallback ou nouvelle API
produit restent exclus. Aucune nouvelle ADR et aucun Accept de0606.

## 2. Valeurs exactes du profil opt-in

Fragments de `oauth-proof/iam.toml`, à compléter sans remplacer les secrets
privés par des valeurs factices :

```toml
[security]
mode = "local_insecure"

[jwt]
public_base_url = "http://127.0.0.1:8080"
audience = "aiforall"
jwks_url = "http://iam:8080/iam/.well-known/jwks.json"
allowed_algorithms = ["RS256"]

[auth.jwt]
platform_issuer = "http://127.0.0.1:8080/iam"
allow_historical_platform_issuer = false
audience = "aiforall"
jwks_url = "http://iam:8080/iam/.well-known/jwks.json"
allowed_algorithms = ["RS256"]

[[idp.connectors]]
id = "gitlab"
base_url = "https://gitlab-connect:8443/gitlab-connect"
redirect_uris = [
  "http://127.0.0.1:18080/iam/api/auth/gitlab/callback",
  "http://127.0.0.1:18080/iam/api/auth/gitlab/relink-callback",
]
pkce_supported = true
```

Même tableau `redirect_uris` dans `[gitlab]` du profil GitLabConnect et même
paire8080/false dans son `[auth.jwt]`. HMAC fourni par entrée privée séparée,
jamais loggé. Un seul redirect par flow ; le registry choisit par suffixe, donc
l'oracle du profil doit contrôler les deux chaînes complètes. Ni localhost,
ni8080, ni slash final, ni alias dans ces redirects.

Roots rendus et sentinel-sync partagent8080/false. Réutiliser les valeurs du lot
identités sans ajouter des overrides identiques par principe. Si nécessaire,
variables YAML string exactes :

```text
HIVE_AUTH__JWT__PLATFORM_ISSUER=http://127.0.0.1:8080/iam
HIVE_AUTH__JWT__ALLOW_HISTORICAL_PLATFORM_ISSUER=false
TELEGRAPH_AUTH__JWT__PLATFORM_ISSUER=http://127.0.0.1:8080/iam
TELEGRAPH_AUTH__JWT__ALLOW_HISTORICAL_PLATFORM_ISSUER=false
MANIFESTO_AUTH__JWT__PLATFORM_ISSUER=http://127.0.0.1:8080/iam
MANIFESTO_AUTH__JWT__ALLOW_HISTORICAL_PLATFORM_ISSUER=false
LAZARET_AUTH__JWT__PLATFORM_ISSUER=http://127.0.0.1:8080/iam
LAZARET_AUTH__JWT__ALLOW_HISTORICAL_PLATFORM_ISSUER=false
SENTINEL_SYNC_PRINCIPAL__PLATFORM_ISSUER=http://127.0.0.1:8080/iam
SENTINEL_SYNC_PRINCIPAL__ALLOW_HISTORICAL_PLATFORM_ISSUER=false
```

Ne pas changer l'issuer `aiforall-platform` des snapshots/grants Manifesto↔Lazaret
(autre audience), ni ajouter un monolithe/GitHubConnect à la preuve. Ext-authz
conserve son URL JWKS HTTPS interne, audience aiforall, poll2s et trust TLS ; son
helper AuthN pur ne possède pas de policy FGA et ne la déduit pas du JWKS.

## 3. État, invariants et portée

Les issuers signés, JWK plateforme et binding restent8080. Les JWK organisation
conservent leur issuer et owner exacts ; aucune réécriture globale du JWKS.
Les URLs JWKS internes explicites restent inchangées, jamais remplacées par un
loopback de Pod sur18080. Le navigateur ne visite pas8080 pour valider l'issuer.

Aucune migration, rotation, suppression ou rebinding n'est autorisé par ce choix.
Un état vivant incompatible, ambigu ou de propriété inconnue reste fermé ou
INCONCLUSIVE ; aucun changement de kid/row/event/tuple pour contourner un conflit.
La réconciliation0314 déjà ratifiée demeure nécessaire. L'absence de fenêtre
historique (false) n'interdit pas les autorités organisation reconnues et leurs
propres grants.

HTTP loopback navigateur18080→Envoy, mTLS/SAN internes précis, IAM→vrai
GitLabConnect HTTPS/HMAC, IdP simulé18081, PKCE S256 et callback sans JWT restent
fixés. Aucun Secure cookie revendiqué en HTTP ni qualification production.
Signature/version autoritatives, révocation et fraîcheur maximale60s même en
panne, JWKS vide sans résurrection, quotas et refus sans effets restent.

## 4. Oracles nommés avant exécution

Réutiliser les suites/fixtures existantes ; sélectionner selon le diff, sans
créer une nouvelle plateforme ni exécuter mécaniquement tous ces noms :

- `oauth_proof_keeps_8080_authority_and_exact_18080_redirects` : configs réellement rendues, deux chaînes exactes uniques, policies8080/false, JWKS internes et défaut inchangés, aucun secret imprimé.
- `oauth_registry_redirect_is_independent_of_platform_public_base_url` : registry18080 sélectionné inchangé malgré base8080 ; détecter doublons et profil erroné.
- `oauth_browser_local_http_policy_preserves_host_only_cookie_across_configured_ports` : même classe de cookie local en8080/18080, Verified strict ; cette unit ne prouve pas le cookie jar d'un moteur.
- `iam_8080_policy_seed_and_emitted_tokens_preserve_existing_authority` : réutiliser seeded_verifier_configuration et ses négatifs, writer/JWK/seed/token concordants, binding compatible inchangée.
- `mesh_8080_authority_rejects_18080_claim_under_same_key` : témoin RS2568080 contre faux claim18080 sous même clé, refus avant IAM ; ne pas bannir les issuers org reconnus.
- `platform_binding_mismatch_never_triggers_automatic_rebind` : conserver le refus bootstrap incompatible, pas de rotation/kid de secours.
- `oauth-browser-P1-P2-P3-N1-N2-N3` : moteur sur18080, callbacks exacts, token réellement émis avec iss8080 àP2 et utilisé seulement au START P3, cookie jar réel et callback sans JWT via Envoy.

Aucun Cargo/test/rendu/runtime n'a été exécuté par ce lot architecte. IT finales
avant Kind ; vrai Envoy bon/mauvais SAN même CA exigé séparément. Aucune preuve
client reqwest ou stand-in ne le remplace.

## 5. Sources et ratification

Sources examinées par architecte : IAM configuration `JwtConfig::{platform_issuer,
organization_issuer,effective_jwks_url,http_verifier_auth}`, `idp.rs` ; setup
`setup_jwt`, capture du seed et binding ; HTTP `OAuthBrowserPolicy`, handlers
START/callback/relink et `idp_registry` ; GitLabConnect `gitlab.rs` authorize et
exchange ; domaines/routes Envoy ; renderer, policies et configs local-full ;
ext-authz check/config et sentinel-sync config ; SDK72845d9 lu par objet Git.
Les nouveaux corps des worktrees non mergés ne sont pas couverts par l'index
canonique. Sources détaillées conservées dans la proposition architecte v2.

Ratification lead : option8080 retenue pour préserver exactement l'autorité
existante tout en permettant le vrai navigateur18080 avec le registry déjà
supporté. L'exigence contraire du contrat secondaire §2 est amendée dans le même
lot. Native team_merge a refusé l'intégration en présence du changement étranger
`rustycog/.serena/project.yml` : ce changement est préservé, sans stash/reset ni
merge forcé. Décision architecte transcrite ici par patch documentaire contrôlé,
branche originale conservée ; aucune identité de contenu intégral avec le
document Proposed n'est revendiquée.
