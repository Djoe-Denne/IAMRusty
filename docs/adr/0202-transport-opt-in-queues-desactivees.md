# ADR-0202 : Les files de test sont désactivées par défaut et ne s’activent que si le contrat de transport est sous test

- Statut : Accepted
- Réalité : Partial
- Date : 2026-09-12
- Décideurs : Architecture AIForAll — photographie rétroactive vague 2 (2026-09-12)
- Jalon concerné : Transversal (plage 0200, harness IT)
- SuperSède : aucune
- SuperSédée par : —

## Contexte

SQS/Kafka/SMTP réels sont lents et partagent des singletons Docker. Le handbook impose `queue.enabled = false` sur `config/test.toml`, puis un descripteur dédié `has_sqs() == true` + override d’env (`HIVE_QUEUE__ENABLED`, `IAM_QUEUE__ENABLED`, `MANIFESTO_QUEUE__ENABLED`) quand le **contrat de transport** est le sujet du binaire.

`rustycog-testing` fournit `sqs_testcontainer` et `kafka_testcontainer`. **Pas de NATS** dans ce crate ni dans les `test.toml` des services.

## Décision

1. Défaut producteur : `[queue] enabled = false` (IAMRusty, Hive, Manifesto). Descripteur HTTP par défaut : `has_sqs() == false`.
2. Opt-in SQS : binaire dédié (`*/tests/sqs_event_routing_tests.rs`) avec `has_sqs() == true`, env `*_QUEUE__ENABLED=true`, assert file **destination** et fallback vide. `#[serial]`.
3. Kafka : helper optionnel (`IAMRusty/tests/signup_kafka.rs` / `TestKafkaFixture`), pas le transport IT par défaut.
4. Ports d’infra de test : `port = 0` (DB, queue, OpenFGA) + env publiées. **Exception historique** : MailHog / SMTP Telegraph `communication.email.smtp.port = 1025`.
5. sentinel-sync : `[queue] type = "disabled"` (équivalent « pas de transport »).

Telegraph (consommateur) **n’est pas aligné** : `Telegraph/config/test.toml` a `enabled = true` et `TelegraphTestDescriptor::has_sqs() == true` + `has_smtp() == true`. C’est un écart, pas une seconde règle adoptée ici.

## Conséquences

- Ne pas activer la queue sur la suite HTTP rapide des producteurs.
- `Partial` : le défaut « false partout » est faux pour Telegraph (consommateur `queue.enabled = true`).
- IAM `IAMRustyTestDescriptorWithMockEvents` (`MockEventPublisher`) reste un faux in-process du port events, distinct de l’opt-in SQS.

## Alternatives rejetées

| Option | Pourquoi pas |
|---|---|
| Queue always-on sur tous les `test.toml` | Coût Docker + `#[serial]` sur toute la suite HTTP |
| Un seul transport Kafka **ou** SQS imposé à tous | Wiki encore `[ambiguous]` ; Kafka et SQS coexistent déjà |
| NATS comme harness | Aucune fixture rustycog-testing / service |

## Non décidé ici

- Unifier Kafka vs SQS par service/environnement (wiki Open Questions).
- Profondeur d’assert events (producteur SQS vs effets consommateur Telegraph).
- Remplacer les sleeps/polling Telegraph par un signal d’achèvement du harness.
- Aligner Telegraph sur l’opt-in producteur (`enabled = false` + descripteur SQS dédié) ou figer l’exception consommateur dans une ADR ultérieure.

## Références

- Handbook : `docs/guides/tests-integration.md` (section Transport)
- Wiki : `skills/using-rustycog-testing`, `concepts/integration-testing-with-real-infrastructure` (Open Questions L65–69)
- Code : `IAMRusty/Hive/Manifesto/config/test.toml` (`enabled = false`) ; `Telegraph/config/test.toml` (`enabled = true`) ; `sentinel-sync/config/test.toml` (`type = "disabled"`) ; descripteurs SQS dans `*/tests/sqs_event_routing_tests.rs` ; `rustycog/rustycog-testing/src/common/sqs_testcontainer.rs`, `kafka_testcontainer.rs`
- Preuve Partial : producteurs opt-in attestés ; Telegraph suite-level SQS+SMTP+MailHog 1025 ; Kafka optionnel IAM ; pas de NATS
