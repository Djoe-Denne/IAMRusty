# ADR-0303 : `sentinel-sync` est un worker événements → tuples FGA, pas un hexagone

- Statut : Accepted
- Réalité : Implemented
- Date : 2026-09-12
- Décideurs : Architecture AIForAll — photographie rétroactive du dépôt
- Jalon concerné : architecture actuelle (hors P-Apparatus)
- SuperSède : aucune
- SuperSédée par : —

`Accepted` ratifie une cible. Le champ `Réalité` indique séparément ce que le code du dépôt réalise.

## Contexte

OpenFGA est le PDP des HTTP Hive / Manifesto / Telegraph (0302). Les tuples doivent suivre les faits de domaine. La tentation est de scaffolder sentinel-sync comme une cinquième vertical slice RustyCog (HTTP, `setup`, ports) ou de le lancer dans le compose par défaut.

Le handbook le décrit déjà comme worker sans HTTP, hors `docker compose up -d`.

## Décision

1. **`sentinel-sync`** est un **worker** : un consumer `QueueConfig` + handler + client Write OpenFGA. **Pas** une slice hexagonale (pas de `RouteBuilder`, pas de prefix HTTP métier).
2. Quatre **translators** dans `sentinel-sync/src/translator/` : **Hive**, **Manifesto**, **IAM**, **Telegraph**. Dispatch par préfixe `event_type`. Event inconnu → `None` (no-op, pas d’erreur, pas de tuple).
3. **Hors compose défaut** : absent de `docker-compose.yml` au `up` par défaut. Démarrage : `cargo run -p sentinel-sync` après bootstrap store/modèle. L’infra OpenFGA (migrate + run) est dans le compose ; le worker non.

Le worker n’est pas la vérité métier : il projette des events (0300 / 0301) vers le store FGA.

## Conséquences

- Un nouvel event qui change l’AuthZ **doit** ajouter un bras translator ; sinon le store dérive.
- Ne pas cloner Manifesto pour étendre sentinel-sync.
- `docker compose up -d` ne synchronise pas OpenFGA ; les HTTP Checkent un store que le worker n’alimente que s’il tourne à part.
- `--reconcile` (rejeu Manifesto → wildcards / tuples) est un mode opérateur du même binaire, pas un service HTTP.

## Alternatives rejetées

| Option | Pourquoi pas |
|---|---|
| Vertical slice hexagonale (http + setup + ports) | Pas d’API métier ; 0100 l’exclut déjà des 4 slices |
| Writer FGA inline dans Hive/Manifesto | Sépare persist métier et projection AuthZ ; le worker est le bras Write |
| Service compose par défaut | Handbook et README : hors compose ; OpenFGA seul est dans le `up` |

## Non décidé ici

- Ledger d’idempotence (in-memory vs Postgres) comme contrat plateforme.
- Couverture `--reconcile` hors Manifesto.
- Déploiement k8s / image compose optionnelle du worker.

## Références

- Handbook : `docs/services/sentinel-sync.md`, `docs/platform/events-outbox.md` (consommateurs, traductions FGA)
- Code : `sentinel-sync/src/main.rs` (consumer + 4 translators), `sentinel-sync/src/translator/{hive,manifesto,iam,telegraph}.rs`, `sentinel-sync/src/fga_client.rs`
- Compose : `docker-compose.yml` (OpenFGA présent ; pas de service `sentinel-sync`)
- Preuve : crate `sentinel-sync` dans `workspace.members` ; `SentinelSyncConfig` = logging + `QueueConfig` + OpenFGA + ledger ; README racine « hors compose »
