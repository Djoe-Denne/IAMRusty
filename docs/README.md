# Handbook AIForAll

Documentation d’implémentation et de parcours métier pour ce workspace. Le vault Obsidian (`obsidian/AI FOR ALL/`) reste la distillation historique ; les skills (`.agents/skills/`) restent les checklists agent. **Ici** : ce qu’il faut pour ajouter un service, brancher un JWT, et comprendre qui peut quoi.

## Commencer ici

| Besoin | Page |
|---|---|
| Carte des services, ports, préfixes | [platform/overview.md](platform/overview.md) |
| Compose, monolith, `/ready` | [platform/runtime.md](platform/runtime.md) |
| JWT émetteur vs consommateur | [platform/authn-jwt.md](platform/authn-jwt.md) |
| OpenFGA et `RouteBuilder` | [platform/authz-openfga.md](platform/authz-openfga.md) |
| Files SQS, outbox, sentinel-sync | [platform/events-outbox.md](platform/events-outbox.md) |
| Sections TOML et préfixes env | [platform/config-cheatsheet.md](platform/config-cheatsheet.md) |

## Implémenter

- [guides/nouveau-service.md](guides/nouveau-service.md) — checklist plateforme (au-delà de Manifesto)
- [guides/jwt-consommateur.md](guides/jwt-consommateur.md) — recette `[auth.jwt]` + tests
- [guides/permissions.md](guides/permissions.md) — UUID profond vs `with_permission_on_param`
- [guides/tests-integration.md](guides/tests-integration.md) — harness, OpenFGA réel, fixtures

## Parcours métier

- [functional/identite.md](functional/identite.md) — IAM (index des scénarios QA)
- [functional/organisation.md](functional/organisation.md) — Hive
- [functional/projet.md](functional/projet.md) — Manifesto
- [functional/notifications.md](functional/notifications.md) — Telegraph
- [functional/autorisation.md](functional/autorisation.md) — modèle FGA + contrats red-team

## Fiches service

- [services/iamrusty.md](services/iamrusty.md)
- [services/hive.md](services/hive.md)
- [services/manifesto.md](services/manifesto.md)
- [services/telegraph.md](services/telegraph.md)
- [services/sentinel-sync.md](services/sentinel-sync.md)
- [services/monolith.md](services/monolith.md)

## Déjà ailleurs (ne pas dupliquer)

- Guides crates RustyCog : [`.agents/skills/rustycog/`](../.agents/skills/rustycog/)
- Guides IAM détaillés : [`IAMRusty/docs/`](../IAMRusty/docs/)
- Guides Manifesto hexagonaux : [`Manifesto/docs/`](../Manifesto/docs/)
- Scénarios QA identité : [`IAMRusty/qa/scenarii/`](../IAMRusty/qa/scenarii/)
- Reviews d’architecture août 2026 : [`docs/reviews/`](reviews/)
- ADR Project Service : [`docs/project/Archi.md`](project/Archi.md)
- Hook `cargo fmt` : [`docs/CARGO_FMT_PRE_COMMIT.md`](CARGO_FMT_PRE_COMMIT.md)
