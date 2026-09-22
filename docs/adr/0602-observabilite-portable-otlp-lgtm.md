# ADR-0602 : L’observabilité V1 sépare le câble (tracing métier + bootstrap OTLP rustycog) du backend cluster (LGTM/Tempo derrière collector/Alloy)

- Statut : Proposed
- Réalité : Unimplemented
- Date : 2026-09-20
- Décideurs : (à remplir à l’acceptation)
- Jalon concerné : Cloud-portable (hors Apparatus P0–P6)
- SuperSède : aucune — **complète** le Non décidé Q3 d’[0600](0600-cloud-portable-opentofu-k8s-gitops.md) **sans** SuperSéder 0600
- SuperSédée par : —

`Accepted` ratifie une cible. `Réalité` décrit le dépôt. Aujourd’hui : `rustycog::logger::setup_logging` (fmt stdout + option `scaleway-loki` → Cockpit, `try_init`, **zéro** OTel) ; `LoggingConfig` = level / filter / console / file / scaleway_loki (`console`/`file` **non lus**) ; HTTP `tracing_middleware` span `http_request` + `x-correlation-id` (**pas** W3C) ; **zéro** crate `opentelemetry` dans les slices ; **zéro** collector / Tempo / Grafana dans Compose. rustycog = crate unique `rustycog-framework` (features `logger` / `http` / `scaleway-loki`). Cette ADR **ne SuperSède pas** [0008](0008-apparatus-p4-k8s-isolation-outside-manifesto.md), [0500](0500-config-typee-et-compose-local.md), [0502](0502-rustycog-framework-feature-gated.md) ni 0600/0601. **On ne réécrit pas** 0502 (feature `otel` = même *pattern* feature-gated, nom indicatif).

**Interdit** : rédiger ceci comme ADR-0009 / 0411 / 0503 / 0603. **Ne pas** toucher 0008.

## Contexte

[0600](0600-cloud-portable-opentofu-k8s-gitops.md) fige le câble portable (collector OTLP ; backends = adaptateurs). Q3 (premier backend staging Grafana OSS vs collector-only) est **fermée ici**. Tentations : façade maison CloudWatch / Cockpit ; Datadog V1 ; Prometheus+Grafana **sans** Tempo ; collector-only comme *cible plateforme* (« le câble suffit pour *voir* ») ; Grafana / Tempo **dans** le process Rust ; `scaleway-loki` comme fil traces ; OTel dans `domain/` / `application/` ; pin crates.io **0.33** aveugle ; façade `init-tracing-opentelemetry`.

Un appel utilisateur IAM → Hive → Manifesto → Telegraph → Lazaret n’est **un** voyage que si chaque hop HTTP **extrait et réinjecte** `traceparent`. Sans ça, un backend traces voit N traces disjointes.

**Épreuve de la proposition utilisateur (pas canon jusqu’ici).** Schéma Rust tracing → OTLP → collector → backends **sans** changer le métier : **retenu**. Liste de crates latest collée : **rejetée** (voir ensemble compatible). Bridge `metrics`→OTel : **pas V1** (immaturité + MSRV). Point 10 (message coupé) : **inféré** ci-dessous ; 0003/0007 **non réécrits**.

## Décision

Deux plans **orthogonaux**. Le SDK Rust n’embarque pas Grafana. LGTM n’est pas un crate.

### Plan A — câble + bootstrap rustycog (ferme Q3 **côté apps**)

1. Métier = **`tracing`** (spans + logs structurés = champs, pas des blobs). Pas d’imports vendor (`datadog`, `prometheus` client, `tracing-loki` hors feature existante) dans `domain/` / `application/` ni dans les slices HTTP métier.
2. OTel / OTLP / exporters = **bootstrap rustycog uniquement** (`setup_logging` reste le **singleton** — layers OTel ajoutées là, **pas** un second `tracing_subscriber::init`). Feature rustycog `otel` (nom indicatif, alignée [0502](0502-rustycog-framework-feature-gated.md) ; **0502 non réécrite** tant que la feature n’est pas livrée).
3. Apps → OTLP vers un collector (ports **4317** gRPC / **4318** HTTP). Jamais directement Tempo, Loki, Prometheus, CloudWatch, Cockpit, Datadog.
4. Propagation **W3C Trace Context** : `opentelemetry_sdk::propagation::TraceContextPropagator` (extract serveur axum + inject client reqwest/tower). **Pas** de parser `traceparent` maison. `x-correlation-id` **peut coexister** ; **il n’est plus** la clé Tempo.
5. Config **sans recompile** : variables **OTEL_*** (`OTEL_EXPORTER_OTLP_ENDPOINT` obligatoire pour *émettre* ; le reste = spec OTel). `HasLoggingConfig` / `LoggingConfig` restent level / filter. **Ne pas** dupliquer l’endpoint dans chaque TOML de slice. `LoggingConfig.console` / `file` non lus = dette hors ce tour.
6. **Marche sans collector.** Absence d’endpoint / exporter down / collector down = **console continue**, **pas de panic** (même motif que `scaleway-loki` aujourd’hui : warn + continue, `try_init`).
7. `tokio::spawn` **garde le span** aux sites rustycog qui spawnnent du travail lié à une requête (`Span::current().in_current_span()` / `Instrument`) — pas un monkey-patch global de Tokio.
8. Métriques **utiles seulement**, **pas** de labels haute cardinalité (user id, path brut non borné, token). Bridge crate `metrics`→OTel **pas canon V1**. Métriques HTTP later = **Meter OTel dans `rustycog-http`**, pas dans le métier.
9. `scaleway-loki` reste **adaptateur logs optionnel** (Cockpit), pas le fil traces.

### Plan B — premier adaptateur *cluster* (pas dans Rust)

Staging + prod remote : **Loki + Grafana + Tempo + Prometheus V1** dans le cluster, derrière collector. **Tempo n’est pas optionnel pour *voir* le voyage.** Mimir = later HA. Grafana Cloud **pas** V1.

**Alloy** = distribution collector Grafana (infra), **pas** un SDK Rust. Côté cluster : Collector contrib **ou** Alloy ; les process Rust parlent **OTLP**.

Namespace : **`aiforall-obs`** ([0601](0601-cluster-trust-namespaces-standalones.md) — déjà listé). Pas `aiforall-platform` / `aiforall-data`. NetworkPolicy : `aiforall-platform` + `aiforall-gateway` + `sentinel-sync` **MAY** OTLP ; **plugins deny** ; UI Grafana via **Gateway**.

« **Collector-only interdit comme cible plateforme V1** » = on ne *voit* pas l’E2E sans backend traces. **Ça n’oblige pas Grafana/Tempo dans le SDK.** Les apps du plan A **doivent** tourner sans collector.

### Couches (0600 inchangé : A/B/C, Compose défaut = 0500)

| Couche | Observabilité V1 |
|---|---|
| **A — laptop-compose** | Profil Compose **`obs`** (opt-in). **`docker compose up` défaut inchangé** — on ne réécrit pas 0500. Preuve fil E2E **DX seulement** : image `grafana/otel-lgtm` *documentée*, **ou** collector+Tempo+Grafana. Pas un modèle prod. |
| **B — kind** | Collector **ou** Alloy + **Tempo** + Grafana **minimum**. Loki / Prometheus = overlay si RAM. **Le fil traces DOIT exister en B** (plateforme). |
| **C — remote k8s** | LGTM **complet** (Loki + Grafana + Tempo + Prometheus V1). |

### Point 10 — inféré (0003 / 0007 non réécrits)

**Formulation inférée :** les traces et logs **ne doivent pas** contenir de secrets, mots de passe, jetons (JWT, HMAC, session Lazaret, Transit), connection strings, ni PII nominative non nécessaire au debug.

**Alignement (esprit, pas une clause existante) :** [0003](0003-apparatus-untrusted-plugin.md) isole les secrets des plugins ; [0007](0007-apparatus-p3-capability-boundary-after-accept.md) impose secrets-by-ref / pas de clair dans le KV plugin. **Aucune** des deux n’énonce aujourd’hui une règle logs/traces. **Ne pas** les réécrire.

**Tension (later / redaction, pas un rewrite V1) :** `tracing_middleware` pose déjà `user_id` dans le span `http_request`. Politique de redaction = later.

### Ensemble crates compatible (vérifié 2026-09-20) — pas un pin magique « latest »

`tracing-opentelemetry` **0.33.0** (GitHub `v0.33.0` / crates.io) déclare `opentelemetry = "0.32.0"` et `tracing-subscriber = "0.3.22"` → semver **`< 0.33`** pour OTel. `cargo search` 0.33.0 pour `opentelemetry` / `_sdk` / `-otlp` est **incompatible** avec ce bridge spans. **Interdit** de coller OTel 0.33 aveugle.

| Crate | Ensemble V1 | Rôle |
|---|---|---|
| `tracing` | `0.1` (dépôt 0.1.41 ; latest 0.1.44 OK en caret) | Spans / events métier |
| `tracing-subscriber` | `0.3` (**≥ 0.3.22** exigé par le bridge ; rustycog déjà `0.3`) | Subscriber / fmt |
| `opentelemetry` | **0.32.0** | API OTel |
| `opentelemetry_sdk` | **0.32.0** | SDK + `TraceContextPropagator` |
| `opentelemetry-otlp` | **0.32.0** | Exporter collector ; MSRV crate 1.75 |
| `tracing-opentelemetry` | **0.33.0** | Bridge **spans** tracing → OTel |

`opentelemetry-otlp` : **`default-features = false`**. Defaults amont = `reqwest-blocking-client` + metrics + logs — **interdit** sur le runtime tokio. V1 = feature **`trace`** + un transport async (`http-proto` + client reqwest **non blocking**, ou `grpc-tonic`). Protocole via **OTEL_***.

**Reporté (pas V1 canon) :**

| Crate | Pourquoi |
|---|---|
| `opentelemetry` / `_sdk` / `-otlp` **0.33.0** | Latest crates.io ; **pas** de `tracing-opentelemetry` publié pour `^0.33` au 2026-09-20 |
| `opentelemetry-appender-tracing` **0.33.0** | Ligne OTel 0.33 ; c’est **OTel Logs**, pas des traces. **0.32.0** = même ligne que l’ensemble, mais signal logs OTel **later** (V1 = tracing structuré + spans) |
| `metrics` 0.24.x + `metrics-opentelemetry` **0.24033.1** | `cargo info` : **rust-version 1.85** vs MSRV dépôt **1.84** |
| `metrics-exporter-opentelemetry` 0.2.1 | rust-version **1.85** |
| `metrics-exporter-otel` 0.3.1 | MSRV 1.75 mais **pas** aligné sur l’ensemble 0.32 ; 1 ★ / surface immature |
| `init-tracing-opentelemetry` | Façade ; rustycog a déjà le singleton |

Bump OTel 0.33 **uniquement** quand `tracing-opentelemetry` (ou successeur) déclare `^0.33` — **ensemble**, pas pièce à pièce.

### Instrumentation V1 vs later

| In V1 (plan A, rustycog) | Later |
|---|---|
| HTTP server extract + HTTP client inject W3C | Spans SQL / métier ; queues ; sentinel-sync ; plugins |
| Slices HTTP via le **même** middleware (IAM, Hive, Manifesto, Telegraph, Lazaret) | Plugins → **invoke seulement** ; **pas** d’OTLP plugin → obs |
| Monolithe : **même** middleware | Unification `setup_logging` worker/monolithe (0502 Non décidé) |
| Sampling A/B **100 %** ; staging **100 %** jusqu’à saturation ; prod **head ~10 %** parent-based | Tail sampling |
| Logs structurés (champs tracing) + console | OTel Logs (`opentelemetry-appender-tracing` 0.32) ; json fmt si on câble `LoggingConfig.console` |
| | Meter OTel HTTP dans rustycog-http ; bridge `metrics` si MSRV + crate mûr |

Rétention cluster (plan B, ordre de grandeur) : staging traces 24–72 h / logs ~7 j / metrics ~15 j ; prod traces 7–14 j / logs ~30 j / metrics 30–90 j. SLA chiffrés = later.

### Interdit V1 (non négociable)

| Interdit | Pourquoi |
|---|---|
| Façade maison devant CloudWatch / Cloud Monitoring / Cockpit / APM natif | Lock-in ; 0600 backends = adaptateurs derrière OTLP |
| Datadog V1 (cœur) | Cœur SaaS |
| Prometheus+Grafana **sans** Tempo (cible **plateforme**) | On ne *voit* pas le voyage |
| Collector-only comme **cible plateforme** V1 | Insuffisant pour relier N sauts **à l’œil** — ≠ « apps sans collector » |
| Grafana / Tempo / Alloy **dans** le process Rust | Plan B = infra |
| Grafana Cloud V1 | SaaS |
| Jaeger seul / SigNoz cœur | LGTM couvre Tempo ; moins Lego |
| `scaleway-loki` comme design traces | Adaptateur logs optionnel |
| Port OTel / crates `opentelemetry*` dans chaque slice / handler métier | Hexagone 0100 / 0502 |
| OTLP depuis les plugins | 0003 / 0008 : untrusted |
| Réimplémenter `traceparent` | Propagateur OTel |
| Pin OTel **0.33** tant que le bridge spans reste sur **0.32** | Ensemble incompatible |
| `init-tracing-opentelemetry` | Façade |
| Bridge `metrics-*` MSRV 1.85 | MSRV 1.84 |
| Réécrire Compose défaut / 0500 | Profil `obs` à part |
| Scaffolder `cloud/` `deploy/obs/` / Grafana **dans le même work** que le bootstrap logger | Works séparés |

## Conséquences

- Q3 0600 est **fermée ici** (plan A = apps ; plan B = premier adaptateur cluster). 0600 **n’est pas** SuperSédée.
- Loki LGTM ≠ `scaleway-loki`. Le premier est un **leg** collector ; le second reste un adaptateur logs applicatif.
- 0601 : ns `aiforall-obs` déjà nommé. Unité **4+1 Deployments non rouverte**.
- 0500 / Compose défaut **intacts**.
- 0502 photographie **non réécrite**.
- 0003 / 0007 **non réécrits** ; point 10 **inféré**.
- 0008 **intacte**. Pas d’OTLP depuis `aiforall-plugins`.
- 0407–0410 intactes. Prochain libre cloud : **0603+**.
- Durable SDK : checkout `rustycog/` (submodule) pour prototyper ; publication = repo **sibling** rustycog + bump gitlink **plus tard**. Pas de commit ce tour.

### Contrat implementer — plan A seulement (après Accept humain)

**In scope :** feature `otel` sur `rustycog-framework` ; layers dans `setup_logging` ; extract/inject `traceparent` dans `rustycog-http` ; ensemble de crates ci-dessus ; tests unitaires rustycog.

**Hors scope :** Grafana, Tempo, Alloy, `cloud/`, `deploy/obs/`, profil Compose `obs`, crates `opentelemetry*` dans IAM/Hive/Manifesto/Telegraph/Lazaret/`domain`/`application`, édition 0008 / 0100–0502 / 0003 / 0007, bump gitlink rustycog, commit.

**Tests (pas `cargo test --workspace`) :**

- Feature `otel` **off** : comportement actuel (fmt + `try_init`).
- Feature `otel` **on**, pas d’endpoint / exporter disabled : console OK, **pas de panic**.
- Exporter / collector down : process **ne panique pas**.
- HTTP : extract + inject header `traceparent` (round-trip W3C).

unsafe interdite. MSRV **1.84**.

## Alternatives rejetées

| Option | Pourquoi pas (maintenant) |
|---|---|
| Collector-only comme cible **plateforme** V1 | Câble sans salon ; Tempo absent = pas de voyage *visible* |
| LGTM / Tempo / Alloy dans le SDK Rust | Plan B = infra ; apps doivent vivre sans collector |
| Prometheus + Grafana sans Tempo | Metrics sans fil inter-services |
| Grafana Cloud / Datadog / New Relic V1 | SaaS cœur |
| CloudWatch / Cockpit / APM natif cœur | Lié au provider (0600) |
| Façade maison / `init-tracing-opentelemetry` | Double câble ; lock-in ; second init |
| Jaeger seul / SigNoz cœur | Moins remplaçable que collector + LGTM |
| `scaleway-loki` = traces | Logs Cockpit ; pas W3C |
| OTel / `opentelemetry*` dans le métier | Casse 0100 / 0502 |
| OTLP depuis les plugins | 0003 / 0008 |
| Réimplémenter `traceparent` | Propagateur OTel |
| Pin OTel 0.33 aveugle | Incompatible `tracing-opentelemetry` 0.33.0 |
| Bridge `metrics` V1 | MSRV 1.85 et/ou pin OTel étranger |
| Réécrire Compose défaut / 0500 | 0600 : A = DX + IT |
| Feature OTel dans chaque slice Cargo.toml | Accroche = rustycog |
| Créer 0603 pour ce sujet | 0602 existe ; filename conservé |

## Non décidé ici

- Produit Helm LGTM « tout-en-un » vs charts séparés. **Contrainte** : Tempo reçoit OTLP **via collector/Alloy** (pas d’apps → Tempo).
- gRPC 4317 vs HTTP 4318 tant que `OTEL_*` standard marche et que le client n’est pas blocking.
- Date du bump ensemble → OTel 0.33 (quand le bridge spans suit).
- Redaction `user_id` / politique PII au-delà du point 10 inféré.
- Unification logging sentinel-sync / monolithe (0502).
- Signal OTel Logs et Meter HTTP rustycog.

## Références

- Wiki : `obsidian/AI FOR ALL/projects/aiforall/decisions/0602-observabilite-portable.md` ; 0600 / 0601 mêmes dossiers
- Canon voisin : [0600](0600-cloud-portable-opentofu-k8s-gitops.md), [0601](0601-cluster-trust-namespaces-standalones.md), [0502](0502-rustycog-framework-feature-gated.md), [0500](0500-config-typee-et-compose-local.md), [0100](0100-services-metier-hexagonaux-rustycog.md)
- Dépôt (citer, pas coder) : `rustycog/rustycog-logger/src/lib.rs` (`setup_logging`, `try_init`, feature `scaleway-loki`) ; `rustycog/rustycog-config/src/lib.rs` (`LoggingConfig`, `HasLoggingConfig`) ; `rustycog/Cargo.toml` (`rust-version = "1.84"`, features `http` / `logger` / `scaleway-loki`) ; `rustycog/rustycog-http/src/builder.rs` + `tracing_middleware.rs` (`x-correlation-id`, `user_id` dans le span, **pas** `traceparent`)
- Contrat : `docs/platform-cloud-v1-implementation-contract.md`
- Pédagogie (pas canon) : `docs/platform-otlp-grafana-oss-explained.md`
- Crates (2026-09-20) : `cargo search` + `cargo info` + GitHub `tokio-rs/tracing-opentelemetry` `v0.33.0` Cargo.toml (`opentelemetry = "0.32.0"`) ; `metrics-opentelemetry` rust-version **1.85**
- Web : [W3C Trace Context](https://www.w3.org/TR/trace-context-1/), [OTLP spec](https://opentelemetry.io/docs/specs/otlp/), [OTel Collector](https://opentelemetry.io/docs/collector/), [Grafana Alloy](https://grafana.com/docs/alloy/latest/)
- Preuve d’implémentation : `aucune`
