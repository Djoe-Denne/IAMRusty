# ADR-0201 : Les mocks d’IT ne couvrent que l’HTTP sortant ; OpenFGA en IT est un testcontainer réel

- Statut : Accepted
- Réalité : Implemented
- Date : 2026-09-12
- Décideurs : Architecture AIForAll — photographie rétroactive vague 2 (2026-09-12)
- Jalon concerné : Transversal (plage 0200, harness IT)
- SuperSède : aucune — retire de facto le fake WireMock `OpenFgaMockService` pour les IT de service
- SuperSédée par : —

## Contexte

Les collaborateurs HTTP externes (OAuth GitHub/GitLab, catalogue composants, provider Hive, parfois SMTP « ce que l’app enverrait ») ne sont pas le SUT. À l’inverse, mocker OpenFGA `Check` ou le use-case laisse passer des IT qui ne voient pas deny-by-default ni le vrai client.

`rustycog-testing` expose WireMock (`MockServerFixture::new` singleton `127.0.0.1:3000`, ou `isolated()`). Le module `permission` **réexporte `TestOpenFga`** : le WireMock `OpenFgaMockService` a été retiré.

## Décision

1. WireMock n’est autorisé que pour l’**HTTP sortant** : catalogue Manifesto (`ComponentServiceMockService`, `isolated()`), OAuth IAM (`tests/fixtures/github|gitlab`, URLs `http://localhost:3000/...` dans `IAMRusty/config/test.toml`), provider Hive (`ExternalProviderMockService`).
2. Interdit : mocker le use-case HTTP sous test. Interdit : `OpenFgaMockService` (ou tout fake `Check`) dès que `has_openfga() == true`.
3. IT OpenFGA = testcontainer réel `openfga/openfga` (`has_openfga() == true`, `openfga_authorization_model_json()`, `cache_ttl_seconds = 0`). Défaut **deny**. Happy path : `openfga.allow(subject, action, resource)`. IAM : `has_openfga() == false` (pas d’OpenFGA).
4. Telegraph : WireMock `SmtpService` si l’assert porte sur la charge émise ; MailHog `TestSmtp` si l’assert porte sur un listener SMTP réel (les deux coexistent sous `Telegraph/tests/fixtures/smtp/`).
5. `#[serial]` dès qu’on touche le singleton WireMock ou `TestOpenFga`.

Le singleton `127.0.0.1:3000` reste le bind de `get_mock_server()`. Ce n’est plus le seul listener : le catalogue Manifesto est isolé.

## Conséquences

- Skills qui présentent encore `OpenFgaMockService` « crate-level only » sont **stales** ; le type n’existe plus. Ne pas le réintroduire pour « aller plus vite ».
- `creating-testcontainer-fixtures` qui dit que Manifesto retourne un handle `OpenFgaMockService` est faux : le tuple expose `TestOpenFga` + le fake **catalogue**.
- Ne pas pointer `openfga.api_url` sur `:3000` dans une IT `has_openfga() == true`.

## Alternatives rejetées

| Option | Pourquoi pas |
|---|---|
| WireMock `Check` OpenFGA en IT Hive/Telegraph/Manifesto | Deux chemins d’authz ; deny réel non exercé |
| Mock du handler / application | Contredit ADR-0200 |
| Reloger le singleton WireMock | Port 3000 figé ; pointer `base_url()` / `isolated()` à la place |

## Non décidé ici

- Unifier WireMock `SmtpService` vs MailHog pour toute la suite Telegraph.
- Kafka vs SQS / sleeps (wiki) — ADR-0202 / hors scope mocks HTTP.

## Références

- Handbook : `docs/guides/tests-integration.md`
- Skills : `.agents/skills/creating-wiremock-fixtures/SKILL.md`, `creating-testcontainer-fixtures/SKILL.md`
- Code : `rustycog/rustycog-testing/src/wiremock/mod.rs`, `src/permission/mod.rs`, `src/common/openfga_testcontainer.rs` ; `Manifesto/tests/fixtures/component_service/service.rs` ; `IAMRusty/tests/fixtures/github/service.rs`, `gitlab/service.rs` ; `Hive/tests/fixtures/external_provider/service.rs` ; `Telegraph/tests/fixtures/smtp/service.rs`, `smtp/testcontainer.rs`
- Preuve : aucun usage IT de `OpenFgaMockService` ; Hive/Manifesto/Telegraph `has_openfga() == true` + `TestOpenFga` ; IAM `false`
