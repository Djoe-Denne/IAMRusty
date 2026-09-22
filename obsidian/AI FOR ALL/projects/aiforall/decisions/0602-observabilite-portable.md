---
title: "ADR-0602 — observabilité portable OTLP / LGTM (Proposed)"
category: decisions
tags: [architecture, platform, observability, visibility/internal]
status: proposed
feature_status: unimplemented
sources:
  - docs/adr/0602-observabilite-portable-otlp-lgtm.md
  - docs/platform-cloud-v1-implementation-contract.md
  - docs/platform-otlp-grafana-oss-explained.md
summary: >-
  Canon : docs/adr/0602. Plan A = tracing métier + bootstrap OTLP rustycog
  (W3C traceparent, marche sans collector). Plan B = LGTM/Tempo derrière
  collector ou Alloy, pas dans Rust. Q3 0600 fermée sans SuperSéder 0600.
  Ensemble crates OTel 0.32 + tracing-opentelemetry 0.33. Point 10 inféré.
created: 2026-09-20T17:00:00Z
updated: 2026-09-20T17:00:00Z
provenance:
  extracted: 0.90
  inferred: 0.08
  ambiguous: 0.02
---

# ADR-0602 — observabilité portable

Canon : `docs/adr/0602-observabilite-portable-otlp-lgtm.md`. Hub : [[projects/aiforall/decisions/index]]. Plateforme : [[projects/aiforall/decisions/0600-cloud-portable]]. Topologie : [[projects/aiforall/decisions/0601-cluster-topology]]. Pédagogie (pas canon) : `docs/platform-otlp-grafana-oss-explained.md`.

## Statut : Proposed (2026-09-20)

Réalité **Unimplemented**. Ne pas présenter comme Accepted. **Complète** Q3 de 0600 **sans** SuperSéder 0600. Ne SuperSède **pas** 0008 / 0500 / 0502.

## Paquet figé (cible)

1. **Plan A** — câble apps : `tracing` dans le métier ; OTel/OTLP uniquement dans rustycog (`setup_logging` singleton, feature `otel` indicative). W3C via `TraceContextPropagator`. Apps **doivent** tourner sans collector.
2. **Plan B** — premier adaptateur cluster : LGTM + Tempo derrière collector **ou** Alloy (pas un SDK). Collector-only interdit comme *cible plateforme* (on ne *voit* pas l'E2E) ≠ Grafana dans Rust.
3. Ensemble compatible **OTel 0.32 + tracing-opentelemetry 0.33** ; pas pin 0.33 aveugle. Bridge `metrics` pas V1 (MSRV 1.85).
4. Point 10 **inféré** (pas de secrets/jetons/PII dans traces/logs) ; 0003/0007 non réécrits ; tension `user_id` dans spans HTTP = later.

Related : [[projects/aiforall/decisions/0500-plateforme-qualite]].
