# ADR-0307 : WorkloadIdentity = port conceptuel ; SPIFFE évalué, pas dépendance obligatoire

- Statut : Accepted
- Réalité : Implemented
- Date : 2026-09-26 (acceptation humaine 2026-09-27)
- Décideurs : Djoé Denne (acceptation humaine 2026-09-27)
- Jalon concerné : architecture actuelle / AuthN s2s & cloud WIF (hors P-Apparatus)
- SuperSède : aucune
- SuperSédée par : —
- Related : [0304](0304-jwt-acces-plateforme-rs256-jwks.md), [0306](0306-hive-iam-configuration-signature.md), [0308](0308-mesh-authn-jwt.md), [0309](0309-remote-signer.md), [0601](0601-cluster-trust-namespaces-standalones.md)

`Accepted` ratifie le port conceptuel `WorkloadIdentity` (pas dépendance SPIFFE). `Réalité : Implemented` = adapters OIDC WIF HTTP + factory `compose_workload_identity` ; pas SPIFFE ; pas X509 mesh ([0308](0308-mesh-authn-jwt.md)).

## Contexte

IAM doit appeler des KMS / remote signers et Hive doit appeler IAM en s2s ([0306](0306-hive-iam-configuration-signature.md)) sans clés de compte de service en clair. SPIFFE/SPIRE est un candidat fort pour fédération cloud et mTLS mesh, mais l’imposer comme dépendance bloquerait 0304.

## Décision

1. **Port conceptuel `WorkloadIdentity`** (pas dépendance SPIFFE). Le domaine dépend du port ; l’adapter choisit le mécanisme.
2. **Évaluer** SPIFFE/SPIRE (ex. `spiffe://aiforall/iam/iamrusty`, JWT-SVID / X509-SVID) pour fédération cloud, s2s, Hive→IAM, OpenBao auth, mTLS mesh.
3. **WIF** AWS / GCP / Azure : préféré. Pas de `service-account-key.json` requis. Ne pas confondre *service account* et *clé privée de SA*.
4. Ordre de préférence credentials (aligné 0304 §19) : OIDC WIF, X509/mTLS, static (fallback OpenBao).

## État runtime / écart

- Port `WorkloadIdentity` + factory `compose_workload_identity` branchés sur le chemin **HTTP** Hive s2s (`HttpIamOrganizationSignerClient` / `iam-internal-token`) et IAM Transit / probe.
- Adapters OIDC WIF AWS / GCP / Azure dans `iam-infra` et `hive-infra` ; `StaticCredential` si provider absent ; fail-closed si config WIF incomplète ou provider inconnu.
- **InProcess monolithe ([0306](0306-hive-iam-configuration-signature.md))** n’utilise **pas** WorkloadIdentity ni token interne : capability `Arc<dyn IamOrganizationSignerClient>` (setter `with_iam_organization_signer_client`). **0307 = credential du chemin HTTP seulement**.
- Pas SPIFFE/SPIRE ; pas X509 mesh. Non décidé SPIRE vs WIF natif inchangé.

## Migration

Le port peut avancer avec WIF / static / PEM sans SPIRE. L’évaluation SPIFFE n’est pas un prérequis de [0304](0304-jwt-acces-plateforme-rs256-jwks.md).

## Conséquences

- 0304 / 0306 peuvent avancer avec StaticCredential / PEM / Transit sans attendre SPIRE.
- Accept de cette ADR ne force pas SPIFFE en prod V1.

## Alternatives rejetées

| Option | Pourquoi pas |
|---|---|
| SPIFFE dépendance obligatoire de 0304 | Runtime absent ; bloque la cible JWT |
| SA key JSON comme chemin nominal | Fuite de clés ; WIF préféré |

## Non décidé ici

- Choix produit SPIRE vs cloud WIF natif vs les deux.
- Identités SPIFFE concrètes et politique d’émission.

## Références

- [0304](0304-jwt-acces-plateforme-rs256-jwks.md) §19–§21 ; [0306](0306-hive-iam-configuration-signature.md) ; [0601](0601-cluster-trust-namespaces-standalones.md)
- Preuves (Implemented — adapters WIF HTTP + factory ; pas SPIFFE ; pas X509 mesh) :
  - Port IAM : `IAMRusty/domain/src/port/signing.rs` (`WorkloadIdentity`, `WorkloadCredential` + `Debug` rédigé)
  - Port Hive : `Hive/domain/src/port/service.rs` (`WorkloadIdentity`, `WorkloadCredential` + `Debug` rédigé)
  - Factory + adapters IAM : `IAMRusty/infra/src/signing/wif/{mod,aws,gcp,azure}.rs` (`compose_workload_identity`) ; static : `IAMRusty/infra/src/signing/static_credential.rs`
  - Factory + adapters Hive : `Hive/infra/src/iam/wif/{mod,aws,gcp,azure}.rs` (`compose_workload_identity`) ; static : `Hive/infra/src/iam/static_credential.rs`
  - Composition : `Hive/setup/src/app.rs`, `IAMRusty/setup/src/app.rs`
  - IT wiremock : `Hive/infra/tests/wif_exchanges.rs` (`http_iam_signer_uses_wif_resolved_token`) ; `IAMRusty/infra/tests/wif_exchanges.rs` (`transit_sign_uses_wif_resolved_token`) ; fixtures `Hive/tests/fixtures/wif/`, `IAMRusty/tests/fixtures/wif/`
  - Fail-closed / static : tests unitaires dans `*/wif/mod.rs` (AWS+GCP incomplets, provider unknown, static si absent)
  - Garde-fou : `IAMRusty/infra/tests/signing_provider_ports.rs` `no_spiffe_spire_binary_in_tree`
- Gaps : pas SPIFFE/SPIRE ; pas X509 mesh ([0308](0308-mesh-authn-jwt.md)) ; choix produit SPIRE vs WIF natif non décidé
