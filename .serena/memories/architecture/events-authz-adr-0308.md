# ADR-0308 Mesh AuthN JWT (events/authz)

- Canon : `docs/adr/0308-mesh-authn-jwt.md`. Note de suite : `docs/plans/2026-10-01-mesh-authn-reste-a-faire.md`.
- Statut : Accepted — Réalité : Partial (**pas Implemented**). Ne pas passer à Implemented tant que Kind, Flux, routes publiques IAM et S2S ne sont pas fermés. Pas de TTL numérique de staleness (0304 §17).
- Produit = ext_authz HTTP (pas filtre JWT Envoy, pas sidecar). `failure_mode_allow: false`. Lua strip `x-principal-*` avant Check. `allowed_upstream_headers` = iss/sub seulement.
- Décision §7 (confiance passerelle) : `auth.mesh.trusted_gateway_san` non vide → le service lit `x-principal-iss` / `x-principal-sub` seulement si le certificat client mTLS a ce SAN DNS. Bearer ignoré. Vide = JWT in-process. Posséder la clé `envoy-mesh` permet d'usurper n'importe quel principal.
- Compose opt-in (`deploy/mesh/`) : routes `/iam/` `/hive/` `/telegraph/` `/manifesto/`, mTLS, overlay `TLS_REQUIRE_CLIENT_CERT` + `TRUSTED_GATEWAY_SAN=envoy-mesh` sur les quatre services.
- rustycog `2290d45` (`mesh_principal`, `middleware_auth`) poussé. Gitlink AIForAll pointé sur ce SHA, **non commité**. Sans ce gitlink, `auth.mesh` ne compile pas.
- E2E `bash scripts/mesh-authn-e2e.sh` : exit 0, 48 OK, 0 FAIL (2026-10-01). Telegraph 200 ne prouve pas le `sub`. Hive vérifie `owner_user_id`, pas l'`issuer` du membre.
- Kind `kind-mesh` : encore un cluster `backend` (préfixe `/`), pas §7. Flux Envoy commenté. Mesh non défaut. Kind = projet suivant (`compose_then_kind`).
- Routes `.might_be_authenticated()` restent anonymes hors Envoy. Signup/login/JWKS ne passent pas par Envoy. Hive → IAM `/internal/*` sur :8080 répond 401 en mode mesh.
- Ne pas copier `auth.mesh` sur l'extracteur HS256 du grant snapshot Manifesto.
- Voir `mem:architecture/mesh-e2e-compose-pitfalls`.
