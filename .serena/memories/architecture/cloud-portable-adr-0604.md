# ADR-0604 — J3 monolithe démo

Statut : Proposed. Réalité : Partial (réconciliation 2026-10-04). Canon : docs/adr/0604-j3-overlay-demo-monolith-kind-invoke.md ; voir le fichier ADR.

- Overlay kind-demo-monolith + image/helpers présents ; ancien Implemented et Job POST invoke HTTP401 conservés comme preuves historiques, pas du root modifié courant.
- Aucun Supersede 0601/0404/0600/0603/0008 ; monolithe démo ≠ unité cluster normative ou preuve standalone.
- host.docker.internal/Compose = limite historique J3 ; local isoprod full utilise deps in-Kind et contexte dédié 0606 Proposed/Partial.
- Source D6/render112/topologie1+2 ≠ runtime/réseau/durabilité ; compilation/IT/E2E courantes non fournies, promotion bornée au parcours effectivement prouvé.
- Ne pas toucher legacy étranger pour rejouer J3 ; mismatch/non-reproductibilité ⇒ INCONCLUSIVE/arbitrage humain. 0404/0500 intacts.