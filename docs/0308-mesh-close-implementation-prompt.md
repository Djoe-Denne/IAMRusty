# Prompt — Clôturer ADR-0308

Tu travailles dans `C:\Users\djden\source\repos\AIForAll`.

## Mission

Ferme **ADR-0308** (Mesh AuthN JWT). À la fin, `Réalité : Implemented`. Puis arrête-toi. Ne commence pas 0304, 0306, 0309, 0604, ni Redis.

L’amendement du 2026-10-02 prime sur le texte plus ancien de l’ADR. Lis `docs/adr/0308-mesh-authn-jwt.md` avant de coder.

## Déjà prouvé — ne pas refaire

- Envoy ext_authz HTTP, pas le filtre JWT, pas de sidecar. `failure_mode_allow: false`. Lua retire `x-principal-*`, Check recrée `iss` / `sub`.
- §7 : le service lit le principal seulement si le certificat client a le SAN `envoy-mesh`.
- Kind `kind-aiforall-local` : routes `/iam/` `/hive/` `/telegraph/` `/manifesto/`, signup / login / JWKS sans Check, workloads réels, Postgres **dans** le cluster sans hostPort 5432. Preuve : `scripts/mesh-authn-kind-e2e.sh` (KIND MESH E2E OK, 2026-10-02).
- Le signataire d’organisation (0306) : JWT utilisateur jusqu’à Hive via Envoy, puis Hive → IAM avec `x-iam-internal-token`. Ne le refais pas passer par un JWT utilisateur.
- HelmRelease Flux commenté : le laisser commenté. Ce n’est pas un critère d’Implemented.
- Tests d’intégration : **pas Kind**. Leurs Postgres / OpenFGA / files restent le harness actuel (testcontainers, `rustycog-testing`). Ne les migre pas vers Kind ni vers le `docker-compose.yml` racine dans ce tour.

## Ce qui ferme l’ADR

### 1. S2S `token` / `revoke`

`POST /iam/internal/{provider}/token` et `DELETE /iam/internal/{provider}/revoke` sont `.authenticated()`. En mode mesh, un appel direct répond 401. C’est le seul trou S2S encore ouvert. Le signataire ne l’est pas.

Contrat à livrer, sur Kind :

- Le hop présente le certificat **envoy-mesh**.
- La gate déjà codée (en-tête interne, voir `IAMRusty/http/src/rate_limit.rs`) reste. N’ajoute pas un JWT utilisateur sur ce hop.
- Un client `mesh-client` (Bearer, spoof, ou le même en-tête interne) vers le Service IAM en direct reste **401**.
- Ajoute le cas dans `scripts/mesh-authn-kind-e2e-cases.sh`. Pas dans `scripts/mesh-authn-e2e.sh`.

Si un appelant réel existe, pointe-le vers Envoy (`https://envoy-mesh:10000/...`) avec le certificat envoy-mesh. S’il n’existe pas, le cas e2e suffit. Ne copie pas `auth.mesh` sur l’extracteur HS256 du grant snapshot Manifesto.

### 2. Durées JWKS (déjà décidées, pas encore toutes dans le binaire)

Ne rouvre pas les chiffres. Applique-les.

- `ext-authz/src/config.rs` : le défaut de `EXT_AUTHZ_JWKS_POLL_SECS` passe de 300 à **60** secondes (staging / prod). La variable reste le réglage.
- Local, Kind et tests : **2** secondes. Le profil Compose mesh est déjà à 2. Pose `EXT_AUTHZ_JWKS_POLL_SECS=2` sur l’ext_authz Kind (`deploy/apps/overlays/kind-mesh/ext-authz.yaml`).
- Staleness max après révocation d’une clé : **60** secondes (0308 point 6, 0304 §17). Un refresh à 2 secondes en local reste sous ce plafond. Pas de push de révocation.

### 3. Gitlink rustycog

Le pin indexé est `2290d45`. Le working tree de `rustycog` contient le 401 des routes `.might_be_authenticated()` quand le SAN n’est pas `envoy-mesh`. Sans ce delta, un clone ne rejoue pas §7.

Ce prompt autorise les commits de clôture, et seulement ceux-là :

1. Commit sur `Djoe-Denne/rustycog` du delta mesh (pas d’autres fichiers).
2. Bump du gitlink AIForAll sur ce SHA.
3. Commit AIForAll du gitlink et des fichiers de cette clôture. Pas de secrets. Pas de `--no-verify`. Pas de force-push.

Compile dans Docker. Un seul build Rust à la fois. Ne monte pas le `target\` hôte. Kind ne compile pas : `kind load` d’images déjà construites. Contexte kubectl : `kind-aiforall-local` uniquement. Jamais `kind-apparatus-p4-it` ni `rancher-desktop`. Pas de `kind delete`, pas de `docker rm -f`, pas de prune, pas de `kubectl delete namespace`.

### 4. Texte ADR, une fois l’e2e vert

`docs/adr/0308-mesh-authn-jwt.md` :

- `Réalité : Implemented`.
- Les décisions du 2026-10-02 sont déjà dans l’ADR (points 6 et 9). Ne les rouvre pas. Retire des Gaps bloquants ce qui est vert : cas Kind token/revoke, défaut poll à 60 s, 2 s sur Kind, gitlink commité. Les durées restent celles du point 6.
- La preuve e2e citée est uniquement `scripts/mesh-authn-kind-e2e.sh`.
- Même tour : `docs/adr/README.md` et le digest `.serena/memories/architecture/events-authz-adr-0308.md` (Statut Accepted, Réalité Implemented, 4–6 puces, pas de dump).

Ne passe pas 0304, 0306 ni 0604 à Implemented. Ne déploie pas Redis. Ne migre pas le Postgres hôte de J3 (`host.docker.internal:5432`) : 0604 garde cette photo.

## Interdit

- Rouvrir les durées JWKS. Décision 2026-10-02 : défaut staging / prod 60 s (`unwrap_or` encore 300 dans `ext-authz/src/config.rs`, à aligner), 2 s en local / Kind / tests, staleness max 60 s. `EXT_AUTHZ_JWKS_POLL_SECS` reste le réglage.
- Filtre JWT Envoy, sidecar, `path_prefix: /`, enlever `.fallback(check)`, `request_headers_to_remove` de `iss` / `sub`.
- Rendre le mesh défaut de `docker compose up` ou de `overlays/kind`.
- Faire passer l’e2e par Compose. Relancer `scripts/mesh-authn-e2e.sh` comme preuve.
- `cargo` Windows. Monter `target\` dans un conteneur.

## Fait quand

1. `bash scripts/mesh-authn-kind-e2e.sh` sort 0, y compris le cas token/revoke (envoy-mesh autorisé, mesh-client 401).
2. Le gitlink AIForAll pointe le commit rustycog qui contient le 401 hors SAN, et ce commit est poussé sur `Djoe-Denne/rustycog`.
3. Le défaut du binaire est 60 s, l’ext_authz Kind est à 2 s, et 0308 dit `Réalité : Implemented` avec les durées du point 6 inchangées.
4. Aucun autre ADR n’a changé de Réalité.
