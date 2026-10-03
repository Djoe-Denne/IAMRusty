# ADR-0308 Mesh AuthN JWT (events/authz)

- Canon : `docs/adr/0308-mesh-authn-jwt.md` points 6 et 9. Chiffres aussi rappelés dans 0304 §17.
- Statut Accepted, Réalité Partial. Pas Implemented.
- Refresh JWKS décidé 2026-10-02 : 60 s défaut staging/prod (`EXT_AUTHZ_JWKS_POLL_SECS`, défaut code encore 300), 2 s local/Kind/tests (Compose mesh déjà à 2). Staleness max après révocation : 60 s. Pas de push.
- S2S token/revoke verrouillé, pas codé : cert envoy-mesh + en-tête interne, pas de JWT utilisateur, mesh-client direct = 401.
- E2e = Kind seul. IT hors Kind. Gitlink 2290d45 non committé (401 hors SAN dans le working tree).
- Voir `mem:architecture/mesh-e2e-compose-pitfalls`.
