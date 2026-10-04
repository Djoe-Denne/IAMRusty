---
title: "ADR-0310 — admission epochs et budget JWKS global"
category: decisions
status: accepted
feature_status: partial
sources:
  - docs/adr/0310-admission-epochs-budget-jwks.md
summary: >-
  Accepted/Partial : ratification humaine le 2026-10-04 ; admission/idempotence et quotas
  non livrés. Budget768KiB/réserve64KiB ratifiés ; correction non livrée, S-10 HIGH BLOCK.
created: 2026-10-04
updated: 2026-10-04
---

# ADR-0310 — admission epochs / budget global

Canon : `docs/adr/0310-admission-epochs-budget-jwks.md`. Hub : [[projects/aiforall/decisions/index]]. Related : [[projects/aiforall/decisions/0304-access-jwt-trust]], [[projects/aiforall/decisions/0308-mesh-authn-jwt]].

**Accepted / Partial** : choix utilisateur explicite « Valider ces limites (Recommended) » le 2026-10-04 ; registry primaire/epochs atomiques/publication existants seulement. L'admission complète et l'idempotence restent à implémenter ; S-10/HIGH reste BLOCK. Le contrat est `.cursor/review-briefings/20261003-auth-fixes-interface-contract.md` §14.

Limites ratifiées : plafond768KiB, réserve plateforme64KiB, entrée4KiB, org8 epochs/4 nouveaux kids par3600s, plateforme16 ; bornes RSA/issuer dans le canon. Comparaison de binding complète+n/e ; no-op conserve Active/kid/timestamps, refus429/409 sans retraite partielle. Même serializer/predicate snapshot DB, toutes clés admissibles publiées. Aucun plafond consommateur relevé, cache60 affaibli ou Retiring valide évincée.

Source/review/SDK22 ne sont pas une preuve de cette correction. SQL races/bytes/publisher réel et token plateforme après60 à prouver ; aucune livraison ni Accept d’autres propositions implicite. S-11 est distinct : [[projects/aiforall/decisions/0606-local-full-kind-isole]], contrat§15.
