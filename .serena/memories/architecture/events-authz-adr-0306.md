# ADR-0306 digest

- Jalon : Hive→IAM configuration signature
- Chemin : `docs/adr/0306-hive-iam-configuration-signature.md`
- Statut : Accepted
- Réalité : Implemented

- Configure / test / rotate N+1 / disable sync ; Hive metadata only.
- Probe PEM + Transit ; pas de secrets dans events ; pas SigningProfile* events.
- Client Hive s2s via WorkloadIdentity/StaticCredential.
- Voir le fichier ADR.