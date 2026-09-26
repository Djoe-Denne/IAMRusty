# ADR-0307 : WorkloadIdentity = port conceptuel ; SPIFFE évalué, pas dépendance obligatoire

- Statut : Proposed
- Réalité : Unimplemented
- Date : 2026-09-26
- Décideurs : Djoé Denne (proposition 2026-09-26 — Accept humain requis)
- Jalon concerné : architecture actuelle / AuthN s2s & cloud WIF (hors P-Apparatus)
- SuperSède : aucune
- SuperSédée par : —
- Related : [0304](0304-jwt-acces-plateforme-rs256-jwks.md), [0306](0306-hive-iam-configuration-signature.md), [0308](0308-mesh-authn-jwt.md), [0309](0309-remote-signer.md), [0601](0601-cluster-trust-namespaces-standalones.md)

`Proposed` : recommandation de frontière. **Pas** une dépendance de [0304](0304-jwt-acces-plateforme-rs256-jwks.md). `Réalité : Unimplemented` : SPIFFE / SPIRE **ABSENT** du runtime (mentions doc seulement, ex. 0601 « pas V1 »).

## Contexte

IAM doit appeler des KMS / remote signers et Hive doit appeler IAM en s2s ([0306](0306-hive-iam-configuration-signature.md)) sans clés de compte de service en clair. SPIFFE/SPIRE est un candidat fort pour fédération cloud et mTLS mesh, mais l’imposer comme dépendance bloquerait 0304.

## Décision

1. **Port conceptuel `WorkloadIdentity`** (pas dépendance SPIFFE). Le domaine dépend du port ; l’adapter choisit le mécanisme.
2. **Évaluer** SPIFFE/SPIRE (ex. `spiffe://aiforall/iam/iamrusty`, JWT-SVID / X509-SVID) pour fédération cloud, s2s, Hive→IAM, OpenBao auth, mTLS mesh.
3. **WIF** AWS / GCP / Azure : préféré. Pas de `service-account-key.json` requis. Ne pas confondre *service account* et *clé privée de SA*.
4. Ordre de préférence credentials (aligné 0304 §19) : OIDC WIF, X509/mTLS, static (fallback OpenBao).

## État runtime

SPIFFE/SPIRE : **ABSENT**. Mentions documentaires seulement. Pas d’implémentation du port `WorkloadIdentity`.

## Migration

Le port peut avancer avec WIF / static / PEM sans SPIRE. L’évaluation SPIFFE n’est pas un prérequis de [0304](0304-jwt-acces-plateforme-rs256-jwks.md). Accept humain requis avant de figer un produit.

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
- Preuve d’absence SPIFFE runtime : dépôt (doc only) — **aucune** implémentation
