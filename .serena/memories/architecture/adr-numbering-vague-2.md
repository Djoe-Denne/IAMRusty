# ADR rétroactives Vague 2 (2026-09-12)

Schéma de numérotation `docs/adr/` :
- 0001–0099 Apparatus (existant 0001–0005, ne pas réécrire)
- 0100–0199 hexagone RustyCog / crates
- 0200–0299 tests
- 0300–0399 events / outbox / AuthN-AuthZ
- 0400–0499 services / runtimes
- 0500–0599 config / CI / rustycog framework

Vague 2 = photographie du dépôt actuel. Statut Accepted. Réalité Partial là où le code diverge (0202 Telegraph queues, 0301 outbox IAM/Telegraph, 0406 P0 vs host, 0501 coverage Sonar < 80%).

NATS n’est pas un transport du dépôt. `apparatus-events` existe sur disque hors workspace.members.

Agents réutilisables : `.cursor/agents/adr-*.md` (+ miroir `.codex/agents/adr-*.toml`).
