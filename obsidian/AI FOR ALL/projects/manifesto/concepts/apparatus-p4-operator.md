---
title: Apparatus P4 — crate apparatus-operator
category: concepts
tags: [architecture, components, kubernetes, visibility/internal]
aliases: [apparatus-operator, P4-core]
sources:
  - C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts/d5c0a4f7-e945-4de1-aa1c-8fa771ba9733/d5c0a4f7-e945-4de1-aa1c-8fa771ba9733.jsonl
  - docs/adr/0008-apparatus-p4-k8s-isolation-outside-manifesto.md
  - docs/apparatus-p4-core-implementation-prompt.md
  - apparatus-operator/src/controller.rs
  - apparatus-operator/src/admit.rs
summary: >-
  BC-A hors Manifesto : admit (ORAS+Cosign+Transit fail-closed), controller Kind,
  Calico v3.29.7 IT, T11 vert, pin CRI @sha256 64 hex, pull zot IfNotPresent.
  ADR-0008 Implemented (A-DEC) ; dette D-TRANSIT-TCB / D-ADMB / D-PROD.
provenance:
  extracted: 0.82
  inferred: 0.16
  ambiguous: 0.02
created: 2026-09-22T06:55:00Z
updated: 2026-09-22T14:00:00Z
---

# Apparatus P4 — crate apparatus-operator

Membre workspace `apparatus-operator`. Canon : [[projects/manifesto/decisions/0008-apparatus-p4-k8s]]. Prompt TDD (historique POST-LOT) : [[projects/manifesto/references/apparatus-p4-implementation-prompt]] / `docs/apparatus-p4-core-implementation-prompt.md`. Tests : [[projects/aiforall/skills/running-apparatus-p4-tests]]. Vision : [[projects/manifesto/concepts/apparatus-platform]].

IT Kind **Calico v3.29.7** (`disableDefaultCNI: true`, kindnet interdit) ; T11 `t11_plugin_exec_cannot_reach_transit_or_system` **vert** (22 sept.) ; Cosign `verify` **fail-closed** — présence du tag `.sig` ≠ preuve d’admission.

## Frontières

- **Hors** Manifesto / Lazaret : zéro token `k8s`/`kubernetes` sous `Manifesto/*/src`. Pas de `KubernetesAdapter` Manifesto. Pas de répertoire `Factory/`.
- Features Cargo : `admit` (ORAS/Cosign/Transit HTTP) ; `controller` (kube + CR `AdmissionRecord`).
- Binaires : `apparatus-admit`, `apparatus-controller`, `apparatus-build`.
- Identité catalogue = digest descripteur 0002. Enveloppe OCI **≠** image CRI. kubelet n’exécute que `name@sha256:<64 hex>`.

## Admission et schedule

`VALID` n’est écrit que par le chemin admit (Adm-A). Le contrôleur lit la CR : phase ≠ `VALID` → `ScheduleRefuse::NotValid` ; CR absente → `MissingAdmissionRecord` ; isolation manquante → refus fermé (pas un Pod « un peu moins isolé »).

`envelope_to_podspec` : `imagePullPolicy: IfNotPresent`, `automountServiceAccountToken: false`. Jamais tag `latest`.

RBAC Kind : SA `apparatus-admit` seul `create`/`patch` `admissionrecords` + `/status`. Controller = `get/list/watch` records ; pods/jobs dans `apparatus-plugins`. Build/gateway n’écrivent pas VALID.

## Registry IT

zot (`anonymousPolicy: [read]`, push `signer` seulement) + OpenBao Transit. Cosign Vault-compat ; admission **fail-closed** si `verify` échoue (tag `.sig` seul ne suffit pas). Cluster Kind `apparatus-p4-it` (T10+) : **Calico v3.29.7** installé avant application des NetworkPolicy (`disableDefaultCNI: true`, kindnet interdit). Image plateforme in-repo `testdata/platform-plugin`.

Preuve Pkg-B : pull kubelet depuis zot (`host.docker.internal:{port}`) **sans** `kind load` de cette réf. `kind load` + Never reste le chemin T10 historique complémentaire. ^[inferred]

- **M1** : artefact zot de `apparatus-reference-kv` (test `m1_reference_kv_pinned_envelope_on_zot`), distinct de `testdata/platform-plugin` ; chaîne M1–M6 vert (0008 **Implemented**, Kind V1).

## Dette hors-jalon (ne bloque plus Implemented)

**D-TRANSIT-TCB** : Transit `-dev` IT ≠ TCB release prod. **D-ADMB** : webhook Adm-B ouvert. **D-PROD** : pas cluster prod. Hors scope V1 : PSS ; protocole invoke busybox complet. **APP-05** / G/E inchangés.
