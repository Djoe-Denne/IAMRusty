Jalon : architecture actuelle / AuthN SigningProvider distant.
Chemin : docs/adr/0309-remote-signer.md
Statut : Proposed · Réalité : Unimplemented

- Contrat minimal Sign(key_id, algorithm, digest) / GetPublicKey. Canal mTLS ou WorkloadIdentity.
- Derrière : HSM, KMIP, PKCS#11. IAM assemble le JWT. Remote signer ABSENT du dépôt.

Voir le fichier ADR.