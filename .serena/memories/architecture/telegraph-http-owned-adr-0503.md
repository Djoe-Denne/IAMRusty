# HTTP possédé — ADR0503

Canon : docs/adr/0503-http-runtime-owned-shutdown.md ; voir le fichier ADR.
Statut : Proposed. Réalité : Implemented (API et adoption Telegraph/monolithe), réconciliation2026-10-28.

- Propriétaire caller, observation parent distincte de join ; descendants observés sous bornes communes grâce5s/stop6s.
- SDKd60a88b livré et intégré main9e9ef27 ; callers application39024b2 intégrés master66c16f1.
- E1 rouge historique conservé ; témoins verts/revues ciblées rapports77/79/95, bilan141, intégration150.
- Drop/abort ne vaut pas async join ; aucune nouvelle garantie legacy ou preuve de toutes les erreurs/courses/runtime plateforme.
- ADR reste Proposed,0301 Partial ; dettes qualité conservées, pas nouveau Cargo/runtime pour réconciliation.
