# ADR-0309 : Remote signer = contrat minimal Sign / GetPublicKey derrière HSM ou KMIP

- Statut : Accepted
- Réalité : Partial
- Date : 2026-09-26 (acceptation humaine 2026-09-27)
- Décideurs : Djoé Denne (acceptation humaine 2026-09-27)
- Jalon concerné : architecture actuelle / AuthN SigningProvider (hors P-Apparatus)
- SuperSède : aucune
- SuperSédée par : —
- Related : [0304](0304-jwt-acces-plateforme-rs256-jwks.md), [0307](0307-workload-identity-port.md)

`Accepted` ratifie le contrat Sign/GetPublicKey ; vendor HSM reste Non décidé. `Réalité : Partial` : adapter HTTP `RemoteSigningProvider` (digest-only Sign, GetPublicKey, WorkloadIdentity, URL fail-closed) + tests wiremock ; **pas** de HSM/KMIP réel. **Pas Implemented.**

## Contexte

[0304](0304-jwt-acces-plateforme-rs256-jwks.md) autorise BYOKMS et remote signer comme `SigningProvider`. Il faut un contrat minimal sans figer le vendor HSM.

## Décision

1. **Contrat minimal** : `Sign(key_id, algorithm, digest)` / `GetPublicKey(key_id)`.
2. **Canal** : mTLS ou WorkloadIdentity ([0307](0307-workload-identity-port.md)).
3. **Derrière** : HSM, KMIP, PKCS#11 (ou équivalent). IAM construit le signing input ; le remote signer signe ; IAM assemble le JWT.
4. Le remote signer **ne** reçoit **pas** les claims en clair comme source de trust — digest seulement (aligné limitation BYOKMS 0304 §6).

## État runtime

**Partial** — adapter HTTP livré, vendor HSM absent :

- `IAMRusty/infra/src/signing/remote.rs` : `RemoteSigningProvider` impl `SigningProvider` — `sign_digest` → `POST {url}/sign` (digest only) ; `GET {url}/keys/{key_id}` ; auth via `WorkloadIdentity` ; URL / `key_id` vides = fail-closed.
- Preuve IT : `IAMRusty/infra/tests/remote_signer.rs` (+ fixtures `IAMRusty/tests/fixtures/remote_signer/`) wiremock.
- OpenBao Transit = Cosign Apparatus / chemin Transit IAM existant — **pas** un HSM remote JWT.
- **Gaps** : vendor HSM / protocole KMIP concret = Non décidé ; pas de HSM réel en IT.

## Migration

Après le port `SigningProvider` ([0304](0304-jwt-acces-plateforme-rs256-jwks.md) §19). Transit / PEM d’abord ; adapter remote HTTP optionnel déjà présent ; brancher un vendor HSM quand choisi.

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
- Preuve Partial : `IAMRusty/infra/src/signing/remote.rs` ; `IAMRusty/infra/tests/remote_signer.rs` ; fixtures `IAMRusty/tests/fixtures/remote_signer/`
