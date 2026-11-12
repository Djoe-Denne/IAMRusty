---
title: "ADR 0611 — OpenBao neuf sur Raft/PVC, existant préservé"
category: decisions
status: proposed
feature_status: unimplemented
created: 2026-11-09
summary: "Pointeur canon : coffre neuf persistant, choix humain Shamir 1/1 local ; DEV mémoire existant intact."
---

# OpenBao neuf sur Raft/PVC, existant préservé

Canon : `docs/adr/0611-openbao-neuf-raft-pvc-sans-conversion-du-coffre-existant.md` — **Proposed / Unimplemented**.

Contrat v1.1 : `docs/contracts/kind-local-openbao-bootstrap.md`, référencé au contrat lanceur v1.3 amendé. Première installation Raft mono-nœud/PVC ; aucun redéploiement du coffre HTTPS existant. Init privée exclusive, anciens fichiers préservés mais pas réutilisés implicitement après reset. **Choix humain acquis « 1/1 local (Recommended) » : Shamir 1 part / seuil 1 en fichier privé protégé, coffre neuf du lab mono-opérateur uniquement.** Réserve de garde levée, sans rekey de l'existant ni auto-unseal statique/KMS ; pas d'Accept global de l'ADR.

0610 reste **Proposed / Partial**, preuves Build/Redis TLS Windows inchangées ; ce delta ne qualifie ni restart/persistance ni démarrage Kind. Voir le fichier ADR, pas cette page comme seconde décision.

Related : [[projects/manifesto/decisions/index]], [[projects/aiforall/decisions/0506-mutualisation-openbao-conditionnelle]].
