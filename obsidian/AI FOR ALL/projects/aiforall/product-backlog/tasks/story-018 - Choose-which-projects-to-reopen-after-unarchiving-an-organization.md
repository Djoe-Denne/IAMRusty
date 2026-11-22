---
id: STORY-018
title: Choose which projects to reopen after unarchiving an organization
status: Accepted intent
assignee: []
created_date: '2026-11-22T12:55:27Z'
updated_date: '2026-11-22T13:58:36Z'
labels: []
dependencies: []
references:
  - >-
    obsidian/AI FOR ALL/projects/aiforall/product-backlog/tasks/story-019 -
    Account-suspension-deletion-and-data-lifecycle.md
documentation:
  - docs/guides/product-story-workflow.md
ordinal: 18000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Goal

As a person authorized to unarchive an organization, I can decide which of its projects to reopen after unarchiving the organization, without accidentally restoring a project archived for another reason.

## Scope

This story covers reopening projects disabled by organization archival. It complements the choices made in [STORY-017](story-017%20-%20Plan-my-responsibilities-across-organizations-and-projects.md). A dialog and a dedicated page are both options. Account reactivation is not organization unarchival. The broader entity-management page and support monitoring are separate.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 E1 - Explicitly unarchiving an organization does not automatically reopen its projects. Prompt the person to choose which projects to unarchive, or let them leave without changing any project.
- [ ] #2 E2 - The person can explicitly unarchive an eligible project individually when authorized to do so.
- [ ] #3 E3 - Unarchive all applies only to projects disabled by the organization archive that the person is authorized to unarchive. A project archived independently remains archived and requires a separate action.
- [ ] #4 E4 - Reactivating a user account alone does not unarchive an organization or any of its projects.
<!-- AC:END -->
