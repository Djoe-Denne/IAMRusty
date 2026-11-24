---
title: ADR 0316 — Password recovery session liveness
category: decisions
status: proposed
feature_status: unimplemented
---

# ADR 0316 — Password recovery session liveness

[Read the proposed ADR](https://github.com/Djoe-Denne/IAMRusty/blob/master/docs/adr/0316-password-recovery-session-liveness.md)
and its [observable contract](https://github.com/Djoe-Denne/IAMRusty/blob/master/docs/contracts/password-recovery-session-liveness.md).
The proposal uses an authoritative account epoch to deny prior access and
renewal after a successful password recovery. Publication does not ratify it
or authorize implementation.
