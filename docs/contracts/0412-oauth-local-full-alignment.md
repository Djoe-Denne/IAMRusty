# Contrat ratifié — aligner la preuve OAuth navigateur sur l'entrée HTTP local-full

- Statut : **Accepted / Partial**, ratification explicite du lead le 2026-10-28 sous délégation utilisateur après examen des alternatives, transports, allowlist et oracles. Assemblage opt-in intégré au36ffab6, tests statiques ciblés verts ; campagne navigateur, confiance TLS de l'image exacte et Envoy réel non exécutés. Aucune preuve de campagne déduite du rendu ; ADR0412 reste Accepted / Partial.
- Canon secondaire : [contrat exact-route 0412](0412-oauth-exact-route-envoy-proof.md), référencé par [ADR 0412](../adr/0412-oauth-transaction-persistante-liee-navigateur.md) **Accepted / Partial**.
- Invariants : [0308](../adr/0308-mesh-authn-jwt.md) **Accepted / Partial**, [0408](../adr/0408-connecteurs-idp-services-http.md), [0409](../adr/0409-confiance-callback-oauth-idp-connect.md), [0411](../adr/0411-idp-provider-slug-registry-fail-closed.md). [0606](../adr/0606-local-full-kind-isole.md) reste **Proposed / Partial**.
- Base examinée : application `c72ae362b950c63d6736a6818c68babc5a8d6f40`, gitlink SDK `d60a88b2f2f499768a807941a1b4c80c9a366b3f`. Aucun changement SDK requis par cette proposition.

## 1. Arbitrage et alternatives

**Retenir la cible existante, corriger son assemblage local seulement.** L'entrée
reste `http://127.0.0.1:18080` → NodePort `30080` → Envoy `10000`, sur
`kind-aiforall-local-full`, 1 control-plane + 2 workers. Le renderer local-full
possède déjà les transformations de namespace et d'exemptions relink : y
transformer explicitement le downstream de `listener_mesh:10000` en HTTP.
Échouer si le listener attendu n'est pas unique ; ne pas supprimer des
`transport_socket` par recherche globale ou sur les clusters upstream.

Alternatives écartées pour cette mission :

- Garder TLS sur `18080` et importer un certificat client dans le navigateur : contredit l'entrée HTTP retenue. Distribuer la clé passerelle serait interdit ; un certificat navigateur distinct resterait une autre preuve, pas une réparation transparente.
- Créer `18443`/une nouvelle edge TLS et utiliser un cookie Secure : autre cible de preuve ; amendement Proposed à ratifier avant ce changement, pas nécessaire ici.
- Retirer TLS du mesh partagé, appeler IAM directement ou utiliser le legacy : casse le périmètre, l'identité des hops ou l'oracle exact-route.
- Remplacer le moteur par reqwest/curl, ou le connecteur par le mock IAM→connecteur des IT : ne prouve pas le parcours demandé.

**Aucune ADR 0413 à créer.** Il n'y a ni nouvelle API produit, ni nouveau stockage,
ni changement des décisions 1–7 de 0412 ou des hops de confiance 0308. Ce contrat
borne l'assemblage d'une preuve déjà ratifiée. Une bascule de cible vers TLS,
un autre cluster ou un protocole non-browser dépasserait ce choix.

## 2. Transports, origines et certificats

- **Navigateur→Envoy : HTTP local uniquement**, sans certificat client, sans proxy qui contourne Envoy et sans tolérance TLS globale. Le mapping hôte reste lié à `127.0.0.1`, pas `0.0.0.0`. Ce n'est pas une qualification d'edge TLS/production.
- **Envoy→ext-authz/IAM/Hive/Telegraph/Manifesto : mTLS inchangé.** CA, certificat client `envoy-mesh`, SNI et vérification précise des noms serveur restent. Le correctif séparé d'identité serveur Envoy doit être intégré avant transfert de ces fichiers ; ne pas écraser ses matchers SAN.
- Les services restent en mode principal mesh : seuls les headers recréés par la passerelle et son certificat client autorisent le principal. Lua strip `x-principal-*`, ext_authz HTTP, `failure_mode_allow: false`, audiences/issuer et fraîcheur maximale 60 s ne changent pas.
- `jwt.public_base_url` IAM reste **`http://127.0.0.1:8080`**, ainsi que l'autorité plateforme exacte `http://127.0.0.1:8080/iam` et ses policies explicites/false. Le navigateur est réellement sur18080 ; ses redirects sont configurés séparément par le registry. Voir le [complément d'autorité ratifié](0412-oauth-proof-authority-alignment.md), qui corrige l'ancienne exigence public_base_url18080 après vérification des corps. Aucun alias ni changement de `jwt.issuer`, audience ou JWKS internes.
- IAM reste explicitement `security.mode = local_insecure` pour cette exception HTTP. Cookie **`iam-oauth-browser`**, host-only `127.0.0.1`, `Path=/iam; HttpOnly; SameSite=Lax; Max-Age=600`, sans `Secure` ni `Domain`. Aucun cookie injecté par le pilote, aucun cookie de remplacement au callback. Pas de variante `__Host-` annoncée comme prouvée.
- Les callbacks enregistrés sont exactement `http://127.0.0.1:18080/iam/api/auth/gitlab/callback` et `http://127.0.0.1:18080/iam/api/auth/gitlab/relink-callback`, identiques chez IAM et GitLabConnect. Ni `localhost`, ni `8080`, ni slash final, ni alias. Ne pas changer Host/schéma par un header pour les faire accepter.

**IAM→GitLabConnect** conserve HTTPS + HMAC-SHA256 (0409), sans Bearer utilisateur.
Le service privé proposé est `gitlab-connect:8443`, préfixe `/gitlab-connect` ;
certificat serveur préparé pour `gitlab-connect` et
`gitlab-connect.aiforall-local-full.svc.cluster.local`. Le client TLS IAM valide
CA et nom. Le client cert reste optionnel sur ce hop selon 0409 : ne pas exiger
un certificat que son client actuel ne fournit pas, ni affaiblir les autres hops.

Le préparateur full actuel ne génère pas ce leaf et le setup IAM appelle
`HttpIdpConnector::with_security_mode`, sans roots additionnelles. Préparer du
matériau **nouveau et possédé**, puis rendre la CA publique disponible dans le
trust OpenSSL du seul process IAM Linux (bundle explicite incluant les roots
usuelles, montage lecture seule ; `SSL_CERT_FILE` si confirmé par le backend
construit). IAM utilise ici reqwest 0.11.27 avec native-tls par défaut et ses
images Debian ont OpenSSL/ca-certificates. Ce chemin de déploiement doit être
prouvé sur l'image exacte ; le lock seul n'en garantit pas la résolution effective.
Pas d'installation de CA hôte, pas de clé de CA/passerelle dans IAM ou le
navigateur, pas de `danger_accept_invalid_*`, pas de fallback HTTP sur ce hop.
Si cette injection de trust échoue, garder la preuve INCONCLUSIVE et revenir
avec le besoin précis ; ne pas ajouter silencieusement une API de config/DI.

Certificats/SAN publics et empreintes suffisent au dossier ; pas de lecture d'un
Secret vivant, ni de clé privée en sortie. Absence actuelle du leaf, d'un moteur
ou d'une fixture = **prérequis futur manquant**, pas blocage de la décision ni
autorisation de changer sa cible.

## 3. Connecteur réel et IdP simulé — choix ratifié

Choisir **`gitlab`** : les endpoints vendor et la capacité PKCE S256 sont déjà
configurables. Le vrai binaire `gitlab-connect-service` traite les appels IAM
HMAC `/gitlab-connect/v1/authorize`, `/gitlab-connect/v1/token`,
`/gitlab-connect/v1/profile` (constantes `crates/idp-connect-contract/src/dto.rs`).

Activer un **overlay de campagne opt-in** sous
`ops/deploy/apps/overlays/local-full/oauth-proof/`, sélectionné uniquement par
une option explicite `--oauth-proof` du renderer/deployer existant. Cette option
est un livrable futur, pas une commande actuellement disponible. Le profil full
par défaut garde `connectors=[]`. Le renderer doit lire le registry du profil
sélectionné, puis calculer ses exemptions relink et le hash de configuration.
Pas de lecture du registry vide du profil de base après sélection de l'overlay.

Un simulateur HTTP minimal de fournisseur, test-only, réside dans le même
namespace full. Réutiliser Python stdlib et l'image Python **déjà épinglée** du
bootstrap si aucune fixture compatible n'est réutilisable ; pas de nouvelle
plateforme IdP, de framework, de dépendance Python/npm ou de compte personnel.
Ce serveur n'est **pas** un remplacement du connecteur et n'émet pas de JWT IAM.

Configuration vendor du connecteur :

- `auth_url = http://localhost:18081/oauth/authorize` : navigation réelle du moteur hôte ; IAM reste sur `127.0.0.1:18080`.
- `token_url = http://oauth-idp-simulator:8080/oauth/token`, `user_url = http://oauth-idp-simulator:8080/api/v4/user` : vrais appels sortants du connecteur dans Kind.
- `gitlab.transport_security = isolated_test`, explicitement limité à ce fournisseur synthétique ; aucun downgrade de la politique globale ou d'un vendor réel. `pkce_supported = true` côté IAM **et** connecteur, après réception du support.
- Secret client et HMAC dédiés au run, sans compte personnel, tenus dans les entrées privées possédées ; pas de valeur en rapport, screenshot, trace/HAR, URL de commande ou argument de process.
- Autorisation avec `response_type=code`, `client_id`, redirect exact, state inchangé et challenge S256. Le simulateur délivre un code à usage unique/TTL borné, lié au client/redirect/challenge ; l'échange vérifie ces bindings, le secret client et `BASE64URL(SHA256(verifier))`. Token/profile synthétiques liés au code, compte GitLab stable et email vérifié propres au run. Pas de succès inconditionnel.
- Observations minimales : autorisation vue par le navigateur, échange token et appel profil vus par le simulateur, PKCE validé ; corrélation par identifiant opaque de run/compteurs, jamais état, code, verifier ou token brut.

Exposer **seulement l'IdP simulé** au navigateur par un port-forward loopback
`127.0.0.1:18081 → Service oauth-idp-simulator:8080`. Cela ne remplace pas
l'entrée IAM/Envoy. `localhost` doit être réellement joignable sur ce mapping ;
le callback reste impérativement sur `127.0.0.1`. L'IdP ne reçoit donc pas le
cookie host-only IAM. Aucun nouveau NodePort, mapping de nœud ou listener `18443`.

Le garde-cible existant ne supporte **pas** encore `port-forward`. Prévoir une
extension locale bornée : namespace full, Service/UID possédé, mapping et bind
ci-dessus exclusivement, kubeconfig fixe, contrôle endpoint/UID avant lancement,
PID/port/bail parent tracés et arrêt gracieux du seul processus possédé. Les
arguments invalides sont rejetés avant probe/API. Aucun appel brut hors garde,
retarget, port libre substitué ou nouveau verbe générique ouvert par facilité.
Le port occupé, une cible ambiguë ou une résolution incompatible arrêtent le run.

NetworkPolicies additives : IAM→connecteur 8443, connecteur→simulateur 8080,
DNS déjà permis, ingress correspondants par labels exacts. Pas d'egress Internet,
de namespace-wide allow ou d'exemption IAM supplémentaire. Le chemin hôte→Envoy
via NodePort est à vérifier réellement ; ne pas ouvrir tout le namespace pour
contourner une source NAT ou une politique non comprise.

L'option vendor réel reste compatible avec le contrat existant, mais n'est pas
la recommandation : elle nécessite ses credentials légitimes, HTTPS et une
configuration/egress explicitement attribués. Ne pas y basculer automatiquement.

## 4. Pilote et scénarios discriminants

Réutiliser un moteur disponible, interactif ou piloté par le MCP Playwright/CDP
existant. Ne pas installer navigateur, proxy ou stack de pilotage par défaut.
Le pilote peut conduire une page/fetch du **moteur** ; son client HTTP propre,
`APIRequestContext`, curl et reqwest ne peuvent remplacer les étapes de preuve.
Le navigateur possède le cookie jar ; ni `addCookies`, ni headers Cookie forgés.
La présence d'un outil au catalogue n'accorde aucune permission : le catalogue
examiné expose notamment `browser_run_code_unsafe` (JS arbitraire côté serveur),
**non appelé ici et non autorisé par ce contrat**. Le préflight doit retenir
un pilote et des opérations admis par le profil exécutant ; à défaut, conduite
interactive avec les mêmes oracles, ou INCONCLUSIVE. Ne pas élargir permissions,
installer une stack ou contourner un refus pour utiliser le scénario JS.

- Préparation : contexte navigateur dédié sous bail, origine IAM exacte, IdP/config/images identifiés. Pour obtenir un compte complet, utiliser les routes réelles d'inscription OAuth puis `complete-registration` si nécessaire, sans injection SQL/clé/JWT. Un callback 202 `registration_required` est un prérequis, **pas** P2 « login réussi ». Ensuite refaire un cycle login complet.
- **P1/P2** : navigation GET login sans Authorization ; le moteur reçoit le cookie puis suit vers l'autorisation du simulateur et son redirect IAM. Même contexte, state/code produits par ce flux, callback sans JWT. Exiger la réponse IAM de login **200**, `operation=login` et le compte attendu ; conserver les credentials uniquement en mémoire de campagne.
- **P3** : depuis l'origine IAM, faire le GET `relink-start` dans le moteur avec le JWT plateforme réel de ce compte, limité à **cette requête**. Le handler renvoie **200 JSON `{auth_url}`**, pas nécessairement une redirection 303. Naviguer ensuite vers cette URL inchangée ; l'IdP renvoie le navigateur sur le **chemin exact** `relink-callback`, avec state/cookie, **sans JWT**. Exiger le succès de relink sur le compte lié au START. Pas de header Authorization global qui fuit vers IdP/callback.
- **N1/N2/N3** : sans JWT, GET link, relink-start, puis l'alias `relink/callback`, par Envoy. Attendre le refus du gateway, pas un 404 applicatif, un 5xx TLS ou un refus par provider absent. Pas de nouvelle transaction/cookie, échange vendor ou consommation. Config effectivement chargée + observations ciblées ext_authz/upstream IAM établissent le refus avant IAM ; une absence de log seule ne suffit pas.
- GET suffit à P1–P3/N1–N3 ; les assertions source conservent HEAD et l'ordre first-match. Le slug absent garde ses exemptions relink absentes. Les matchers regex existants ne sont ni élargis ni remplacés par un préfixe public.

Le succès P1–P3 et N1–N3 est exigé **dans le même run**. Les IT restent l'oracle
anti-rejeu/multi-replica, liaisons négatives, PKCE négatif et cleanup : ne pas les
réimplémenter intégralement dans le navigateur. Conserver leurs gates au SHA
intégré si parcours/matchers changés. Ni ce run HTTP ni la simulation ne qualifient
les cookies Secure, une edge TLS, le vrai vendor, HA ou la production.

## 5. Allowlist implementer et séquencement

Édits futurs permis après intégration vérifiée et publication du consommateur par le lead, limités au besoin de preuve :

1. `ops/deploy/render-local-full.py` : transformation downstream full seulement, choix fermé du profil de campagne, registry et hash effectifs ; le rendu Kustomize brut reste une entrée intermédiaire, pas la config finale.
2. `ops/deploy/deploy-local-full.py`, `ops/deploy/local-full-images.py` : propager le choix explicite et intégrer le connecteur/image exacts, certificats et ressources possédées ; conserver vérifications préalables/baux/registre et mode par défaut.
3. Nouveaux fichiers fermés `ops/deploy/apps/overlays/local-full/oauth-proof/{kustomization.yaml,iam.toml,gitlab.toml,resources.yaml}` : surcouche de campagne, ConfigMaps/Secrets locaux, connecteur/simulateur et politiques minimales. Pas de changement de contrat produit.
4. `ops/scripts/prepare-local-full-certs.sh` : préparation optionnelle du leaf GitLabConnect dans une destination nouvelle, sans écraser/rotater des entrées vivantes ; sélection des seules clés nécessaires par workload. Pas de clé CA/`envoy-mesh` ajoutée au connecteur.
5. `ops/deploy/local_full_target.py`, `ops/tests/test_local_full_target.py` : seul support borné de forwarding décrit plus haut et ses refus avant API. Ne pas refondre le garde ni ses autorisations existantes.
6. Nouveaux `ops/tests/support/oauth-idp-simulator.py`, `ops/tests/oauth-browser-local-full.js`, `ops/tests/test_oauth_browser_alignment_source.py`, `ops/tests/test_oauth_idp_simulator.py` : fixture/test/scénario uniquement. Le fichier JS est une séquence pour le pilote existant, pas un nouveau runner/framework installé.
7. `ops/tests/mesh-public-route-cases-local-full.json`, `ops/tests/test_local_full_acceptance_source.py` : séparer les attentes provider absent du profil par défaut et provider activé en campagne, conserver les négatifs ; ce tableau HTTP n'est jamais la preuve navigateur.

Attribution de compatibilité du lead après inventaire du consommateur :
`ops/tests/infra-static.sh`, uniquement ajout de l'overlay de preuve à
l'inventaire fermé et au nombre attendu de rendus. Inventaire vérifié ensuite :
neuf entrées historiques, dont `ops/deploy/p4/knative` déjà suivi ; total **dix**
avec oauth-proof. Conserver toutes les entrées et vérifier l'opt-in, sans ignorer de rendu,
relâcher l'égalité d'inventaire ou ajouter une nouvelle infrastructure de tests.

Les sources `ops/deploy/mesh/envoy.yaml` et
`ops/deploy/apps/overlays/kind-mesh/envoy-mesh.yaml` restent sous ownership du
correctif SAN séparé ; ce lot ne retire pas leur TLS. Configuration/service
partagés IAM/SDK/GitLabConnect, schéma, migrations, FGA, Cargo, image de base,
cluster YAML et scénarios historiques sont hors allowlist. Toute nécessité
supplémentaire revient au lead avec fichier, cause et invariant, pas un edit tacite.

Migration : **aucune migration métier ni remplacement de cluster**. Overlay de
preuve ajouté sur ressources dont le bail atteste la propriété ; pas de lecture
ou remplacement d'un Secret étranger. Default full inchangé hors correction de
son entrée HTTP annoncée. Le compteur historique 112 n'est pas figé pour la
surcouche ; consigner le nombre effectivement rendu, sans déduire une preuve réseau.

## 6. Validation prévue — aucune commande exécutée dans cette décision

Ajouter les noms de régression suivants au plan avant leur exécution :

- `test_full_http_downstream_preserves_internal_mtls_and_shared_tls`, `test_proof_registry_drives_exact_relink_exemption`, `test_proof_origin_redirects_pkce_and_private_connector` : rendu **final en mémoire**, pas de main écrivain ni Secrets imprimés.
- `test_forward_rejects_foreign_target_bind_service_and_mapping_before_api`, `test_forward_requires_owned_service_and_stops_only_owned_pid` : garde de forwarding avec commandes factices.
- `test_simulator_consumes_code_once_and_verifies_s256_client_redirect`, `test_simulator_receives_real_authorize_token_profile_without_secret_output` : fixture seule, pas qualification du produit.
- Campagne moteur `oauth-browser-P1-P2-P3-N1-N2-N3` et contrôle chargé `full-http-edge-internal-mtls` : run futur après gates et préflight, résultats par cas, pas un PASS de renderer.

Commandes ciblées à livrer/utiliser après ajout des tests, **pas exécutées ici** :

```powershell
python -B -m unittest discover -s ops/tests -p test_local_full_target.py
python -B -m unittest discover -s ops/tests -p test_local_full_acceptance_source.py
python -B -m unittest discover -s ops/tests -p test_oauth_browser_alignment_source.py
python -B -m unittest discover -s ops/tests -p test_oauth_idp_simulator.py
cargo test --locked -p iam-service --test oauth_browser_transactions --test auth_transactions_postgres --test oauth_cleanup_lifecycle
```

Les trois suites Rust sont sous slot Cargo du lead, Windows natif, jobs=24,
cible E: et fixtures testcontainers existantes ; pas Kind pour les IT. Le reste
des checks de mission se sélectionne selon diff, sans relance exhaustive pour ce
contrat documentaire. Une commande Kustomize brute n'établit pas les
transformations du renderer ni le réseau. L'implementer rend les appels exacts
du pilote existant et du forwarding **après** validation de leurs capacités,
avant la campagne ; aucune CLI future n'est présentée ici comme existante.

Préflight/campagne : mêmes SHA application/SDK, images immuables, hash du profil,
CA/SAN publics, IDs nœuds et UID API liés au bail ; origine `18080`, bind/PID du
forward `18081`, moteur/version et slug consignés. Kind seulement après
corrections/IT finales et revues sans blocage. Réutiliser les gardes, skills de
preuve/lifecycle et inventaires existants ; ne lancer aucun script historique
mesh tel quel. Le test Envoy bon nom/mauvais nom même CA appartient au lot SAN :
son vrai trafic Envoy reste exigé, ce contrat ne le remplace pas par reqwest.

PASS 0412 seulement avec matrice complète, vrai connecteur et observations du
fournisseur, IT valides pour le snapshot, preuves liées aux images/configs.
Prérequis absent = **INCONCLUSIVE**, sans retarget/installation implicite ; refus
fonctionnel discriminant observé = FAIL. Tous les nouveaux processus/ports et
ressources possédés inutiles sont arrêtés gracieusement par identités exactes,
sans toucher aux baux actifs/étrangers ni supprimer cluster/namespace/volumes.
Aucune promotion autonome de 0412/0308 ou Accept de 0606.

## 7. Sources et limites vérifiées

- `ops/deploy/apps/overlays/local-full/platform/kustomization.yaml:9-12` importe les services, ext-authz et Envoy de kind-mesh ; `kind-mesh/envoy-mesh.yaml:19-39` configure le downstream TLS/client cert. `render-local-full.py:20-57` n'enlève pas ce TLS et filtre déjà les exemptions relink selon le registry. Rendu historique 112/TLS réutilisé du ledger, **pas réexécuté**.
- `ops/deploy/apps/overlays/local-full/config/iam.toml:1-3,29-36` : local_insecure, connectors vide et public_base_url 8080, distinct de la cible 18080. `platform/policies.yaml` : default-deny ; pas de connecteur/simulateur déclaré dans ce profil examiné.
- `services/IAMRusty/http/src/oauth_browser.rs::OAuthBrowserPolicy::new/cookie_headers`, références vers `setup/src/app.rs::open_iam_signing_pool` ; `handlers/auth.rs::handle_login_callback`, `generate_relink_provider_start_url`, `relink_provider_callback` : réponses 200/202, START JSON et consommation liée avant échange.
- `services/IAMRusty/tests/support/browser_flow.rs:1-60` utilise **reqwest**, extrait le cookie et l'URL ; ce support ne constitue pas un moteur. `tests/fixtures/idp_connect/service.rs` simule le connecteur sortant des IT, pas le fournisseur derrière un vrai connecteur.
- `services/GitLabConnect/configuration/src/lib.rs::GitLabConfig`, `infra/src/gitlab.rs::from_config_with_transport_and_pkce`, `validate_vendor_url`, `authorize_with_pkce`, `exchange_code_with_pkce`, `user_profile` : endpoints configurables, exception transport explicite, S256 et requêtes HTTP vendor réelles.
- `services/IAMRusty/setup/src/app.rs::setup_http_idp_clients`, `infra/src/auth/http_connector.rs::with_security_mode_and_roots`, `infra/Cargo.toml:36`, `Cargo.lock:6080-6121`, Dockerfiles IAM/GitLabConnect : roots disponibles dans l'adapter mais non injectées par le setup ; HTTPS/HMAC conservés, trust de déploiement à prouver.
- `ops/scripts/prepare-local-full-certs.sh:28-34`, `ops/deploy/deploy-local-full.py::cert_inputs` : liste actuelle sans GitLabConnect. `local_full_target.py:34-54,295-316` : grammaire sans port-forward et vérifications avant commande.
- Wiki consulté par QMD : `projects/iamrusty/decisions/0412-oauth-transaction-browser-binding.md` (pointeur Accepted/Partial), pas autorité supérieure au canon. ADR 0200/0201 bornent les IT et les simulations HTTP sortantes ; pas de nouveau harness par principe.

Serena a analysé les symboles de la racine canonique ; ce worktree documentaire
est à la même base pour ces sources. Son futur delta non mergé ne sera pas
couvert par cet index : validation des fichiers worktree obligatoire. Aucune
fixture déployée, capacité moteur/CDP, identité de certificat vivant, commande
Cargo, navigateur ou opération Docker/Kind/WSL n'a été exécutée pour cette décision.
