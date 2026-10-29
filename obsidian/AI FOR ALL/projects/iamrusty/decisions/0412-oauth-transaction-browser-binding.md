---
title: "ADR-0412 — transaction OAuth persistante liée au navigateur"
category: decisions
status: accepted
feature_status: implemented
sources:
  - docs/adr/0412-oauth-transaction-persistante-liee-navigateur.md
  - docs/contracts/20261029-auth-security-qualified-state.md
tags: [security, oauth, visibility/internal]
summary: >-
  Accepted/Implemented pour le laboratoire local : transaction OAuth liée au navigateur, parcours Chrome via Envoy et refus ciblés qualifiés. Production et HA non qualifiées.
provenance:
  extracted: 1.0
  inferred: 0.0
  ambiguous: 0.0
created: 2026-10-03
updated: 2026-10-29
---

# ADR-0412 — OAuth lié au navigateur

Canon : `docs/adr/0412-oauth-transaction-persistante-liee-navigateur.md`.
Contrat A/B/C : `.cursor/review-briefings/20261003-auth-fixes-interface-contract.md`.
Hub : [[projects/iamrusty/decisions/index]]. Related : [[projects/iamrusty/decisions/0409-confiance-oauth]].

**Accepted / Implemented pour la preuve locale**, après proposition architecte et ratification explicite du lead sous délégation utilisateur le 29 octobre. L'état Partial du 8 octobre reste historique. Transaction persistante, cookie nonce hash, state obligatoire, consommation atomique writer et PKCE restent les garanties du contrat ; aucun alias ni contournement non-browser n'est ajouté.

Le correctif impose issuer et audience lors de la création des tokens ; l'inscription puis la nouvelle connexion ne retombent plus sur une autorité legacy. Chrome réel, profil neuf, traverse les routes exactes via Envoy : login 200, callback exact sans Bearer et trois refus 403. Le connecteur configuré est GitLabConnect avec fournisseur synthétique : ce n'est pas une qualification du fournisseur GitLab réel. Le parcours n'utilise pas une nouvelle interface utilisateur.

Preuves finales : 17 tests d'intégration (9 navigateur transactionnel, 6 PostgreSQL, 2 cleanup), sans échec ni test ignoré/filtré ; 19 tests domaine, 3 RS256, check IAM et fmt réussis. Les trois fixtures sont retirées et les nœuds Kind possédés arrêtés sans suppression. Refus avant IAM et absence d'effets base sont inférés des corrélations, pas mesurés directement en SQL. Le contrat qualifié conserve cette limite et les compteurs absents.

Cette qualification ne couvre ni production, ni haute disponibilité, ni deux répliques déployées, ni toutes les projections runtime. L'atomicité de consommation ne promet pas de rollback global après consommation. Les autres ADR gardent leurs statuts propres.

Synthèse de campagne et revues : [[projects/aiforall/references/auth-security-closure-20261029]]. Suite proposée, non exécutée : [[projects/aiforall/concepts/k6-functional-linux]].
