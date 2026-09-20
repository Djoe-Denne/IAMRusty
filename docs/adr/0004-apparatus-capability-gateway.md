# ADR-0004 : Toute I/O d’un Apparatus passe par une gateway de capacités ; l’état durable est un KV plateforme

- Statut : Accepted
- Réalité : Implemented
- Date : 2026-09-10
- Décideurs : Architecture AIForAll — ratification orchestrée du 2026-09-10 ; Réalité Implemented A-DEC 2026-09-20
- Jalon concerné : P0 (contrat de capacités), P3 (implémentation gateway)
- SuperSède : aucune
- SuperSédée par : —

`Accepted` ratifie le contrat ci-dessous. `Réalité : Implemented` : gateway = Lazaret T5–T14b — HTTP `invoke` + connecteurs **nommés** (pas Manifesto) ; consentement close-at-commit ; KV Postgres+Redis ; secrets-by-ref OpenBao / Vault HTTP (produit T12 ; IT T6 reste wiremock) ; `kv_purge` branché sur `component_removed` ; session mTLS optionnelle (T11b). `APP-05` reste Non décidé. Preuve V1 privé : T7 `t7_public_project_without_consent_does_not_open_call`.

## Contexte

Aujourd’hui l’authz Manifesto croise session IAM, ACL projet/composant et parfois `enforce_world_read_or_principal` (projet public lisible). Un JWT consommateur HS256 (`iss=iamrusty`, `aud=aiforall`) ne doit pas être remis à du code tiers : le secret HMAC permettrait de forger des identités.

Les bases PostgreSQL des services et le réseau interne ne sont pas un stockage d’extension. `Archi.md` faisait posséder sa base à chaque composant ; pour du code communautaire, c’est une fuite de surface.

OpenFGA actuel décrit projet / membre / instance de composant, pas `storage.kv.write` ni un connecteur réseau nommé.

## Décision

Cette décision est un **contrat normatif dès P0** ; sa gateway réseau sera implémentée en P3.

1. Le plugin **ne reçoit jamais** le bearer IAM utilisateur, un JWT interne de service plateforme ni le secret HMAC servant à les signer.
2. Chaque opération (interactive ou de fond) est autorisée par la **gateway**, à l’appel, par l’intersection de : principal actif selon l’état transactionnel DB **et** ACL du projet/de l’instance, release admise, capacités **déclarées** par la release, capacités **consenties** sur le binding, `desired_state` et `grant_revision` courants, politique opérateur. Une capacité inconnue est un refus.
3. L’état DB ferme l’accès dès le commit d’une suspension, d’une révocation ou d’un retrait de membre. Une projection OpenFGA ou un cache périmé ne suffit pas à maintenir l’autorisation.
4. Un projet public **n’ouvre pas** invoke, KV ni secrets. Lecture anonyme d’opérations explicitement publiques = décision `APP-05`; le contrat V1 reste privé tant qu’elle n’est pas prise.
5. État durable V1 : API **KV plateforme** namespacée par binding (quota, CAS). Pas d’accès SQL Manifesto/Hive/IAM. Secrets : référence opaque, injection seulement dans l’opération accordée.
6. Réseau sortant : uniquement via proxy/gateway et **connecteurs nommés** admis. Le manifeste ne choisit pas un domaine arbitraire en V1.
7. OpenFGA reste la projection des droits **utilisateur / projet / instance**. Les grants de capacités sont un contrat distinct (consentement + révision).

L’implémentation mTLS / rotation (P3) n’est pas figée ici ; le contrat d’identité de workload l’est : instance, binding, release, génération, `grant_revision` et audience gateway, jamais l’utilisateur. Cette identité emploie une autorité distincte du HMAC IAM ; un jeton cryptographiquement valide ne suffit pas.

## Conséquences

- P0 : taxonomie minimale dans le manifeste (`project.read`, `storage.kv.read/write` pour la référence). Pas de gateway réseau réelle alors (2026-09-10).
- P3 : gateway réseau = Lazaret `POST /invoke` (T7).
- Tests ultérieurs : deux projets / deux bindings adverses ; révocation ; membre suspendu avec projection FGA périmée.
- Interdire `fetchInternal` et toute URL interne dans le SDK UI comme dans le backend.
- Une implémentation ne peut pas utiliser `enforce_world_read_or_principal` comme autorisation d’invoke.

## Alternatives rejetées

| Option | Pourquoi pas |
|---|---|
| Plugin appelle Manifesto/Hive avec le JWT utilisateur | Bearer volable, surface IAM dans le plugin |
| Plugin possède PostgreSQL / volume host | Sort du tenant, hors quota plateforme, backup incontrôlé |
| Hériter de la lecture publique pour invoke | Fuite de données d’extension |
| Encoder chaque capacité en relation OpenFGA V1 | Explosion du modèle ; l’instance ACL existe déjà |

## Non décidé ici

- `APP-05` (UI publique, tâches de fond).
- `APP-03` (rétention / purge).
- `APP-06` (quotas chiffrés, SLO).

Produit secrets et transport d’identité workload : tranchés en ADR-0007 (T12 OpenBao ; T3 + T11b hybride mTLS / session). TTL / produit CA restent Non décidé **dans 0007**, pas ici.

### APP-05 — lecture anonyme / ops publiques (non tranché)

Le ticket **vit ici** (décision L28 + cette section). Ce n’est **pas** une ADR. Il n’est **pas** clôturé. **Statut : Non décidé.** P3 n’a pas préempté.

**Question :** un jour, surface Apparatus anonyme / UI publique / jobs sur un projet public ?

**V1 déjà livré :** un projet public **n’ouvre pas** invoke / KV / secrets. Preuve T7 : `t7_public_project_without_consent_does_not_open_call`.

Deux options, **non choisies** :

- **A** — ratifier le V1 privé comme politique **durable** (won't do surface anonyme). Surtout docs + fail-closed ; cheap. Préempte 0007 L189 si on le faisait. Invoke / KV / secrets restent privés. La lecture monde Manifesto est orthogonale.
- **B** — lame publique limitée **plus tard**. Nouvelle ADR produit (quand on tranche, pas ici). Identité anonyme. Ops **explicitement** publiques, **sans** hériter invoke (Conséquences L48 : « Hériter de la lecture publique » **rejeté** ; jamais via `enforce_world_read_or_principal`). UI / P5, jobs, OpenFGA distinct, tests de non-fuite ; plus de surface.

Ne pas choisir A ou B ici. Pointeur : ADR-0007 checklist 13 / L288.

## Références

- Wiki : `apparatus-capabilities-and-isolation`, `immediate-membership-acl`, `component-instance-permissions`
- Code : JWT consommateur (`docs/platform/authn-jwt.md`), `enforce_world_read_or_principal`, OpenFGA Manifesto
- Preuve d’implémentation (2026-09-10, P1 2026-09-12, P3 T5–T14b 2026-09-15…2026-09-20) : taxonomie minimale déclarée dans le manifeste (`project.read`, `storage.kv.read/write`) ; port `KvStore` en domaine (`apparatus-contracts/src/ports.rs`), KV de référence namespacé par `binding_id` via le harness in-process (feature `test-harness`). Aucun bearer IAM ni JWT transmis au plugin. P1 : INSERT managed même txn que composant+ACL+outbox (`Manifesto/infra/src/transaction.rs`), grants projet inchangés, revoke→403 TTL0, ownership conservés, `grep apparatus openfga/model.fga` 0, zéro nouveau type FGA. P3 : gateway = Lazaret T5–T14b (`POST /invoke` + connecteurs nommés, pas Manifesto) ; KV plateforme Postgres+Redis (`apparatus_kv_entries`) ; secrets-by-ref OpenBao / Vault HTTP KV v2 (produit T12 ; IT T6 reste wiremock) ; `kv_purge` sur `component_removed` ; session mTLS optionnelle (T11b). `APP-05` reste Non décidé (V1 privé : T7 `t7_public_project_without_consent_does_not_open_call`). Réalité **Implemented**.
