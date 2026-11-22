---
id: STORY-019
title: Account suspension, deletion, and data lifecycle
status: Accepted intent
assignee: []
created_date: '2026-11-22T13:58:29Z'
labels:
  - epic
dependencies: []
documentation:
  - docs/guides/product-story-workflow.md
ordinal: 19000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Purpose

Keep account suspension, reactivation, and deletion coherent across AIForAll services. Each affected service must account for the data, rights, and community entities it owns or projects. Suspension is reversible and distinct from definitive deletion.

## Linked stories

- [STORY-015: Resolve community responsibilities before deletion](story-015%20-%20Resolve-my-community-responsibilities-before-deleting-my-account.md). Production prerequisite for deletion: indispensable responsibilities are effectively covered or the entity is archived before the account becomes inaccessible.
- [STORY-016: Handle messages and contributions](story-016%20-%20Handle-my-messages-and-contributions-on-account-deletion.md). Defines what is erased or retained on definitive deletion, not while deletion is pending or the account is suspended.
- [STORY-017: Plan responsibilities across organizations and projects](story-017%20-%20Plan-my-responsibilities-across-organizations-and-projects.md). Covers the choices and dependencies the person sees before suspension or deletion; complements STORY-015.
- [STORY-018: Choose which projects to reopen](story-018%20-%20Choose-which-projects-to-reopen-after-unarchiving-an-organization.md). Covers explicit project reopening after organization unarchival, not account reactivation.

## Ongoing scope

Keep this epic open as services are added. For each new service, review suspension, reactivation, pending deletion, cancellation, and definitive erasure of account-linked data and rights. Record an explicit no-impact decision where applicable. Each release has its own bounded service scope and verifiable outcomes; finishing these stories does not close the epic.

This entry groups product intent; it does not approve an end-to-end release. The IAM suspension and deletion journeys still need reconciliation, and support monitoring belongs to a separate operations follow-up.
<!-- SECTION:DESCRIPTION:END -->
