# ADR-0103 : Le domaine ne dépend que de ports ; commandes via factory à clé string

- Statut : Accepted
- Réalité : Implemented
- Date : 2026-09-12
- Décideurs : Architecture AIForAll — photographie rétroactive du dépôt
- Jalon concerné : architecture actuelle (hors P-Apparatus)
- SuperSède : aucune
- SuperSédée par : —

## Contexte

Sans ports, le domaine importe SeaORM. Sans factory, une route HTTP peut exister sans handler de commande (cas Hive documenté). Le guide hexagonal impose ports domaine + registre exhaustif.

## Décision

1. **Domaine** = ports (`domain/src/port/**`, traits `async_trait`) + services. Aucun client DB/HTTP concret.
2. **Infra** = adapters de ces ports (`IAMRusty/infra/src/lib.rs` : « implementations of domain ports » ; repos, OAuth/JWT, clients externes).
3. **HTTP** = adapter entrant : construit une commande typée, exécute via `GenericCommandService`, mappe `CommandError` → `HttpError` **local** (dette partagée, pas `ServiceError` unifié).
4. Les commandes sont **enregistrées par clé string** dans une factory application :
   - Manifesto : `ManifestoCommandRegistryFactory::create_manifesto_registry`
   - Hive : `HiveCommandRegistryFactory::create_hive_registry`
   - Telegraph : `TelegraphCommandRegistryFactory::create_telegraph_registry`
   - IAMRusty : `CommandRegistryFactory::create_iam_registry`
5. Une commande non enregistrée ne doit pas être routée. L’OpenAPI plus large que le registre (Hive) est une **divergence de contrat**, pas une permission de skip factory.

## Conséquences

- Ajouter une route = port (si I/O) + use case + `register::<Cmd, _>("clé".to_string(), …)` + handler mince.
- Telegraph réinjecte le même `GenericCommandService` dans le consommateur SQS.
- Mapping d’erreurs reste **par service** (`http/src/error.rs`).

## Alternatives rejetées

| Option | Pourquoi pas |
|---|---|
| Use cases appelés depuis le handler sans registre | Double surface ; retries/command config contournés |
| Enregistrement par type seul, sans clé | L’API rustycog est `register::<C, _>(String, …)` ; les 4 passent une clé |
| DI concrète dans le domaine | Inverse la règle de dépendance |

## Non décidé ici

- Parité OpenAPI Hive (`obsidian/AI FOR ALL/projects/hive/concepts/command-registry-route-parity.md`).
- Telegraph : échecs queue aplatis en `ServiceError::infrastructure`.
- Transport d’événements vs contrat `*-events` (0300, 0301).
- ACL générique / CAS global (0401).

## Références

- `Manifesto/domain/src/port/service.rs` (`ComponentServicePort`)
- `IAMRusty/domain/src/port/mod.rs`, `Hive/domain/src/port/service.rs`, `Telegraph/domain/src/port/communication.rs`
- `*/application/src/command/factory.rs` ; `Manifesto/docs/rustycog-implementation-and-usage-guide.md` §1–3
- Preuve : clés string dans les 4 factories ; handlers `command_service.execute` sans `sea_orm`
