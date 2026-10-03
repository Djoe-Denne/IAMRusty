---
description: Apply project routing without recursion; accept orchestrator or an authorized emergency-thinker takeover as active control plane.
mode: subagent
---

Harness rule for the root agent of an OpenCode session on this repository.

If `orchestrator` is already the primary/default agent, it is the control plane: handle the request directly and do not delegate to another `orchestrator` subagent. Never auto-delegate or recursively invoke `orchestrator`.

An authorized `emergency-thinker` takeover is the only alternate control plane: authorization comes from the orchestrator's stalled-task handoff or explicit user selection as primary. Accept it directly; never bounce it back through this harness to orchestrator. It may coordinate, investigate and patch within the closed contract, and owns final validation until the user ends or reassigns takeover.

If a different ordinary agent is primary (not an authorized thinker), it must delegate the complete project request to `orchestrator` before editing code, designing a solution, investigating the repository, running a substantial workflow, or giving a project-level technical recommendation. The root must not independently implement, plan, or architect when `orchestrator` can be invoked.

When the active control plane is a subagent at the effective nesting limit of 1, it must return a `WORKER_REQUEST` with the chosen `agent`, `description`, complete work package, and required validation. The root relays only that choice/package and reports evidence to the active control plane; no substitution, depth retries or permission bypass. Initially orchestrator retains final validation. After a takeover request naming emergency-thinker, relay to that thinker and subsequently relay its worker requests/results; the original orchestrator/root stop redundant technical analysis and final validation. Workers remain leaves except this authorized thinker, whose delegation remains subject to runtime depth limits. Never invoke orchestrator/self/another thinker recursively.

Return the active control plane's final result to the user. Add root-level content only when necessary for communication. Default routing remains orchestrator; this protocol does not change default_agent, nesting settings or session model selection.

Review findings are persisted under `.cursor/review-briefings/` (gitignored except README + TEMPLATE). The orchestrator must write/confirm a briefing after every review and pass those paths to implementers. Implementers must read matching briefings before exploring. Do not treat chat-only review output as sufficient.

If neither an authorized thinker nor a primary/invokable orchestrator is available, say so explicitly. Do not silently bypass the harness.

Exception: purely conversational messages that do not concern this project may be answered directly, so the harness does not loop.
