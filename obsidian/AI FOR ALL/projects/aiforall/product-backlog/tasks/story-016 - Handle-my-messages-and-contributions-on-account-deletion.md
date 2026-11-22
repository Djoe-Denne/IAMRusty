---
id: STORY-016
title: Handle my messages and contributions on account deletion
status: Accepted intent
assignee: []
created_date: '2026-11-22T12:29:38Z'
updated_date: '2026-11-22T13:58:36Z'
labels: []
dependencies: []
references:
  - >-
    obsidian/AI FOR ALL/projects/aiforall/product-backlog/tasks/story-019 -
    Account-suspension-deletion-and-data-lifecycle.md
documentation:
  - docs/guides/product-story-workflow.md
ordinal: 16000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Goal

A person permanently deleting their AIForAll account can have their individual messages and unincorporated individual work erased without taking away other people's replies, co-authored work, or contributions already incorporated into a shared project.

## Scope

This is approved product intent v1, not a delivery commitment. It does not define IAM profile erasure, suspension attribution, or the handling of community resources and responsibilities (STORY-015). Removing identifying account attribution does not make retained shared content anonymous. Targeted requests about personal details in shared content are separate.

Quoted copies in other people's messages, attachments used elsewhere, and external copies are not specified here. This story introduces no retention exception for erased IAM data.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 E1 - A pending deletion request does not erase messages or contributions. Only definitive deletion triggers these content rules.
- [ ] #2 E2 - On definitive deletion, erase the person's own message text and attachments while leaving other people's replies in place. Where a placeholder is needed, show Deleted message without identifying attribution or the erased content.
- [ ] #3 E3 - Erase solely authored drafts or pending proposals not yet incorporated into a project. Do not erase other people's replies or co-authored work under this rule.
- [ ] #4 E4 - Keep work already incorporated into a shared project. Remove identifying attribution and the link to the deleted account. If a label is needed, use Deleted account.
- [ ] #5 E5 - Do not automatically inspect, edit, or hide shared content itself, even if it contains the person's own contact details. Do not report retained content as anonymized.
<!-- AC:END -->
