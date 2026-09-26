Jalon : architecture actuelle / AuthN SigningProvider.
Chemin : docs/adr/0309-remote-signer.md
Statut : Proposed · Réalité : Unimplemented

- Contrat minimal Sign(key_id, algorithm, digest) et GetPublicKey, canal mTLS ou workload identity.
- Derrière : HSM, KMIP, PKCS#11. Pas un dépendance de 0304.
- Runtime : aucun remote signer JWT.

Voir le fichier ADR.
