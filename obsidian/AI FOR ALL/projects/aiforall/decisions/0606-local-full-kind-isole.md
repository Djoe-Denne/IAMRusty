---
title: "ADR-0606 — lab local-full Kind dédié (Proposed)"
category: decisions
status: proposed
feature_status: partial
sources:
  - docs/adr/0606-local-full-kind-isole.md
summary: >-
  Canon 0606 Proposed / Partial : nouveau Kind local-full 1+2 isolé du legacy,
  dépendances/états in-cluster ; D6 render112 source seulement, runtime non prouvé.
created: 2026-10-04
updated: 2026-10-04
---

# ADR-0606 — lab local-full Kind dédié

Canon : `docs/adr/0606-local-full-kind-isole.md`. Hub : [[projects/aiforall/decisions/index]]. Related Manifesto : [[projects/manifesto/decisions/index]], pas une ADR Apparatus.

**Proposed / Partial.** Autorisation humaine du contexte/périmètre le 2026-10-04 ; pas Accept implicite des choix techniques. `kind-aiforall-local-full` dédié, 1 control-plane + 2 workers, namespaces locaux isolés, standalones/dépendances in-Kind, CA projet/public-only et PVC locaux déclarés. Legacy étranger intact ; inventaire/bail IDs exacts, mismatch ⇒ STOP/INCONCLUSIVE sans delete/recreate.

Source actuelle : `ops/deploy/apps/overlays/local-full/`, `ops/deploy/kind/cluster-local-full.yaml` et helpers `ops/deploy/*local-full*`. D6 rendu offline112, topologie et mapping HTTP loopback18080 = SOURCE, pas trafic/NetworkPolicy/TLS ou reprise/restore. SDK22 purs PASS selon parent ne valide pas IAM/mesh/root.

DEV OpenBao/SQS volatils, SMTP/catalogue simulés, OAuth externe désactivé et PVC node-local restent des limites : pas HA ni retrait automatique de constats locaux. Root compilation/units puis IT testcontainers, ensuite E2E full autorisé après intégration ; qualification LIVE / SIMULÉ / NOT RUN et restore sur cible distincte.

S-11/MEDIUM reste ouvert : leasev2 doit lier ID control-plane exact/mapping6443 au server du kubeconfig fixe avant tout API GET, puis vérifier kube-system UID ; contrat§15. Garde à implémenter/prouver, pas capacité livrée. S-10 distinct : [[projects/aiforall/decisions/0310-admission-epochs-budget-jwks]].

Historique : [[projects/aiforall/decisions/0603-tranche-locale]], [[projects/aiforall/decisions/0604-j3-overlay-demo-monolith]], [[projects/aiforall/decisions/0605-gold-path-kind]]. Auth/trust : [[projects/aiforall/decisions/0308-mesh-authn-jwt]], [[projects/iamrusty/decisions/0412-oauth-transaction-browser-binding]]. Aucun Supersede ; 0404/0500 et cloud0600–0602 inchangés.
