# E2E mesh Compose — pièges durables

- Commande : `bash scripts/mesh-authn-e2e.sh` (cas dans `scripts/mesh-authn-e2e-cases.sh`). Compiler dans Docker, pas `cargo` Windows. Ne pas monter le `target\` hôte.
- Construire `build-artifacts` **avant** les images services. Un `compose build` parallèle copie des binaires périmés depuis `local/build-artifacts`.
- Hive : binaire `hivemigration` (collision avec Lazaret `migration` dans `target/release` partagé). Le Dockerfile copie `hivemigration` vers `/app/migration`. Sinon Hive applique les migrations Lazaret et `organizations` n'existe pas.
- Hive ne démarre pas si `iam_service.api_key` est vide (StaticCredential fail-closed). `docker-compose.yml` (fichier de base, pas seulement l'overlay mesh) pose `IAM_INTERNAL_SERVICE_TOKEN` et `HIVE_IAM_SERVICE__API_KEY` au même secret de dev.
- `create-databases` dans le compose de base : `DROP DATABASE … WITH (FORCE)`. Le pod Kind `oodhive-monolith` via le port hôte 5432 bloquait le DROP. Chaque `docker compose up` coupe ces sessions et recrée les bases.
- Healthchecks Docker Hive, Telegraph, Manifesto : `curl http://localhost:8080/health` est faux. La route réelle est `/{préfixe}/health`. IAM n'a pas de HEALTHCHECK. Conteneurs marqués unhealthy alors que l'e2e joint `https://{svc}-service:8443/{svc}/health`.
- `jwks_url` des `development.toml` pointe `http://127.0.0.1:8080/iam/.well-known/jwks.json`. Dans un conteneur ce n'est pas IAM. Le mode mesh ne fetch pas le JWKS, donc l'e2e ne le montre pas. Hors overlay, Hive/Telegraph/Manifesto ne vérifient pas un JWT RS256. IAM injecte le JWKS en mémoire.
- `deploy/apps/overlays/kind-mesh/README.md` décrit le mesh Compose ; `envoy-mesh.yaml` à côté route encore `prefix: /` vers `cluster: backend`.
- Voir `mem:architecture/events-authz-adr-0308`.
