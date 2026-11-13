# Manifesto IT CI repair — QA proportionate (approved)

- RED: failed GitHub run43473245852/job132053039776, T6 POST404vs201 and Tokio double-poll panic. Wiremock explicitly receives the application POST on cached server port39407. Do not weaken HTTP assertions or simulate trusted authentication.
- SDK regression: hold cached old ephemeral port as a fixture listener, resolve next server port and assert different/nonzero/bindable. Preserve configured fixed port even occupied: fail at bind, not silent retarget. Store selected URL on running instance for reuse and pass resolved ServerConfig through descriptor startup. Public spawn_test_server API preserved; no production cache policy change.
- Supervisor regressions: consumed successful/error handles not awaited twice; unselected finished handle drained; unselected pending task aborted and completion observed. Caller select flags reviewed separately; helpers alone do not prove wiring.
- Native Windows, one Cargo, configured24jobs/targetE. SDK filtered tests, Manifesto setup supervisor tests, check/fmt and targeted Clippy as justified. SDK sources developed in sibling repo, published before bumping the gitlink.
- Real IT: existing it-native lifecycle harness, suite manifesto / exact binary apparatus_p1_t6_http newly allowlisted with pure selection regression. All4T6cases must pass, no skip/assertion change. No new harness. Record fixture ledger IDs/cleanup, preserve foreign runtime; no Kind.
- Publish SDK main, verify remote, update AIForAll pin then publish master; follow corrective CI and report actual Manifesto IT/global outcomes separately. Unrun/error-environment is not PASS or functional RED.
- No mandatory agent chain. Direct execution approved, no workers/team/worktrees created. No architectural public API/ADR change; internal test-harness and supervisor bugfix only.

## SDK CI followups discovered by control
- SDK Format initially failed on4preexisting JWKS assertion-format hunks: whitespace-only change, all-format check and nonwhitespace equality verified, no assertion/logic change.
- SDK unit CI136/137passed; existing process-exit parent spawned an incomplete --exact child name,zero tests then missing ledger. Local targeted RED reproduced, exactchildname corrected, assert stdout reports exactly1test added. Full native SDK --lib full,testing now132/132PASS,0ignored/filtered; Kafka remains disabled on Windows. This hermetic exit test is not real Docker teardown evidence.
- Reuse Manifesto supervisor/T6 evidence when SDK delta is confined to cfg(test) and formatting; no production dependency code change. Final SDK/root publication and CI controls remain required.
