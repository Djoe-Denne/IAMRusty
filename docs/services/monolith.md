# oodhive-monolith

Un listener, quatre bounded contexts. Compose les `create_router` + background tasks. Pas d’appel à `run()` des services.

- Package : `oodhive-monolith`
- Nests : `/iam`, `/telegraph`, `/hive`, `/manifesto` ([`monolith/src/routes.rs`](../../monolith/src/routes.rs))
- `/health` monolithe + `/ready` (crate `readiness`)
- Hors compose par défaut

Contrats de chemins **identiques** au standalone. Tests IT préfixés. Détail : [../platform/runtime.md](../platform/runtime.md).
