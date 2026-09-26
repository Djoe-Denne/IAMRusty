# ADR-0604 — overlay démo J3 monolithe kind

- Jalon : Cloud-portable (écart de séquence, pas SuperSéde 0601)
- Canon : `docs/adr/0604-j3-overlay-demo-monolith-kind-invoke.md`
- Statut : Proposed | Réalité : Implemented (`just deploy-j3`)
- SuperSède : aucune (ne SuperSède pas 0601/0404/0600/0603/0008)
- Overlay : `deploy/apps/overlays/kind-demo-monolith/` (distinct de `overlays/kind` M2 nginx)
- Image : `aiforall-oodhive-monolith:j3` ; ns `aiforall-gateway` ; Service DNS `lazaret.aiforall-gateway.svc.cluster.local:8080`
- Preuve : Job `invoke-probe-j3` dans `aiforall-plugins` POST `/lazaret/invoke` → HTTP 401 `{"error":"unauthorized"}`
- Infra : Compose hôte via IP `host.docker.internal` (Docker Desktop) ; pas de Postgres in-kind
- **Écart gold path (2026-09-26)** : « Calico hors J3 » reste vrai pour le livrable J3 (overlay inchangé). Enforcement CNI = Calico v3.29.7 sur `aiforall-local` seulement (0605), pas un SuperSéde.

Voir le fichier ADR.
