# Coverage 92% — Manifesto runtime HTTP

Three integration tests exercise all eight runtime routes through the
in-process Axum router, real command service and PostgreSQL writer. They
assert binding pagination and each instance selector, exact error envelopes
for invalid requests and role denials, persisted observation and revocation,
and a single effect reservation and completion despite replay attempts.
The original writer tests keep their existing assertions. Only the origin
verdict is substituted; the tests do not qualify mTLS, network transport,
Kubernetes, or the production composition root.

All 13 cases passed in `runtime_writer_postgres` on Windows with LLVM
instrumentation. The existing launcher removed both owned fixture
containers and preserved the container inventory. No runtime or Sonar
configuration was changed.

## New-code coverage measurement

The pinned SonarCloud analysis `1d289873-4126-45f5-a489-102b1670ce10`
on `83695bf90ce3eade909260540436d9a4c1de5efe` had 29,314 executable new
lines and 3,520 uncovered, with no measured conditions: **25,794 /
29,314 = 87.99%**. The overall-code metric uses a separate denominator.

Match the native LCOV export's positive counts to unchanged production
coordinates that Sonar marked both new and uncovered:

- HTTP handlers `runtime_control.rs`: **144** additional lines.
- HTTP middleware `runtime_service_origin.rs`: **10**.
- Application use case `runtime_control.rs`: **74**.
- Command dispatch `runtime_control_command.rs`: **18**.

The **246** matched lines project **88.83%** (26,040 / 29,314) at the
same denominator. That leaves **343** additional covered new lines for
90%, or **929** for 92%. This is local evidence, not a new hosted
analysis or a Quality Gate result.
