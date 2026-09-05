# Manifesto

Projets, composants, membership projet. Service de référence pour scaffolder.

- Préfixe : `/manifesto` — compose : **8083**
- JWT : `[auth.jwt]` consommateur
- OpenFGA : types `project` / `component` ; `user:*` via sentinel-sync sur création `public` ou flip `ProjectVisibilityChanged`, jamais sur `publish`. GET/details/composants et liste anonyme fail-closed si la ligne n’est plus world-readable (`public` + `draft|active`). Flip impliquant `public` = `Admin`. Pas de partenariat / join. Détail : [../functional/projet.md](../functional/projet.md).
- Events : `project_created` / `project_visibility_changed` / `project_published` / `project_archived` / membre / permission → `sentinel-sync-events` ; conso optionnelle `component_status_changed`
- Collaborateur HTTP : catalogue composants (`service.component_service`)

## Docs

- [`Manifesto/README.md`](../../Manifesto/README.md), [`Manifesto/docs/`](../../Manifesto/docs/)
- [../functional/projet.md](../functional/projet.md)
- [../guides/nouveau-service.md](../guides/nouveau-service.md)
