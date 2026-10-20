# Apparatus P4 — ADR-0009

- Statut : Proposed ; Réalité : Unimplemented.
- Canon : docs/adr/0009-gate-preprod-scale-on-demand-isolation-instance.md ; voir le fichier ADR.
- Gate pré-prod scale on demand et isolation d’instance explicite ouverte ; limite H24/partage digest acceptable seulement hors prod.
- P4-core 0008 reste Accepted / Implemented ; ne supersède ni 0003 ni 0008.
- Mécanisme proposé : 0012 Serving/KPA par binding ; 0011 Rejected, historique.
- Fermeture exige 0012 Accepted + Implemented ET critères propres 0009 prouvés/revus ; aucun Accept ou cold-start isolé ne suffit.
- Aucune clôture pré-prod globale ni de 0010 ; contrat docs/contracts/apparatus-knative-runtime.md.
