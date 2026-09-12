# ADR-0100 : Les 4 services métier sont des vertical slices hexagonales RustyCog

- Statut : Accepted
- Réalité : Implemented
- Date : 2026-09-12
- Décideurs : Architecture AIForAll — photographie rétroactive du dépôt
- Jalon concerné : architecture actuelle (hors P-Apparatus)
- SuperSède : aucune
- SuperSédée par : —

`Accepted` ratifie une cible. Le champ `Réalité` indique séparément ce que le code du dépôt réalise.

## Contexte

Le monorepo mélange des processus de natures différentes (IdP, organisations, projets, notifications, worker FGA, runtime monolithe, crate health). La tentation est de les traiter tous comme « un service RustyCog », ou de scaffolder un nouveau métier hors du gabarit déjà câblé.

Le handbook Manifesto et le wiki de cohérence (août 2026) décrivent déjà un socle unique pour **quatre** vertical slices.

## Décision

1. **IAMRusty**, **Hive**, **Telegraph** et **Manifesto** sont les seuls **services métier** : chacun est une vertical slice hexagonale RustyCog (ports domaine, adapters infra/HTTP, composition root `setup`).
2. **Golden path scaffold** = **Manifesto** (guides `Manifesto/docs/rustycog-*.md`, `docs/guides/nouveau-service.md`).
3. **Golden path IdP** = **IAMRusty** (OAuth, tokens, JWT émetteur ; pas un PDP OpenFGA).
4. **`sentinel-sync`**, **`monolith` / `oodhive-monolith`**, **`readiness`** ne sont **pas** des vertical slices — voir 0303, 0404, 0405.
5. Les crates Apparatus P0 (`apparatus-contracts`, KV de référence) ne sont pas un service hexagonal — voir 0001–0005 et 0406.

Les divergences JWT, mapping d’erreurs, fidélité OpenAPI Hive et les quatre câblages OpenFGA **restent** ; elles ne cassent pas cette décision.

## Conséquences

- Un nouveau métier clone la slice Manifesto, pas sentinel-sync ni le monolithe.
- IAMRusty reste l’autorité d’identité ; les trois autres sont consommateurs JWT rustycog-http.
- Préfixes runtime : `/manifesto`, `/telegraph`, `/iam`, `/hive`.

## Alternatives rejetées

| Option | Pourquoi pas |
|---|---|
| Un monolithe métier unique | Le dual runtime existe déjà (0404) ; le métier reste des slices séparées |
| Hexagone aussi pour sentinel-sync | Worker événements → tuples, pas un bounded context HTTP |
| Scaffold depuis IAMRusty pour un CRUD | IAM est l’IdP ; le gabarit HTTP/OpenFGA est Manifesto |

## Non décidé ici

- Unification JWT HS256 consommateur vs RS256 IAM (0302).
- Unification des 4 stratégies OpenFGA.
- Logging : wiki encore « Manifesto hand-rolled » ; le code actuel réexporte `rustycog::logger::setup_logging` dans les 4 `configuration`.
- NATS, CAS global, ACL générique, host Apparatus.

## Références

- Wiki : `obsidian/AI FOR ALL/concepts/architecture-coherence-across-services.md`, `projects/iamrusty/concepts/hexagonal-architecture.md`, `skills/building-rustycog-services.md`
- Handbook : `Manifesto/docs/rustycog-hexagonal-web-service-guide.md`, `docs/guides/nouveau-service.md`, `docs/reviews/iam-architecture-comparison.md`
- Code : `IAMRusty/`, `Hive/`, `Telegraph/`, `Manifesto/` (crates de couche + `src/main.rs`) ; hors slice : `sentinel-sync/`, `monolith/`, `readiness/`
- Preuve : les 4 ont `setup/src/app.rs`, `http/src/lib.rs` (`RouteBuilder`, `SERVICE_PREFIX`), `tests/` ; layout conforme revue 2026-08-29
