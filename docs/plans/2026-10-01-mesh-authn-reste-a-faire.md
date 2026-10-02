# Mesh AuthN — reste à faire (note de travail, 2026-10-01)

Note temporaire. Ce n’est pas une ADR. 0308 reste **Accepted / Partial**. Ne pas la passer à Implemented à partir de ce fichier.

## Objectif de la session

IAM émet un JWT RS256. Envoy (ext_authz HTTP du dépôt, pas le filtre JWT Envoy) le vérifie. Quand la requête atteint IAM, Hive, Telegraph ou Manifesto, le service n’a plus à revérifier le JWT : il consomme le principal posé par Envoy (`x-principal-iss`, `x-principal-sub`), et seulement si le certificat client mTLS a le SAN DNS `envoy-mesh`. Un contournement d’Envoy ne doit pas authentifier. Une commande unique monte la pile, joue les cas, échoue si un cas rate, puis démonte.

Décisions déjà prises : confiance à la passerelle (`trust_envoy`) ; Compose d’abord, Kind ensuite (`compose_then_kind`). Pas de TTL numérique de staleness (0304 §17).

## Déjà fait

- rustycog `2290d45` (`feat(http): trust gateway principal in mesh mode`) poussé sur `Djoe-Denne/rustycog`. `auth.mesh.trusted_gateway_san` vide = JWT in-process. Tests Docker verts (`mesh_principal`, `mesh_gateway_auth`, mTLS, JWT RS256).
- AIForAll : gitlink local sur `2290d45a746ce73f5ddb84fde30566f79da2634c`. IAM recopie `auth.mesh`. Manifesto en TLS 8443 (`manifesto-service`, port hôte 8448). Envoy Compose : `/iam/`, `/hive/`, `/telegraph/`, `/manifesto/` en mTLS. Overlay `deploy/mesh/compose.yaml` : certificat client obligatoire et `TRUSTED_GATEWAY_SAN=envoy-mesh` sur les quatre services.
- `bash scripts/mesh-authn-e2e.sh` : exit 0, 48 OK, 0 FAIL, pile démontée. Les 23 cas gateway d’avant sont dans ce total.
- ADR 0302, 0304, 0308, 0400–0403 et le vault `obsidian/AI FOR ALL` décrivent §7. QMD `aiforall-wiki` réindexé (7 nouvelles, 12 mises à jour, vecteurs inclus).

Rien de tout ça n’est commité dans AIForAll.

## 1. Fermer le contrat Compose (encore ouvert)

Le passage Envoy → service authentifié est prouvé. Ces trous empêchent de dire que toute requête utile a été vérifiée avant d’agir.

1. **Routes optionnelles hors Envoy.** `.might_be_authenticated()` (recherche d’organisations Hive, lecture de projets Manifesto) continue en anonyme si le principal passerelle est absent. L’e2e ne couvre que des routes `.authenticated()`, qui répondent 401. Un client qui parle à `:8080` ou à `:8443` avec un certificat `mesh-client` peut encore utiliser ces lectures sans JWT.
2. **Telegraph ne prouve pas l’identité.** `GET /telegraph/api/notifications` pour un utilisateur neuf répond 200 et le jq ne teste que `type == "object"`. Le `sub` n’est pas relu. Il faut une réponse qui porte l’utilisateur (compteur, notification semée pour ce `sub`, ou champ explicite).
3. **Hive ne prouve pas `iss`.** La création enregistre `principal.iss` sur le membre (`CreateOrganizationCommand`). L’e2e ne vérifie que `owner_user_id == sub`. Lire le membre et comparer `issuer` au `iss` du JWT.
4. **Porte d’entrée unique.** Les ports HTTP (`8080`–`8083`) et HTTPS (`8443`, `8444`, `8445`, `8448`) restent publiés. Les routes authentifiées y répondent 401 ; le réseau Compose peut encore joindre signup, santé et les routes optionnelles sans Envoy.
5. **Routes publiques IAM absentes d’Envoy.** Signup, login et JWKS sont appelés en direct sur `iam-service:8443` par l’e2e. ext_authz refuse toute requête sans JWT, donc Envoy ne peut pas exposer ces routes tant qu’il n’existe pas un chemin sans Check (ou un Check qui laisse passer ces chemins).
6. **Appels de service à service.** En mode mesh, une route `.authenticated()` appelée en direct (Hive → IAM `/internal/*` sur `:8080`) répond 401. La création d’organisation de l’e2e ne fait pas cet appel. Le profil mesh casse le flux signataire d’organisation tant qu’il ne passe pas par Envoy avec le certificat `envoy-mesh`.
7. **Suites de tests des services non rejouées** après le bump rustycog. Seule la compilation Docker de `github-connect-service`, `gitlab-connect-service`, `manifesto-setup` et `iam-setup` a réussi. Hive, Telegraph, Manifesto et IAM (tests d’intégration) n’ont pas tourné.
8. **Livrer le couple git.** Le code AIForAll dépend du gitlink `2290d45`. Sans lui, `auth.mesh` ne compile pas. rustycog est déjà sur GitHub ; AIForAll ne l’a pas enregistré. Commit seulement sur demande.

## 2. Kind — projet suivant, pas commencé

Décision `compose_then_kind`. L’objectif « comme en prod, en local, via Kind » n’est pas tenu.

- `deploy/apps/overlays/kind-mesh/envoy-mesh.yaml` route encore `prefix: /` vers le cluster `backend` (IAM seul). Pas de `/hive/`, `/telegraph/`, `/manifesto/`. Pas de §7 sur les workloads Kind.
- `deploy/apps/overlays/kind-mesh/ext-authz.yaml` indique encore un IAM stub/pause comme source JWKS.
- `deploy/apps/overlays/kind/helmrelease-envoy-gateway.yaml` reste commenté et hors `resources`.
- Les services Kind ne sont pas les images Linux construites ici. Kind ne compile pas : il faut `kind load` d’images déjà buildées en Docker.
- Il manque un e2e Kind du même contrat (deny sans JWT, allow avec principal utilisé, contournement refusé).

## 3. Ne pas faire pour « finir » 0308

- Passer 0308 à Implemented tant que la section Gaps de l’ADR est vraie (Kind, Flux, routes publiques, S2S, staleness).
- Choisir un TTL numérique de révocation (0304 §17). `EXT_AUTHZ_JWKS_POLL_SECS` est le poll déjà codé, pas cette décision.
- Filtre JWT Envoy, sidecar, `path_prefix: /`, enlever `.fallback(check)`, ou `request_headers_to_remove` de `iss`/`sub` sur la route.
- Copier `auth.mesh` sur l’extracteur du grant snapshot Manifesto (`grant_snapshot_user_id_extractor`). Ce jeton HS256 n’est pas le JWT IAM.

## 4. Petites erreurs

1. **Healthcheck Docker au mauvais chemin.** `Hive/Dockerfile`, `Telegraph/Dockerfile`, `Manifesto/Dockerfile` font `curl -f http://localhost:8080/health`. La route réelle est sous le préfixe (`/hive/health`, `/telegraph/health`, `/manifesto/health`) : l’e2e joint `https://{svc}-service:8443/{svc}/health` avec succès, et Telegraph comme Manifesto étaient `(unhealthy)`. `IAMRusty/Dockerfile` n’a pas de `HEALTHCHECK`, donc IAM ne passe pas à unhealthy pour la même raison. Hive a le même `curl` ; on ne l’a pas vu unhealthy seulement parce que le conteneur était déjà sorti, ou la pile déjà démontée.
2. **`jwks_url` vers 127.0.0.1.** `IAMRusty`, `Hive`, `Telegraph` et `Manifesto` `config/development.toml` ont `jwks_url = "http://127.0.0.1:8080/iam/.well-known/jwks.json"`. Dans un conteneur, cette adresse est le conteneur lui-même, pas IAM. Le mode mesh ne va pas chercher le JWKS, donc l’e2e ne le montre pas. Sans l’overlay mesh, Hive, Telegraph et Manifesto ne peuvent pas vérifier un JWT RS256. IAM s’en sort parce qu’il injecte le JWKS en mémoire au démarrage.
3. **README Kind en avance sur son YAML.** `deploy/apps/overlays/kind-mesh/README.md` décrit mTLS, JWKS IAM et renvoie à l’e2e Compose. Le YAML à côté route encore tout vers `cluster: backend`.
4. **Deux dates pour la même décision.** L’ADR §7 et `obsidian/AI FOR ALL/journal/2026-09-30.md` disent 2026-09-30. L’e2e vert et la ligne QMD de `log.md` sont du 2026-10-01.
5. **Effets de bord sur tout Compose, pas seulement le profil mesh.** `docker-compose.yml` (fichier de base) contient `DROP DATABASE … WITH (FORCE)` et `IAM_INTERNAL_SERVICE_TOKEN` / `HIVE_IAM_SERVICE__API_KEY`. Chaque `docker compose up` coupe les sessions Postgres, y compris le pod Kind `oodhive-monolith` branché sur le port hôte 5432, puis recrée les bases. Le secret de dev est dans le compose par défaut, au même titre que `postgres`/`postgres`.
6. **Libellé du cas Telegraph.** « Telegraph liste les notifications via Envoy » se lit comme une preuve d’identité. Le corps du test ne regarde pas `sub`. L’ADR, elle, le dit.
7. **Mémoires Serena non confirmées.** Le MCP Serena répondait `-32603`. Les fichiers `.serena/memories/architecture/events-authz-adr-0308.md` et `mesh-e2e-compose-pitfalls.md` ont été écrits à la main. On n’a pas revu qu’Serena les charge.
8. **`qmd update` n’a pas de filtre utile.** `qmd update -c aiforall-wiki` a quand même parcouru toutes les collections. Au premier passage, `ff8-wiki` a réindexé un fichier déjà modifié sur le disque. Aucun fichier de ce dépôt n’a été écrit dans les autres vaults.
