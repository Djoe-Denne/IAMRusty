# ADR-0309 Remote signer (events/authz)

- Canon : `docs/adr/0309-remote-signer.md`
- Statut : Accepted — Réalité : Partial (**pas Implemented**)
- Preuve : `IAMRusty/infra/src/signing/remote.rs` + tests wiremock `remote_signer` — Sign digest-only, GetPublicKey, WorkloadIdentity, URL fail-closed
- Gaps : vendor HSM / KMIP Non décidé ; pas de HSM réel
- Contrat minimal Sign / GetPublicKey ; IAM assemble le JWT
- Voir le fichier ADR.
