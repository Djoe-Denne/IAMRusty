# E2E mesh — pièges durables

- Preuve e2e (2026-10-02) : `bash ops/scripts/mesh-authn-kind-e2e.sh` sur `kind-aiforall-local`. Exit 0. `ops/scripts/mesh-authn-e2e.sh` (Compose) n'est plus la preuve.
- Build image Linux pour Kind : `docker compose build` / `build-artifacts`, puis `kind load` — Kind ne compile pas. (2026-10-07 : dev/test est natif Windows ; Docker est réservé au build d'image et à l'infra testcontainers. Ne pas monter le `target\` hôte dans un conteneur Linux.)
- Hive : binaire `hivemigration` (collision Lazaret `migration`). Dockerfile copie vers `/app/migration`.
- Hive refuse de démarrer si `iam_service.api_key` est vide. Secret de dev partagé IAM/Hive dans le compose de base.
- `DROP DATABASE … WITH (FORCE)` dans le compose de base. Incompatible avec un pod Kind sur le port hôte 5432. Le Postgres `kind-mesh` n'a pas de hostPort.
- Healthchecks : `/{préfixe}/health`. `jwks_url` des toml = 127.0.0.1 pour l'hôte ; Compose override `*_AUTH__JWT__JWKS_URL` vers `iam-service`.
- Kind `aiforall-platform` est default-deny. Sans `networkpolicy.yaml`, DNS et JWKS expirent. Le pod e2e alpine a besoin d'egress 80/443 pour `apk`. Appeler Envoy en `https://envoy-mesh:10000` (SAN = `envoy-mesh`, pas le FQDN).
- Images multi-arch (postgres, openfga, alpine) : `kind load` échoue tant qu'un `docker build --platform linux/amd64` n'a pas écrasé le tag.
- Telegraph avec file désactivée : le consommateur no-op termine et arrêtait le processus. Le working tree garde le serveur HTTP dans ce cas. Image Kind rechargée.
- IT Manifesto : conteneur Docker déjà là `openfga_test-fga` → 409. Ne pas `docker rm -f` spontanément.
- Voir `mem:architecture/events-authz-adr-0308`.
