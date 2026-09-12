# ADR-0002 : Les contrats versionnés et un Apparatus de référence précèdent Factory et host

- Statut : Accepted
- Réalité : Partial
- Date : 2026-09-10
- Décideurs : Architecture AIForAll — ratification orchestrée du 2026-09-10
- Jalon concerné : P0
- SuperSède : aucune
- SuperSédée par : —

## Contexte

La plateforme cible (Factory Git→OCI, contrôleur, host UI, SDK iframe) n’existe pas. Un `apparatus.toml` conceptuel et des exemples YAML/TOML divergents ne sont reconnus par aucun code. La Factory et le host figeraient le premier schéma qu’on leur donne. La macro `#[manifesto::apparatus]` et la CLI `check/dev/publish` n’existent pas.

`APP-04` demande un premier Apparatus métier (Git, wiki, …) : en généraliser l’API réseau trop tôt produirait le mauvais contrat.

## Décision

1. **P0 avant P4/P5.** On écrit le schéma canonique `apparatus.toml` (version de schéma distincte du SDK et du protocole wire), les DTO release / binding / opération, l’API de capacités et le protocole backend privé (`/.well-known/apparatus`, health/ready, bind, configure, invoke, unbind).
2. **Identité de release** : SemVer dans le manifeste ; identité d’installation = digest du descripteur canonique. L’admission de ce digest est une décision distincte (ADR-0005). `latest`, une branche ou un tag flottant sont des erreurs de validation, pas des identités.
3. **Une seule syntaxe de manifeste** : TOML canonique. Les DTO HTTP ne constituent pas un manifeste YAML parallèle.
4. **Un seul validateur** est spécifié en P0. La Factory et la CLI devront le réutiliser lorsqu’elles seront créées ; la macro et l’ergonomie CLI viennent **après** le contrat wire.
5. **Apparatus de référence KV** : backend Rust, stockage `kv-v1`, UI `schema` (pas iframe), **aucune** capacité réseau. Il sert à qualifier les contrats, pas le métier Git/wiki. Son `apparatus_id` est un choix d’implémentation, pas une décision de cette ADR.

Bornes de contrat V1 : le manifeste peut déclarer `absent` | `schema` | `sandbox` statique (Vite). Cela ne valide aucune isolation UI de production ; origine, CSP et bridge seront décidés avant P5. Pas de Node/SSR par Apparatus. Le mode `component` (JS dans le host) est hors V1. L’identifiant reste compatible avec la limite actuelle de 100 caractères de `component_type`.

Les noms de crates (`apparatus-contract`, …) sont une conséquence d’implémentation, pas cette décision.

## Conséquences

- Preuve P0 : manifeste accepté/refusé de façon déterministe par la bibliothèque de validation ; DTO sans credentials ; bind/unbind idempotents dans un harness in-process.
- Manifesto (SQL, routes) n’est pas modifié en P0 — seulement consommé comme ancre d’identité (ADR-0001).
- Un Apparatus « Git » n’est pas le premier livrable code.
- Le harness et `apparatus dev` sont des doubles de test : ils ne produisent ni `VALID`, ni `VERIFIED`, et ne prouvent aucune isolation de production.
- La Factory (P4), la CLI (P5) et le host (P5) **doivent** réutiliser les contrats P0 ; ils n’en définissent pas un second.

## Alternatives rejetées

| Option | Pourquoi pas |
|---|---|
| Factory d’abord | Fige un schéma instable ; dépend de `APP-01` (registry, isolation, signatures) |
| SDK host / iframe d’abord | Pas de frontend Manifesto ; le SDK n’a pas d’autorité sans protocole serveur |
| Double syntaxe TOML+YAML | Deux validateurs, divergence garantie |
| Premier Apparatus = Git ou wiki | Généralise réseau et données métier avant le contrat (`APP-04`) |
| `latest` comme identité d’install | Non reproductible |

## Non décidé ici

- Contenu exact de chaque champ TOML au-delà du contrat cible wiki (itérable tant que le validateur reste unique).
- Commandes CLI et proc-macro.
- Pipeline OCI (P4).
- Origine, sandbox, CSP et bridge du host UI (avant P5).

## Références

- Wiki : `apparatus-implementation-plan` (P0), `apparatus-factory-and-distribution`, `apparatus-ui-and-protocol`, `apparatus-platform`
- Code P0 : validateur unique dans `apparatus-contracts` (`validation.rs`, TOML canonique) ; catalogue legacy = `ComponentServicePort` (inchangé)
- Preuve d’implémentation (2026-09-10, tests élargis 2026-09-11, micro P0.1) : crates `apparatus-contracts` (schéma `apparatus.toml`, validateur unique, digest sha256 canonique BTreeMap+serde_json, protocole wire `manifesto-apparatus/1`, port `KvStore`, harness TEST-ONLY feature-gaté `test-harness`) et `apparatus-reference-kv` (`apparatus_id` `io.aiforall.reference-kv`, UI `schema`, stockage `kv-v1`). Tests déterministes `contracts_p0.rs` (27) + `kv_p0.rs` (10) = 37/37 socle, + `apparatus_p01_micro.rs` (3+2) = 42/42 total avec `cargo test -p apparatus-contracts --features test-harness` et `cargo test -p apparatus-reference-kv`. Gates verts : `cargo fmt --check`, `cargo check`, `cargo test`, `cargo clippy`, `cargo doc`, `cargo metadata`.
