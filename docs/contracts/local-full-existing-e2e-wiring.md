# Local-full — entrée bornée et raccord des E2E existantes

2026-11-02 — contrat du **GO utilisateur « oui »** consigné dans
`.cursor/handoffs/20261102-local-platform-e2e.md`, base `af5315b`, SDK `93b890f`.
Implémentation et preuves actuelles à produire ; aucun résultat live apporté ici.
Complément de [séparation runtime/validation](ops-kind-runtime-validation-separation.md)
et de son [manifeste](ops-kind-runtime-validation-manifest.md), sans nouveau framework.
**Aucune ADR nouvelle** : application locale des frontières existantes, pas de
changement produit/protocole/stockage. ADR0308/0314/0412 restent canoniques ;
0606 reste **Proposed / Partial**, aucune ratification globale implicite.

## 1. Entrée : conserver l'autorité et distinguer les témoins

- Cible unique : bail d'origine vérifié, `kind-aiforall-local-full`, entrée
  `127.0.0.1:18080 → CP:30080 → Envoy:10000`. Les trois nodes préexistants ne
  deviennent pas possédés par cette campagne. Aucun bootstrap/deploy/reset/CA,
  Service/listener/route, egress, Secret GET ou changement de politique globale.
- Réutiliser `LocalClusterTargetAnchor.envoy_tuple_tooling_preflight`,
  `envoy_tuple_begin` / `envoy_tuple_finish` et `envoy_admin_snapshot` :
  pas de nouveau collecteur. Le résultat de l'agent Envoy actuel est une entrée
  de revue, pas une preuve de source déduite de l'IP bridge du CP.
- `configure_envoy_ingress` reste une opération spécialisée du parent, hors
  `deploy/main` et hors préparation E2E. Conserver les sept projections privées,
  leurs RAW SHA, les huit bases causales et les gardes de `_ingress_evidence` :
  binding NAT frais unique, source mesurée égale à l'IPIP annotée du CP ancré,
  différente de son IP bridge, baseline/passive stables, une requête de mesure,
  incarnations/UID stables, revue causale actuelle et limites explicites.
  Source non démontrée ou différente de cette forme ⇒ STOP, pas de fallback.
- Capture actuelle avec helper figé : utiliser `IDENTICAL_RAW` existant.
  Ne pas réécrire les anciens proofs, ajouter une paire historique pour éviter
  une recapture, ni traiter un JSON utilisateur comme autorité de mutation.
  Changement des octets du helper après capture ⇒ nouvelle capture, sauf paire
  exacte réellement revue selon le [contrat ACK](local-full-envoy-ingress-patch-ack.md).
  Pour ce raccord, **figer `local_full_target.py` de la capture à la clôture du
  patch/témoin/rollback ingress**. Intégrer une éventuelle extension de cleanup
  seulement après cette clôture, ou avant une nouvelle capture à la version
  courante ; jamais au milieu, jamais par un pin de compatibilité fabriqué.
- Mutation maximale : **une entrée source attestée `/32`, TCP10000**, dans
  `allow-envoy` inchangé par ailleurs, conformément au [contrat ingress](local-full-envoy-nodeport-ingress.md).
  Confiance L3 de laboratoire seulement ; JWT, principal, gate interne, SAN et
  TLS/mTLS inchangés. Conserver ACK JSON figé UID/RV/spec, refus de concurrence,
  `NOOP_NOT_OWNED`, perte d'ACK incertaine et rollback de la seule entrée ACKée.

**Sélection par profil EFFECTIF, jamais par le résumé ou le nom « full ».**
La découverte RO transmise par le parent le 2026-11-02 contredit l'hypothèse
`connectors=[]` : IAM utilise une configuration montée `oauth-proof`, projection
non secrète `connector_tables=1`, `empty_array_lines=0`, SHA256
`ba3e0bc3ae84e993426fc037ea34f2b3542849bacc5b41eec11bbe0aa9c6e84e`.
Cette projection seule ne prouvait pas l'identifiant ; le préflight final du
parent a depuis confirmé **GitLab**, sans Secret GET, dump de configuration ni clé.
Lier le profil à la configuration effectivement sélectionnée/montée et à son
hash, ainsi qu'aux identités courantes ; une étiquette ou un input JSON ne suffit
pas. GitLab confirmé ⇒ **activation existante oauth-proof : témoin v1/303 strict**.
Ne pas modifier IAM pour obtenir le profil attendu. Profil ambigu, changé ou non
couvert par un témoin fermé ⇒ STOP ; jamais fallback automatique 303 ↔ 422.

**Amendement catalogue historique — deuxième activation bornée.** Le parent
atteste ensuite le préflight GitLab PASS : ce runtime utilise donc le v1/303,
pas le témoin base-full. La première activation a refusé **avant journal/ACK/
mutation** une seule policy absente du catalogue source courant :
`aiforall-local-full/allow-bao-seed`, UID
`415e741e-f6a1-42c4-aaf0-326f35a7387f`, possédée par le bail d'origine ; source
historique supprimée par `f60910589e3867236c71beacb3ddbce447db74f7`
(faits transmis par le parent, pas remesurés par l'auteur).
Reconnaître cette seule entrée par **nom/namespace exacts, UID égal à celui du
bail original et spec entière strictement égale** à :

```json
{"podSelector":{"matchLabels":{"app.kubernetes.io/name":"openbao-seed"}},"policyTypes":["Egress"],"egress":[{"to":[{"podSelector":{"matchLabels":{"app.kubernetes.io/name":"openbao"}}}],"ports":[{"protocol":"TCP","port":8200}]}]}
```

Cette reconnaissance est une exception fermée de catalogue dans
`configure_envoy_ingress`, pas un manifeste à appliquer ni une autorité fournie
par inputs. Préserver la policy inchangée ; pas de suppression, adoption,
recréation ou restauration de son ancienne source. Le selector seul ne suffit
jamais ; autre spec/UID/namespace/nom/ownership ou autre policy inconnue ⇒ refus.
Elle ne sélectionne pas Envoy ; tous les contrôles des policies applicables,
du default-deny, des identités et de l'ACK demeurent inchangés.
Tests parent : nominal exact et préservation ; négatifs port/peer/selector/
policyTypes/champ supplémentaire, UID remplacé, ownership absent et autre nom
inconnu. Relecture security du delta figé avant la deuxième activation, avec
revalidation courante des gardes ; aucun retry aveugle. `local_full_target.py`
reste figé, aucun pin de compatibilité ni permission réseau nouvelle.

**Seul delta ingress de capacité : un témoin base-full distinct et fermé.**
Ne pas élargir `aiforall.envoy.ingress-parent-witness.v1` : son GET GitLab/303/
`IAM_CONNECTOR_NONCE_REDIRECT` et ses négatifs 200/302/etc. restent stricts.
Ajouter la variante `aiforall.envoy.ingress-base-full-witness.v1`, liée au même
journal/ACK, utilisable **seulement si le profil effectif est attesté sans
connecteurs** ; sa création n'autorise pas son activation sur l'oauth-proof
actuel. Aucune méthode, URL,
route, statut attendu ou décision de succès paramétrable par les inputs.
Pour ce profil base uniquement, le parent exécute ces trois oracles sur l'entrée fixe :
1. GET `/iam/.well-known/jwks.json` : **200**, JWKS JSON avec clé publique utilisable ;
2. GET `/iam/api/auth/gitlab/login` : **422**, connecteur indisponible, sans
   redirection OAuth réussie ;
3. GET `/iam/api/me`, sans Bearer : **401 ou 403**, jamais timeout/TLS/HTTP000.
Revalider profil/configuration non secrète et identité avant/après ; exiger
progression du compteur IAM upstream, incarnation Admin stable et erreurs
`ssl.fail_verify_san` / `ssl.fail_verify_error` inchangées. Le reçu parent lie
les observations actuelles et leurs hashes au journal ; `finish` vérifie leur
forme/liaison, ne prouve pas à lui seul la vérité HTTP. Échec ⇒ rollback ACK
conditionnel existant, jamais nouvelle propriété par GET. Ce témoin prouve
l'entrée et les refus, **ni login réussi ni E2E métier**.

## 2. Trois actions existantes, entrées privées, aucun setup caché

Conserver `ops/tests/local-full-acceptance.py`, `--environment` et son mutex/bail.
Raccorder `prepare-probes`, `e2e --suite mesh-authn|apparatus`, `cleanup-probes`
par des adaptateurs locaux bornés ; ne jamais lancer les wrappers legacy,
`KindCluster::ensure`, builds/loads, `SignRegistryStack::start`, serveur TestServer,
bootstrap FGA, set-env/rollout ou nouveau namespace destiné à masquer un écart.

- Inputs privés v1, parsés strictement par action/suite : `run_id` UUID neuf,
  suite, références privées de credentials/fixtures nécessaires, chemin de reçu
  et de preuves owner-only ; pas de commande/SQL/URL/cible/attente HTTP libre.
  `prepare-probes` choisit sa suite dans ces inputs, sans inventer un flag
  contournant la validation actuelle de `--suite`. Un fichier n'accorde aucune
  propriété : bail, UID et provenance de création sont revérifiés.
- **Prepare** : qualifier clients/outils, transport et nettoyage AVANT création ;
  préparer seulement fixtures déclarées UUID, artefacts déjà épinglés et probes
  nécessaires. Reçu privé lié à run/cible/namespaceUID/source : IDs/PK exacts,
  ACK de création, état initial et inverse autorisé. Pas d'adoption par nom/label.
- **E2E** : scénarios sur les services **déjà déployés**, via l'entrée pour les
  appels publics ; accès interne seulement pour les oracles S2S/isolation
  concernés. Chaque mutation métier ajoute immédiatement ses IDs au reçu.
  Préférer les références privées aux certificats/clefs leaf **déjà présents dans
  la PKI hôte owner-only du déploiement**, ouverts en lecture seule par le
  processus de test parent trusted (`ssl.load_cert_chain` ou client équivalent).
  Chemins canoniques/ACL/provenance qualifiés, identité attendue par oracle et
  contexte TLS séparé ; jamais clé de CA, export/copie de fichier ou Secret GET.
  Pour l'interne : port-forward au Pod/Service précisément attesté UID, vérification
  CA et SAN serveur conservée malgré l'adresse loopback, sans `insecure`.
  Pas de certificat gateway utilisable par un client/endpoint/principal libre :
  son emploi S2S est limité aux cas fixes et au seul utilisateur créé par ce run.
  Alternative si nécessaire : probe par identité, projection `items` minimale,
  jamais bundle global ni extraction de clé montée. Outil absent ⇒ pas
  d'installation/modification du Pod de service ; qualifier l'autre couture sûre.
- Auth locale : signup → complete-registration → MailHog, message adressé
  uniquement à l'email UUID du run → GET `/iam/api/auth/verify?email=…&token=…`
  → login réel → refresh réel. Le token reste privé/en mémoire ; pas d'UPDATE
  `is_verified`, de session inventée ou de seed SQL arbitraire. Qualifier d'abord
  l'accès MailHog/API disponible ; absence de couture sûre ⇒ BLOCKED précis.
- **Cleanup**, aussi après échec : inverse des seules créations attestées,
  préconditions UID/PK/provenance avant retrait ; jamais pré-delete, DROP,
  TRUNCATE, reset, suppression de namespace ou cascade étrangère. API existante
  d'abord ; SQL fixe/paramétré limité aux vérifications, à la fixture provider
  ci-dessous et au nettoyage des seules fixtures possédées ; jamais SQL fourni
  par inputs, utilisateur/session fabriqués ou contournement de vérification email.
  Le router IAM n'expose pas de suppression de compte : qualifier cet inverse
  précis avant signup, sinon bloquer cette création, sans inventer d'API produit.
  Fermer les port-forwards/processus/probes possédés, pas les nodes/registry
  préexistants. Résidu ou ACK incertain ⇒ reçu conservé, cleanup non clôturé.

**Qualification — fixture provider et PKI hôte.** L'interdiction absolue de seed
provider / l'obligation de rester dans les Pods étaient trop restrictives :
[séparation §Interface](ops-kind-runtime-validation-separation.md#interface-validation-proposed-pas-un-cli-déjà-livré)
interdit le SQL **caché**, mais permet un setup isolé explicite/réversible.
ADR0606 §Décision3 protège la CA/projection publique et interdit Secret GET ;
aucune remise de clé CA ou nouvelle projection privée n'est autorisée ici.
L'usage local de leafs déjà possédées n'en est pas une exportation ; ADR0308 §7
rappelle toutefois que la clé gateway permet l'usurpation de principal : entrée
privée trusted et appels fixes ci-dessus indispensables, à revoir avec security.

Mode explicite `SIMULATED_PROVIDER` : après signup/completion/verification/login
réels, l'adaptateur de préparation des données insère **une** ligne synthétique
`provider_tokens` pour cet utilisateur UUID, provider fixe `github`,
`provider_user_id` nonce UUID du run et token synthétique sans valeur vendor.
SQL versionné et paramétré, transaction/précondition d'absence pour ce user/provider
et ce nonce, jamais UPSERT/UPDATE ni emprunt d'une ligne existante ; ACK
`RETURNING id` enregistré immédiatement avec run/user/provider/nonce dans le reçu.
Le schéma source a une PK **entière auto-incrémentée**, pas un UUID de ligne
(`services/IAMRusty/migration/src/m20220101_000001_initial_schema.rs:213–273`).
ACK perdu/conflit ⇒ STOP sans retry/adoption. Vérifier que les refus laissent
cette ligne intacte ; revoke réel doit la retirer. Cleanup résiduel uniquement
par PK ACKée ET user/provider/nonce/contenu attendu, refus de toute ligne changée
ou étrangère ; absence après revoke attendue, pas prétexte à supprimer une autre
ligne. Tester ces gardes avant live. HTTP/mTLS/auth/S2S restent **LIVE**, fixture
provider déclarée **SIMULATED_PROVIDER** : aucune preuve de login OAuth vendor,
aucune modification des statuts, exigences d'identité ou compteurs des 39 cas.

**Amendement après clôture ingress — couture `e2e_forward`.** Le parent atteste
ingress PASS LIVE, ACK41542 et témoin v1/303, puis contrôle infra indépendant
PASS (25 policies, 24 peers inchangés, identités et 15 IDs runtime). Helper de
cette preuve : `fa7d3489`. L'adaptateur mesh ne crée **aucune probe Kubernetes** :
pas de primitive delete/cleanup Kubernetes à ajouter pour ce raccord.

Interface spécialisée :
`anchor.e2e_forward(service, namespace_uid=…, service_uid=…)`, contextmanager
rendant uniquement un port loopback. Enum fermée **iam→8443, hive→8443,
telegraph→8443, mailhog→8025**, namespace `aiforall-local-full` ; remote port et
nom de port attendu fixés/versionnés par service, jamais fournis par inputs.
Réutiliser `_EnvoyAdminForward` pour le lifecycle enfant et les mécanismes
existants prefix/environment/ancrage ; aucun shell, transport ou argv libre,
nouvelle route, policy, exposition publique ou modification de grammaire kubectl.

Avant spawn puis avant yield : vérifier namespace UID et Service UID du bail
d'origine, un seul backend Pod Ready non terminating, owner chain contrôlée
jusqu'au Deployment UID du bail, incarnation conteneur stable ; EndpointSlices
liées au Service UID, targetRef Pod UID, adresse et ports nommés résolus doivent
désigner exactement ce backend et le port fixe. Zéro backend, plusieurs,
ambiguïté, remplacement ou port incohérent ⇒ refus, pas de choix du premier.
Port-forward **au Pod attesté**, pas au Service sélectionnant implicitement un
autre Pod : écoute `127.0.0.1` uniquement, remote port fixe. Exiger marqueur de
binding exclusif de cet enfant, process handle possédé et mapping stable ;
un port ouvert seul ou un PID emprunté n'est pas une preuve d'ownership.
Revalider cible/incarnation/mapping avant yield et après usage ; changement ou
sortie prématurée ⇒ échec sans réattachement/adoption/retry silencieux.
Fermer puis attendre le seul enfant possédé dans tous les chemins, y compris
exception, timeout, échec de revalidation et sortie du contextmanager ; aucun
kill par port/nom. Fenêtre forward courte à borne fixe du helper (pas un input), deadline de **20 s par
requête** ; chaque tentative de polling MailHog ouvre puis ferme son tunnel.
Les clients conservent CA/SAN serveur et identité TLS fixe de chaque oracle,
sans `insecure` malgré la connexion loopback ; MailHog garde son transport8025
existant. Tests hermétiques : quatre nominaux, cible/port étrangers, stale UID,
backend ambigu, chaîne/incarnation/EndpointSlice erronée, marker/mapping étranger,
échec avant/après yield, timeout et fermeture attendue sur chaque chemin.

L'évolution de `local_full_target.py` est désormais **postérieure à la clôture
ingress**, pas compatible à rebours par réécriture d'une preuve : conserver
capture/ACK/témoin anciens immuables, sans pin ajouté ou modifié. Toute future
activation `configure_envoy_ingress` utilise une nouvelle capture du helper à
sa version alors courante si ses octets ont changé. Aucun live réalisé par
l'auteur ; l'implémentation, les tests et les reviews restent au parent.

Le parser kubectl de `LocalClusterTargetAnchor` **n'autorise pas `delete`**.
Ne pas le contourner par kubectl natif, subprocess libre ou transport alternatif.
Avant de créer un Pod/Job/Secret de probe, identifier une primitive de retrait
existante réellement compatible ; à défaut, seule extension admise : opération
spécialisée de retrait de probes, pas ajout de `delete` générique. Elle exige le
bail/namespace UID courant, namespace full exact, nonce du run, kind/name/UID
issus de l'ACK de création et du ledger, refus de l'objet préexistant/remplacé ;
la suppression transmet une **précondition UID atomique côté API**, pas seulement
un GET suivi d'un retrait par nom. Refuser namespace/selector/wildcard/collection,
force et cible arbitraire. Tester refus étranger, course UID, ACK perdu et reprise
sans adoption. Cleanup non disponible ⇒ aucune création dépendante, BLOCKED.
Un Secret éventuel est une fixture neuve strictement nécessaire, jamais une copie
d'identité existante ; Secret GET reste interdit. Cette spécialisation suit
l'ordre de gel du helper ci-dessus ; aucun namespace temporaire entier à retirer.

## 3. Inventaire d'oracles : ne pas réduire le dénominateur

**Mesh-authn : 39 cas HTTP, 12 positifs + 27 négatifs**, dérivés de
`ops/scripts/mesh-authn-kind-e2e-cases.sh:55–253` ; les sous-assertions de corps,
principal, persistance et absence d'effet sont obligatoires, pas des cas doublés.
- Positifs : signup, completion, login, JWKS, me/sub, me/Bearer+spoof utilise le
  vrai sub, refresh/rotation/persistance, notifications/user_id, S2S token200,
  S2S revoke200, organisation créée, membership/issuer : **12**.
- Négatifs : me sans JWT, me spoof sans JWT, bypass Telegraph, bypass Hive : **4** ;
  pour token ET revoke : gateway+gate+Bearer sans principal (**2**), mesh-client
  avec bearer/spoof/internal/all (**8**), quatre identités applicatives directes
  (**8**), gateway avec gate absent/erroné (**4**) ; token après revoke404 (**1**).
  Total **27** ; contrôler non-divulgation et absence de mutation sur refus.
- Le write FGA legacy est du setup, PAS un 40e oracle. Remplacer son bootstrap
  par API Hive/projection sentinel réelles avec attente bornée, sujets selon le
  canon0314/code vérifié ; jamais injecter un tuple pour rendre le test vert.
  Réutiliser la fixture provider explicitement bornée ci-dessus et les leafs
  hôte par identité, pas le seed interpolé/bundle legacy. BLOCKED seulement si
  une précondition vérifiée manque, pas par interdiction de principe ; jamais
  404/422 promu en token200 ni activation d'un vendor OAuth.

La matrice publique conserve ses **33 IDs** mais déclare sa variante de profil
avant exécution : base sans connecteurs ou oauth-proof effectivement attesté.
Les attentes spécifiques des routes OAuth viennent de cette variante versionnée
fermée, jamais du statut reçu ni d'overrides libres. Une matrice base exécutée
sur oauth-proof doit garder ses résultats/désaccords et être déclarée non adaptée,
pas réécrite rétroactivement en PASS. Ne retirer aucun cas ni élargir les statuts
pour masquer l'écart. Un start OAuth303 avec nonce n'est pas un login vendor
abouti ; un422 reste une indisponibilité, pas un succès OAuth.

**Apparatus : 6 obligations, 4 positives + 2 négatives**, pas « un test Rust vert » :
H1 enveloppe réellement signée/vérifiée, digest catalogue ≠ OCI ≠ pin CRI (**+**) ;
H2 admission VALID persistée/relecture et scheduling/image/labels/CR exacts (**+**) ;
H3 invoke sans consentement403 ET zéro hop (**−**), puis consentement réel,
invoke200/hop réel/hostname du Pod attendu (**+**) ; H4 canary vivant attesté,
contrôle HTTP404 avec corps attendu (**+**), puis GNU wget depuis le plugin,
connect-timeout borné identifié, endpoints/UID revalidés (**−**).
Réutiliser les formats/classifieurs `tests/fixtures/{evidence,canary}.rs`,
`canary::observe_canary` / `CanaryPort`, et les clients
réels adaptés à la cible ; les helpers Rust sont des oracles, pas une bibliothèque
Python prétendument importable. `apparatus_m6_e2e_chain.rs` utilise Manifesto stub,
MemoryPort, CA/session et Lazaret TestServer locaux : cela ne valide PAS le full.
Artefact signé/invoke/enrôlement/consentement/compteur de hops réel indisponible,
namespace embarqué incompatible ou nom digest déjà étranger ⇒ BLOCKED identifié,
pas nouvelle pile, CA, stub, flag « namespace confirmé » ou pré-nettoyage.

## 4. Validation et rayon du changement

Rayon **cross-file / contrat opérationnel** : driver/cas sous `ops/tests` et
`ops/scripts`, branche témoin `EnvoyIngressChange.finish`, éventuelle primitive
de cleanup spécialisée et couture `e2e_forward` dans `local_full_target.py`
après clôture ingress, et
tests ciblés ; pas de
Rust applicatif, SDK, migration, image ou manifeste runtime. Écart produit
structurel indispensable ⇒ remontée au parent, hors raccord ; ne pas l'implémenter.
Tests hermétiques à compléter dans `test_local_full_acceptance_source.py`,
`test_local_full_envoy_ingress.py` et, si le helper évolue même sans changement de grammaire,
`test_local_full_target.py` : témoins v1/v2 non interchangeables, profil/UID/hash
stale, statut/corps/transport invalide, TLSdelta, ACK perdu/concurrence/rollback,
inputs étrangers, nettoyage exact, prérequis absents et comptes non mensongers.
Conserver les gates et négatifs existants ; aucune commande live/build/test par
l'auteur de ce contrat. Le parent vérifie les octets intégrés avant exécution.

Rapport durable : `docs/missions/20261102-local-full-validation.md`. Séparer
smoke21, routes33 et variante déclarée, témoin ingress choisi (v1 oauth-proof ou
base-full/3 oracles), mesh39 et Apparatus6, ainsi que setup/cleanup/requêtes
auxiliaires. Par cas : attendu, tenté, complété PASS/FAIL, BLOCKED/NOT RUN et
preuve LIVE ou SIMULÉ ; **attendu = PASS + FAIL + BLOCKED/NOT RUN**, sans double
compte. Un HTTP tenté mais sans oracle complété n'est pas un PASS ni un contrôle
live complété. Code0 seulement si toutes les obligations ET cleanup sont acquis ;
si une suite est partiellement exécutable, conserver son inventaire complet et
sa raison source. GO routine acquis : pas de redemande du même arbitrage.
