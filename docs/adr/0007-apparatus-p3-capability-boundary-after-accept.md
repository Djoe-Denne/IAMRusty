# ADR-0007 : La frontière Apparatus P3 de capacités et de données est le bounded context Lazaret, distinct de Manifesto, sans lever 0006 G ni E

- Statut : Accepted
- Réalité : Partial
- Date : 2026-09-13
- Décideurs : Architecture AIForAll — Accept explicite utilisateur (« accepté », 2026-09-13) ; checklist 1–14 figée pendant Proposed puis ratifiée
- Jalon concerné : P3
- SuperSède : aucune
- SuperSédée par : —

`Accepted` ratifie la cible ci-dessous. `Réalité : Partial` : T1 (absence) + T2 (gate Lazaret, grants Manifesto, scaffold health/ready) + T3 (identité workload hybride : CSR → certificat client, puis jeton de session dédié `iss`/`aud`=`lazaret` ; jeton ≠ autorisation) + T4 (grants/consents live via GET domaine Manifesto `/api/projects/{project_id}/bindings/{component_id}` + intersection Lazaret) + T5 (consentement write Manifesto `PUT /api/projects/{project_id}/bindings/{component_id}/consents`, Admin sur `project_id` ; upsert `apparatus_capability_consents` + bump `grant_revision` même txn ; révocation close-at-commit : membre DELETE ferme `principal.active` en DB, FGA allow laissé en place ; Lazaret refuse révision d’identité périmée et principal inactif) + T6 (KV plateforme Lazaret `apparatus_kv_entries` ; `KvStore::kv_put(..., expected_cas: Option<i64>) -> Result<i64>` ; adaptateurs Postgres **et** Redis ; SecretResolver + adaptateur Vault HTTP KV v2 — preuve protocole IT = wiremock (T6) **et** produit compose/testcontainer (T12) ; l’IT T6 reste wiremock ; référence opaque `secret:{path}#{field}` ; quota `MAX_KV_ENTRIES_PER_BINDING=256` + limites clé/valeur P0) + T7 (Lazaret `POST /invoke`, DTO P0 / `INVOKE_PATH` ; nest standalone `/lazaret/invoke` ; connecteurs nommés depuis la config Lazaret ; URL brute / `fetchInternal` refusés ; invoke lié à `desired_generation` + `grant_revision` courants ; adversaire deux projets / deux bindings ; projet public n’ouvre pas invoke ; JWT IAM ≠ session Lazaret) + T9 (`Application::prefixed_router` → `create_prefixed_router` ; IT `POST /lazaret/invoke` 2xx ; `POST /invoke` sur routeur préfixé → 404 ; `INVOKE_PATH` reste `/invoke` ; `Application::router()` reste non préfixé) + T10 (enrollment persisté dans Postgres Lazaret `apparatus_enrollments`, même DB que KV ; `revoke_binding` sur Manifesto `component_removed` ; InMemory réservé aux tests unitaires) + T11b (session mTLS live HTTPS : authentification client optionnelle rustycog, `PeerClientCertificate` → `VerifiedClientCertificate`, même instance CA pour enroll et `tls_client_ca_path`) + T12 (OpenBao produit pin `openbao/openbao:2.6.2` dans docker-compose + testcontainer service-local ; KV v2 `secret/` ; IT `apparatus_p3_t12_openbao`) + T13 (CA plateforme persistée generate-if-absent ; leaf serveur signée par cette CA ; compose Lazaret HTTPS `tls_port` 8080 + volume `./Lazaret/certs:/app/certs` + HEALTHCHECK `curl -fk https://localhost:8080/lazaret/health` ; IT `apparatus_p3_t13_ca_persist`). T1 n’est pas une preuve de mécanisme P3 à elle seule. **Pas** Implemented — holes (dette, pas des bloqueurs T5–T13) : `APP-05` ouvert ; 0006 G/E encore en vigueur sur Manifesto ; pas d’isolation K8s ; pas de second protocole ; mTLS Hive-IAM-Telegraph encore absent (pas un rustls mTLS production complet). Hole persistance clé CA **fermé** (T13). Hole compose TLS **Lazaret** **fermé** (T13). Hole `kv_purge` **fermé** : branché sur Manifesto `component_removed` / file dédiée `lazaret-kv-events` (T8). Hole enrollment T3 in-memory **fermé** (T10). Hole mTLS `/session` **fermé** (T11b : HTTPS live + authentification client optionnelle rustycog). Hole OpenBao **produit** hors compose **fermé** (T12).

## Contexte

ADR-0004 **Accepted** fixe la cible : toute I/O via une gateway de capacités ; KV plateforme namespacé par binding ; pas de bearer / JWT / HMAC IAM dans le plugin. Sa **Réalité** est **Partial** (taxonomie P0, port `KvStore`, harness). La gateway **réseau** n’est pas livrée. ADR-0004 **ne fige pas** mTLS, rotation, TTL, autorité émettrice, ni le produit secrets.

ADR-0006 **Accepted**, Réalité **Implemented** : contrôleur in-process, ticker `/ready`, CAS desired, lease/fencing, cleanup. **G encore en vigueur dans le code** : pas d’`invoke` sur `ApparatusRuntime` (`apparatus-contracts/src/ports.rs` : `bind` / `configure` / `unbind` / `observe` / `teardown` seulement) ; **zéro** identifiant `gateway` sous `Manifesto/*/src`. **E encore en vigueur** : pas de nouvelles routes, pas de 202, `/components` gelé (5 registrations dans `Manifesto/http/src/lib.rs`). `desired_generation` **≠** `grant_revision` (0006 J) ; `grant_revision` n’est pas une colonne P2.

Contradiction **ADR-0004 vs gate P2** : `Manifesto/tests/apparatus_p2_t7_gate.rs` (`t7_p3_tokens_forbidden_even_on_p2_allowlist`) interdit encore `gateway` (et `kubernetes`/`k8s`, `wasm`/`wasi`/`wasmtime`, `iframe`, `messagechannel`, `apparatus_host`, `ui_host`) partout dans `Manifesto/*/src`. Le token `invoke` **n’est pas** dans cette liste. Les DTO P0 exposent déjà `INVOKE_PATH = "/invoke"` (`apparatus-contracts/src/protocol.rs`) ; le harness in-process n’est pas l’`invoke` serveur lié au binding. `grep` `invoke` dans `Manifesto/**/*.rs` : absence. Harness P0 / `apparatus-reference-kv` = doubles de test, pas le KV plateforme.

Le wiki (`apparatus-implementation-plan` P3, `apparatus-capabilities-and-isolation`, `apparatus-platform`) est **conception** `^[inferred]`, **pas** un contrat. Interdit d’y copier mTLS, SQL, URLs ou un microservice comme si figés. Note factuelle 2026-09-13 (utilisateur) : le service n’a **jamais** été déployé ; **pas** de tenants de production. Pointeur unique : checklist **9**. Notes de **Réalité** les plus légères sur ADR-0001 et ADR-0006 **M** seulement — **sans** réécrire les listes Accepted, **sans** SuperSède.

Style SQL déjà livré (preuve de forme, pas une migration P3) : `Manifesto/migration/src/m20260912_000012_create_apparatus_bindings_table.rs` (`apparatus_bindings`, `id` BIGSERIAL interne, `component_id` UUID UNIQUE, FK `fk_apparatus_bindings_component` → `project_components(id)` CASCADE) ; `m20260912_000013_apparatus_p2_runtime.rs` (colonnes additives + `apparatus_cleanup_jobs`, `chk_apparatus_bindings_*`, `down` réversible sans DROP de `apparatus_bindings`). Port `KvStore` aujourd’hui : `kv_get` / `kv_put` / `kv_delete` / `kv_purge` (`apparatus-contracts/src/ports.rs`). Nommage services : dossier Pascal + package `*-service` (`Hive` / `hive-service`, `Manifesto` / `manifesto-service`, `Telegraph` / `telegraph-service`) ; libs apparatus en kebab (`apparatus-contracts`). **Lazaret** / `lazaret-service` : **nommage Accepted** (point 1). Le scaffold T2 est autorisé ; il n’est pas cette rédaction.

## Décision

P3 est le jalon qui **doit** réaliser la frontière serveur de capacités et de données visée par ADR-0004 : autorisation à l’appel, identité de **workload** (instance, binding, release, génération, `grant_revision`, audience gateway — jamais l’utilisateur, jamais le HMAC IAM), grants interactifs et de fond, consentement / révocation qui ferme l’accès au **commit** DB (projection FGA insuffisante), KV plateforme, secrets par **référence opaque**, réseau sortant via proxy/gateway et connecteurs **nommés**, `invoke` serveur lié au **binding courant**.

**Accept 2026-09-13** : les points **1 à 14** (figés pendant Proposed) sont **ratifiés**. Le **nommage** du BC (point 1) est **Accepted** : dossier `Lazaret`, package `lazaret-service`. Ce n’est **pas** le scaffold par lui-même ; le scaffold T2 est **autorisé**. Layout Redis comme schéma gelé, TTL numériques, produit / bibliothèque CA : **non figés**.

### Checklist 1 — décidée le 2026-09-13, ratifiée à l’Accept

Question d’origine : gateway P3, in-process Manifesto vs nouveau service RustyCog ?

L’utilisateur **écarte** :

- **(A)** détourner le bounded context / domaine in-process de Manifesto pour y héberger la gateway ;
- **(B)** un nouveau service qui partage le Postgres de Manifesto, ou qui consomme les événements domaine Manifesto pour tenir une réplique comme source de vérité d’autorisation (trop lourd).

**Troisième voie** (nommée « couche applicative », identifiée ensuite comme **Lazaret** — la gateway de capacités) : **Lazaret** est un **bounded context applicatif / plateforme distinct** (même famille qu’IAM, Hive, Manifesto, Telegraph — chacun possède sa base, dialogue par APIs). Il **a le droit d’appeler les APIs publiques de Manifesto à l’instant de l’appel** pour des contrôles live (l’état transactionnel reste chez Manifesto, propriétaire). Si l’intersection ADR-0004 tient, Lazaret **appelle l’Apparatus cible**. Pas de base partagée. Pas de réplique event-sourcée des grants comme source de vérité.

Analogie déjà dans le dépôt (fait, pas une invention) : Hive configure `[iam_service] base_url = "http://iam-service:8080"` et un client HTTP ; sa base est `hive_dev`, pas celle d’IAM. Manifesto parle au catalogue via le port `ComponentServicePort` (client HTTP) ; il ne partage pas la base de ce service. Hive/Manifesto → sentinel-sync → OpenFGA est une **projection asynchrone**. ADR-0004 : une projection OpenFGA ou un cache périmé **ne suffit pas** à l’autorisation de capacités ; **ne pas** copier ce motif de réplique événementielle pour les grants P3. **Pas** de crate `apparatus-gateway-events`, **pas** de crate `lazaret-events`, ni de traducteur sentinel-sync pour les grants.

Composition : comme IAM, Hive, Manifesto et Telegraph, **Lazaret** peut tourner **standalone et dans le monolithe** sans fusionner les domaines.

**Nommage Accepted** (figé le 2026-09-13, ratifié à l’Accept) : le bounded context s’appelle **Lazaret** (même veine que Hive / Manifesto / Telegraph / Apparatus : station de quarantaine victorienne — un Apparatus non fiable n’entre dans la plateforme que par ce BC). Dossier `Lazaret`, package `lazaret-service`. **Pas** `ApparatusGateway`, **pas** `apparatus-gateway` / `apparatus-gateway-service`. **Pas** de crate `lazaret-events`. Manifesto n’a **pas conscience** de la gateway : **zéro** identifiant `gateway` sous `Manifesto/*/src` (0006 G / point 8). La skill `aiforall-new-service` s’applique au scaffold **T2** (Accept obtenu).

### Checklist 2 — décidée le 2026-09-13, ratifiée à l’Accept

Question d’origine : transport d’identité workload — mTLS, rotation, TTL, autorité émettrice ? (ADR-0004 a figé le contrat d’identité, pas le transport concret.)

**Transport = hybride** (décidé par l’utilisateur) :

- certificat client **mTLS** (autorité distincte du HMAC IAM) ;
- **puis** un jeton de session gateway **court**, signé avec une **clé dédiée** (pas IAM).

Les claims sont déjà figés par ADR-0004 : instance, binding, release, génération, `grant_revision`, audience, expiration. Un jeton **cryptographiquement valide ne suffit pas** : la gateway fait toujours les contrôles live Manifesto (point 1). Une IP privée / le même VPS **n’est pas** une identité. Le protocole doit encore fonctionner plus tard pour un Apparatus **externe**.

Le texte wiki `^[inferred]` (mTLS + jeton de session) est une **illustration**, pas un contrat recopié.

**Autorité émettrice (V1, « pour commencer ») = CA interne plateforme** — décidé par l’utilisateur le 2026-09-13 comme le plus simple ; ce n’est **pas** un PKI de fournisseur cloud. Direction **préférée** d’émission : **auto-émission plateforme** au bind ou à l’enrollment. **Reste dette / Non décidé** (point 2) : produit ou bibliothèque CA (aucun nom ici) ; TTL et rotation numériques exacts ; CSR depuis le workload vs keypair injecté.

### Checklist 3 — décidée le 2026-09-13, ratifiée à l’Accept

Question d’origine : quelles tables / colonnes (grants, consentement, identité, KV) ?

Identité = `project_components.id`. **Pas** de second UUID public. Style existant : table `apparatus_bindings`, ids internes BIGSERIAL, `component_id` UUID, `down` réversible. Split par bounded context. **Aucune URL inventée.** Accept **autorise** la migration additive T2 (noms ci-dessous). Réalité Unimplemented jusqu’aux preuves.

#### Manifesto DB (source de vérité grants / consentement ; interrogée via APIs domaine)

Colonne additive sur `apparatus_bindings` existante :

- `grant_revision BIGINT NOT NULL DEFAULT 0`
- CHECK `>= 0` nommé `chk_apparatus_bindings_grant_revision_non_negative`
- Distinct de `desired_generation`

Table enfant **`apparatus_capability_consents`** (nom figé) :

- `id BIGSERIAL PRIMARY KEY` (surrogate interne)
- `component_id UUID NOT NULL`
- FK `fk_apparatus_capability_consents_component` → `project_components(id) ON DELETE CASCADE` (identité de binding = cet UUID ; 1:1 avec les bindings)
- `capability TEXT NOT NULL`
- `status TEXT NOT NULL` CHECK IN (`consented`, `revoked`) — `chk_apparatus_capability_consents_status`
- `grant_revision BIGINT NOT NULL` CHECK `>= 0` — `chk_apparatus_capability_consents_grant_revision_non_negative` (lien de révision)
- UNIQUE (`component_id`, `capability`) — `uq_apparatus_capability_consents_component_capability`
- `down` réversible : DROP de la table + DROP de la colonne `grant_revision` ; **ne pas** DROP `apparatus_bindings`
- Pas de type FGA. **Pas** de lignes KV dans la DB Manifesto.

#### Postgres de Lazaret (base propre du BC Lazaret)

Table **`apparatus_kv_entries`** :

- `id BIGSERIAL PRIMARY KEY`
- `binding_id UUID NOT NULL` — copie de l’id composant, **pas** de FK inter-DB vers Manifesto
- `key TEXT NOT NULL`
- `value BYTEA NOT NULL`
- `cas_version BIGINT NOT NULL DEFAULT 0` CHECK `>= 0` — `chk_apparatus_kv_entries_cas_version_non_negative`
- UNIQUE (`binding_id`, `key`) — `uq_apparatus_kv_entries_binding_key`
- `down` réversible : DROP de la table
- Pas de schéma relationnel possédé par le plugin

Table **`apparatus_enrollments`** (même Postgres Lazaret que KV ; pas une table Manifesto) :

- `fingerprint TEXT PRIMARY KEY` — SHA-256 hex du DER certificat (même que `lazaret_domain::fingerprint_sha256`)
- `binding_id UUID NOT NULL UNIQUE` — first-wins ; copie de `project_components.id`, **pas** de FK inter-DB vers Manifesto
- `instance UUID NOT NULL`
- `release TEXT NOT NULL`
- `generation BIGINT NOT NULL`
- `grant_revision BIGINT NOT NULL`
- `down` réversible : DROP de la table
- Pas de colonnes `VALID` / `VERIFIED`. Pas de réplique de grants.

### Checklist 4 — décidée le 2026-09-13, ratifiée à l’Accept

Question d’origine : `invoke` serveur — méthode `ApparatusRuntime`, nouveau port, ou handler HTTP DTO P0 seulement ?

**Décision utilisateur (ratification de la lecture)** : l’`invoke` serveur P3 est un **handler HTTP** de **Lazaret** (point 1). Il **réutilise** les DTO P0 et `INVOKE_PATH` (`/invoke`). Ce n’est **pas** une nouvelle méthode sur `ApparatusRuntime` de Manifesto. Le port P2 reste `bind` / `configure` / `unbind` / `observe` / `teardown`.

Cette décision **ne lève pas** 0006 G. Aucune URL n’est inventée.

### Checklist 5 — décidée le 2026-09-13, ratifiée à l’Accept

Question d’origine : lève-t-on 0006 E (nouvelles routes) en gelant les 5 `/components` ?

**Décision utilisateur (ratification de la lecture)** : on **ne lève pas** 0006 E sur les 5 routes `/components` (gelées : pas de 202, pas de nouvelle registration Manifesto sur ce gel). Le HTTP **nouveau** (dont `invoke`) appartient à **Lazaret**. Manifesto n’ajoute une route que si un contrat de lecture **domaine** (binding, consentement, grants) manque — **résidu non figé**. Tout HTTP Manifesto nouveau est du **langage domaine**, utilisable par **tout appelant privilégié** — jamais « pour la gateway » / jamais « pour Lazaret ». **Pas d’API Manifesto spécifique à Lazaret.** Aucune URL n’est inventée. Cette décision **ne lève pas** 0006 E. L’Accept **ne lève pas** E.

### Checklist 6 — décidée le 2026-09-13, ratifiée à l’Accept

Question d’origine : produit secrets — quoi, où, référence opaque seulement ?

Même forme que le KV : un **port** (résoudre une référence opaque et **injecter** uniquement dans l’opération accordée ; le plugin **ne lit jamais** le clair) + des **adaptateurs**. **Ne pas** inventer ici un nom de trait / méthode Rust.

Il n’existe **pas** de « protocole Vault » IETF. Le de facto = API HTTP HashiCorp Vault (KV). **OpenBao** (Linux Foundation / OpenSSF) est le fork OSI, API HTTP compatible Vault (~Vault 1.14). C’est la surface portable « plusieurs produits la parlent ».

« Active Manager » = **Secrets Manager** cloud (classe AWS Secrets Manager / GCP Secret Manager) — un **adaptateur fournisseur**, **pas** le port.

Direction V1 (alignée CA interne, VPS, cloud non requis) : **adaptateur OpenBao / Vault HTTP API** comme premier backend secrets. Cloud Secrets Manager = **adaptateur additionnel plus tard**, pas exclusif V1. **Ne pas** nommer un cluster Vault commercial comme obligatoire. **Ne pas** mettre le clair des secrets dans le KV plugin.

### Checklist 7 — décidée le 2026-09-13, ratifiée à l’Accept

Question d’origine : KV plateforme persistant : quel adaptateur ? (harness P0 / apparatus-reference-kv ≠ cette décision)

**Décision utilisateur** : l’état durable plugin V1 est un **magasin clé-valeur** (pas Mongo / base document, pas un schéma relationnel possédé par le plugin). Ce n’est **pas** un magasin in-memory, **pas** `apparatus-reference-kv`, **pas** le harness de test comme production.

**Hexagone** : **un port**, **deux adaptateurs de production : Redis et PostgreSQL**. Pas le harness P0, pas `apparatus-reference-kv` comme prod, **pas le Postgres Manifesto** (ne pas y mettre de lignes KV), **pas** un défaut unique Redis XOR Postgres.

CAS + quota = ADR-0004 sur ces adaptateurs. À l’implémentation (**pas T2**) : étendre `KvStore` dans `apparatus-contracts` (`kv_get` / `kv_put` / `kv_delete` / `kv_purge` aujourd’hui). **Ne pas** inventer un nom de méthode au-delà de get/put/delete/purge + l’**exigence** CAS/quota.

Namespace binding : `BindingId` = `project_components.id` ; l’appelant ne fournit que **`key`** ; l’adaptateur / la gateway impose le préfixe. **Ne pas** figer un format pointé `projectId.pluginId.key`.

Noms SQL de l’adaptateur Postgres Lazaret : **figés au point 3** (`apparatus_kv_entries`). Layout des clés Redis comme schéma gelé : **non figé**. TTL KV : **non figé**. Crate / dossier / binaire : **Accepted** au point 1 (`Lazaret` / `lazaret-service`).

### Checklist 8 — décidée le 2026-09-13, ratifiée à l’Accept

Question d’origine : tokens de gate + chemins allowlist : `gateway` déjà gated ; `invoke` ne l’est pas. Que gater ? Où ?

**Décision utilisateur** : Manifesto **n’a pas conscience** de la gateway. Sens unique : **Lazaret** appelle les APIs publiques / domaine de Manifesto ; Manifesto **ne sait pas** qu’une « gateway » existe (pas de types, pas de token `gateway` sous `Manifesto/*/src`, pas de routes spécifiques gateway, pas de client Manifesto→Lazaret).

Le **gate P2 reste tel quel pour Manifesto** : le token `gateway` reste **interdit partout** sous `Manifesto/*/src` (**aucun** chemin d’allowlist Manifesto pour `gateway`). `invoke` n’est **toujours pas** un sujet Manifesto (point 4 : le HTTP vit sur **Lazaret**). Un gate P3 pour **Lazaret** (`lazaret-service`) est un **scan séparé** (T2, Accept obtenu), pas un trou dans Manifesto. Le nom **Lazaret** **n’est pas** une allowlist Manifesto. Les tokens P4+ restent interdits.

### Checklist 9 — décidée le 2026-09-13, ratifiée à l’Accept

Question d’origine : consentement capacités/révocation vs legacy→managed (0001) — même ADR P3 ou hors P3 ?

**Décision utilisateur** : le **consentement / révocation de capacités** reste un travail **P3** (cette ADR). La migration / backfill **legacy→managed** est **mise de côté** : le service n’a **jamais** été déployé ; il n’y a **pas** de tenants de production. Hors jalon P3 comme travail actif. Le modèle Accepted `source` ∈ `legacy|managed` (0001, 0006) n’est **pas** réécrit.

### Checklist 10 — décidée le 2026-09-13, ratifiée à l’Accept

Question d’origine : confirmer zéro nouveau type FGA ou escalade ?

**Décision utilisateur** : **zéro nouveau type FGA** (pas d’escalade). OpenFGA reste la projection user / projet / instance. Grants et consentement = domaine Manifesto + contrôles live gateway, **pas** un nouveau type FGA. Pas de crate `apparatus-gateway-events`, **pas** de crate `lazaret-events`, ni de traducteur sentinel-sync **sauf** si le modèle FGA change — il **ne doit pas** changer.

### Checklist 11 — décidée le 2026-09-13, ratifiée à l’Accept

Question d’origine : connecteurs réseau nommés — qui les admet, comment, sans domaine arbitraire V1 ?

**Décision utilisateur** : connecteurs **nommés** ; **admis par l’opérateur / la plateforme** (la gateway **applique**) ; **pas** de domaine V1 arbitraire ; le plugin **ne choisit pas** d’URLs brutes.

### Checklist 12 — décidée le 2026-09-13, ratifiée à l’Accept

Question d’origine : SuperSède / levée partielle **0006 G** : libellé exact (à l’Accept seulement).

**Décision utilisateur (même intention que le point 8)** : l’Accept **ne lève pas** 0006 G pour Manifesto. **G demeure** : pas d’`invoke` sur `ApparatusRuntime` de Manifesto ; **zéro** identifiant `gateway` sous `Manifesto/*/src`. Le mot `gateway` **n’entre pas** dans `Manifesto/*/src`. **Lazaret** est **hors** de ce glob, donc G **n’a pas besoin** d’une SuperSède pour que le BC existe. Le nom **Lazaret** / `lazaret-service` (nommage Accepted) **n’est pas** une allowlist Manifesto.

### Checklist 13 — décidée le 2026-09-13, ratifiée à l’Accept

Question d’origine : `APP-05` reste-t-il ouvert ?

**Décision utilisateur** : `APP-05` **reste ouvert** ; **ne pas** préempter. Un projet public **n’ouvre pas** invoke / KV / secrets. V1 reste **privé** pour ces surfaces. **Ne pas** clôturer `APP-05`.

### Checklist 14 — décidée le 2026-09-13, ratifiée à l’Accept

Question d’origine : deux writers / lease P2 — l’invoke s’appuie-t-il sur le binding courant sans second fencing ?

**Décision utilisateur** : `invoke` utilise le **binding courant** (`desired_generation` + `grant_revision`) ; le fencing P2 (génération + lease) est **inchangé** ; **pas** de second fencing / epoch d’invoke.

### Invariants déjà Accepted (rappel, pas une ré-acceptation)

Tant qu’une ADR **Accepted** ne les lève : pas de second UUID public ; identité = `project_components.id` ; tuples `component:{id}` ; 5 routes `/components` ; `ComponentAdded` / `ComponentRemoved` inchangés ; 1:1 ; `source` ∈ `legacy|managed` ; pas de nouveau type FGA sans escalade ; `component_type` non réécrit ; pas de `VALID` / `VERIFIED` persistants ; harness P0 ≠ runtime de production ; ticker P2 in-process conservé ; `desired_generation` ≠ `grant_revision` ; pas de `trusted_skip_gateway` (0005) ; pas de bearer / JWT / HMAC IAM dans le plugin (0004). OpenFGA reste la projection des droits utilisateur / projet / instance ; les grants de capacités sont un contrat distinct (0004). Pas d’`enforce_world_read_or_principal` comme autorisation d’invoke. Projet public n’ouvre pas invoke / KV / secrets (`APP-05` ouvert).

### 0006 G — encore en vigueur (point 12 : pas de levée Manifesto)

**G reste en vigueur** (point 12, ratifié à l’Accept 2026-09-13) : pas d’`invoke` sur `ApparatusRuntime` de Manifesto ; **zéro** identifiant `gateway` sous `Manifesto/*/src`. L’Accept **ne lève pas** G pour Manifesto. **Lazaret** est **hors** du glob `Manifesto/*/src`, donc G **n’a pas besoin** d’une SuperSède pour que ce BC existe. Le nom **Lazaret** / `lazaret-service` **n’est pas** une allowlist Manifesto. Les décisions des points **1–14** **ne lèvent pas** G. Le point 4 enregistre la **cible** (`invoke` HTTP sur **Lazaret**, pas une méthode `ApparatusRuntime`) ; le point 7 enregistre la **cible** KV (clé-valeur, un port, deux adaptateurs persistants) ; le point 8 enregistre que le gate P2 Manifesto **reste tel quel** (aucun chemin d’allowlist `gateway`). Ce n’est pas une levée.

### 0006 E — encore en vigueur

**E reste en vigueur** : l’Accept 0007 **ne lève pas** E. Pas de nouvelles routes Manifesto, pas de 202, `/components` gelé. **Aucune URL n’est inventée ici.** Les décisions des points **1–14** **ne lèvent pas** E. Le point 3 fige des **noms SQL** (pas des routes). Le point 5 enregistre que le HTTP nouveau (dont `invoke`) est sur **Lazaret**, que les 5 `/components` restent gelées, et que tout HTTP Manifesto éventuel est du **langage domaine** (binding, consentement, grants) pour tout appelant privilégié — **pas** d’API spécifique à Lazaret.

T1 de **caractérisation** (absence `invoke` runtime / `gateway` Manifesto) **n’est pas** une preuve de mécanisme P3. T2–T10 (TDD) sont **autorisés** par cet Accept. Réalité **Partial** (T1–T10 + T11b + T12 + T13 livrés) ; **pas** Implemented (holes : `APP-05` ouvert, 0006 G/E Manifesto, pas K8s, pas de second protocole, mTLS Hive-IAM-Telegraph). Hole OpenBao produit hors compose **fermé** (T12). Hole persistance clé CA **fermé** (T13). Hole compose TLS Lazaret **fermé** (T13). `kv_purge` est branché sur `component_removed` / file dédiée. Enrollment T3 in-memory **fermé** (T10). Hole mTLS `/session` **fermé** (T11b).

Si une contradiction exigeait un second protocole, `trusted_skip_gateway`, un bearer IAM, K8s comme isolation P3, un nouveau type FGA, ou une levée G/E : **escalade humaine**, ne pas inventer.

## Conséquences

- Accept 2026-09-13 : T2–T7 **autorisés** (TDD). T8 (P3-close) livré. Réalité **Partial**. T1 (absence) n’est pas une preuve de mécanisme P3.
- T3 livré (Lazaret) : identité hybride ; **CSR-from-workload** par défaut (la clé privée ne quitte pas le workload) ; CA logicielle `platform-internal-ca` ; jeton de session dédié EdDSA/Ed25519 (`iss`=`lazaret`, `aud`=`lazaret`, claims instance/binding/release/generation/`grant_revision`/`exp`) ; **jeton ≠ autorisation** (`authorization_from_session` → `LiveManifestoCheckRequired`). Défauts d’implémentation (non figés par Accept) : session 15 min, certificat 24 h, produit CA `platform-internal-ca`.
- T4 livré : consult live Manifesto `GET /api/projects/{project_id}/bindings/{component_id}` (langage domaine, appelant privilégié, pas une API « pour la gateway ») ; intersection Lazaret à l’appel (interactive et fond). **Pas** de levée 0006 G/E.
- T5 livré : Manifesto `PUT /api/projects/{project_id}/bindings/{component_id}/consents` (Admin sur `project_id`) ; upsert `apparatus_capability_consents` + bump `grant_revision` même txn ; close-at-commit (DELETE membre ferme `principal.active`, FGA allow intact) ; Lazaret refuse révision périmée / principal inactif. **Pas** de type FGA apparatus.
- T6 livré : table Lazaret `apparatus_kv_entries` ; CAS `kv_put` ; adaptateurs Postgres **et** Redis ; SecretResolver + Vault HTTP KV v2 (preuve protocole IT = wiremock ; T6 IT **reste** wiremock). Preuve produit OpenBao = T12 (compose + testcontainer). **Pas** de KV dans Manifesto.
- T7 livré : Lazaret `POST /invoke` (DTO P0 / `INVOKE_PATH`) ; connecteurs nommés ; URL brute / `fetchInternal` refusés ; binding courant `desired_generation` + `grant_revision` ; adversaire deux projets / deux bindings ; projet public n’ouvre pas invoke ; JWT IAM ≠ session Lazaret. **Pas** de levée 0006 G/E ; `ApparatusRuntime` inchangé.
- T8 livré (P3-close) : Lazaret consomme `component_removed` sur `lazaret-kv-events` / `test-lazaret-kv-events` ; `purge_binding_namespace` ; voisin intact ; second handle no-op. **Pas** de crate `lazaret-events`, **pas** de réplique grants.
- T9 livré : `Application::prefixed_router` → `create_prefixed_router` ; IT `POST /lazaret/invoke` 2xx ; `POST /invoke` sur routeur préfixé → 404 ; `INVOKE_PATH` reste `/invoke` ; `Application::router()` reste non préfixé. **Pas** Implemented.
- T10 livré : enrollment persisté (`apparatus_enrollments`, même DB que KV) ; `revoke_binding` sur `component_removed` (fail-closed avec le purge KV) ; voisin encore sessionnable ; second enroll autre empreinte → `BindingAlreadyEnrolled` / HTTP 409. Preuve : `Lazaret/tests/apparatus_p3_t10_enrollment_persist.rs`. InMemory réservé aux tests unitaires. **Pas** Implemented.
- T11b livré : trou mTLS `/session` **fermé** (HTTPS live via `serve_router` + authentification client optionnelle rustycog ; mapping `PeerClientCertificate` → `VerifiedClientCertificate` sans écraser T3/T10 `extensions_mut`). Preuve : `Lazaret/tests/apparatus_p3_t11b_session_mtls.rs`. **Pas** Implemented.
- T12 livré : trou OpenBao **produit** hors compose **fermé** (`openbao/openbao:2.6.2` pin compose + testcontainer service-local `lazaret_test-openbao` ; KV v2 `secret/` ; `-dev` écoute `0.0.0.0:8200`). T6 IT reste wiremock. Preuve : `Lazaret/tests/apparatus_p3_t12_openbao.rs`. **Pas** Implemented.
- T13 livré : trou persistance clé CA **fermé** ; trou compose TLS **Lazaret** **fermé** (HTTPS `tls_port` 8080, volume `./Lazaret/certs:/app/certs`, HEALTHCHECK `curl -fk https://localhost:8080/lazaret/health` ; generate-if-absent au boot dans `Application::new`, sans openssl dans l’entrypoint). Défaut d’implémentation CA TTL `24 * 365` heures (8760) — **non figé** par Accept. Preuve : `Lazaret/tests/apparatus_p3_t13_ca_persist.rs`. **Pas** Implemented.
- Holes (bloquent Implemented, pas T5–T13) : `APP-05` ouvert ; 0006 G/E Manifesto ; pas K8s ; pas de second protocole ; mTLS Hive-IAM-Telegraph (pas un rustls mTLS production complet).
- Scaffold Lazaret autorisé via `aiforall-new-service` (`Lazaret` / `lazaret-service` **nommage Accepted**) **sans** nouveau type OpenFGA, **sans** `lazaret-events`, **sans** translator sentinel-sync (checklist 10).
- Manifesto : toujours zéro `gateway` sous `Manifesto/*/src` ; gate P2 t7 **reste** ; T2 = gate **Lazaret** (tokens P4+ encore interdits : `kubernetes`/`k8s`, `wasm`/`wasi`/`wasmtime`, `iframe`, `messagechannel`, `apparatus_host`, `ui_host`). **Ne pas** retargeter `Manifesto/tests/apparatus_p2_t7_gate.rs` pour autoriser `gateway` dans Manifesto.
- Points **1–14** ratifiés : **ne lèvent pas** G/E.
- Point 1 : **Lazaret** (`Lazaret` / `lazaret-service`) = BC distinct ; APIs Manifesto live ; pas de DB partagée ; pas de réplique événements comme vérité authz.
- Point 2 : transport hybride (mTLS puis jeton dédié) ; CA interne plateforme V1 (pas PKI cloud) ; produit CA / TTL numériques non figés.
- Point 3 : noms SQL additifs **figés** (Manifesto `grant_revision` + `apparatus_capability_consents` ; Lazaret `apparatus_kv_entries` + `apparatus_enrollments`) ; identité = `project_components.id` ; pas de FK inter-DB ; pas de type FGA ; pas de KV dans la DB Manifesto.
- Point 4 : `invoke` HTTP sur **Lazaret**, DTO P0 / `INVOKE_PATH` ; pas de méthode `ApparatusRuntime`.
- Point 5 : **pas** de levée de 0006 E ; HTTP nouveau = **Lazaret** ; résidu = HTTP **domaine** Manifesto seulement si le contrat grants/consentement manque — **pas** d’API Manifesto spécifique à Lazaret ; aucune URL.
- Point 6 : secrets = **port** + adaptateurs ; V1 = OpenBao / Vault HTTP API ; Secrets Manager cloud = adaptateur plus tard ; pas de clair dans le KV plugin.
- Point 7 : KV V1 = **un port**, **deux adaptateurs prod Redis et PostgreSQL** ; pas Manifesto Postgres ; CAS/quota = exigence 0004 sans nom de méthode inventé ; SQL Lazaret figé (point 3) ; Redis layout / TTL **non figés** ; crate **Accepted** point 1.
- Point 8 : Manifesto **sans conscience** de la gateway ; **Lazaret** est le nom du BC ; gate P2 Manifesto **inchangé** (zéro `gateway` sous `Manifesto/*/src`).
- Point 9 : consentement capacités = **P3** ; legacy→managed **mise de côté**.
- Point 10 : **zéro** nouveau type FGA.
- Point 11 : connecteurs nommés, admission opérateur/plateforme ; pas d’URL brute choisie par le plugin.
- Point 12 : **pas** de levée de 0006 G pour Manifesto.
- Point 13 : `APP-05` **reste ouvert** ; projet public n’ouvre pas invoke / KV / secrets.
- Point 14 : invoke = binding courant (`desired_generation` + `grant_revision`) ; fencing P2 inchangé ; pas de second epoch.
- Preuves visées : `Manifesto/tests/apparatus_p3_t*.rs` + gate Lazaret. Éventuelle levée E **future** seulement si un HTTP **domaine** Manifesto est ratifié (résidu point 5) — **pas** cette Accept. Wiki et canvas restent conception / lecture seule. Cette ADR ne SuperSède pas 0001–0006.

## Alternatives rejetées

Déjà rejetées par des ADR **Accepted** (rappel) ; P3 ne les rouvre pas.

| Option | Pourquoi pas |
|---|---|
| Copier le wiki (mTLS, SQL, URLs, microservice) comme contrat | Conception `^[inferred]`, pas canon |
| Bearer / JWT / HMAC IAM dans le plugin | ADR-0004 |
| `trusted_skip_gateway` | ADR-0005 |
| Second UUID public / nouveau type FGA V1 | ADR-0001, 0004, 0006 J ; point 10 |
| Harness P0 comme runtime ou KV de production | ADR-0003, 0004 Réalité Partial |
| K8s / WASM comme isolation **P3** | Hors jalon ; `APP-01` / P4 |

Pour le **point 1** (2026-09-13, ratifié à l’Accept) l’utilisateur écarte aussi (A) gateway dans le domaine in-process Manifesto et (B) service à Postgres Manifesto partagé ou réplique événements comme vérité authz. Il écarte les noms `ApparatusGateway`, `apparatus-gateway` et `apparatus-gateway-service`. Ce n’est pas une réécriture des ADR 0001–0006.

Pour le **point 2** (2026-09-13, ratifié à l’Accept) l’utilisateur écarte l’IP privée / le même VPS comme identité, le HMAC IAM comme autorité d’identité workload, un jeton seulement crypto-valide sans contrôle live Manifesto, et un **PKI de fournisseur cloud** comme autorité d’émission V1. Le wiki n’est pas copié comme contrat. L’autorité émettrice V1 est la **CA interne plateforme** ; le produit / bibliothèque CA n’est pas nommé ici.

Pour les **points 4–5** (2026-09-13, ratifié à l’Accept) l’utilisateur écarte : une méthode `invoke` sur `ApparatusRuntime` Manifesto ; un second protocole wire ; une levée de 0006 E sur les 5 `/components` ; une API Manifesto « pour la gateway ».

Pour le **point 6** (2026-09-13, ratifié à l’Accept) l’utilisateur écarte : un cluster Vault commercial obligatoire ; Secrets Manager cloud comme unique backend V1 ; « Active Manager » comme **port** ; le clair des secrets dans le KV plugin ; un « protocole Vault » IETF inventé ; un nom de trait Rust figé ici.

Pour le **point 7** (2026-09-13, ratifié à l’Accept) l’utilisateur écarte : Mongo / base document ; schéma relationnel possédé par le plugin ; magasin in-memory ; `apparatus-reference-kv` ou harness comme production ; KV dans le Postgres **Manifesto** ; un unique défaut Redis XOR Postgres ; un format de clé pointé `projectId.pluginId.key` fourni par l’auteur ; une API CAS inventée au-delà du trait `KvStore` actuel.

Pour le **point 3** (2026-09-13, ratifié à l’Accept) l’utilisateur écarte : un second UUID public ; un type FGA ; des lignes KV dans la DB Manifesto ; une FK inter-DB gateway→Manifesto ; un DROP de `apparatus_bindings` au `down` P3.

Pour le **point 9** (2026-09-13, ratifié à l’Accept) l’utilisateur **met de côté** la migration / backfill legacy→managed comme travail actif (jamais déployé, pas de tenants). Le consentement capacités reste P3. Les ADR 0001–0006 **ne SuperSèdent pas** elles-mêmes.

Pour les **points 8 et 12** (2026-09-13, ratifié à l’Accept, même intention) l’utilisateur écarte : que Manifesto ait conscience de la gateway ; un chemin d’allowlist `gateway` sous `Manifesto/*/src` ; une levée ou SuperSède de 0006 G pour que **Lazaret** existe ; un client Manifesto→Lazaret ; des routes Manifesto « pour la gateway ». **Lazaret** n’est **pas** une allowlist Manifesto.

Pour les **points 10, 11, 13, 14** (2026-09-13, ratifié à l’Accept) l’utilisateur écarte : un nouveau type FGA / une escalade FGA ; un domaine V1 arbitraire ou des URLs brutes choisies par le plugin ; clôturer `APP-05` ou ouvrir invoke / KV / secrets sur projet public ; un second fencing / epoch d’invoke.

« Pas de nouveau microservice » n’est plus le défaut de travail du point 1.

## Non décidé ici

- Produit ou bibliothèque CA (dette restante du point 2 ; aucun nom **figé** ici) ; rotation et TTL numériques exacts. L’implémentation T3 enregistre des **défauts** (session 15 min, certificat 24 h, nom logiciel `platform-internal-ca`) dans Conséquences / Réalité — ils ne deviennent pas des chiffres Accepted. T13 enregistre un défaut CA TTL `24 * 365` h (8760) de même nature.
- Modalités d’émission : CSR depuis le workload vs keypair injecté (options résiduelles, pas figées). T3 implémente CSR-from-workload par défaut sans clôturer l’option keypair injecté.
- Layout des clés Redis comme schéma gelé (point 7) ; TTL KV (aucun chiffre ici).
- Route Manifesto de lecture **domaine** (binding, consentement, grants) si le contrat HTTP manque (**résidu** du point 5) — jamais une API « pour la gateway » ; aucune URL inventée.
- SuperSède ou levée partielle de 0006 E (libellé ultérieur, si un HTTP domaine Manifesto est ratifié).
- `APP-01` … `APP-07` (`APP-05` **reste ouvert** — ne pas préempter, point 13).
- Factory, host UI, iframe, MessageChannel, pipeline OCI (P4–P6).
- Drain / destruction / rétention des bindings déjà `ready` (0005, P4/P6).

## Références

- Prompt : `docs/apparatus-p3-implementation-prompt.md`
- Canon : ADR-0001 … 0006 (`docs/adr/`)
- Code : `apparatus-contracts/src/ports.rs` (`KvStore` : `kv_get` / `kv_put` / `kv_delete` / `kv_purge`), `apparatus-contracts/src/protocol.rs`, `Manifesto/tests/apparatus_p2_t7_gate.rs`, `Manifesto/http/src/lib.rs`, `Manifesto/infra/src/apparatus_runtime/`, `openfga/model.fga`
- Style SQL : `Manifesto/migration/src/m20260912_000012_create_apparatus_bindings_table.rs`, `Manifesto/migration/src/m20260912_000013_apparatus_p2_runtime.rs`
- Analogie BC distinct / API : `Hive/config/development.toml` (`[iam_service]`, `db = "hive_dev"`), `Manifesto/infra/src/adapters/component_service_client.rs` (`ComponentServicePort`) ; packages `Hive/Cargo.toml` (`hive-service`), `Manifesto/Cargo.toml` (`manifesto-service`), `Telegraph/Cargo.toml` (`telegraph-service`) ; nommage P3 **Accepted** : `Lazaret` / `lazaret-service`
- Wiki (conception, pas contrat) : plan P3, capabilities, platform, bindings
- Preuve d’implémentation : T1 absence ; T2 gate + grants + scaffold ; T3 `Manifesto/tests/apparatus_p3_t3_identity.rs` + `Lazaret/tests/apparatus_p3_t3_identity.rs` (identité hybride) ; T4 `Manifesto/tests/apparatus_p3_t4_gateway.rs` + `Lazaret/tests/apparatus_p3_t4_grants.rs` (consult live + intersection) ; T5 `Manifesto/tests/apparatus_p3_t5_consent.rs` + `Lazaret/tests/apparatus_p3_t5_consent.rs` (consent write / close-at-commit) ; T6 `Manifesto/tests/apparatus_p3_t6_kv.rs` + `Lazaret/tests/apparatus_p3_t6_kv.rs` (KV Postgres+Redis, secrets-by-ref wiremock) ; T7 `Manifesto/tests/apparatus_p3_t7_invoke.rs` + `Lazaret/tests/apparatus_p3_t7_invoke.rs` (invoke HTTP + proxy nommé) ; T8 `Lazaret/tests/apparatus_p3_t8_kv_purge.rs` (`kv_purge` sur `component_removed` / file dédiée) ; T9 `Lazaret/tests/apparatus_p3_t9_invoke_prefix.rs` (chemin public `/lazaret/invoke` sur `prefixed_router`) ; T10 `Lazaret/tests/apparatus_p3_t10_enrollment_persist.rs` (enrollment Postgres + revoke sur `component_removed`) ; T11b `Lazaret/tests/apparatus_p3_t11b_session_mtls.rs` (session mTLS live HTTPS, trou `/session` fermé) ; T12 `Lazaret/tests/apparatus_p3_t12_openbao.rs` (OpenBao produit compose/testcontainer, KV v2 ; T6 IT reste wiremock) ; T13 `Lazaret/tests/apparatus_p3_t13_ca_persist.rs` (CA persistée + compose Lazaret HTTPS). Réalité **Partial** ; **pas** Implemented.
