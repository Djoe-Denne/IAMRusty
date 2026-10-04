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

`Accepted` ratifie ext_authz HTTP (pas filtre JWT Envoy, pas sidecar, pas SPIFFE). [0307](0307-workload-identity-port.md) reste Implemented : port WIF, pas maillage X509. `Réalité : Partial` au 2026-10-04 : code mesh/SDK de fraîcheur monotone60, snapshot vide, trust/métadonnées et transport écrit ; SDK publié et 22 tests purs SDK PASS selon parent. Root/IAM/mesh intégré non compilé ni testé au complet. LKG illimité/empty rejeté décrivent la baseline historique f060d47/ba69c9e, pas les nouvelles sources. Aucun nouveau résultat réseau Envoy/Kind ; IT puis E2E après intégration. **Pas Implemented.**

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

## État source et preuves historiques

**Partial** — sources disponibles ; mesh opt-in et nouveau profil local-full séparé ([0606](0606-local-full-kind-isole.md)). Aucun nouveau runtime prouvé :

- Crate `workers/ext-authz/` : HTTP Check (`POST /`, `POST /check`) ; strip de tout `X-Principal-*` client puis recreate `x-principal-iss` / `x-principal-sub` ; cache JWKS process-local + poll fond ; `EXT_AUTHZ_AUDIENCE` obligatoire (fail-closed boot).
- **Profil Compose opt-in** : `docker compose --profile mesh up` démarre `ext-authz` + `envoy-mesh` (`ops/deploy/mesh/envoy.yaml`, HTTP `ext_authz` → Check). Absent de `docker compose up` plain. Envoy route `/iam/`, `/hive/`, `/telegraph/`, `/manifesto/` vers les clusters homonymes en mTLS (cert `envoy-mesh`). Lua retire tout `x-principal-*` client ; `allowed_upstream_headers` ne forward que `x-principal-iss` / `x-principal-sub` recréés par Check ; `failure_mode_allow: false` ; pas de filtre JWT Envoy. L’overlay `ops/deploy/mesh/compose.yaml` pose `*_SERVER__TLS_REQUIRE_CLIENT_CERT=true` et `*_AUTH__MESH__TRUSTED_GATEWAY_SAN=envoy-mesh` sur ces quatre services.
- **Preuve E2E** : `bash ops/scripts/mesh-authn-kind-e2e.sh` sur `kind-aiforall-local` — KIND MESH E2E OK (2026-10-02). Deny sans JWT, `sub` utilisé (`/iam/api/me`), `user_id` Telegraph, `issuer` du membre Hive, contournement certificat `mesh-client` refusé, signup / login / JWKS via Envoy sans Bearer. `ops/scripts/mesh-authn-e2e.sh` (Compose `--profile mesh`) reste un harnais, plus une preuve e2e. Cette preuve est historique, pas celle des nouveaux correctifs. Baseline de reprise `f060d47` : gitlink rustycog `ba69c9e`, propre ; les nouveaux runs devront enregistrer les hashes du SDK, des manifests et des scripts.
- **Overlay Kind opt-in source** : `ops/deploy/apps/overlays/kind-mesh/`, distinct de `ops/deploy/apps/overlays/local-full/`. Les exemptions JWT doivent suivre `ops/deploy/mesh/public-routes.md` : méthodes/chemins exacts et callbacks OAuth transactionnels, sans ouvrir les START Link/Relink ou S2S. Source seulement ; Kind ne compile pas et le résultat réseau reste à prouver.
- Overlay Flux `ops/deploy/apps/overlays/kind/helmrelease-envoy-gateway.yaml` : **COMMENTÉ** et hors `resources` (Flux hors aiforall-local).
- Preuve E2E sans Envoy runtime : harness cargo **stand-in** `workers/ext-authz/tests/mesh_path_standin.rs` (contrat Check ; n’affirme pas qu’Envoy a tourné).
- **Gates de preuve qui bloquent Implemented** : compilation root/ABI après intégration, units IAM/publisher et IT réellement exécutées, puis fraîcheur60/outage/vide/non-résurrection et trust/S2S/spoof/routes exactes via Envoy au hash courant. Le code des caches est écrit et le SDK a sa gate pure limitée ; cela ne vaut ni test du publisher IAM ni test réseau mesh. Le signataire organisation 0306 et les futures capacités cloud ne remplacent pas ces preuves.
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

## Réconciliation source — 2026-10-04

Le défaut poll 60 s et override local/Kind 2 s sont conservés. Les nouveaux caches SDK/ext-authz bornent la confiance par horloge monotone à 60 s ; hits/échecs ne prolongent pas la confiance, et un snapshot valide vide retire les clés. Les métadonnées/trust et le vrai transport HTTP/TLS sont du code, pas une preuve de trafic mesh. Source : `workers/ext-authz/src/{jwks_cache,check}.rs` et SDK épinglé ; 22 tests purs SDK PASS confirmés par le parent, sans assimilation à units CORE B ou à Envoy.

Les résultats Kind/S2S datés du 2026-10-02 restent **historiques**. Le root HEAD `5348a63` est non committé ; setup partagé final et E4 attendent compilation/units/IT/E2E après intégration. Le nouveau contexte full autorisé est `kind-aiforall-local-full`, uniquement après IT finale et avec inventaire/bail, sans recréer le cluster legacy étranger. D6 render 112 et CA publique seule = preuve SOURCE, pas test TLS/réseau. Voir [0606](0606-local-full-kind-isole.md), `ops/deploy/apps/overlays/local-full/README.md` et contrat interface ; aucune fermeture courante n’est déduite de f060d47.

## Références

- [0304](0304-jwt-acces-plateforme-rs256-jwks.md) §14, §17 ; [0305](0305-account-identity-trust-domain.md)
- Overlay Flux Kind : `ops/deploy/apps/overlays/kind/helmrelease-envoy-gateway.yaml` (commenté)
- Opt-in : `docker compose --profile mesh` ; `ops/deploy/mesh/envoy.yaml` ; `ops/deploy/apps/overlays/kind-mesh/`
- Preuve e2e : `ops/scripts/mesh-authn-kind-e2e.sh` (Kind). Stand-in `workers/ext-authz/tests/mesh_path_standin.rs` ne prouve pas qu’Envoy a tourné. `ops/scripts/mesh-authn-e2e.sh` n’est plus la preuve e2e.
- Mode passerelle services : SDK courant sélectionné `ca2e35fcd56279e9e52625d0df9381f240f3390d` (publié selon parent ; gitlink worktree root non committé). `ba69c9e`/`2290d45` sont des références historiques, pas le pin courant. Sources `rustycog/rustycog-http/src/{mesh_principal,middleware_auth,jwks}.rs`, `ops/deploy/mesh/compose.yaml` et IAM setup ; aucune attestation réseau nouvelle.
