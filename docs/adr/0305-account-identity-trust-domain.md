# ADR-0305 : HumanAccount ≠ Identity ; principal = (iss, sub) ; trust platform vs organization-managed

- Statut : Accepted
- Réalité : Unimplemented
- Date : 2026-09-26
- Décideurs : Djoé Denne (acceptation humaine 2026-09-26)
- Jalon concerné : architecture actuelle / AuthN identité (complète 0302 / 0304 ; hors P-Apparatus)
- SuperSède : aucune
- SuperSédée par : —
- Related : [0302](0302-authn-jwt-authz-openfga.md), [0304](0304-jwt-acces-plateforme-rs256-jwks.md), [0306](0306-hive-iam-configuration-signature.md), [0400](0400-iamrusty-identite-hexagonale.md), [0402](0402-hive-organisations.md)

`Accepted` ratifie le modèle Account / Identity / Trust. `Réalité : Unimplemented` : Hive stocke encore `user_id` seul ; IAM n’expose pas encore HumanAccount / Identity / trust domains au sens de cette ADR.

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

Hive : `OrganizationMember { organization_id, user_id, … }` (`Hive/domain/src/entity/organization_member.rs`) ; routes `/hive` … `/members/{user_id}` ; events `OrganizationCreatedEvent`, `MemberJoinedEvent` (pas `MembershipAdded`). IAM : `TokenClaims` sans claim `org` ; `iss=iamrusty` unique. Pas de modèle HumanAccount / Identity / trust domain au sens de cette ADR.

## Migration

Membership Hive devra porter `(iss, sub)` ; tokens `iss=iamrusty` = platform historique jusqu’au cutoff issuer ([0304](0304-jwt-acces-plateforme-rs256-jwks.md)). Linking et OrganizationManagedIdentity = après SigningKey / issuer par domain.

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
- Preuve runtime membership : `Hive/domain/src/entity/organization_member.rs` ; handlers `Hive/http/src/handlers/members.rs` ; events `hive-events` (`OrganizationCreatedEvent`, `MemberJoinedEvent`)
- Preuve d’implémentation de **cette** cible : **aucune** (`Réalité : Unimplemented`)
