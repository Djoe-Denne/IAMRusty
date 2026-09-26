Jalon : architecture actuelle / AuthN mesh.
Chemin : docs/adr/0308-mesh-authn-jwt.md
Statut : Proposed · Réalité : Unimplemented

- Mesh/gateway valide alg/kid/sig/typ/iss/aud/exp/iat/nbf/trust/kid→issuer puis produit (iss, sub).
- Strip + recreate X-Principal-*. Cache JWKS = 0304 §17. Staleness max révocation clé à figer.
- Runtime : helmrelease-envoy-gateway COMMENTÉ ; pas d’ext_authz JWT.

Voir le fichier ADR.