# ADR-0405 : Le contrat `/ready` est une crate partagée, pas un hexagone

- Statut : Accepted
- Réalité : Implemented
- Date : 2026-09-12
- Décideurs : Architecture AIForAll — photographie rétroactive du dépôt
- Jalon concerné : architecture actuelle (hors P-Apparatus)
- SuperSède : aucune
- SuperSédée par : —

`Accepted` ratifie une cible. Le champ `Réalité` indique séparément ce que le code du dépôt réalise.

## Contexte

Chaque process HTTP a un `/health` de liveness rustycog. Les factories de queue peuvent **réussir en no-op** : un boot « OK » ne prouve pas que SQS/Kafka est live. La tentation est de créer un « service readiness » hexagonal, ou de confondre liveness et ready.

La crate `readiness/` expose déjà `GET /ready` et le signalement des queues.

## Décision

1. **`readiness`** est une **crate partagée** (bibliothèque) : `classify`, `factory`, `http` (`attach_ready`), `probe`. Elle n’est **pas** une vertical slice (pas de `setup`, pas de bounded context métier).
2. Les composition roots (IAM, Hive, Telegraph, Manifesto, monolithe) **attachent** `/ready` sur le routeur. `/health` reste la liveness ; `/ready` expose le rapport de probes (dont files classées no-op).
3. Un boot réussi n’implique pas un broker live : `classify_*` / `signal_queue_status` doivent le dire sur `/ready`.

Pas de préfixe métier. Consommée par les standalones et `oodhive-monolith` (0404).

## Conséquences

- On ne scaffold pas `readiness-service`.
- Les IT / probes k8s qui n’appellent que `/health` ne détectent pas une queue no-op.
- Nouveau process HTTP de la plateforme : dépendre de `readiness` et `attach_ready`, pas recopier un handler ad hoc.

## Alternatives rejetées

| Option | Pourquoi pas |
|---|---|
| Hexagone `readiness` | Pas de domaine métier ni ports applicatifs |
| `/health` suffit | Les factories rustycog peuvent no-op sans fail startup |
| Probe maison par service | Le contrat `/ready` serait quatre fois divergents |

## Non décidé ici

- SLO / sémantique exacte ready vs Kafka vs SQS en prod.
- `OpenFgaClientConfig.port = 0` (fixture) vs ports compose — hors cette crate.
- Readiness d’un futur host Apparatus (P5).

## Références

- Wiki : concept `queue-readiness-signaling`
- Handbook : `docs/platform/runtime.md` (section Readiness), `README.md`
- Code : `readiness/src/lib.rs`, `attach_ready` dans `IAMRusty/http`, `Hive/http`, `Telegraph/http`, `Manifesto/http`, `monolith`
- Preuve : crate unique ; `/ready` monté sur standalones et monolithe
