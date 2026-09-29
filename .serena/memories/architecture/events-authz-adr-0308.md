# ADR-0308 Mesh AuthN JWT (events/authz)

- Canon : `docs/adr/0308-mesh-authn-jwt.md`
- Statut : Accepted — Réalité : Partial (**pas Implemented**)
- Preuve : crate `ext-authz/` — Check HTTP, strip+recreate `X-Principal-Iss`/`Sub`, cache JWKS + poll, `aud` obligatoire ; stand-in `tests/mesh_path_standin.rs`
- Opt-in : `docker compose --profile mesh` ; overlay séparé `deploy/apps/overlays/kind-mesh/` (pas dans `kind/`)
- Gaps : overlay Flux Kind toujours commenté ; mTLS absent ; mesh non défaut ; staleness chiffrée Non décidé
- Produit = ext_authz HTTP (pas filtre JWT Envoy, pas sidecar)
- Voir le fichier ADR.
