# ADR-0306 : Config signature org = commande synchrone Hive→IAM ; pas de secret dans les events

- Statut : Accepted
- Réalité : Unimplemented
- Date : 2026-09-26
- Décideurs : Djoé Denne (acceptation humaine 2026-09-26)
- Jalon concerné : architecture actuelle / AuthN signature org (complète 0304 / 0305 ; hors P-Apparatus)
- SuperSède : aucune
- SuperSédée par : —
- Related : [0304](0304-jwt-acces-plateforme-rs256-jwks.md), [0305](0305-account-identity-trust-domain.md), [0400](0400-iamrusty-identite-hexagonale.md), [0402](0402-hive-organisations.md), [0403](0403-telegraph-notifications-event-driven.md)

`Accepted` ratifie le canal de configuration signer. `Réalité : Unimplemented` : pas de client HTTP Hive→IAM pour configurer un signer ; pas de secrets KMS dans Hive ; Telegraph = notifications seulement.

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

Pas de client HTTP Hive→IAM pour config signer. Events Hive : `OrganizationCreatedEvent`, `MemberJoinedEvent` (`hive-events`) — pas de secret KMS. Telegraph consomme `iam-events` (notifications) — pas de bus commandes. OpenBao Transit = Cosign Apparatus, pas Sign JWT user.

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
- Preuves absences : pas de client Hive→IAM config ; `hive-events` sans secret KMS ; Telegraph = `iam-events` notifications
- Preuve d’implémentation de **cette** cible : **aucune** (`Réalité : Unimplemented`)
