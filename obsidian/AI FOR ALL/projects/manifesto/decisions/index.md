---
title: "Apparatus — index des ADR"
category: references
tags: [architecture, components, visibility/internal]
status: accepted
feature_status: future
summary: "Hub wiki vers les ADR Apparatus ratifiées dans docs/adr/ ; décision cible et réalité d’implémentation restent distinctes."
created: 2026-09-10T06:32:00Z
updated: 2026-09-10T06:32:00Z
---

# Apparatus — index des ADR

Les **décisions** vivent dans le handbook git, pas dans les notes de conception :

- Index : `docs/adr/README.md`
- Template : `docs/adr/template.md`

Les ADR Apparatus ci-dessous sont **Accepted** depuis la ratification orchestrée du 10 septembre 2026. `Accepted` fixe une cible ; il ne signifie pas `Implemented`. Les pages [[projects/manifesto/concepts/apparatus-platform]] et suivantes restent le rationnel, et la baseline antérieure aux ADR est conservée dans [[projects/manifesto/references/apparatus-implementation-plan#Recommandations du 9 septembre — baseline de traçabilité]].

## Vague 1

| ADR | Décision | Réalité |
|---|---|---|
| [0001](../../../../../docs/adr/0001-apparatus-binding-owned-by-manifesto.md) | Binding = extension 1:1 de `ProjectComponent`, propriété métier Manifesto | Partial |
| [0002](../../../../../docs/adr/0002-apparatus-contract-first.md) | Contrats + Apparatus KV de référence avant Factory et host | Unimplemented |
| [0003](../../../../../docs/adr/0003-apparatus-untrusted-plugin.md) | Code auteur hors des processus privilégiés | Unimplemented |
| [0004](../../../../../docs/adr/0004-apparatus-capability-gateway.md) | Gateway de capacités ; KV plateforme ; pas de bearer IAM | Unimplemented |
| [0005](../../../../../docs/adr/0005-apparatus-same-protocol-valid-verified.md) | Même protocole ; admission, `VALID`, `VERIFIED` et installabilité distincts | Unimplemented |

## Conception (pas des ADR)

- [[projects/manifesto/concepts/apparatus-platform]]
- [[projects/manifesto/concepts/apparatus-bindings-and-lifecycle]]
- [[projects/manifesto/concepts/apparatus-capabilities-and-isolation]]
- [[projects/manifesto/references/apparatus-factory-and-distribution]]
- [[projects/manifesto/references/apparatus-ui-and-protocol]]
- [[projects/manifesto/references/apparatus-implementation-plan]]
- [[projects/manifesto/references/apparatus-source-reconciliation]]

Arbitrages encore ouverts (`APP-01` … `APP-07`) : [[projects/manifesto/references/apparatus-implementation-plan#Questions encore ouvertes]].
