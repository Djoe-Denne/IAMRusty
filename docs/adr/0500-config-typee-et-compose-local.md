# ADR-0500 : La config runtime est typée via rustycog-config ; Compose local sans brokers

- Statut : Accepted
- Réalité : Implemented
- Date : 2026-09-12
- Décideurs : Architecture AIForAll — photographie rétroactive du dépôt
- Jalon concerné : architecture actuelle (hors P-Apparatus)
- SuperSède : aucune
- SuperSédée par : —

`Accepted` ratifie une cible. Le champ `Réalité` indique séparément ce que le code du dépôt réalise.

## Contexte

Chaque vertical slice a besoin d’une config serveur / base / logs / file, sans parser TOML à la main ni inventer une variable d’environnement parallèle. Un Compose « plateforme complète » tenterait d’ajouter NATS, Redis ou un broker Kafka — absents du fichier réel. `sentinel-sync` est un worker du workspace, pas un service Compose.

## Décision

1. **Loader unique** = `rustycog-config` (`build_config_with_env_prefix`) : `.env` optionnel, fichier TOML selon `RUN_ENV`, puis overlay des variables d’environnement préfixées (`PREFIX_`, séparateur `__`).
2. **Sélecteur d’environnement** = `RUN_ENV` uniquement (`test` → `config/test.toml`, `production` → `config/production.toml`, défaut `development`). **`RUST_ENVIRONMENT` n’est pas lu** par le loader (aucun usage dans `rustycog/`).
3. **Compose local** (`docker-compose.yml`) embarque exactement ces services : `build-artifacts`, `postgres`, `openfga-migrate`, `openfga`, `localstack`, `iam-service`, `telegraph-service`, `hive-service`, `manifesto-service`, `truncate-db`, `verify-emails`, `list-databases`, `create-databases`.
4. **Hors Compose** : pas de broker NATS, Redis ou Kafka ; `sentinel-sync` n’est pas un service Compose (mentionnée seulement en commentaire OpenFGA Write).

La CI pose `RUN_ENV: test` (`.github/workflows/ci.yml`).

## Conséquences

- Un nouveau service ajoute `config/{development,test,production}.toml` et un préfixe env, pas un second loader.
- Ne pas documenter ni scaffolder un broker dans Compose « pour plus tard ».
- Ne pas substituer `RUST_ENVIRONMENT` à `RUN_ENV`.

## Alternatives rejetées

| Option | Pourquoi pas |
|---|---|
| `RUST_ENVIRONMENT` comme sélecteur | Le loader ne le lit pas |
| Brokers (NATS / Redis / Kafka) dans Compose | Absents du fichier ; Kafka reste une feature rustycog opt-in (0502) |
| `sentinel-sync` dans Compose | Worker workspace, pas un service du compose actuel |

## Non décidé ici

- Unification des préfixes env entre slices.
- Broker Kafka en local (feature `kafka` hors `full` — 0502).
- Déploiement Compose de `sentinel-sync` ou du monolithe.

## Références

- Code : `rustycog/rustycog-config/src/lib.rs` (`RUN_ENV`, TOML, `Environment::with_prefix`)
- Compose : `docker-compose.yml` (liste ci-dessus ; commentaire l.53 OpenFGA / sentinel-sync)
- CI : `.github/workflows/ci.yml` (`RUN_ENV: test`)
- Preuve : loader + Compose + CI alignés ; `RUST_ENVIRONMENT` introuvable dans `rustycog/`
