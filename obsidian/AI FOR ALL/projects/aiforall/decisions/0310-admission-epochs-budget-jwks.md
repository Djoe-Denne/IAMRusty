---
title: "ADR-0310 — admission epochs et budget JWKS global"
category: decisions
status: accepted
feature_status: partial
sources:
  - docs/adr/0310-admission-epochs-budget-jwks.md
summary: >-
  Accepted/Partial : ratification initiale le 2026-10-04 ; calcul/capacité partiellement
  supersédés par 0312 Accepted/Implemented le 2026-10-08. Autres invariants conservés ; pas clôture sécurité globale.
created: 2026-10-04
updated: 2026-10-08
---

# ADR-0310 — admission epochs / budget global

Canon : `docs/adr/0310-admission-epochs-budget-jwks.md`. Hub : [[projects/aiforall/decisions/index]]. Related : [[projects/aiforall/decisions/0304-access-jwt-trust]], [[projects/aiforall/decisions/0308-mesh-authn-jwt]].

**Accepted / Partial** : ratification initiale « Valider ces limites (Recommended) » le 2026-10-04. Depuis l’Accept explicite de 0312 le 2026-10-08, le mécanisme de calcul/capacité est **partiellement supersédé** par les slots à coût maximal fixe livrés et prouvés : [[projects/aiforall/decisions/0311-signing-simplification]]. Le reste du contrat demeure ; pas de retrait de 0310 entière ni de clôture sécurité globale S-10 dans ce lot. Contrat historique : `.cursor/review-briefings/20261003-auth-fixes-interface-contract.md` §14, calcul/capacité désormais régis par 0312.

Limites ratifiées : plafond768KiB, réserve plateforme64KiB, entrée4KiB, org8 epochs/4 nouveaux kids par3600s, plateforme16 ; bornes RSA/issuer dans le canon. Comparaison de binding complète+n/e ; no-op conserve Active/kid/timestamps, refus429/409 sans retraite partielle. Publication entière sous même predicate/horloge DB, réservation par slots selon 0312 et mesure du snapshot matérialisé ; toutes clés admissibles publiées. Aucun plafond consommateur relevé, cache60 affaibli ou Retiring valide évincée.

Units + IT IAM vertes sur master confirmées par Djoé Denne le 2026-10-08 : attestation utilisateur, pas artefact CI archivé. Mécanisme livré et preuves numériques référencés dans 0312 ; pas de validation E2E ni de clôture sécurité globale S-10 implicite. Source/review/SDK22 ne remplacent pas ces preuves. S-11 est distinct : [[projects/aiforall/decisions/0606-local-full-kind-isole]], contrat§15.
