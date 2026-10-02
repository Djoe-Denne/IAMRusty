# ADR-0308 : Mesh / gateway AuthN JWT — valide trust puis émet (iss, sub)

- Statut : Accepted
- Réalité : Partial
- Date : 2026-09-26 (acceptation humaine 2026-09-27)
- Décideurs : Djoé Denne (acceptation humaine 2026-09-27)
- Jalon concerné : architecture actuelle / AuthN mesh (hors P-Apparatus)
- SuperSède : aucune
- SuperSédée par : —
- Related : [0304](0304-jwt-acces-plateforme-rs256-jwks.md), [0305](0305-account-identity-trust-domain.md), [0307](0307-workload-identity-port.md)

`Accepted` ratifie ext_authz (pas filtre JWT Envoy, pas sidecar). `Réalité : Partial` : crate `ext-authz/` (Check HTTP, strip+recreate `X-Principal-Iss`/`Sub`, cache JWKS + poll, `aud` obligatoire) ; profil Compose / overlay Kind **opt-in** (`--profile mesh`, `kind-mesh/`) ; mTLS `platform-mesh` sur chaque hop du profil Compose ; services IAM/Hive/Telegraph/Manifesto en mode passerelle (§7) dans ce profil ; overlay Flux Envoy Kind **toujours commenté** ; staleness chiffrée Non décidé. **Pas Implemented.**

## Contexte

Après RS256 + JWKS + issuer par trust domain ([0304](0304-jwt-acces-plateforme-rs256-jwks.md)), la validation doit pouvoir vivre au gateway / mesh (pas seulement dans chaque service). Le produit cible reste un service **ext_authz HTTP** branché sur Envoy ; l’overlay Kind n’est pas encore activé.

## Décision

1. Gateway / mesh valide : `alg`, `kid`, signature, `typ`, `iss`, `aud`, `exp`, `iat`, `nbf` si présent, trust scope, `kid`→issuer, `kid`→org, key status — puis produit le principal `(iss, sub)`.
2. **Headers client `X-Principal-*`** : supprimés à l’entrée et **recréés** par le mesh (jamais confiance client).
3. **mTLS / isolation** entre hops (détail WorkloadIdentity = [0307](0307-workload-identity-port.md)).
4. **Cache JWKS** comme [0304](0304-jwt-acces-plateforme-rs256-jwks.md) §17 (pas de fetch par requête ; singleflight ; negative cache ; last-known-good).
5. **Produit** : service **ext_authz HTTP** (pas filtre JWT Envoy, pas sidecar). Overlay Envoy Kind reste **commenté** jusqu’à activation mesh.
6. **Propagation révocation clé** : cache JWKS comme [0304](0304-jwt-acces-plateforme-rs256-jwks.md) §17 ; **staleness maximale chiffrée** reste Non décidé (0304 §17 n’a pas de TTL numérique).
7. **Services derrière la passerelle** (décision 2026-09-30) : en mode mesh, un service ne revérifie pas le JWT. Il lit le principal dans `x-principal-iss` / `x-principal-sub` **seulement** sur une connexion mTLS dont le certificat client porte le SAN DNS de la passerelle (`auth.mesh.trusted_gateway_san`, rustycog `mesh_principal`). Le Bearer est ignoré ; un autre pair mTLS, une connexion TLS sans certificat ou le port HTTP clair donnent 401 sur les routes authentifiées. `trusted_gateway_san` vide (défaut) garde la vérification JWT in-process. Conséquence : posséder la clé `envoy-mesh` permet d’injecter n’importe quel principal ; cette clé est un secret de la passerelle.

## État runtime

**Partial** — livré dans le dépôt ; mesh **opt-in uniquement** (pas le défaut) :

- Crate `ext-authz/` : HTTP Check (`POST /`, `POST /check`) ; strip de tout `X-Principal-*` client puis recreate `x-principal-iss` / `x-principal-sub` ; cache JWKS process-local + poll fond ; `EXT_AUTHZ_AUDIENCE` obligatoire (fail-closed boot).
- **Profil Compose opt-in** : `docker compose --profile mesh up` démarre `ext-authz` + `envoy-mesh` (`deploy/mesh/envoy.yaml`, HTTP `ext_authz` → Check). Absent de `docker compose up` plain. Envoy route `/iam/`, `/hive/`, `/telegraph/`, `/manifesto/` vers les clusters homonymes en mTLS (cert `envoy-mesh`). Lua retire tout `x-principal-*` client ; `allowed_upstream_headers` ne forward que `x-principal-iss` / `x-principal-sub` recréés par Check ; `failure_mode_allow: false` ; pas de filtre JWT Envoy. L’overlay `deploy/mesh/compose.yaml` pose `*_SERVER__TLS_REQUIRE_CLIENT_CERT=true` et `*_AUTH__MESH__TRUSTED_GATEWAY_SAN=envoy-mesh` sur ces quatre services.
- **Preuve E2E runtime** : `bash scripts/mesh-authn-e2e.sh` + `scripts/mesh-authn-e2e-cases.sh` (Envoy réel, ext-authz Linux, JWT RS256 émis par IAM) — 48 OK, 0 FAIL, pile démontée. Pour chaque service : refus sans JWT (upstream non appelé, `ext_authz.denied`) ; allow où le service **utilise** le principal (`/iam/api/me` id = `sub`, org Hive `owner_user_id` = `sub`, projet Manifesto 201 `owner_id` = `created_by` = `sub`) ; Telegraph 200 prouve l’authn, pas l’identité (user neuf sans notifications). Contournement refusé : `:8443` Bearer, `:8443` `x-principal` avec cert `mesh-client`, `:8080` spoof+JWT, handshake sans cert. Témoin : cert `envoy-mesh` + spoof vers `iam-service:8443` = 200 (posséder la clé passerelle = usurper n’importe quel principal). Les 23 cas gateway antérieurs restent verts. Rustycog : pin sibling `2290d45` (`feat(http): trust gateway principal in mesh mode`) ; tests `rustycog-http/tests/mesh_gateway_auth.rs`. Le gitlink AIForAll pointe ce SHA ; **non commité** dans AIForAll.
- **Overlay Kind opt-in séparé** : `deploy/apps/overlays/kind-mesh/` (`kubectl apply -k …`) — **pas** dans `overlays/kind/kustomization.yaml`. `envoy-mesh.yaml` a encore un seul cluster `backend` (IAM) ; pas de routes Hive/Telegraph/Manifesto ni §7. Kind = projet ultérieur (décision 2026-09-30 `compose_then_kind`).
- Overlay Flux `deploy/apps/overlays/kind/helmrelease-envoy-gateway.yaml` : **COMMENTÉ** et hors `resources` (Flux hors aiforall-local).
- Preuve E2E sans Envoy runtime : harness cargo **stand-in** `ext-authz/tests/mesh_path_standin.rs` (contrat Check ; n’affirme pas qu’Envoy a tourné).
- **Gaps** : mesh non défaut ; overlay `kind-mesh` sans services réels ni §7 ; overlay Flux Kind toujours commenté ; routes publiques IAM (signup, login, JWKS) non exposées par Envoy (ext_authz refuse toute requête sans JWT) ; en mode §7, les appels S2S directs vers une route `.authenticated()` (ex. Hive → IAM `/internal/*` sur :8080) répondent 401 tant qu’ils ne passent pas par la passerelle ; staleness chiffrée révocation = Non décidé (0304 §17). **Pas Implemented.**

## Migration

Validation JWT reste aussi dans rustycog-http (défaut services, `auth.mesh.trusted_gateway_san` vide). Overlay Envoy Flux Kind **reste commenté**. Profil `--profile mesh` / overlay `kind-mesh` = opt-in local ; ne pas traiter comme activation mesh plateforme.

## Conséquences

- Consommateurs mesh ne parlent pas au KMS ; JWKS IAM seulement.
- Révocation clé ≠ révocation session ([0304](0304-jwt-acces-plateforme-rs256-jwks.md) §14–§15).

## Alternatives rejetées

| Option | Pourquoi pas |
|---|---|
| Faire confiance aux `X-Principal-*` client | Spoofing trivial |
| Service fait confiance aux `X-Principal-*` de tout pair mTLS `platform-mesh` | Tout service de la CA pourrait usurper un utilisateur ; §7 exige le SAN passerelle |
| Service revérifie aussi le JWT derrière Envoy | Choix 2026-09-30 : la passerelle est le point de vérification ; double vérification = double cache JWKS et double politique |
| Fetch JWKS / KMS par requête | Latence + failure domain |
| Filtre JWT Envoy | Hors contrat : produit = ext_authz HTTP |
| Sidecar AuthN | Hors contrat : produit = ext_authz HTTP |

## Non décidé ici

- Valeur chiffrée de max staleness révocation (0304 §17 sans TTL numérique).
- Mécanisme exact push vs poll vs TTL cache (poll présent dans `ext-authz` ; TTL numérique max Non décidé).

## Références

- [0304](0304-jwt-acces-plateforme-rs256-jwks.md) §14, §17 ; [0305](0305-account-identity-trust-domain.md)
- Overlay Flux Kind : `deploy/apps/overlays/kind/helmrelease-envoy-gateway.yaml` (commenté)
- Opt-in : `docker compose --profile mesh` ; `deploy/mesh/envoy.yaml` ; `deploy/apps/overlays/kind-mesh/`
- Preuve Partial : `ext-authz/` (`src/check.rs`, …) ; stand-in `tests/mesh_path_standin.rs` (pas preuve Envoy runtime) ; e2e Envoy runtime `scripts/mesh-authn-e2e.sh` + `scripts/mesh-authn-e2e-cases.sh`
- Mode passerelle services : rustycog `2290d45` (`rustycog-http/src/mesh_principal.rs`, `middleware_auth.rs`, `AuthConfig.mesh.trusted_gateway_san`) ; overlay `deploy/mesh/compose.yaml` ; IAM `setup` copie `config.auth.mesh` dans `http_verifier_auth`
