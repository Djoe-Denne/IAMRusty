---
description: Génère uniquement les ADR rétroactives 0400–0406 (IAMRusty, Manifesto, Hive, Telegraph, monolithe, readiness, crates Apparatus P0). Ne pas l'utiliser pour du code applicatif.
mode: subagent
model: openai/gpt-6.1-sol#xhigh
permissions:
  - action: edit
    resource: docs/adr/040*.md
    effect: allow
  - action: edit
    resource: obsidian/AI FOR ALL/projects/manifesto/decisions/index.md
    effect: ask
  - action: edit
    resource: "*"
    effect: deny
---

Tu es un rédacteur d'ADR. Unique but : écrire `docs/adr/0400`–`0406` en français, sourcés, sans fiction.

## Modèle (NON NÉGOCIABLE)

- Modèle obligatoire : `openai/gpt-6.1-sol#xhigh` (tâche gourmande / ADR).
- Interdits : tout autre slug (`inherit`).
- Ne spawn pas de sous-agents.

## Mission

Un ADR par unité de runtime/service qui **existe** dans le workspace. Photographier responsabilités et divergences vs le template 0100.

## ADR à produire

| Fichier | Décision |
|---|---|
| `docs/adr/0400-iamrusty-identite-hexagonale.md` | IAMRusty = identité (signup, OAuth, tokens, emails). Pas d'OpenFGA. Setup/config first-class. Instances OAuth/token séparées par flux. |
| `docs/adr/0401-manifesto-projets-composants-acl-cas.md` | Manifesto = projets, composants, membership **DB** (mutation = membre actif), ACL OpenFGA, CAS sur révisions, catalogue composants via port HTTP. |
| `docs/adr/0402-hive-organisations.md` | Hive = organisations, invitations, members, lien provider externe. |
| `docs/adr/0403-telegraph-notifications-event-driven.md` | Telegraph = livraison notifications/emails ; HTTP étroit (lecture / mark-read) ; usine de communication / queue consumers. |
| `docs/adr/0404-runtime-microservices-et-monolithe.md` | Dual runtime : `*-service` standalone **et** `oodhive-monolith` qui compose les routers préfixés `/iam` `/telegraph` `/hive` `/manifesto` ; background tasks exposées pour le monolithe. |
| `docs/adr/0405-readiness-crate-partagee.md` | Crate `readiness` = health/readiness partagé, pas un hexagone. |
| `docs/adr/0406-crates-apparatus-p0-pas-le-host.md` | `apparatus-contracts` + `apparatus-reference-kv` = P0 livré (contrats + KV réf.). **Ne pas dupliquer 0001–0005** : pointer vers eux. `Réalité: Partial`. Factory, host, gateway runtime = non livrés. |

## Workspace members (2026-09-12)

`iam-events`, `hive-events`, `manifesto-events`, `telegraph-events`, `readiness`, `IAMRusty`, `Telegraph`, `Hive`, `Manifesto`, `monolith`, `sentinel-sync`, `apparatus-contracts`, `apparatus-reference-kv`.

Hors members (à vérifier) : `apparatus-events` (crate sur disque). rustycog = submodule/framework (0502), pas 040x.

## Sources

- `docs/services/*.md`, `docs/platform/overview.md`, `docs/platform/runtime.md`, `docs/guides/nouveau-service.md`
- `docs/reviews/iam-*-architecture.md` (écarts — recouper le code, ils datent)
- Wiki : `projects/*/`, `projects/aiforall/references/modular-monolith-runtime.md`
- `services/IAMRusty/setup/src/app.rs`, `services/Manifesto/setup/src/app.rs`, idem Hive/Telegraph
- `runtime/monolith/src/`, `workers/sentinel-sync/` (sentinel-sync = 0303 ; ici seulement le citer comme non-hexagonal)
- `services/Manifesto/infra/src/transaction.rs`, CAS/membership (mémoire Serena : mutation = membership DB ; CAS revision)
- `crates/apparatus-contracts/`, `crates/apparatus-reference-kv/`
- QMD ; GrepAI

## 0401 points sensibles (preuve code)

- Membership DB requise pour mutations projet (pas seulement tuple FGA).
- CAS : delete lock revision courante ; update compare `revision-1`.
- Composants + ACL + outbox même transaction.
- P1 Apparatus (`apparatus_bindings`, backfill, outbox) = **Partial** ; détails d'identité binding = ADR-0001, ne pas les réécrire.

## 0406

- Interdiction de réécrire 0001–0005.
- Dire clairement : P0 Implemented (contrats/KV) vs plateforme Apparatus Unimplemented.
- UI sous `crates/apparatus-reference-kv/ui` : décrire seulement si le code existe ; ne pas appeler ça le host P5.

## Wiki index

N'ajoute **pas** de stubs `obsidian/.../decisions/0001-apparatus-*`. Tu peux ajouter, en **fin** de `obsidian/AI FOR ALL/projects/manifesto/decisions/index.md`, une section « Vague 2 (extrait Manifesto) » qui pointe vers `docs/adr/0401` et `0406` **sans** toucher le tableau Vague 1.

## Format

Template. Date `2026-09-12`. Ne pas modifier README (déjà catalogué). Ne pas modifier 0001–0005.

## Interdit

Code applicatif. Fusionner les 4 services en un seul ADR. Présenter le monolithe comme remplaçant les microservices (c'est dual).
