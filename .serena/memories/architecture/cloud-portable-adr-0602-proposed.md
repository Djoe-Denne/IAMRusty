Jalon : Cloud-portable (hors Apparatus P0–P6).
Chemin : `docs/adr/0602-observabilite-portable-otlp-lgtm.md`.
Statut : Proposed. Réalité : Unimplemented.
- Deux plans : A = tracing métier + bootstrap OTLP rustycog (`setup_logging` singleton, feature `otel`, W3C TraceContextPropagator) ; apps doivent tourner sans collector.
- B = LGTM/Tempo derrière collector ou Alloy (pas dans Rust). Collector-only interdit comme cible plateforme (œil E2E) ≠ Grafana-dans-le-SDK.
- Ensemble V1 vérifié 2026-09-20 : OTel 0.32 + tracing-opentelemetry 0.33 ; pas pin 0.33 aveugle. Bridge metrics pas V1 (MSRV 1.85 vs 1.84).
- Point 10 inféré (pas secrets/jetons/PII dans traces/logs) ; 0003/0007 non réécrits ; tension `user_id` HTTP later.
- Ferme Q3 0600 sans SuperSéder 0600. Filename 0602 conservé.
Voir le fichier ADR.
