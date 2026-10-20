---
title: "ADR 0009–0012 — gates préprod et mécanisme HTTP Apparatus"
category: decisions
tags: [architecture, components, kubernetes, visibility/internal]
status: proposed
feature_status: unimplemented
sources:
  - docs/adr/0009-gate-preprod-scale-on-demand-isolation-instance.md
  - docs/adr/0010-gate-preprod-workload-certificate-ca.md
  - docs/adr/0011-apparatus-p4-pod-par-binding.md
  - docs/adr/0012-apparatus-http-knative-par-binding.md
  - docs/contracts/apparatus-knative-runtime.md
summary: >-
  Gates 0009/0010 ouvertes, Proposed / Unimplemented. Proposition 0011 Rejected.
  Mécanisme 0012 Serving/KPA par binding Proposed / Unimplemented, à ratifier.
provenance:
  extracted: 0.92
  inferred: 0.06
  ambiguous: 0.02
created: 2026-09-27T09:20:00Z
updated: 2026-10-19
---

# ADR 0009–0012 — gates préprod et mécanisme HTTP

Canon : `docs/adr/0009-gate-preprod-scale-on-demand-isolation-instance.md`, `0010-gate-preprod-workload-certificate-ca.md`, `0011-apparatus-p4-pod-par-binding.md`, `0012-apparatus-http-knative-par-binding.md`. Hub : [[projects/manifesto/decisions/index]]. Le moteur P4-core reste [[projects/manifesto/decisions/0008-apparatus-p4-k8s]] (Accepted / Implemented). Une supersession partielle de sa gestion du runtime est seulement envisagée à l’Accept de 0012.

## 0009 — scale on demand (Proposed / Unimplemented)

Un pod H24 partagé par digest est acceptable hors production et bloquant avant prod. Cible : scale on demand et isolation d’instance non silencieuse. Fermer la Réalité de 0009 exige 0012 Accepted et Implemented **et** critères propres 0009 prouvés/revus. Aucune clôture pré-prod globale ni de 0010.

## 0010 — certificat workload et CA (Proposed / Unimplemented)

La modalité d’émission et la CA logicielle actuelle sont acceptables hors production et bloquantes avant prod. Complète [[projects/manifesto/decisions/0007-apparatus-p3-lazaret]]. Le certificat Lazaret déjà livré n’est pas cette gate : [[projects/lazaret/concepts/workload-identity]].

## 0011 — ancienne proposition Pod par binding (Rejected / Unimplemented)

Proposition historique rejetée le 2026-10-19 sur autorisation explicite utilisateur ; corps conservé dans l’ADR. Elle n’est plus le mécanisme courant proposé de 0009.

## 0012 — Serving/KPA par binding (Proposed / Unimplemented)

Canon : `docs/adr/0012-apparatus-http-knative-par-binding.md`. Contrat : `docs/contracts/apparatus-knative-runtime.md`. Artefact OCI par release réutilisable, installation et identités par projet × binding ; activation logique, Service pré-déclaré à zéro, invoke autorisé via Lazaret puis Knative. Pas d’autoscaler/proxy HTTP maison. Identité du premier appel, intégration mTLS, révocation pendant attente, projection/générations, sandbox, bornes et retrait restent à préciser avant GO code. Direction discutée ≠ Accept du paquet ; rien n’est livré par cette note.

Le gold path local [[projects/aiforall/decisions/0605-gold-path-kind]] cite ces gates et ne les réécrit pas. La clé DNS par digest y est locale, pas un modèle prod.

## Related

- [[projects/manifesto/concepts/apparatus-p4-operator]]
- [[projects/aiforall/decisions/0600-cloud-portable]]
- [[journal/2026-09-27]]
