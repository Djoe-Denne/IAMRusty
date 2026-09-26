# ADR-0308 : Mesh / gateway AuthN JWT — valide trust puis émet (iss, sub)

- Statut : Proposed
- Réalité : Unimplemented
- Date : 2026-09-26
- Décideurs : Djoé Denne (proposition 2026-09-26 — Accept humain requis)
- Jalon concerné : architecture actuelle / AuthN mesh (hors P-Apparatus)
- SuperSède : aucune
- SuperSédée par : —
- Related : [0304](0304-jwt-acces-plateforme-rs256-jwks.md), [0305](0305-account-identity-trust-domain.md), [0307](0307-workload-identity-port.md)

`Proposed` : contrat mesh pour la validation JWT une fois 0304 livré côté émetteur / extracteurs. `Réalité : Unimplemented`.

## Contexte

Après RS256 + JWKS + issuer par trust domain ([0304](0304-jwt-acces-plateforme-rs256-jwks.md)), la validation doit pouvoir vivre au gateway / mesh (pas seulement dans chaque service). Aujourd’hui Envoy Gateway overlay kind est **commenté** ; pas d’ext_authz JWT ; pas de headers `X-Principal-*`.

## Décision

1. Gateway / mesh valide : `alg`, `kid`, signature, `typ`, `iss`, `aud`, `exp`, `iat`, `nbf` si présent, trust scope, `kid`→issuer, `kid`→org, key status — puis produit le principal `(iss, sub)`.
2. **Headers client `X-Principal-*`** : supprimés à l’entrée et **recréés** par le mesh (jamais confiance client).
3. **mTLS / isolation** entre hops (détail WorkloadIdentity = [0307](0307-workload-identity-port.md)).
4. **Cache JWKS** comme [0304](0304-jwt-acces-plateforme-rs256-jwks.md) §17 (pas de fetch par requête ; singleflight ; negative cache ; last-known-good).
5. **Propagation révocation clé** : documenter une **staleness maximale** acceptable ; mécanisme exact (push vs poll vs TTL cache) à figer à l’Accept / implémentation — ne pas inventer un ext_authz déjà livré.

## État runtime

`deploy/apps/overlays/kind/helmrelease-envoy-gateway.yaml` **COMMENTÉ**. Pas de policy JWT Envoy. Pas d’ext_authz JWT. Headers `X-Principal-*` : **ABSENT**.

## Migration

Tant que [0304](0304-jwt-acces-plateforme-rs256-jwks.md) est Unimplemented, la validation reste dans rustycog-http (HS256). Le mesh n’est pas inventé comme livré ; Accept + implémentation après JWKS consommateur.

## Conséquences

- Consommateurs mesh ne parlent pas au KMS ; JWKS IAM seulement.
- Révocation clé ≠ révocation session ([0304](0304-jwt-acces-plateforme-rs256-jwks.md) §14–§15).

## Alternatives rejetées

| Option | Pourquoi pas |
|---|---|
| Faire confiance aux `X-Principal-*` client | Spoofing trivial |
| Fetch JWKS / KMS par requête | Latence + failure domain |

## Non décidé ici

- Produit exact (Envoy ext_authz vs filtre JWT vs sidecar).
- Valeur chiffrée de max staleness révocation.

## Références

- [0304](0304-jwt-acces-plateforme-rs256-jwks.md) §14, §17 ; [0305](0305-account-identity-trust-domain.md)
- Overlay : `deploy/apps/overlays/kind/helmrelease-envoy-gateway.yaml` (commenté)
- Preuve d’implémentation : **aucune**
