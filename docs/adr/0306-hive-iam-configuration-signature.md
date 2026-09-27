# ADR-0306 : Config signature org = commande synchrone Hive→IAM ; pas de secret dans les events

- Statut : Accepted
- Réalité : Implemented
- Date : 2026-09-26
- Décideurs : Djoé Denne (acceptation humaine 2026-09-26)
- Jalon concerné : architecture actuelle / AuthN signature org (complète 0304 / 0305 ; hors P-Apparatus)
- SuperSède : aucune
- SuperSédée par : —
- Related : [0304](0304-jwt-acces-plateforme-rs256-jwks.md), [0305](0305-account-identity-trust-domain.md), [0400](0400-iamrusty-identite-hexagonale.md), [0402](0402-hive-organisations.md), [0403](0403-telegraph-notifications-event-driven.md)

`Accepted` ratifie le canal de configuration signer. `Réalité : Implemented` : client HTTP Hive→IAM via WorkloadIdentity/StaticCredential + routes admin configure/test/rotate/disable + rotate N+1 + probe PEM/Transit ; persist metadata only ; pas d’adapters AWS/GCP/Azure ; pas d’events SigningProfile*.

## Contexte

Les orgs doivent pouvoir activer une clé dédiée ou un BYOKMS ([0304](0304-jwt-acces-plateforme-rs256-jwks.md)) depuis le contexte admin Hive. La tentation est de pusher un secret dans un domain event rejouable, ou d’utiliser Telegraph / un bus de commandes. **Correction de vocabulaire** : le brief utilisateur disait parfois « Telegraf » — le service du dépôt est **Telegraph** ([0403](0403-telegraph-notifications-event-driven.md) = notifications), **pas** un canal de config KMS.

## Décision

1. **Admin configure depuis le contexte org (UI Hive).** Hive vérifie org admin (OpenFGA / rôle Hive) **puis** appelle IAM.
2. **Pas de secret dans les domain events.** Interdit : `OrganizationKmsConfigured { secret }` (ou équivalent).
3. **Config sensible = commande / RPC synchrone s2s**, pas un event rejouable. Canal = **port synchrone applicatif** (RPC / HTTP s2s). **Aucun bus de commandes Hive→IAM n’existe.** **Pas Telegraph.**
4. **Commandes conceptuelles** (réponse synchrone) : `ConfigureOrganizationSigner`, `TestOrganizationSigner`, `RotateOrganizationSigner`, `DisableOrganizationSigner`. IAM valide provider, credentials, public key, éventuel challenge ; crée `SigningKey` + `kid` ; stocke le secret éventuel dans OpenBao.
5. **Hive ne stocke pas les secrets cloud.** Métadonnées UX seulement : `signing_profile_id`, `signing_status`. IAM stocke `SigningKey`, provider refs, `credential_ref`, issuer, `kid`, status. DB = référence ; secret dans OpenBao.
6. **Events OK sans secrets** (IAM peut projeter) : OrganizationCreated / Deleted ; MembershipAdded / Removed / Changed (**runtime actuel** : `MemberJoinedEvent`) ; OrganizationSecurityPolicyChanged ; SigningProfileActivated / Disabled.

## État runtime

`HttpIamOrganizationSignerClient` câblé dans `Hive/setup` ; routes admin Hive `POST /api/organizations/{id}/signer/{configure,test,rotate,disable}` (permission Admin). Sur succès : persist org `signing_profile_id` + `signing_status` uniquement. IAM refuse BYOKMS aws/gcp/azure (pas d’adapters). Pas de secret dans les domain events. Telegraph = notifications seulement. Les PEM d’organisation vivent sous `{keys}/organizations/{org_id}/…` ; la clé plateforme reste hors de cette racine.

`RotateOrganizationSigner` runtime : N+1 réel (Pending puis Active, ancienne Active → Retiring) ; PEM biclé sous pem_root ou Transit create+read public ; probe Transit live.

## Migration

Introduire le port synchrone Hive→IAM après (ou avec) SigningKeyRegistry ([0304](0304-jwt-acces-plateforme-rs256-jwks.md)). Ne jamais backfiller des secrets via events. Métadonnées UX Hive seulement.

## Conséquences

- IAM reste seul détenteur des refs crypto et du JWKS.
- Hive reste SoT org / membership ([0402](0402-hive-organisations.md), [0305](0305-account-identity-trust-domain.md)) ; IAM n’est pas un second membership.
- Telegraph reste hors de ce flux ([0403](0403-telegraph-notifications-event-driven.md)).

## Alternatives rejetées

| Option | Pourquoi pas |
|---|---|
| Secret dans domain event | Rejouable, fuites logs / bus, pas de challenge synchrone |
| Telegraph / bus commandes Hive→IAM | Telegraph = notifications (0403) ; aucun bus commandes n’existe |
| Hive stocke credentials cloud | Surface et SoT crypto chez IAM / OpenBao |
| Config signer via outbox seule | Besoin réponse synchrone + validation challenge |

## Non décidé ici

- Forme exacte du transport s2s (HTTP path, gRPC, mTLS) — préfère WorkloadIdentity ([0307](0307-workload-identity-port.md)).
- Schéma précis des events SigningProfile* vs projection IAM.

## Références

- Crypto : [0304](0304-jwt-acces-plateforme-rs256-jwks.md) ; trust : [0305](0305-account-identity-trust-domain.md)
- Hive / Telegraph : [0402](0402-hive-organisations.md), [0403](0403-telegraph-notifications-event-driven.md)
- Preuves runtime (Implemented) :
  - Hive handlers : `Hive/http/src/handlers/organization_signer.rs` (`configure`/`test`/`rotate`/`disable`)
  - Client s2s : `Hive/infra/src/iam/organization_signer_client.rs` `HttpIamOrganizationSignerClient` (`x-iam-internal-token` via WorkloadIdentity/StaticCredential (`iam-internal-token`)), câblé `Hive/setup`
  - Routes IAM internes : `IAMRusty/http/src/handlers/organization_signer.rs`
  - Persist UX only : `Hive/domain/src/entity/organization.rs` (`signing_profile_id`, `signing_status`) ; pas de secret dans events
- Gaps vs cible : pas d’adapters BYOKMS aws/gcp/azure ; pas d’events SigningProfile*
