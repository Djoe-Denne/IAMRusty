# ADR-0308 — source/validation 2026-10-04

Statut : Accepted. Réalité : Partial. Canon : docs/adr/0308-mesh-authn-jwt.md ; voir le fichier ADR.

- ext_authz HTTP, pas filtre JWT/sidecar/SPIFFE ; source mesh/SDK trust et transport écrite, aucune nouvelle preuve réseau.
- Poll60/local2 conservé ; confiance monotone60, hits/erreurs sans prolongation et vide autoritatif dans source. LKG illimité à f060d47/ba69c9e = historique seulement.
- SDK ca2e35fcd56279e9e52625d0df9381f240f3390d publié selon parent/pin worktree ; 22 pure SDK PASS selon parent, pas IAM/publisher/Envoy.
- Kind/S2S 2026-10-02 historique ; nouvelles routes exactes/PKCE et S2S/spoof/trust/outage doivent être prouvées au hash courant après compilation root et IT.
- local-full séparé : 0606 Proposed/Partial, kind-aiforall-local-full autorisé après IT ; CA-only/render112 ne prouvent pas TLS/cluster. Aucune promotion Implemented.