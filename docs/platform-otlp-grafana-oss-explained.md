# OTLP et « Grafana OSS in-cluster » — explication (pédagogie)

> **Pédagogie.** Ce document n’est **pas** une ADR. Canon V1 = [ADR-0602](adr/0602-observabilite-portable-otlp-lgtm.md) : **plan A** = tracing métier + bootstrap OTLP rustycog (apps **sans** collector OK) ; **plan B** = Grafana OSS LGTM / Tempo **derrière** collector ou Alloy, **pas** dans le process Rust. Câble portable : [ADR-0600](adr/0600-cloud-portable-opentofu-k8s-gitops.md). Topologie ns : [ADR-0601](adr/0601-cluster-trust-namespaces-standalones.md) (`aiforall-obs`).

Observabilité actuelle du dépôt : `rustycog::logger::setup_logging` (stdout + option `scaleway-loki`) ; **zéro** SDK OpenTelemetry ; `x-correlation-id` ≠ `traceparent`. `user_id` est déjà un champ du span HTTP (tension point 10 inféré — 0602).

## Qu’est-ce que OTLP ?

**OTLP** = *OpenTelemetry Protocol*. C’est le **câble**, pas le tableau de bord.

| Signal | En français | V1 rustycog |
|---|---|---|
| **Traces** | Fil d’une requête IAM → Lazaret | **Oui** (spans `tracing` → OTel → OTLP) |
| **Logs** | Lignes `tracing` structurées | Console / champs ; signal OTel Logs = later |
| **Metrics** | Compteurs / jauges | Later (Meter dans rustycog-http ; **pas** bridge `metrics` MSRV 1.85) |

```
Apps Rust  →  SDK OTel (rustycog only)  →  Collector ou Alloy  →  Backend (adaptateur)
              « comment on parle »         « le tri postal »        « où on range / affiche »
```

- **SDK** : dans rustycog (`setup_logging`), jamais dans `domain/` / `application/`.
- **Collector** ou **Grafana Alloy** (distribution collector) : ports **4317** / **4318**. Alloy n’est **pas** un crate Rust.
- **Backend** : Tempo / Loki / Prometheus / plus tard autre — **changeable sans** recompiler le métier.

OTLP n’est **pas** Grafana, **pas** Datadog, **pas** Cockpit.

## Plan A vs plan B (0602)

| Plan | Où | Pour quoi |
|---|---|---|
| **A — câble** | Process Rust (feature rustycog `otel`) | Émettre des traces W3C ; **survivre** sans collector |
| **B — salon** | Cluster ns `aiforall-obs` | *Voir* le voyage (Tempo obligatoire **côté plateforme**) |

« Collector-only interdit comme cible V1 » = on n’a pas de salon pour relier N sauts. Ça **n’oblige pas** Grafana dans le SDK.

## « Grafana OSS in-cluster » (plan B)

Stack **LGTM** dans Kubernetes (staging / prod) :

| Lettre | Produit | Rôle |
|---|---|---|
| **L** | Loki | Logs |
| **G** | Grafana | UI |
| **T** | Tempo | Traces |
| **M** | Prometheus V1 (Mimir later) | Metrics |

Collector **ou** Alloy distribue : traces → Tempo, logs → Loki, metrics → Prometheus.

Kind (couche B) : LGTM complet est lourd ; minimum = collector/Alloy + Tempo + Grafana. Image démo `grafana/otel-lgtm` = laptop DX, **pas** modèle prod.

Ce n’est **pas** Grafana Cloud.

RAM indicative staging LGTM complet : **≈ 2–4 Gi**.

## Ensemble crates (ne pas coller latest 0.33)

Au 2026-09-20, `tracing-opentelemetry` **0.33.0** pin `opentelemetry` **0.32**. OTel crates.io **0.33** est **incompatible** avec ce bridge. Détail canon : ADR-0602.

## Alternatives (rappel)

Façade CloudWatch/Cockpit, Datadog cœur, `traceparent` maison, OTel dans le métier, LGTM dans Rust, pin 0.33 aveugle : **rejetées** (0602).

`scaleway-loki` = adaptateur logs optionnel, pas le fil traces.

## Sources

- Canon : [ADR-0602](adr/0602-observabilite-portable-otlp-lgtm.md)
- [Spécification OTLP](https://opentelemetry.io/docs/specs/otlp/), [Collector](https://opentelemetry.io/docs/collector/), [Alloy](https://grafana.com/docs/alloy/latest/), [W3C Trace Context](https://www.w3.org/TR/trace-context-1/)
