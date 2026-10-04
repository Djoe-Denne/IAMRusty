---
title: "ADR-0412 — transaction OAuth persistante liée au navigateur"
category: decisions
status: accepted
feature_status: partial
sources:
  - docs/adr/0412-oauth-transaction-persistante-liee-navigateur.md
summary: >-
  Ratification humaine : transaction writer + cookie nonce hash pour login/link/relink,
  consommation multi-replica et PKCE supporté. Cible Accepted, source Partial ; preuves intégrées encore manquantes.
created: 2026-10-03
updated: 2026-10-04
---

# ADR-0412 — OAuth lié au navigateur

Canon : `docs/adr/0412-oauth-transaction-persistante-liee-navigateur.md`.
Contrat A/B/C : `.cursor/review-briefings/20261003-auth-fixes-interface-contract.md`.
Hub : [[projects/iamrusty/decisions/index]]. Related : [[projects/iamrusty/decisions/0409-confiance-oauth]].

**Accepted / Partial.** Accord explicite utilisateur du 2026-10-03 : transaction persistante, cookie transactionnel HttpOnly/Secure hors local, state obligatoire, consommation atomique entre replicas et PKCE quand supporté. Sources writer/cookie/PKCE/cleanup et contexte partagé écrites ; aucune preuve intégrée root/IT/E2E nouvelle. Relink-callback GET/HEAD exact github/gitlab configurés sans access JWT, autorisé par transaction cookie/state ; START Link/Relink restent JWT plateforme. Aucun auth-cookie présumé, alias/wildcard ou mode non-browser exceptionnel. Voir le canon, pas SDK22 comme preuve IAM.
