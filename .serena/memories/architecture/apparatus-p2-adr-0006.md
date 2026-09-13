# Apparatus P2 — ADR-0006

- Canon : `docs/adr/0006-apparatus-p2-reconciliation-in-process.md`
- Jalon : P2
- Statut : Accepted
- Réalité : Implemented — T1–T7 et writer backoff §D livrés ; aucun écart P2 ouvert
- Contrôleur in-process Manifesto (ticker + scan DB) : génération CAS commande, lease 30 s, fencing génération+epoch ; pas de K8s, pas de HTTP 202, pas de nouvel event
- Voir le fichier ADR. Ce digest n’est pas le canon.
