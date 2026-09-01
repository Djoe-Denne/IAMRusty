# Manifesto

Projets, composants, membership projet. Service de référence pour scaffolder.

- Préfixe : `/manifesto` — compose : **8083**
- JWT : `[auth.jwt]` consommateur
- OpenFGA : types `project` / `component` ; lectures publiques via `user:*`
- Events : projet/membre/permission → `sentinel-sync-events` ; conso optionnelle `component_status_changed`
- Collaborateur HTTP : catalogue composants (`service.component_service`)

## Docs

- [`Manifesto/README.md`](../../Manifesto/README.md), [`Manifesto/docs/`](../../Manifesto/docs/)
- [../functional/projet.md](../functional/projet.md)
- [../guides/nouveau-service.md](../guides/nouveau-service.md)
