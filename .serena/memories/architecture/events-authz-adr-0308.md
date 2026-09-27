# ADR-0308 Mesh AuthN JWT (events/authz)

- Canon : `docs/adr/0308-mesh-authn-jwt.md`
- Statut : Accepted — Réalité : Partial (**pas Implemented**)
- Preuve : crate `ext-authz/` — Check HTTP, strip+recreate `X-Principal-Iss`/`Sub`, cache JWKS + poll, `aud` obligatoire
- Gaps : overlay Envoy Kind toujours commenté ; mTLS absent ; staleness chiffrée Non décidé
- Produit = ext_authz HTTP (pas filtre JWT Envoy, pas sidecar)
- Voir le fichier ADR.
