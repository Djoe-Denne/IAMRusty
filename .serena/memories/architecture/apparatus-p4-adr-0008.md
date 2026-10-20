# Apparatus P4 — ADR-0008

- Statut : Accepted ; Réalité : Implemented (A-DEC 2026-09-22).
- Canon : docs/adr/0008-apparatus-p4-k8s-isolation-outside-manifesto.md ; voir le fichier ADR et docs/adr/0008-closeout.md, preuves historiques T2–T12 + M1–M6 ; Kind environnement V1.
- Kubernetes hors Manifesto ; Cosign/Transit ; Adm-A seule source VALID ; enveloppe OCI distincte image CRI ; quatre responsabilités/SA séparées ; pas de nest HTTP/Factory/new-service.
- 0002/0003/0005/0004/0006/0007 Implemented ; 0001 Partial. Dettes D-TRANSIT-TCB (Transit-dev ≠ TCB), D-ADMB (optionnel jamais source VALID), D-PROD conservées.
- Note du 2026-10-19 : 0012 Proposed / Unimplemented envisage la supersession partielle de la gestion runtime seulement à son Accept ; aucune supersession effective/globale aujourd’hui.
- 0011 Rejected ; gate 0009 toujours ouverte, mécanisme proposé 0012 et critères propres à prouver ; 0010 inchangée.
