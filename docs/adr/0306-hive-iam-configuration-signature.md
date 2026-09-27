# ADR-0306 : Config signature org = commande synchrone Hive→IAM ; pas de secret dans les events

- Statut : Accepted
- Réalité : Partial
- Date : 2026-09-26 (amendement transport : 2026-09-27)
- Décideurs : Djoé Denne (acceptation humaine 2026-09-26 ; amendement transport accepté 2026-09-27)
- Jalon concerné : architecture actuelle / AuthN signature org (complète 0304 / 0305 ; hors P-Apparatus)
- SuperSède : aucune
- SuperSédée par : —
- Related : [0102](0102-setup-composition-root.md), [0104](0104-outbound-overrides-composition-root.md), [0304](0304-jwt-acces-plateforme-rs256-jwks.md), [0305](0305-account-identity-trust-domain.md), [0307](0307-workload-identity-port.md), [0400](0400-iamrusty-identite-hexagonale.md), [0402](0402-hive-organisations.md), [0403](0403-telegraph-notifications-event-driven.md), [0404](0404-runtime-microservices-et-monolithe.md)

`Accepted` ratifie le canal de configuration signer **et** le choix de transport au composition root (amendement 2026-09-27). `Réalité : Partial` : HTTP + metadata only + rotate N+1 livrés ; pont InProcess monolithe présent dans le working tree (setter nommé) ; gaps restants = BYOKMS + events SigningProfile*. Le motif plateforme `*OutboundOverrides` est [0104](0104-outbound-overrides-composition-root.md) (Proposed / Partial) — **pas** une SuperSède de 0306.

## Contexte

Les orgs doivent pouvoir activer une clé dédiée ou un BYOKMS ([0304](0304-jwt-acces-plateforme-rs256-jwks.md)) depuis le contexte admin Hive. La tentation est de pusher un secret dans un domain event rejouable, ou d’utiliser Telegraph / un bus de commandes. **Correction de vocabulaire** : le brief utilisateur disait parfois « Telegraf » — le service du dépôt est **Telegraph** ([0403](0403-telegraph-notifications-event-driven.md) = notifications), **pas** un canal de config KMS.

Le dual runtime ([0404](0404-runtime-microservices-et-monolithe.md)) impose deux déploiements du même port synchrone : microservices séparés (HTTP) et `oodhive-monolith` (même process).

## Décision

1. **Admin configure depuis le contexte org (UI Hive).** Hive vérifie org admin (OpenFGA / rôle Hive) **puis** appelle IAM.
2. **Pas de secret dans les domain events.** Interdit : `OrganizationKmsConfigured { secret }` (ou équivalent).
3. **Config sensible = commande / RPC synchrone s2s**, pas un event rejouable. Canal = **port synchrone applicatif**. **Aucun bus de commandes Hive→IAM n’existe.** **Pas Telegraph.**
4. **Transport choisi au composition root** : **InProcess** si Hive et IAM tournent dans le **même process** (monolithe) ; **HTTP** sinon (microservices / standalones). **Pas de gRPC obligatoire.** **Pas de crate partagée Hive+IAM** pour ce flux.
5. **Confiance par mode** :
   - **InProcess** : capability `Arc<dyn IamOrganizationSignerClient>` injectée à la construction — **pas** de token interne.
   - **HTTP** (standalone Hive→IAM, et routes `/iam/internal/...` nestées dans le monolithe) : token `x-iam-internal-token` **obligatoire**, fail-closed. Credential HTTP = WorkloadIdentity / StaticCredential ([0307](0307-workload-identity-port.md)) — **0307 n’est pas** le mode InProcess.
6. **Composition** : [0102](0102-setup-composition-root.md) — `hive-setup` reste le composition root **du service** Hive ; défaut = client HTTP. **Politique de transport inchangée** (InProcess si même process ; HTTP sinon ; capability vs token fail-closed — points 4–5). Le setter actuel `with_iam_organization_signer_client` est l’**instance** Hive→IAM (injection `Option` / `Arc`). Le **motif plateforme** (sac typé `*OutboundOverrides` / `with_outbound`) est [0104](0104-outbound-overrides-composition-root.md) — Proposed / Partial ; 0306 ne le SuperSède pas et n’est pas SuperSédée. Overlay monolithe autorisé pour adapters **cross-hexagone** : l’hôte [0404](0404-runtime-microservices-et-monolithe.md) peut connaître les deux hexagones. **InProcess vit uniquement dans `oodhive-monolith`.** Interdit : dépendances `hive-application` / `hive-infra` / `hive-setup` → crates `iam-*`.
7. **Commandes conceptuelles** (réponse synchrone) : `ConfigureOrganizationSigner`, `TestOrganizationSigner`, `RotateOrganizationSigner`, `DisableOrganizationSigner`. IAM valide provider, credentials, public key, éventuel challenge ; crée `SigningKey` + `kid` ; stocke le secret éventuel dans OpenBao.
8. **Hive ne stocke pas les secrets cloud.** Métadonnées UX seulement : `signing_profile_id`, `signing_status`. IAM stocke `SigningKey`, provider refs, `credential_ref`, issuer, `kid`, status. DB = référence ; secret dans OpenBao.
9. **Events OK sans secrets** (IAM peut projeter) : OrganizationCreated / Deleted ; MembershipAdded / Removed / Changed (**runtime actuel** : `MemberJoinedEvent`) ; OrganizationSecurityPolicyChanged ; SigningProfileActivated / Disabled.

## État runtime

`HttpIamOrganizationSignerClient` câblé par défaut dans `Hive/setup` ; routes admin Hive `POST /api/organizations/{id}/signer/{configure,test,rotate,disable}` (permission Admin). Sur succès : persist org `signing_profile_id` + `signing_status` uniquement. IAM refuse BYOKMS aws/gcp/azure (pas d’adapters). Pas de secret dans les domain events. Telegraph = notifications seulement. Les PEM d’organisation vivent sous `{keys}/organizations/{org_id}/…` ; la clé plateforme reste hors de cette racine.

`RotateOrganizationSigner` runtime : N+1 réel (Pending puis Active, ancienne Active → Retiring) ; PEM biclé sous pem_root ou Transit create+read public ; probe Transit live.

**InProcess monolithe (working tree)** : pont `monolith/src/in_process_iam_signer.rs` + câblage `monolith/src/runtime.rs` via le sucre `with_iam_organization_signer_client` (écrit le champ `iam_organization_signer` du sac `HiveOutboundOverrides`, [0104](0104-outbound-overrides-composition-root.md) Partial). Politique de transport 0306 inchangée.

**Gap Partial** : pas d’adapters BYOKMS aws/gcp/azure ; pas d’events SigningProfile*.

## Migration

Introduire le port synchrone Hive→IAM après (ou avec) SigningKeyRegistry ([0304](0304-jwt-acces-plateforme-rs256-jwks.md)). Ne jamais backfiller des secrets via events. Métadonnées UX Hive seulement. Câbler InProcess uniquement dans `oodhive-monolith` ; laisser `hive-setup` en HTTP par défaut.

## Conséquences

- IAM reste seul détenteur des refs crypto et du JWKS.
- Hive reste SoT org / membership ([0402](0402-hive-organisations.md), [0305](0305-account-identity-trust-domain.md)) ; IAM n’est pas un second membership.
- Telegraph reste hors de ce flux ([0403](0403-telegraph-notifications-event-driven.md)).
- Le monolithe peut overlayer un client InProcess sans violer 0102 côté service ni créer de crate Hive+IAM.
- WorkloadIdentity reste le credential du chemin HTTP seulement ([0307](0307-workload-identity-port.md)).

## Alternatives rejetées

| Option | Pourquoi pas |
|---|---|
| Secret dans domain event | Rejouable, fuites logs / bus, pas de challenge synchrone |
| Telegraph / bus commandes Hive→IAM | Telegraph = notifications (0403) ; aucun bus commandes n’existe |
| Hive stocke credentials cloud | Surface et SoT crypto chez IAM / OpenBao |
| Config signer via outbox seule | Besoin réponse synchrone + validation challenge |
| gRPC obligatoire | Transport = InProcess (même process) ou HTTP ; pas de troisième protocole imposé |
| Crate partagée Hive+IAM | Frontière hexagone : InProcess seulement dans l’hôte monolithe |

## Non décidé ici

- Schéma précis des events SigningProfile* vs projection IAM.

## Références

- Crypto : [0304](0304-jwt-acces-plateforme-rs256-jwks.md) ; trust : [0305](0305-account-identity-trust-domain.md)
- Composition / dual runtime : [0102](0102-setup-composition-root.md), [0404](0404-runtime-microservices-et-monolithe.md)
- Motif overrides sortants (plateforme) : [0104](0104-outbound-overrides-composition-root.md) (Proposed / Partial)
- Credential HTTP : [0307](0307-workload-identity-port.md) (Proposed — pas Accepté ici)
- Hive / Telegraph : [0402](0402-hive-organisations.md), [0403](0403-telegraph-notifications-event-driven.md)
- Preuves runtime (Partial — HTTP + InProcess setter nommé) :
  - Hive handlers : `Hive/http/src/handlers/organization_signer.rs` (`configure`/`test`/`rotate`/`disable`)
  - Client s2s HTTP : `Hive/infra/src/iam/organization_signer_client.rs` `HttpIamOrganizationSignerClient` (`x-iam-internal-token` via WorkloadIdentity/StaticCredential (`iam-internal-token`)), câblé `Hive/setup`
  - Routes IAM internes : `IAMRusty/http/src/handlers/organization_signer.rs`
  - Persist UX only : `Hive/domain/src/entity/organization.rs` (`signing_profile_id`, `signing_status`) ; pas de secret dans events
  - InProcess monolithe : `monolith/src/in_process_iam_signer.rs`, câblage `monolith/src/runtime.rs` → `with_iam_organization_signer_client`
- Gaps vs cible : pas d’adapters BYOKMS aws/gcp/azure ; pas d’events SigningProfile*
