---
title: "ADR 0009–0011 — gates préprod Apparatus (Proposed)"
category: decisions
tags: [architecture, components, kubernetes, visibility/internal]
status: proposed
feature_status: unimplemented
sources:
  - docs/adr/0009-gate-preprod-scale-on-demand-isolation-instance.md
  - docs/adr/0010-gate-preprod-workload-certificate-ca.md
  - docs/adr/0011-apparatus-p4-pod-par-binding.md
summary: >-
  Trois gates Proposed / Unimplemented : scale on demand (0009),
  CA certificat workload (0010), un Pod par binding (0011). Pas le moteur P4.
provenance:
  extracted: 0.92
  inferred: 0.06
  ambiguous: 0.02
created: 2026-09-27T09:20:00Z
updated: 2026-09-27T09:20:00Z
---

# ADR 0009–0011 — gates préprod

Canon : `docs/adr/0009`, `0010`, `0011`. Hub : [[projects/manifesto/decisions/index]]. Le moteur P4 Kind V1 est [[projects/manifesto/decisions/0008-apparatus-p4-k8s]] (Accepted / Implemented). Ces trois ADR ne le remplacent pas.

## 0009 — scale on demand (Proposed / Unimplemented)

Un pod H24 partagé par digest est acceptable hors production et bloquant avant prod. Cible : scale on demand et isolation d’instance non silencieuse. `Réalité : Implemented` de 0009 seulement si 0011 est aussi Accepted et Implemented.

## 0010 — certificat workload et CA (Proposed / Unimplemented)

La modalité d’émission et la CA logicielle actuelle sont acceptables hors production et bloquantes avant prod. Complète [[projects/manifesto/decisions/0007-apparatus-p3-lazaret]]. Le certificat Lazaret déjà livré n’est pas cette gate : [[projects/lazaret/concepts/workload-identity]].

## 0011 — Pod par binding (Proposed / Unimplemented)

Runtime plugin V1 cible = un Pod par binding (clé projet × binding). L’operator le crée, le met au repos ou le détruit. Deux projets qui partagent un digest ne partagent pas le Pod. Absent du dépôt.

Le gold path local [[projects/aiforall/decisions/0605-gold-path-kind]] cite ces gates et ne les réécrit pas. La clé DNS par digest y est locale, pas un modèle prod.

## Related

- [[projects/manifesto/concepts/apparatus-p4-operator]]
- [[projects/aiforall/decisions/0600-cloud-portable]]
- [[journal/2026-09-27]]
