---
title: >-
  Réconciliation APP-01 (méthode, pas canon)
category: references
tags: [architecture, apparatus, visibility/internal]
sources:
  - docs/adr/0008-app01-reconciliation.md
  - docs/adr/0008-apparatus-p4-k8s-isolation-outside-manifesto.md
summary: >-
  Méthode α/β 2026-09-20, RATIFIÉE. Pas une ADR. Canon = ADR-0008
  Accepted / Implemented (A-DEC 2026-09-22 ; dette TCB). Paquet Reg-A+D, Adm-A, BC-A, Pkg-B, Run-A.
provenance:
  extracted: 0.86
  inferred: 0.12
  ambiguous: 0.02
created: 2026-09-20T15:55:00Z
updated: 2026-09-22T14:00:00Z
---

# Réconciliation APP-01 (méthode, pas canon)

**Pas** ADR-0008. **Pas** Proposed. **Pas** Accepted. **RATIFIÉ 2026-09-20** (« Je valide tout. Je ratifie tout. »). Canon : [[projects/manifesto/decisions/0008-apparatus-p4-k8s]].

Méthode : deux architectes (α sécu / β ops) puis réconciliateur. Priorité : (1) sécurité (2) maintenabilité (3) extensibilité ; sécu gagne.

## Paquet ratifié

| Q | Choix |
|---|---|
| Registry | Cosign + OpenBao **Transit** (≠ KV Lazaret T12) + OCI portable + push signer-only ; ACL **sur le registre** |
| Admission | **Adm-A** worker seule source `VALID` ; Adm-B enforceur optionnel |
| BC | **BC-A** operator + Jobs ; pas de nest ; pas de nom `Factory` ; skill `aiforall-new-service` **non** |
| Package | **Pkg-B** enveloppe OCI/ORAS ≠ image CRI ; identité = digest 0002 |
| Processus | **Run-A** 4 SA K8s ; Job build sans token API ; plugins autre ns |

T1 absence déjà livrée. T2+ **débloqué** par l’Accept 0008 ; au moment de l’ingest (2026-09-20) Réalité **Unimplemented** — **aujourd’hui (2026-09-22)** T2–T12 + M1–M6 livrés ; 0008 **Implemented** (A-DEC) ; dette D-TRANSIT-TCB / D-ADMB / D-PROD.

## Related

- [[projects/manifesto/references/0007-closeout]]
- [[projects/manifesto/references/apparatus-p4-implementation-prompt]]
- [[projects/lazaret/lazaret]] — Transit ≠ KV plugin
- [[journal/2026-09-20]]
