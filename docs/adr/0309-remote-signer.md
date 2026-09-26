# ADR-0309 : Remote signer = contrat minimal Sign / GetPublicKey derrière HSM ou KMIP

- Statut : Proposed
- Réalité : Unimplemented
- Date : 2026-09-26
- Décideurs : Djoé Denne (proposition 2026-09-26 — Accept humain requis)
- Jalon concerné : architecture actuelle / AuthN SigningProvider (hors P-Apparatus)
- SuperSède : aucune
- SuperSédée par : —
- Related : [0304](0304-jwt-acces-plateforme-rs256-jwks.md), [0307](0307-workload-identity-port.md)

`Proposed` : contrat minimal pour un SigningProvider distant. `Réalité : Unimplemented` — pas de remote signer JWT dans le dépôt.

## Contexte

[0304](0304-jwt-acces-plateforme-rs256-jwks.md) autorise BYOKMS et remote signer comme `SigningProvider`. Il faut un contrat minimal sans figer le vendor HSM.

## Décision

1. **Contrat minimal** : `Sign(key_id, algorithm, digest)` / `GetPublicKey(key_id)`.
2. **Canal** : mTLS ou WorkloadIdentity ([0307](0307-workload-identity-port.md)).
3. **Derrière** : HSM, KMIP, PKCS#11 (ou équivalent). IAM construit le signing input ; le remote signer signe ; IAM assemble le JWT.
4. Le remote signer **ne** reçoit **pas** les claims en clair comme source de trust — digest seulement (aligné limitation BYOKMS 0304 §6).

## État runtime

Remote signer JWT : **ABSENT**. OpenBao Transit = Cosign Apparatus seulement.

## Migration

Après le port `SigningProvider` ([0304](0304-jwt-acces-plateforme-rs256-jwks.md) §19). Transit / PEM d’abord ; adapter remote signer optionnel. Accept humain requis.

## Conséquences

- Adapter remote signer derrière le port `SigningProvider` (0304 §19) ; pas de fuite vendor dans le domaine.

## Alternatives rejetées

| Option | Pourquoi pas |
|---|---|
| Exporter la clé privée vers IAM | Contredit HSM / non-export |
| Signer des claims sans digest figé par IAM | Contredit le modèle émetteur logique |

## Non décidé ici

- Vendor HSM / protocole KMIP concret.
- SLA et retry policy.

## Références

- [0304](0304-jwt-acces-plateforme-rs256-jwks.md) §6, §19, §22 ; [0307](0307-workload-identity-port.md)
- Preuve d’implémentation : **aucune**
