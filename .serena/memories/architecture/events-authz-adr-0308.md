Jalon : architecture actuelle / AuthN mesh.
Chemin : docs/adr/0308-mesh-authn-jwt.md
Statut : Proposed · Réalité : Unimplemented

- Gateway valide le JWT (kid, issuer, trust) puis produit (iss, sub). Pas d'appel KMS par requête.
- Headers client X-Principal-* détruits et recréés. Cache JWKS (singleflight, last-known-good).
- Runtime : pas d'ext_authz JWT ; Envoy gateway du Kind est commenté ou hors de ce rôle.

Voir le fichier ADR.
