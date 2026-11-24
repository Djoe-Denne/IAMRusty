---
description: Plan, launch, or resume approved AIForAll lots in independent Build sessions
subagent: false
---

Load `aiforall-parallel-delivery`; if it is not exposed, read
`.agents/skills/aiforall-parallel-delivery/SKILL.md`. Treat `$ARGUMENTS` as
story IDs or a private wave ledger reference, not as shell text. If missing,
ask which approved stories or existing wave to use. Plan, launch, or resume
only within the delivery, agent and infrastructure scope the user authorized.

For an authorized launch, use **one independent OpenCode session with
`agent: "build"` and no `parentID` per admitted lot**, in that lot's worktree.
Follow the skill's native session preflight before sending work. This applies
also to design-only lots. Do not use the subagent tool or an `architecte`,
`implementer`, or other profile as a lot owner. A specialist assignment
from the primary session is separate from a Build slot. If independent
session creation is unavailable, report the blocker; do not substitute
child sessions. Monitor the recorded Build session IDs from this session.
