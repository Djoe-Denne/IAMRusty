---
title: "ADR 0506 — mutualisation OpenBao conditionnelle"
category: decisions
status: proposed
feature_status: unimplemented
created: 2026-10-31
summary: "Veille sans engagement : n’élargir le SDK qu’après preuve de récurrence réelle, de sémantique proche et d’abstraction utile."
---

# Mutualisation OpenBao conditionnelle

Canon : `docs/adr/0506-mutualisation-openbao-conditionnee-aux-usages.md` — **Proposed / Unimplemented** pour l’extension éventuelle, opportunité conditionnelle en veille ; aucun calendrier ni implémentation décidée.

Preuves : `docs/missions/20261031-openbao-lazaret-iam-reuse-audit.md` (publié en `2d00e5a`). Le KV existant reste distinct : [[projects/aiforall/decisions/0505-startup-secrets-openbao]], **Proposed / Implemented**.

GitHubConnect : KV boot ; Lazaret : KV à la demande autorisée, délégation à évaluer ; IAM : Transit distinct, un consommateur indépendant démontré ici, donc pas d’extraction aujourd’hui. Même fournisseur ou wrapper HTTP déplacé ne suffisent pas ; transport/credentials et port multi-backend ne sont pas des modules/cibles déjà décidés.

Le prochain lot compare les faits du jour et recommande explicitement : **conserver local / adapter l’usage au SDK existant / proposer une extension**. Récurrence, sémantique commune et gain net sur le coût de maintenance sont nécessaires ensemble ; aucun compteur, migration, backend hypothétique ou rappel automatique.

Related : [[projects/manifesto/decisions/index]]. Le fichier ADR donne les signaux et preuves de réouverture ; aucun contrat d’implémentation/API n’est créé par cette veille.
