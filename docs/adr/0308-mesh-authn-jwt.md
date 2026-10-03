# ADR-0308 : Mesh / gateway AuthN JWT — valide trust puis émet (iss, sub)

- Statut : Accepted
- Réalité : Partial
- Date : 2026-09-26 (acceptation humaine 2026-09-27)
- Amendement : 2026-10-02 (Djoé Denne — canaux IT / e2e / isoprod et contrat S2S). Réouverture explicitement ratifiée par l’utilisateur le 2026-10-03 : **fail-closed JWKS à 60 s**, y compris pendant une panne de refresh ; remplace le compromis antérieur « pas de TTL pour Implemented ». Cette écriture enregistre la décision et le plan, pas une preuve d’implémentation.
- Décideurs : Djoé Denne (acceptation humaine 2026-09-27)
- Jalon concerné : architecture actuelle / AuthN mesh (hors P-Apparatus)
- SuperSède : aucune
- SuperSédée par : —
- Related : [0304](0304-jwt-acces-plateforme-rs256-jwks.md), [0305](0305-account-identity-trust-domain.md), [0307](0307-workload-identity-port.md)

`Accepted` ratifie ext_authz HTTP (pas filtre JWT Envoy, pas sidecar, pas SPIFFE). [0307](0307-workload-identity-port.md) reste Implemented : port WIF, pas maillage X509. `Réalité : Partial` : overlay Kind opt-in et parcours historiques présents ; défaut poll binaire 60 s et override Kind 2 s déjà présents à la baseline `f060d47` / gitlink rustycog `ba69c9e`. La fermeture fail-closed à 60 s n’est pas démontrée : caches last-known-good non bornés et état JWKS vide à corriger. Les nouvelles IT et E2E devront être exécutées après intégration de tous les lots. **Pas Implemented.**

## Contexte

Après RS256 + JWKS + issuer par trust domain ([0304](0304-jwt-acces-plateforme-rs256-jwks.md)), la validation doit pouvoir vivre au gateway / mesh (pas seulement dans chaque service). Le produit cible reste un service **ext_authz HTTP** branché sur Envoy. L’activation locale isoprod est l’overlay opt-in `kind-mesh/`, pas le défaut plateforme.

## Décision

1. Gateway / mesh valide : `alg`, `kid`, signature, `typ`, `iss`, `aud`, `exp`, `iat`, `nbf` si présent, trust scope, `kid`→issuer, `kid`→org, key status — puis produit le principal `(iss, sub)`.
2. **Headers client `X-Principal-*`** : supprimés à l’entrée et **recréés** par le mesh (jamais confiance client).
3. **mTLS / isolation** entre hops (détail WorkloadIdentity = [0307](0307-workload-identity-port.md)).
4. **Cache JWKS** comme [0304](0304-jwt-acces-plateforme-rs256-jwks.md) §17 (pas de fetch par requête ; singleflight ; negative cache ; last-known-good).
5. **Produit** : service **ext_authz HTTP** (pas filtre JWT Envoy, pas sidecar). L’activation locale est l’overlay opt-in `kind-mesh/` (`kubectl apply -k`), pas le défaut. Le HelmRelease Flux Envoy Gateway reste commenté et ne mesure pas Implemented.
6. **Propagation révocation clé et fraîcheur de confiance** (chiffres du 2026-10-02 ; arbitrage fail-closed ratifié le 2026-10-03) : cache JWKS comme [0304](0304-jwt-acces-plateforme-rs256-jwks.md) §17.
   - Poll staging / prod : **60 secondes** ; local, Kind et tests : **2 secondes**. Le défaut binaire 60 s et l’override Kind 2 s existent déjà. Le poll n’est pas une preuve de fraîcheur.
   - **À 60 s d’âge du dernier snapshot JWKS autoritatif validé, aucune clé de ce snapshot ne permet plus d’authentifier.** Âge mesuré par temps monotone dans chaque process ; un refresh échoué, un cache hit ou un negative-cache hit ne remet jamais cette horloge à zéro. Même règle pour ext_authz et les validators rustycog in-process. Aucun réglage ne doit relever cette borne.
   - **Outage = fail-closed** après cette borne, même pour un `kid` connu et un JWT non expiré. Last-known-good est utilisable uniquement avant la borne ; disponibilité dégradée assumée. Au boot, pas de snapshot validé = pas de confiance accordée.
   - Un JWKS autoritatif valide `keys: []` retire immédiatement toutes les clés du cache. Ne pas le traiter comme une panne ni ressusciter le bootstrap parce qu’un registry initialisé ne contient plus de clés.
   - Staleness de confiance maximale : **60 secondes**, y compris en panne de refresh. La garantie suppose un publisher canonique exact : distinguer registry jamais initialisé, registry vide après révocation et erreur de lecture ; ces corrections font partie de la clôture.
7. **Services derrière la passerelle** (décision 2026-09-30) : en mode mesh, un service ne revérifie pas le JWT. Il lit le principal dans `x-principal-iss` / `x-principal-sub` **seulement** sur une connexion mTLS dont le certificat client porte le SAN DNS de la passerelle (`auth.mesh.trusted_gateway_san`, rustycog `mesh_principal`). Le Bearer est ignoré ; un autre pair mTLS, une connexion TLS sans certificat ou le port HTTP clair donnent 401 sur les routes authentifiées. `trusted_gateway_san` vide (défaut) garde la vérification JWT in-process. Conséquence : posséder la clé `envoy-mesh` permet d’injecter n’importe quel principal ; cette clé est un secret de la passerelle.
8. **Canaux** (2026-10-02, prioritaire sur `compose_then_kind`) :
   - **Tests d’intégration** : pas Kind. Les third parties (Postgres, Redis, OpenFGA, files, et les autres) sont servies par Compose. État du code : le harness [0200](0200-integration-tests-serveur-reel-base-reelle.md) les lance encore en testcontainers (`rustycog-testing`), pas par le `docker-compose.yml` racine. Cette phrase ne constate pas une migration vers ce fichier.
   - **E2E** : entièrement Kind. Preuve = `ops/scripts/mesh-authn-kind-e2e.sh` (KIND MESH E2E OK, 2026-10-02, `kind-aiforall-local`) : deny sans JWT, `sub` / `user_id` / `issuer`, contournement `mesh-client` refusé, signup / login / JWKS via Envoy. `ops/scripts/mesh-authn-e2e.sh` n’est plus une preuve e2e.
   - **Local isoprod hors IT** : Postgres, Redis et les autres third parties via Kind, pas via Compose. Redis suit ce canal ; il n’est pas déployé dans `kind-mesh` aujourd’hui. Le but de Kind est un environnement local aussi proche que possible de la prod, pour tester en local.
9. **Clôture verrouillée** (2026-10-02, décisions seules) :
   - Poll : 60 s staging / prod, 2 s local / Kind / tests. Expiration de confiance à 60 s sans succès autoritatif, même pendant un outage ; état vide autoritatif accepté et cache vidé. Le défaut poll 60 s existe déjà, mais cela ne ferme pas le point 6.
   - `POST /iam/internal/{provider}/token` et `DELETE .../revoke` : certificat client SAN `envoy-mesh`, plus l’en-tête interne déjà exigé (`services/IAMRusty/http/src/rate_limit.rs`). Pas de JWT utilisateur sur ce hop. Un `mesh-client` en direct reste 401. Le code S2S est présent à la baseline ; sa preuve Kind au hash courant reste à rejouer après intégration.
   - Baseline de reprise : gitlink rustycog `ba69c9e`, propre. Toute nouvelle correction SDK doit être publiée dans le dépôt rustycog avant un bump du gitlink AIForAll ; publication/commit relèvent d’une autorisation et d’une intégration parent explicites, pas d’un automatisme de ce plan.

## État runtime

**Partial** — livré dans le dépôt ; mesh **opt-in uniquement** (pas le défaut) :

- Crate `workers/ext-authz/` : HTTP Check (`POST /`, `POST /check`) ; strip de tout `X-Principal-*` client puis recreate `x-principal-iss` / `x-principal-sub` ; cache JWKS process-local + poll fond ; `EXT_AUTHZ_AUDIENCE` obligatoire (fail-closed boot).
- **Profil Compose opt-in** : `docker compose --profile mesh up` démarre `ext-authz` + `envoy-mesh` (`ops/deploy/mesh/envoy.yaml`, HTTP `ext_authz` → Check). Absent de `docker compose up` plain. Envoy route `/iam/`, `/hive/`, `/telegraph/`, `/manifesto/` vers les clusters homonymes en mTLS (cert `envoy-mesh`). Lua retire tout `x-principal-*` client ; `allowed_upstream_headers` ne forward que `x-principal-iss` / `x-principal-sub` recréés par Check ; `failure_mode_allow: false` ; pas de filtre JWT Envoy. L’overlay `ops/deploy/mesh/compose.yaml` pose `*_SERVER__TLS_REQUIRE_CLIENT_CERT=true` et `*_AUTH__MESH__TRUSTED_GATEWAY_SAN=envoy-mesh` sur ces quatre services.
- **Preuve E2E** : `bash ops/scripts/mesh-authn-kind-e2e.sh` sur `kind-aiforall-local` — KIND MESH E2E OK (2026-10-02). Deny sans JWT, `sub` utilisé (`/iam/api/me`), `user_id` Telegraph, `issuer` du membre Hive, contournement certificat `mesh-client` refusé, signup / login / JWKS via Envoy sans Bearer. `ops/scripts/mesh-authn-e2e.sh` (Compose `--profile mesh`) reste un harnais, plus une preuve e2e. Cette preuve est historique, pas celle des nouveaux correctifs. Baseline de reprise `f060d47` : gitlink rustycog `ba69c9e`, propre ; les nouveaux runs devront enregistrer les hashes du SDK, des manifests et des scripts.
- **Overlay Kind opt-in** : `ops/deploy/apps/overlays/kind-mesh/` — **pas** dans `overlays/kind/kustomization.yaml`. Routes `/iam/` `/hive/` `/telegraph/` `/manifesto/`, chemins exacts signup / login / JWKS sans Check, quatre workloads, `TRUSTED_GATEWAY_SAN=envoy-mesh`, Postgres in-cluster sans hostPort 5432, NetworkPolicy (le namespace est default-deny). Images chargées par `kind load` ; Kind ne compile pas.
- Overlay Flux `ops/deploy/apps/overlays/kind/helmrelease-envoy-gateway.yaml` : **COMMENTÉ** et hors `resources` (Flux hors aiforall-local).
- Preuve E2E sans Envoy runtime : harness cargo **stand-in** `workers/ext-authz/tests/mesh_path_standin.rs` (contrat Check ; n’affirme pas qu’Envoy a tourné).
- **Gaps qui bloquent Implemented** : expiration fail-closed à 60 s dans tous les caches, traitement autoritatif du JWKS vide et non-résurrection de clés révoquées ; revalidation du contrat de trust et des parcours S2S via Envoy sur Kind au hash courant. Tests à écrire pendant les lots, IT et E2E à exécuter seulement après leur intégration complète. Le signataire d’organisation (0306) n’est pas ce trou.
- **Ne bloquent plus Implemented** : preuve e2e Compose ; HelmRelease Flux commenté ; mesh opt-in ; `kind-mesh` sans services ni §7 ; signup / login / JWKS hors Envoy. Le poll 60 s / 2 s existe ; il ne remplace pas la borne fail-closed du point 6. **Pas Implemented.**

## Migration

Validation JWT reste aussi dans rustycog-http (défaut services, `auth.mesh.trusted_gateway_san` vide). Le HelmRelease Flux reste commenté et hors critère d’Implemented. `kind-mesh/` est le canal e2e et isoprod opt-in ; le profil Compose mesh n’est plus la preuve e2e. Ni l’un ni l’autre n’est le défaut plateforme.

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

- Mécanisme push de révocation (le poll est le mécanisme retenu ; pas de push). Les durées sont au point 6.

## Références

- [0304](0304-jwt-acces-plateforme-rs256-jwks.md) §14, §17 ; [0305](0305-account-identity-trust-domain.md)
- Overlay Flux Kind : `ops/deploy/apps/overlays/kind/helmrelease-envoy-gateway.yaml` (commenté)
- Opt-in : `docker compose --profile mesh` ; `ops/deploy/mesh/envoy.yaml` ; `ops/deploy/apps/overlays/kind-mesh/`
- Preuve e2e : `ops/scripts/mesh-authn-kind-e2e.sh` (Kind). Stand-in `workers/ext-authz/tests/mesh_path_standin.rs` ne prouve pas qu’Envoy a tourné. `ops/scripts/mesh-authn-e2e.sh` n’est plus la preuve e2e.
- Mode passerelle services : baseline rustycog `ba69c9e` (référence historique `2290d45`, pas le gitlink courant) (`rustycog-http/src/mesh_principal.rs`, `middleware_auth.rs`, `AuthConfig.mesh.trusted_gateway_san`) ; overlay `ops/deploy/mesh/compose.yaml` ; IAM `setup` copie `config.auth.mesh` dans `http_verifier_auth`
