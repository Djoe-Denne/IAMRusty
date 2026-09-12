# ADR-0401 : Manifesto possède projets, composants, membership en base, CAS et ACL+outbox atomiques

- Statut : Accepted
- Réalité : Implemented
- Date : 2026-09-12
- Décideurs : Architecture AIForAll — photographie rétroactive du dépôt
- Jalon concerné : architecture actuelle (hors P-Apparatus pour le cœur ; P1 Apparatus = 0001)
- SuperSède : aucune — n’absorbe pas ADR-0001
- SuperSédée par : —

`Accepted` ratifie une cible. Le champ `Réalité` indique séparément ce que le code du dépôt réalise.

## Contexte

Manifesto est le golden path HTTP/OpenFGA (`docs/guides/nouveau-service.md`). La tentation est de traiter le membership projet comme des tuples OpenFGA seuls, ou d’écrire l’ACL et l’outbox dans deux commits distincts (AuthZ et événements divergents).

Le dépôt stocke déjà `project_members` (et grants d’instance) en SQL, versionne le projet, et enveloppe mutation AuthZ + outbox dans une même transaction.

## Décision

1. **Manifesto** est le bounded context **projets, composants, membership projet**. Préfixe `/manifesto` (compose 8083). JWT consommateur `[auth.jwt]`.
2. **Membership = vérité SQL** (`project_members`, rôles / `ProjectMemberRolePermission`). OpenFGA (`project`, `component`) est le PDP de Check, alimenté par l’outbox → `sentinel-sync` (0301, 0303), pas le seul registre des membres.
3. **CAS `projects.revision`** : toute mutation qui change l’AuthZ prend un verrou `SELECT … FOR UPDATE` sur `(id, revision)`, incrémente, et mappe un conflit en **409**.
4. **ACL d’instance + outbox = même transaction** (`ProjectAuthorizationUnitOfWorkImpl`) : écriture projet/membre/grants SQL + `OutboxRecorder.record` puis commit. Rollback du domaine = rollback de l’outbox. Permission refusée → 403.
5. **P1 Apparatus** (table `apparatus_bindings` 1:1, backfill, alias HTTP) **n’est pas re-décidé ici** : voir **ADR-0001**. Cette ADR photographie le cœur projets/composants/ACL déjà en place.

Collaborateur HTTP : catalogue composants (`ComponentServicePort`). Events métier → `manifesto-events` / `sentinel-sync-events`.

## Conséquences

- Un grant projet ne s’ajoute pas « dans FGA seulement » : d’abord SQL + outbox atomiques.
- Les tests IT AuthZ utilisent OpenFGA réel (0200, 0302) ; le membership listé par l’API lit la base.
- Scaffold d’un nouveau CRUD+ACL : cloner Manifesto, pas IAM (0400) ni le monolithe (0404).

## Alternatives rejetées

| Option | Pourquoi pas |
|---|---|
| Membership = tuples FGA uniquement | Perte de source métier, listes et CAS impossibles sans miroir SQL |
| Outbox après commit ACL | Fenêtre où l’AuthZ SQL et `sentinel-sync` divergent ; le UoW l’interdit déjà |
| Réécrire 0001 dans cette ADR | Le binding Apparatus est une décision séparée (Partial) |

## Non décidé ici

- Binding 1:1, backfill, collisions, alias `?binding` : **0001**.
- Factory, host, gateway, `VALID`/`VERIFIED` : **0002–0005**, **0406**.
- Lease / fencing / génération (P2).
- Partenariat / join public (hors modèle actuel).

## Références

- Wiki : `projects/manifesto/manifesto.md`, `projects/manifesto/concepts/component-instance-permissions` (si présent), `docs/functional/projet.md`
- Handbook : `docs/services/manifesto.md`, `Manifesto/IMPLEMENTATION_STATUS.md`, `Manifesto/docs/`
- Code : `Manifesto/infra/src/transaction.rs` (`lock_project_revision`, outbox dans la txn), `Manifesto/domain/src/entity/project.rs` (`revision`), `Manifesto/migration/src/m20260905_000010_add_project_revision.rs`, tables `project_members`
- Preuve : `Manifesto/tests/transaction_readiness_tests.rs` (UoW + outbox + conflit de révision)
