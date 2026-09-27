# ADR-0305 : HumanAccount ≠ Identity ; principal = (iss, sub) ; trust platform vs organization-managed

- Statut : Accepted
- Réalité : Implemented
- Date : 2026-09-26
- Décideurs : Djoé Denne (acceptation humaine 2026-09-26)
- Jalon concerné : architecture actuelle / AuthN identité (complète 0302 / 0304 ; hors P-Apparatus)
- SuperSède : aucune
- SuperSédée par : —
- Related : [0302](0302-authn-jwt-authz-openfga.md), [0304](0304-jwt-acces-plateforme-rs256-jwks.md), [0306](0306-hive-iam-configuration-signature.md), [0400](0400-iamrusty-identite-hexagonale.md), [0402](0402-hive-organisations.md)

`Accepted` ratifie le modèle Account / Identity / Trust. `Réalité : Implemented` : table `identities` + `ensure_platform_identity` sur login/refresh ; `ensure_organization_managed_identity` + RPC interne IAM ; Hive membership porte `issuer` depuis `JwtPrincipal.iss` ; pas d’UX switch d’identity ni second JWT org.

## Contexte

Aujourd’hui un `sub` UUID user sert à la fois d’identité AuthN et de clé de membership Hive (`OrganizationMember.user_id`). La cible crypto [0304](0304-jwt-acces-plateforme-rs256-jwks.md) introduit des trust domains et des issuers distincts ; un `user_id` global ne suffit plus comme principal. Il faut séparer personne UX, identité authentifiable, et membership org — sans faire d’IAM un second registre de membership.

## Décision

1. **HumanAccount** = personne / UX (préférences, avatar, notifs, récupération, identities liées). **Pas** le principal AuthZ.
2. **Identity** = identité authentifiable dans **un** trust domain (`issuer` + `subject` + email de ce domain).
3. **Principal** = `(iss, sub)` — jamais `sub` seul ([0304](0304-jwt-acces-plateforme-rs256-jwks.md) §10).
4. **1 HumanAccount → N Identities → N trust domains.** Linking contrôlé par IAM ; jamais unilatéral par l’org.
5. **Platform-managed** : **une** PlatformIdentity peut être membre de **plusieurs** orgs. Pas une identity par org.
6. **Organization-managed / BYOKMS** : OrganizationManagedIdentity **distincte** par trust domain. Switch d’identity = opération **explicite** quand le trust domain change — ≠ changer d’org OpenFGA.
7. **Terminologie** : *platform-managed trust* vs *organization-managed trust*. UI : « Security managed by AIForAll » / « Security managed by your organization ». **Pas** « public / private organization ».
8. **Hive** = SoT Organization, Membership, rôles, admins, invitations. IAM n’est **pas** un second membership. Membership **devra** désigner le principal `(iss, sub)` ; `user_id` global seul ne suffit plus.
9. **IAM** = HumanAccount / Identity, sessions, refresh, émission JWT, SigningKeyRegistry, providers, credentials, JWKS, trust domains, linkage ([0304](0304-jwt-acces-plateforme-rs256-jwks.md), [0306](0306-hive-iam-configuration-signature.md)).
10. **Révocation membership ≠ révocation de clé.** OpenFGA retire les droits ; refresh révocable ; access JWT reste valide jusqu’à `exp` sauf mécanisme stateful. `auth_version` / `session_version` = Non décidé / ADR future.
11. **Multi-org platform-managed** ne réémet **pas** un JWT par org.

## État runtime

IAM : table `identities` ; `SeaOrmIdentityRepository::ensure_platform_identity` câblé sur login + refresh (`platform_issuer = {public_base_url}/iam`). Hive : `OrganizationMember.issuer` + lookups `find_by_organization_issuer_and_user` ; handlers extraient `JwtPrincipal.iss`. Events `MemberJoined` / `Removed` / `RolesUpdated` portent `issuer: Option<String>`. Sentinel-sync continue `user:{uuid}` (ignore issuer). OrganizationManagedIdentity persistée via RPC interne ; pas de JWT org ni linking multi-domain UX.

## Migration

Membership Hive porte `(iss, sub)` ; tokens `iss=iamrusty` = platform historique jusqu’au cutoff issuer ([0304](0304-jwt-acces-plateforme-rs256-jwks.md)). OrganizationManagedIdentity est persistée (RPC interne) ; linking UX et schéma FGA multi-issuer restent Non décidé.

## Conséquences

- OpenFGA Check continue de porter l’AuthZ ; claim `org` (si présent) = contexte trust, pas permission ([0304](0304-jwt-acces-plateforme-rs256-jwks.md) §11).
- Changer d’org UI (tuples FGA) ≠ switch d’identity (trust domain).
- Config signer org = [0306](0306-hive-iam-configuration-signature.md), pas du membership.

## Alternatives rejetées

| Option | Pourquoi pas |
|---|---|
| Identity = HumanAccount | Mélange UX et principal AuthN |
| Une PlatformIdentity par org | Contredit multi-org platform-managed sans réémission JWT |
| IAM second membership | Hive = SoT org/membres (0402) |
| « Public / private organization » | Confond UX marketing et trust crypto |
| Membership reste `user_id` seul à la cible | Principal = `(iss, sub)` dès issuers multiples |

## Non décidé ici

- `auth_version` / `session_version` / deny-list access.
- UX exacte du switch d’identity et du linking.
- Schéma FGA détaillé pour principals multi-issuer.

## Références

- Crypto / issuer : [0304](0304-jwt-acces-plateforme-rs256-jwks.md)
- Config signer : [0306](0306-hive-iam-configuration-signature.md)
- IdP / Hive : [0400](0400-iamrusty-identite-hexagonale.md), [0402](0402-hive-organisations.md)
- AuthN/AuthZ : [0302](0302-authn-jwt-authz-openfga.md)
- Preuves runtime (Implemented) :
  - Table `identities` : `IAMRusty/migration/src/m20220101_000003_identities.rs` ; `IAMRusty/infra/src/repository/entity/identities.rs`
  - `ensure_platform_identity` sur login/refresh : `IAMRusty/application/src/usecase/{login,token}.rs`, `IAMRusty/infra/src/repository/identity_repository.rs`
  - Membership `(iss, sub)` : `Hive/domain/src/entity/organization_member.rs` `OrganizationMember.issuer` ; handlers `Hive/http/src/handlers/members.rs` (`JwtPrincipal.iss`) ; `find_by_organization_issuer_and_user`
  - Events issuer optionnel : `hive-events/src/member.rs` (`MemberJoinedEvent`, `MemberRemovedEvent`, `MemberRolesUpdatedEvent`)
  - Unit : `IAMRusty/domain/tests/identity_ensure.rs`, `Hive/domain/tests/member_issuer.rs`
  - FGA reste `user:{uuid}` : `sentinel-sync/src/fga_client.rs`
- Gaps vs cible : linking multi-domain, switch d’identity UX, mint JWT org
