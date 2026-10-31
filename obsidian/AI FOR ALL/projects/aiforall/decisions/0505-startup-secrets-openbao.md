---
title: "ADR 0505 — secrets de démarrage SDK / OpenBao"
category: decisions
status: proposed
feature_status: implemented
created: 2026-10-31
updated: 2026-10-31
summary: "Proposed / Implemented dans l’arbre validé : SDK publié, tests Windows verts ; publication finale service en cours."
---

# Secrets de démarrage SDK / OpenBao

Canon : `docs/adr/0505-startup-secrets-sdk-openbao.md` — **Proposed / Implemented** dans l’arbre validé, sans acceptation implicite.

Contrat implementer : `docs/contracts/startup-secrets-openbao.md`.

Premier consommateur implémenté : GitHubConnect, sans changement OAuth/HMAC ni migration IAM/Lazaret. SDK `main@93b890f` publié ; 11 tests config SDK et 17 tests service Windows verts, dont 3 nouveaux startup. Publication finale service/gitlink AIForAll encore en cours chez le parent : `docs/missions/20261031-github-connect-openbao-results.md`.

Bootstrap toujours fourni ; endpoint, fichier d’identité et références configurables. Aucun PASS CI Linux/TLS live/coffre réel implicite ; pas de déploiement, rotation à chaud ou provider workload natif livré par ce lot.

Related : [[projects/iamrusty/decisions/index]], [[projects/manifesto/decisions/index]]. Cette page n’est ni une ADR Accepted ni une preuve d’implémentation.
