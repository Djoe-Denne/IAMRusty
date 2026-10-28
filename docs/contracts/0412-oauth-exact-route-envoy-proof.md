# Contrat de preuve — E2E exact-route Envoy de l'ADR 0412

Canon secondaire de [ADR 0412](../adr/0412-oauth-transaction-persistante-liee-navigateur.md).
Statut ADR : Accepted. Réalité ADR : Partial. Ce fichier ne les modifie pas.

Il rend exécutable, plus tard, la preuve E2E encore ouverte. Il n'est pas le lab,
pas un harness, et pas une autorisation de démarrer Kind pour le rédiger.

## Alignement ratifié du 2026-10-28 — assemblage intégré, preuve réseau restante

Le profil opt-in et son scénario moteur sont intégrés au36ffab6. Tests ciblés
de configuration/fixtures réussis, mais aucun navigateur, Envoy live ou trust
TLS de l'image IAM exacte exécuté à ce jalon. Les verdicts de cette matrice
restent à établir ; les tests source ne sont pas une campagne P1–P3/N1–N3.

**Ratification explicite du lead sous délégation utilisateur, campagne non exécutée.** Le [contrat d'alignement local-full](0412-oauth-local-full-alignment.md)
précise le listener, les origines, le vrai connecteur GitLab, son fournisseur
simulé et les contrôles discriminants. La cible ci-dessous reste inchangée :
HTTP loopback `18080`, navigateur réel, contexte full, P1–P3/N1–N3.

**Contradiction source vérifiée à la base `c72ae362` :** local-full importe
`kind-mesh/envoy-mesh.yaml`, dont `listener_mesh:10000` exige TLS et un certificat
client. `render-local-full.py::render` réécrit les namespaces et les exemptions
relink, pas ce transport. Le rendu historique de 112 objets ne réalise donc
pas l'entrée HTTP annoncée. Ce n'est ni une preuve navigateur, ni un motif pour
substituer `https`, `18443`, le legacy ou une clé client dans le navigateur.

Décision ratifiée : rendre **ce seul listener downstream local-full**
HTTP dans le renderer existant ; conserver les sources mesh partagées en TLS,
les filtres et matchers, et les hops internes TLS/mTLS avec vérification des
identités serveur. Activer le connecteur et sa simulation par un profil de
preuve explicite, sans modifier `connectors=[]` par défaut. Le lead a examiné et
ratifié ce texte et son complément ; leur intégration précède les consommateurs.
Aucun changement de Statut ou Réalité ADR ici. **Pas de nouvelle ADR 0413** :
la cible Accepted et ses exceptions locales ne changent pas.

## Décision

La preuve qui peut autoriser `Réalité : Implemented` est un parcours
navigateur, entré par l'Envoy du contexte `kind-aiforall-local-full`, sur les
routes déjà présentes dans le dépôt, avec un connecteur `github` ou `gitlab`
activé. Le profil local-full livré avec `connectors=[]` ne satisfait pas cette
preuve : prérequis absent, pas preuve rouge d'un défaut OAuth.

La conséquence ADR ligne 37 (navigateur / `kind-aiforall-local-full`, 3 nœuds)
n'est pas amendée. Aucune ADR 0413 n'est ouverte.

L'actualisation du 2026-10-08 prime sur une lecture qui redemanderait, dans
cette E2E, le rejeu inter-replicas écrit « ensuite » à la ligne 37. Ce rejeu
est déjà couvert par les IT citées. La phrase de la ligne 37 reste dans l'ADR.

## Cluster

| Élément | Valeur figée |
|---|---|
| Contexte kubectl | `kind-aiforall-local-full` |
| Cluster | `aiforall-local-full` |
| Topologie | 1 control-plane + 2 workers |
| Entrée HTTP | `127.0.0.1:18080` → control-plane `30080` → Envoy `10000` |
| Listener cible, non encore réalisé par le rendu de référence | HTTP. Pas de listener TLS `18443` à inventer |

Liste close de contextes de ce dépôt : `kind-aiforall-local` ou
`kind-aiforall-local-full`. Cette preuve utilise le second, déjà nommé par
l'ADR 0412. Le défaut legacy `kind-aiforall-local` ne la satisfait pas.

Contextes protégés, exclus : `kind-apparatus-p4-it`, `rancher-desktop`.

ADR 0606 reste Proposed. Elle décrit la frontière de lab (noms, topologie,
mapping, `connectors=[]` par défaut). Elle n'est pas Acceptée par ce contrat.

## Sens d'« exact-route »

Premier match gagne. Les deux fichiers portent les mêmes matchers ; les lignes
diffèrent.

`ops/deploy/mesh/envoy.yaml` :

- POST `/iam/api/auth/signup` ligne 57, `ext_authz` désactivé. Hors oracle OAuth.
- POST `/iam/api/auth/login` ligne 66, `ext_authz` désactivé. Hors oracle OAuth.
- GET/HEAD, regex ligne 77 :
  `^/iam/api/auth/(verify|username/check|[^/]+/(login|callback))$`,
  `ext_authz` désactivé.
- GET/HEAD `/iam/api/auth/github/relink-callback` ligne 99, chemin exact,
  `ext_authz` désactivé.
- GET/HEAD `/iam/api/auth/gitlab/relink-callback` ligne 108, chemin exact,
  `ext_authz` désactivé.
- préfixe `/iam/` ligne 117, `ext_authz` laissé actif.

`ops/deploy/apps/overlays/kind-mesh/envoy-mesh.yaml` : les mêmes matchers aux
lignes 66, 75, 86, 107, 116, et le préfixe `/iam/` à la ligne 125.

`relink-callback` ne matche pas la regex : le dernier segment n'est ni `login`
ni `callback`. L'alias `/iam/api/auth/{slug}/relink/callback` ne matche ni la
regex (segment en trop) ni les chemins exacts. Il tombe sur le préfixe
authentifié.

Exact-route veut dire : la requête entre par cet Envoy, et l'effet d'authz est
celui du matcher ci-dessus, pas celui d'un accès direct au pod ni du seul
préfixe `/iam/`. Les routes n'ont pas de nom Envoy : l'oracle est cet effet.

Attention à la configuration **finalement rendue et chargée** :
`render-local-full.py::render` retire les exemptions exactes relink des slugs
non configurés. Avec `connectors=[]`, les deux exemptions ci-dessus ne sont
donc pas conservées. Le profil de preuve doit retenir celle de son slug
configuré, sans ouvrir les START Link/Relink ni ajouter d'alias.

Routes applicatives correspondantes (`services/IAMRusty/http/src/lib.rs`) :

- GET `/api/auth/{provider_name}/login` public (départ login).
- GET `/api/auth/{provider_name}/callback` public (callback login et link).
- GET `/api/auth/{provider_name}/link` authentifié (départ link).
- GET `/api/auth/{provider_name}/relink-start` authentifié (départ relink).
- GET `/api/auth/{provider_name}/relink-callback` public, exemption exacte.

Le préfixe de service `/iam` est ajouté une fois devant ces chemins.

## Client

Navigateur. Le moteur possède le cookie jar, suit la redirection d'autorisation
et renvoie le cookie au callback. Un pilote CDP qui conduit ce moteur compte
comme navigateur. curl, même avec un en-tête `Cookie` forgé, ne compte pas :
cela changerait la conséquence « navigateur » de la ligne 37.

Sur le mapping HTTP du lab, l'origine publique IAM est `http` et le mode de
sécurité n'est pas `Verified`. Décision 3, exception locale explicite
(`OAuthBrowserPolicy`) : cookie `iam-oauth-browser`,
`Path=/iam; HttpOnly; SameSite=Lax; Max-Age=600`, sans `Secure`. Le callback
ne pose pas un cookie de remplacement. Le cookie `__Host-iam-oauth-browser`
(`Path=/; Secure`) n'est pas l'oracle de ce mapping HTTP.

## IdP

Un seul slug de campagne, `github` ou `gitlab`, présent dans la config du
connecteur de ce run (`connectors` non vide pour ce slug). Redirects exacts :

- `/iam/api/auth/{slug}/callback`
- `/iam/api/auth/{slug}/relink-callback`

L'IdP derrière ce slug est soit le vendor réel, soit un serveur d'autorisation
simulé que le connecteur appelle vraiment (autorisation puis échange du code).
Les deux autorisent le même verdict. Un troisième slug est hors allowlist
(décision 6, ADR 0411).

`connectors=[]`, état par défaut du profil local-full, ne constitue pas la
preuve. Désactiver PKCE alors que ce connecteur est `pkce_supported` pour faire
passer le run est un échec. L'échec PKCE lui-même n'est pas redemandé : il est
déjà dans les IT.

## Preuve historique réutilisable — ne pas redemander sans cause

Suites IAM : `oauth_browser_transactions`, `oauth_cleanup_lifecycle`,
`auth_transactions_postgres`. Elles couvrent le rejeu inter-replicas, le cookie
absent ou faux, state / provider / intention / cible / redirect / expiration,
la consommation avant échange, PKCE et l'échec, la purge sous backlog et
l'arrêt supervisé.

L'attestation utilisateur du 2026-10-08 (units + IT, sans artefact CI archivé)
reste la preuve de ces suites. Cette E2E ne les rejoue pas et ne les remplace
pas. Un commit qui modifie le parcours OAuth ou les matchers Envoy cités doit
les avoir vertes à nouveau avant le verdict ; ce contrat ne les lance pas.

## Matrice minimale

`S` est le slug configuré. Toutes les requêtes passent par l'Envoy du contexte
full. GET suffit ; HEAD partage le même matcher et n'est pas une campagne
séparée.

| Id | Requête | Oracle |
|---|---|---|
| P1 | Navigateur, GET `/iam/api/auth/S/login`, sans JWT | Regex, `ext_authz` off. `Set-Cookie` du nonce local. Redirection vers l'IdP du connecteur activé. |
| P2 | Même navigateur, GET `/iam/api/auth/S/callback`, cookie + code IdP | Regex, pas le préfixe. Réponse applicative IAM de login réussi. Pas de cookie inventé au callback. |
| P3 | GET `/iam/api/auth/S/relink-start` avec JWT plateforme, puis navigateur GET `/iam/api/auth/S/relink-callback` sans JWT, avec state + cookie | Le départ traverse le préfixe authentifié. Le callback matche le chemin exact, `ext_authz` off. |
| N1 | GET `/iam/api/auth/S/link` sans JWT | Préfixe `/iam/`. Pas de transaction, pas de `Set-Cookie`. |
| N2 | GET `/iam/api/auth/S/relink-start` sans JWT | Identique à N1. |
| N3 | GET `/iam/api/auth/S/relink/callback` sans JWT | Ni regex ni chemin exact. Pas de consommation. |

P1–P3 et N1–N3 verts, sur un même run, forment la matrice. Le login de P2 est
le callback regex. Le relink de P3 est la route exacte qui n'est pas cette
regex. Le départ link n'a pas de succès E2E séparé : N1 montre qu'il reste
authentifié.

## Verdict qui autorise Implemented

Ce contrat ne passe pas la Réalité à Implemented.

Un enregistrement peut autoriser cette promotion, par accord humain ou PR,
seulement s'il contient :

- le SHA du commit prouvé ;
- contexte `kind-aiforall-local-full`, cluster `aiforall-local-full`, ID du
  control-plane, port loopback observé ;
- le slug, et si l'IdP était simulé ou vendor ;
- le pilote navigateur (interactif ou CDP) ;
- P1, P2, P3, N1, N2, N3 verts ;
- les trois suites IT toujours vraies pour ce SHA, dans le sens de la section
  précédente.

Ne l'autorisent pas : la matrice incomplète, `connectors=[]`, curl, l'entrée
directe au pod, `kind-aiforall-local`, les IT seules, un rendu YAML, des pods
Ready, le gold path 0605, ou l'Accept de 0606.

Sans ID de control-plane et port loopback concordants avec le bail, le run est
INCONCLUSIVE. Il ne devient pas un échec détourné vers un autre contexte.

## Teardown et bail

Avant tout démarrage futur de cette preuve : inventaire, puis bail qui nomme
le contexte, le cluster, les IDs de nœuds (vides seulement si le cluster est
encore absent) et la demande. Le nom ou l'image ne prouvent pas la propriété.
Réutiliser exige les mêmes IDs, la topologie 1+2 et le mapping loopback.

Port `18080` occupé : STOP, sans tuer le propriétaire. API Kind : port loopback
aléatoire (`apiServerPort: 0`), pas l'ancien port fixe du legacy.

Le worker qui exécutera le lab rend l'inventaire. Il n'arrête pas un runtime
encore tenu par le bail du parent. À la fin, succès ou échec : arrêt gracieux
des nœuds Kind possédés par cette preuve, par IDs exacts. Annoncer la perte de
mémoire volatile. Stop n'est pas delete : pas de prune, `rm -f`, reset, ni
suppression de volume, de namespace ou de cluster.

Ledger compact, sans secret, avant les opérations longues. Kind reste éteint
pour écrire ou relire ce contrat.

## Hors scope du lot documentaire initial

Lab 0412, script, images Linux, fixture IdP, gold path 0605, code applicatif,
`workers/apparatus-operator`, `rustycog`, `.cargo/config.toml`, harness Python,
promotion de Réalité, Accept de 0606, gates Apparatus 0009–0011.

La mission de correction et qualification approuvée autorise ses futurs lots de
laboratoire/preuve uniquement dans l'allowlist du contrat d'alignement ratifié,
après leurs prérequis. Cette liste historique ne constitue ni un refus de ces
lots ni une autorisation générale de modifier le produit ou Apparatus.

## Décisions laissées ouvertes

1. Slug ratifié pour cette campagne : `gitlab`.
2. Fournisseur simulé borné derrière le vrai connecteur, sans compte personnel.
3. Pilote interactif ou CDP, tant que le moteur est un navigateur.
4. Hypothèse non retenue : `kind-aiforall-local`, deux IAM, curl, au plus 3
   conteneurs. La retenir changerait la ligne 37 et demanderait un amendement
   Proposed séparé (prochain libre services/runtimes : 0413). Il n'est pas
   écrit. Elle n'est pas retenue et n'est pas une étape en attente de GO.
5. Cookie `Secure` / `__Host-iam-oauth-browser` et listener TLS : hors de ce
   mapping HTTP. Les prouver ne fait pas partie de ce verdict.
6. Forme du lab et de la fixture : bornée par le contrat d'alignement ratifié ; implémentation et preuves restent à livrer contre cette matrice.

## Sources

- ADR 0412, décisions 1–7 et conséquence ligne 37 ; actualisation 2026-10-08.
- `services/IAMRusty/http/src/oauth_browser.rs`, `public_routes.rs`,
  `lib.rs` (routes login, callback, link, relink-start, relink-callback).
- `services/IAMRusty/domain/src/entity/oauth_transaction.rs` (consommation).
- `ops/deploy/mesh/envoy.yaml`, `ops/deploy/apps/overlays/kind-mesh/envoy-mesh.yaml`.
- `ops/deploy/apps/overlays/local-full/README.md` : contexte full, mapping,
  `connectors=[]`.
- `ops/deploy/kind/cluster-local-full.yaml` : `hostPort` 18080, `containerPort` 30080.
- ADR 0606 Proposed : frontière de lab, non Acceptée ici.

- Ratification du 2026-10-28 : [alignement et allowlist](0412-oauth-local-full-alignment.md), Accepted / Unimplemented ; validation documentaire seulement, aucun nouveau run.
