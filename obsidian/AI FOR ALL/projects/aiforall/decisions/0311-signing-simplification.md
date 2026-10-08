---
title: "ADR0311–0312 — simplification signing IAM"
category: decisions
status: accepted
feature_status: implemented
sources:
  - docs/adr/0311-crypto-privee-deleguee-aux-providers.md
  - docs/adr/0312-admission-jwks-par-slots-bornes.md
updated: 2026-10-08
---
# ADR0311–0312 — simplification signing IAM

Canon : `docs/adr/0311-crypto-privee-deleguee-aux-providers.md` et `docs/adr/0312-admission-jwks-par-slots-bornes.md`.
0311 et 0312 **Accepted / Implemented**, Accept explicite de Djoé Denne le 2026-10-08. Crypto privée production déléguée, versions épinglées et PEM dev/test explicite ; slots JWKS livrés et prouvés (`4096`, caps org/plateforme `8/16`, global/org global `191/175`, frame dérivé `782538` octets). Supersession ciblée du calcul/capacité de 0310 effective ; 0310 reste **Accepted / Partial**, autres invariants conservés. Autorité entreprise et protocole workload Lazaret inchangés.

Units + IT IAM vertes sur master confirmées par l’utilisateur le 2026-10-08 : attestation utilisateur, pas artefact CI archivé. Pas d’E2E exact-route/Envoy 0412 ni de clôture sécurité globale implicite. Voir les fichiers ADR.
Prompt historique du paquet livré : `docs/local/20261007-signing-implementation-prompt.md` ; ne remplace pas les canons actualisés.
Hub : [[projects/aiforall/decisions/index]] ; [[projects/manifesto/decisions/index]].