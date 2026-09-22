# Apparatus P0 — ADR-0003

- Jalon P0 (confiance) / P3–P4 (runtime). Canon : `docs/adr/0003-apparatus-untrusted-plugin.md`. Statut : **Accepted**. Réalité : **Implemented** (A-DEC 2026-09-22). SuperSède : aucune.
- Décision Accepted inchangée : plugin hors processus privilégiés ; 4 identités OS ; harness in-process = double de test.
- Réalité : isolation Kind/Calico + 4 SA sur le chemin invoke (M5–M6). Harness TEST-ONLY **peut rester**.
- Pas de SuperSède. Factory / host = P5/P6 **après**. Dette : **D-TRANSIT-TCB**, **D-ADMB**, **D-PROD**.
- Voir le fichier ADR. Ce digest n’est pas le canon.
