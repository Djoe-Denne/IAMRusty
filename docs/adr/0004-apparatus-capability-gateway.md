# ADR-0004 : Toute I/O d’un Apparatus passe par une gateway de capacités ; l’état durable est un KV plateforme

- Statut : Accepted
- Réalité : Partial
- Date : 2026-09-10
- Décideurs : Architecture AIForAll — ratification orchestrée du 2026-09-10
- Jalon concerné : P0 (contrat de capacités), P3 (implémentation gateway)
- SuperSède : aucune
- SuperSédée par : —

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

- P0 : taxonomie minimale dans le manifeste (`project.read`, `storage.kv.read/write` pour la référence). Pas de gateway réseau réelle.
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
- Mécanisme concret de secrets (produit à choisir en P3).
- Transport concret de l’identité workload (mTLS, rotation, durée de vie).

## Références

- Wiki : `apparatus-capabilities-and-isolation`, `immediate-membership-acl`, `component-instance-permissions`
- Code : JWT consommateur (`docs/platform/authn-jwt.md`), `enforce_world_read_or_principal`, OpenFGA Manifesto
- Preuve d’implémentation (2026-09-10, P1 2026-09-12) : taxonomie minimale déclarée dans le manifeste (`project.read`, `storage.kv.read/write`) ; port `KvStore` en domaine (`apparatus-contracts/src/ports.rs`), KV de référence namespacé par `binding_id` via le harness in-process (feature `test-harness`). Aucun bearer IAM ni JWT transmis au plugin. P1 : INSERT managed même txn que composant+ACL+outbox (`Manifesto/infra/src/transaction.rs`), grants projet inchangés, revoke→403 TTL0, ownership conservés, `grep apparatus openfga/model.fga` 0, zéro nouveau type FGA (T5 4/4). La gateway réseau réelle, l’identité workload et le KV plateforme persistent restent hors P1 (P3) → `Partial` maintenu.
