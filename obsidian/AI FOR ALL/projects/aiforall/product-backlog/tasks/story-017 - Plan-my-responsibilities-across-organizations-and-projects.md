---
id: STORY-017
title: Plan my responsibilities across organizations and projects
status: Accepted intent
assignee: []
created_date: '2026-11-22T12:55:14Z'
updated_date: '2026-11-22T13:58:36Z'
labels: []
dependencies: []
references:
  - >-
    obsidian/AI FOR ALL/projects/aiforall/product-backlog/tasks/story-019 -
    Account-suspension-deletion-and-data-lifecycle.md
documentation:
  - docs/guides/product-story-workflow.md
ordinal: 17000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Goal

As a person preparing to suspend or delete my AIForAll account, I can review my responsibilities across organizations and their projects and choose a consistent plan so no active entity loses a person able to exercise an indispensable responsibility.

## Scope

This story records the agreed planning rules for organizations and their own projects. It complements STORY-015, which defines succession and archival before deletion. It does not define a new account state, change the deletion window, or approve the entire account lifecycle epic for delivery. Partner and multi-parent relationships, permanent failure handling, and the broader entity-management page are outside this story. Support monitoring is separate.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 E1 - Show affected organizations and the projects that belong to them together. If another authorized administrator already covers an organization responsibility, require no decision for it; still flag any project whose own indispensable responsibility would be left uncovered.
- [ ] #2 E2 - For an uncovered project, suggest eligible administrators of its organization first. If several are eligible, leave the choice unselected. A suggestion grants no rights and never replaces an explicit selection.
- [ ] #3 E3 - If the organization nominee has effective inherited rights sufficient to cover a project responsibility, show it as covered without adding a direct grant. If inherited rights are insufficient, offer the same person for that project only if eligible.
- [ ] #4 E4 - A bulk nomination applies only to eligible projects in the plan with no relay selected. Replacing a selected relay requires an explicit action for that project.
- [ ] #5 E5 - Selecting archival of an organization shows which of its projects will be disabled and removes their relay selections from the plan without reserving them for later or revoking grants already in effect.
- [ ] #6 E6 - Before suspending the account or making it inaccessible for pending deletion, require effective coverage or completed archival for each entity with an indispensable responsibility at risk. An unanswered check or queued change is not enough; keep the request in preparation.
- [ ] #7 E7 - Once the account can no longer act, it cannot create a project or move one to a different organization. A project created later by another person receives no direct nomination from the previous plan; authority may still be inherited under the normal organization rules. A move by another authorized administrator does not reopen an organization responsibility already covered.
<!-- AC:END -->
