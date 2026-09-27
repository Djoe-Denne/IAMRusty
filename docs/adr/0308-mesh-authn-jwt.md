# ADR-0308 : Mesh / gateway AuthN JWT — valide trust puis émet (iss, sub)

- Statut : Accepted
- Réalité : Partial
- Date : 2026-09-26 (acceptation humaine 2026-09-27)
- Décideurs : Djoé Denne (acceptation humaine 2026-09-27)
- Jalon concerné : architecture actuelle / AuthN mesh (hors P-Apparatus)
- SuperSède : aucune
- SuperSédée par : —
- Related : [0304](0304-jwt-acces-plateforme-rs256-jwks.md), [0305](0305-account-identity-trust-domain.md), [0307](0307-workload-identity-port.md)

`Accepted` ratifie ext_authz (pas filtre JWT Envoy, pas sidecar). `Réalité : Partial` : crate `ext-authz/` (Check HTTP, strip+recreate `X-Principal-Iss`/`Sub`, cache JWKS + poll, `aud` obligatoire) ; overlay Envoy Kind **toujours commenté** ; mTLS absent ; staleness chiffrée Non décidé. **Pas Implemented.**

## Contexte

Après RS256 + JWKS + issuer par trust domain ([0304](0304-jwt-acces-plateforme-rs256-jwks.md)), la validation doit pouvoir vivre au gateway / mesh (pas seulement dans chaque service). Le produit cible reste un service **ext_authz HTTP** branché sur Envoy ; l’overlay Kind n’est pas encore activé.

## Décision

1. Gateway / mesh valide : `alg`, `kid`, signature, `typ`, `iss`, `aud`, `exp`, `iat`, `nbf` si présent, trust scope, `kid`→issuer, `kid`→org, key status — puis produit le principal `(iss, sub)`.
2. **Headers client `X-Principal-*`** : supprimés à l’entrée et **recréés** par le mesh (jamais confiance client).
3. **mTLS / isolation** entre hops (détail WorkloadIdentity = [0307](0307-workload-identity-port.md)).
4. **Cache JWKS** comme [0304](0304-jwt-acces-plateforme-rs256-jwks.md) §17 (pas de fetch par requête ; singleflight ; negative cache ; last-known-good).
5. **Produit** : service **ext_authz HTTP** (pas filtre JWT Envoy, pas sidecar). Overlay Envoy Kind reste **commenté** jusqu’à activation mesh.
6. **Propagation révocation clé** : cache JWKS comme [0304](0304-jwt-acces-plateforme-rs256-jwks.md) §17 ; **staleness maximale chiffrée** reste Non décidé (0304 §17 n’a pas de TTL numérique).

## État runtime

**Partial** — livré dans le dépôt, pas branché mesh :

- Crate `ext-authz/` : HTTP Check (`POST /`, `POST /check`) ; strip de tout `X-Principal-*` client puis recreate `x-principal-iss` / `x-principal-sub` ; cache JWKS process-local + poll fond ; `EXT_AUTHZ_AUDIENCE` obligatoire (fail-closed boot).
- Overlay `deploy/apps/overlays/kind/helmrelease-envoy-gateway.yaml` : **COMMENTÉ** (pas de policy JWT Envoy, pas de wiring ext_authz en Kind).
- **Gaps** : overlay commenté ; mTLS absent ; staleness chiffrée révocation = Non décidé (0304 §17).

## Migration

Validation JWT reste aussi dans rustycog-http tant que le mesh n’est pas activé. Overlay Envoy Kind **reste commenté**. Ne pas déployer un ext_authz mesh sans décommenter / câbler l’overlay.

## Conséquences

- Consommateurs mesh ne parlent pas au KMS ; JWKS IAM seulement.
- Révocation clé ≠ révocation session ([0304](0304-jwt-acces-plateforme-rs256-jwks.md) §14–§15).

## Alternatives rejetées

| Option | Pourquoi pas |
|---|---|
| Faire confiance aux `X-Principal-*` client | Spoofing trivial |
| Fetch JWKS / KMS par requête | Latence + failure domain |
| Filtre JWT Envoy | Hors contrat : produit = ext_authz HTTP |
| Sidecar AuthN | Hors contrat : produit = ext_authz HTTP |

## Non décidé ici

- Valeur chiffrée de max staleness révocation (0304 §17 sans TTL numérique).
- Mécanisme exact push vs poll vs TTL cache (poll présent dans `ext-authz` ; TTL numérique max Non décidé).

## Références

- [0304](0304-jwt-acces-plateforme-rs256-jwks.md) §14, §17 ; [0305](0305-account-identity-trust-domain.md)
- Overlay : `deploy/apps/overlays/kind/helmrelease-envoy-gateway.yaml` (commenté)
- Preuve Partial : `ext-authz/` (`src/check.rs`, `src/jwks_cache.rs`, `src/config.rs`, `src/main.rs` ; tests Check / strip / aud / poll dans `check.rs`)
