# ADR-0200 : Les IT exercent un serveur HTTP réel, une base réelle et le harness rustycog-testing

- Statut : Accepted
- Réalité : Implemented
- Date : 2026-09-12
- Décideurs : Architecture AIForAll — photographie rétroactive vague 2 (2026-09-12)
- Jalon concerné : Transversal (plage 0200, harness IT)
- SuperSède : aucune
- SuperSédée par : —

## Contexte

La tentation est de remplacer le graphe HTTP + persistence par des doubles du use-case. Le dépôt teste déjà les services métier via `rustycog-testing` : serveur Axum réel, Postgres de test, migrations `Migrator::up/down`, JWT de harness, exécution `#[serial]` dès qu’un singleton process (OpenFGA, WireMock, files) est touché.

Le handbook (`docs/guides/tests-integration.md`) et le wiki (`integration-testing-with-real-infrastructure`) photographient la même recette. Les matrices Apparatus P1 (`Manifesto/tests/apparatus_p1_t*`) réutilisent ce harness ; elles ne définissent pas la politique IT.

## Décision

1. Toute IT HTTP d’un service métier passe par `setup_test_server()` déclaré dans `tests/common.rs` (Hive, IAMRusty, Manifesto, Telegraph).
2. Le bootstrap `rustycog::testing::setup_test_server` démarre un **serveur réel** ; le descripteur exécute les **migrations** (`has_db() == true`) et une **DB réelle**.
3. Les bodies utilisent la base URL **préfixée** (`SERVICE_PREFIX` : `/iam`, `/telegraph`, `/hive`, `/manifesto`), jamais l’origin brut du harness.
4. Les callers authentifiés utilisent `rustycog::testing::http::jwt::create_jwt_token` (HS256 `rustycog-test-hs256-secret`, `iss=iamrusty`, `aud=aiforall`). IAMRusty, en tant qu’émetteur, peut en plus exercer `tests/utils/jwt.rs` sur la forme des tokens.
5. `#[serial]` est obligatoire sur tout test qui touche `TestOpenFga`, le WireMock process-global, ou une file partagée.
6. Un seul `#[path = "fixtures/mod.rs"]` par crate d’IT dans `tests/common.rs` (Clippy `duplicate_mod`) ; les autres fichiers font `mod common`.

Hors décision : pyramide unitaire vs IT (non écrite comme norme chiffrée). Les `#[cfg(test)]` dans `src` et les IT dans `tests/` coexistent sans quota.

## Conséquences

- Ne plus pointer un client de test sur l’origin non préfixé (casse le mode monolithe `oodhive-monolith`).
- IAM n’a pas d’OpenFGA (`has_openfga() == false`) : JWT + DB + parfois OAuth WireMock.
- Hive / Telegraph / Manifesto retournent aussi `TestOpenFga` (ADR-0201).
- Écarts mineurs : `[server] port = 8081` (IAM, Telegraph) contre `port = 0` (Hive, Manifesto) ; IAM `common.rs` n’embarque pas `#[path]` (les fixtures sont `mod fixtures` par binaire, ex. SQS) ; Manifesto pointe `fixtures/component_service/mod.rs` ; `IAMRusty/docs/TESTING_GUIDE.md` cite encore des chemins `tests/common/*_testcontainer.rs` déplacés dans rustycog-testing.

## Alternatives rejetées

| Option | Pourquoi pas |
|---|---|
| Mock du use-case / handler à la place du serveur | Ne prouve ni routing préfixé, ni migrations, ni JWT réel |
| Origin brut `rustycog_testing::setup_test_server()` dans les bodies | Divergent standalone vs monolithe |
| OpenFGA mocké pour « aller plus vite » en IT HTTP | ADR-0201 : Check réel |

## Non décidé ici

- Kafka vs SQS unifié, profondeur d’assert events, sleeps Telegraph (wiki Open Questions) — pas une décision Implemented.
- NATS comme harness de test : absent de `rustycog-testing` et des `config/test.toml`.
- Harmoniser le port HTTP d’écoute des IT (`8081` vs `0`).

## Références

- Wiki : `concepts/integration-testing-with-real-infrastructure`, `skills/using-rustycog-testing`
- Handbook : `docs/guides/tests-integration.md`, `docs/guides/jwt-consommateur.md`
- Code : `*/tests/common.rs`, `Hive/http` `/hive`, `IAMRusty/http` `/iam`, `Telegraph/http` `/telegraph`, `Manifesto/http` `/manifesto` ; `rustycog/rustycog-testing/src/common/test_server.rs`, `src/http/jwt.rs`, `src/common/database.rs`
- Preuve : `setup_test_server` + `prefixed_url` + `Migrator` dans les quatre `common.rs` ; `#[serial]` massif sous `*/tests/` ; P1 Partial `apparatus_p1_t6_http.rs` (même JWT + serveur préfixé, pas la politique générale)
