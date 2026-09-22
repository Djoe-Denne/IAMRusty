# Apparatus P4 — ADR-0008

- Jalon P4. Canon : `docs/adr/0008-apparatus-p4-k8s-isolation-outside-manifesto.md`. Statut : **Accepted**. Réalité : **Implemented** (A-DEC 2026-09-22). SuperSède : aucune.
- Kind = environnement V1. 0002 / 0003 / 0005 Implemented ce tour. 0001 reste Partial. 0004 / 0006 / 0007 inchangés (Implemented).
- Décision Accepted inchangée : moteur K8s hors Manifesto ; Adm-A seule source VALID ; 4 SA ; pas de nest / Factory / skill new-service.
- Dette hors-jalon (ne bloque plus Implemented) : **D-TRANSIT-TCB** (Transit `-dev` ≠ TCB) ; **D-ADMB** (webhook optionnel, jamais source VALID) ; **D-PROD** (pas de cluster prod, N/A).
- Preuve : T2–T12 + chaîne Kind M1–M6 (`m6_e2e_0002_0008_chain`). Closeout : `docs/adr/0008-closeout.md`.
- Voir le fichier ADR. Ce digest n’est pas le canon.
