# Tests d’intégration

Skill détaillé : [using-rustycog-testing.md](../../.agents/skills/rustycog/references/using-rustycog-testing.md).

## Harness

Chaque service expose `setup_test_server()` dans `tests/common.rs` :

- base URL **préfixée** (`/iam`, `/telegraph`, `/hive`, `/manifesto`)
- client HTTP
- fixture DB / app
- Hive, Telegraph, Manifesto : handle `TestOpenFga`
- Manifesto : mock catalogue composants (wiremock)

Ne pas utiliser l’origin brut de `rustycog_testing::setup_test_server()` dans les bodies.

## Auth

```rust
let token = rustycog::testing::http::jwt::create_jwt_token(user_id);
```

Voir [jwt-consommateur.md](jwt-consommateur.md).

## OpenFGA réel

Défaut deny. Happy path :

```rust
openfga
    .allow(
        Subject::new(owner_id),
        Permission::Admin,
        ResourceRef::new("organization", org_id),
    )
    .await?;
```

- `#[serial]` sur tout test qui touche `TestOpenFga` ou wiremock (singletons process).
- Un seul `#[path = "fixtures/mod.rs"]` dans `tests/common.rs` (Clippy `duplicate_mod`).
- Interdit : `OpenFgaMockService` dans une IT `has_openfga() == true`.

## Transport

`test.toml` : `queue.enabled = false`. Suites SQS : descripteur `has_sqs() == true` + `HIVE_QUEUE__ENABLED=true` (ou IAM/MANIFESTO). Assert la file **destination** et la file fallback vide. Références : `Hive/tests/sqs_event_routing_tests.rs`, IAM, Manifesto.

## Fixtures HTTP sortantes

Skill [creating-wiremock-fixtures](../../.agents/skills/creating-wiremock-fixtures/SKILL.md) : wrapper `MockServerFixture`, helpers `mock_*`, `reset()` mid-test. Ex. catalogue composants Manifesto, GitHub/GitLab IAM, provider externe Hive.

## Fixtures Docker

Skill [creating-testcontainer-fixtures](../../.agents/skills/creating-testcontainer-fixtures/SKILL.md) : SQS, Kafka, OpenFGA, MailHog (Telegraph). `port = 0` + env publiées, pas de port fixe sauf MailHog historique.

## IAM

Pas d’OpenFGA. JWT + fixtures DB + parfois wiremock OAuth. Scénarios fonctionnels documentés : [../functional/identite.md](../functional/identite.md) et `IAMRusty/qa/scenarii/`.
