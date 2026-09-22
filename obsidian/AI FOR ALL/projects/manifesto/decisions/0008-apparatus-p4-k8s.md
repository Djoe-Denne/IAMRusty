---
title: >-
  ADR-0008 — moteur P4 Kubernetes hors Manifesto (Accepted)
category: decisions
tags: [architecture, components, visibility/internal]
status: accepted
feature_status: implemented
sources:
  - docs/adr/0008-apparatus-p4-k8s-isolation-outside-manifesto.md
  - docs/adr/0008-closeout.md
  - docs/adr/0008-app01-reconciliation.md
  - docs/apparatus-p4-implementation-prompt.md
  - docs/apparatus-p4-core-implementation-prompt.md
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/d5c0a4f7-e945-4de1-aa1c-8fa771ba9733/d5c0a4f7-e945-4de1-aa1c-8fa771ba9733.jsonl
summary: >-
  ADR-0008 Accepted 2026-09-20. Réalité Implemented (A-DEC 2026-09-22, Kind V1).
  T2–T12 + M1–M6 ; dette D-TRANSIT-TCB / D-ADMB / D-PROD. Moteur K8s hors Manifesto.
created: 2026-09-20T15:53:00Z
updated: 2026-09-22T14:00:00Z
provenance:
  extracted: 0.88
  inferred: 0.10
  ambiguous: 0.02
---

# ADR-0008 — moteur P4 Kubernetes hors Manifesto

Canon : `docs/adr/0008-apparatus-p4-k8s-isolation-outside-manifesto.md`. Hub ADR : [[projects/manifesto/decisions/index]]. Méthode (pas canon) : [[projects/manifesto/references/0008-app01-reconciliation]]. Prompt : [[projects/manifesto/references/apparatus-p4-implementation-prompt]]. Plan : [[projects/manifesto/references/apparatus-implementation-plan]]. Lazaret (P3, pas le moteur P4) : [[projects/lazaret/lazaret]].

## Statut : Accepted (2026-09-20)

Ratification chat « Je valide tout. Je ratifie tout. » — même force que 0007 « accepté » (2026-09-13). README L17/L128 : `Accepted` typiquement après PR ; **écart documenté** comme 0006/0007.

Réalité **Implemented** (A-DEC 2026-09-22). Kind = environnement V1. Crate [[projects/manifesto/concepts/apparatus-p4-operator]] T2–T12 + chaîne M1–M6 ; Calico v3.29.7 ; T11 vert ; Cosign fail-closed. **0002 / 0003 / 0005** Implemented ce tour. Dette hors-jalon : **D-TRANSIT-TCB** (Transit `-dev`) ; **D-ADMB** (webhook, jamais source `VALID`) ; **D-PROD**. Closeout canon : `docs/adr/0008-closeout.md`. Journal : [[journal/2026-09-22]].

Ne SuperSède **pas** 0003 ni 0005. 0004/0006/0007 inchangés (Implemented). **APP-05**, G/E, pas de 2ᵉ protocole ; `invoke` = Lazaret.

## Paquet figé

1. **Reg-A+D-ACL portable** — Cosign + OpenBao Transit (≠ KV Lazaret) + registre OCI portable + push signer-only ; ACL sur le registre.
2. **Adm-A** — worker seule source `VALID` (digest 0002 + politique + rapport). Adm-B enforceur optionnel, jamais source.
3. **BC-A** — operator + Jobs ; pas de nest ; pas de nom `Factory` ; skill `aiforall-new-service` **non**.
4. **Pkg-B** — enveloppe OCI ≠ image CRI ; kubelet n’exécute que l’image pinée de l’enveloppe admise.
5. **Run-A** — 4 SA K8s ; Job build sans token API ; plugins autre namespace.

K8s-as-P3 reste **interdit**. Zéro token `k8s`/`kubernetes` sous `Manifesto/*/src`.

## Related

- [[projects/manifesto/decisions/0003-apparatus-untrusted]]
- [[projects/manifesto/decisions/0005-apparatus-protocol]]
- [[projects/manifesto/decisions/0007-apparatus-p3-lazaret]]
- [[projects/manifesto/references/0007-closeout]]
- [[projects/lazaret/lazaret]]
- [[projects/manifesto/references/apparatus-implementation-plan]]
- [[journal/2026-09-20]]
- [[journal/2026-09-22]]
- [[projects/manifesto/concepts/apparatus-p4-operator]]
- [[projects/aiforall/skills/running-apparatus-p4-tests]]
