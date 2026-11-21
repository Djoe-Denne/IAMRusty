# AIForAll

[![Quality Gate](https://sonarcloud.io/api/project_badges/measure?project=Djoe-Denne_IAMRusty&metric=alert_status)](https://sonarcloud.io/dashboard?id=Djoe-Denne_IAMRusty)
[![Coverage](https://sonarcloud.io/api/project_badges/measure?project=Djoe-Denne_IAMRusty&metric=coverage)](https://sonarcloud.io/component_measures?id=Djoe-Denne_IAMRusty&metric=coverage)
[![Reliability](https://sonarcloud.io/api/project_badges/measure?project=Djoe-Denne_IAMRusty&metric=reliability_rating)](https://sonarcloud.io/dashboard?id=Djoe-Denne_IAMRusty)
[![Security](https://sonarcloud.io/api/project_badges/measure?project=Djoe-Denne_IAMRusty&metric=security_rating)](https://sonarcloud.io/dashboard?id=Djoe-Denne_IAMRusty)
[![Maintainability](https://sonarcloud.io/api/project_badges/measure?project=Djoe-Denne_IAMRusty&metric=sqale_rating)](https://sonarcloud.io/dashboard?id=Djoe-Denne_IAMRusty)
[![Duplicated Lines](https://sonarcloud.io/api/project_badges/measure?project=Djoe-Denne_IAMRusty&metric=duplicated_lines_density)](https://sonarcloud.io/dashboard?id=Djoe-Denne_IAMRusty)
[![CI](https://github.com/Djoe-Denne/IAMRusty/actions/workflows/ci.yml/badge.svg?branch=master)](https://github.com/Djoe-Denne/IAMRusty/actions/workflows/ci.yml)
[![E2E](https://img.shields.io/endpoint?url=https%3A%2F%2Fdjoe-denne.github.io%2FIAMRusty%2Fbadge.json)](https://djoe-denne.github.io/IAMRusty/)

AIForAll is a platform for finding, sharing, and collaborating on projects.
The goal is to let people start projects, discover work they can contribute
to, and collaborate on behalf of organizations. Reading a project, proposing
a contribution, and changing the project are distinct actions.

This repository provides the Rust backend and deployment tooling, not a
general-purpose web UI. The APIs already support accounts, organizations,
and public project discovery. A structured contribution proposal and review
workflow is not implemented.

## What is available today

- [**IAMRusty**](docs/functional/identite.md) (`/iam`): platform accounts,
  email signup and verification, password recovery, and token issuance.
- [**Hive**](docs/functional/organisation.md) (`/hive`): organizations,
  membership, roles, and invitations.
- [**Manifesto**](docs/functional/projet.md) (`/manifesto`): project creation,
  visibility, publication and archiving, member permissions, and component
  attachments. Public projects can be discovered without an account.
  Self-joining an active public project grants read access, not editing rights.
- [**Telegraph**](docs/functional/notifications.md) (`/telegraph`): email
  and in-app notifications, including messages triggered by account events.
- [**Lazaret**](services/Lazaret/README.md) (`/lazaret`): workload identity and
  scoped access to platform capabilities such as key-value storage. It is
  separate from Manifesto's project and component management.
- [**OAuthConnect**](docs/services/oauth-connect.md): a standalone
  federated sign-in service with GitHub OAuth and GitLab OIDC profiles.
  IAMRusty still owns platform accounts and issues platform tokens.
- [**sentinel-sync**](workers/sentinel-sync/README.md): a separate worker that
  projects domain events into OpenFGA authorization relationships.

The five core APIs can run as standalone services or together behind one
listener in the [modular monolith](runtime/monolith/README.md), with the same
route prefixes. OAuthConnect and sentinel-sync run separately. See the
[architecture guide](docs/architecture/README.md) for service boundaries
and deployment diagrams.

## Dependencies

- **Build:** Git and Rustup. [`rust-toolchain.toml`](rust-toolchain.toml) pins
  Rust, `rustfmt`, and `clippy`. The shared
  [RustyCog SDK](https://github.com/Djoe-Denne/rustycog) is pinned as a Git
  submodule at `rustycog/`. The repository's Git hooks use Git LFS.
- **Application runtime:** PostgreSQL for persistent data, OpenFGA for
  [relationship-based authorization](docs/platform/authz-openfga.md),
  and SQS-compatible queues for [domain events and outbox delivery](docs/platform/events-outbox.md).
  LocalStack supplies SQS in the local stack. The local stack also provisions
  OpenBao for services that need it.
- **Local environments:** Docker supplies containers for integration tests;
  Docker Compose starts dependencies for a host run. The Windows host
  launchers use `just` and PowerShell. The
  [Kind deployment](ops/deploy/README.md) runs Linux images in Kubernetes
  and uses Helm and `kubectl`; Envoy handles edge routing and TLS. Federated
  sign-in requires separately supplied [OAuthConnect configuration](docs/services/oauth-connect.md).

## Get started

```sh
git clone --recurse-submodules https://github.com/Djoe-Denne/IAMRusty.git AIForAll
cd AIForAll
rustup show active-toolchain
cargo check --locked -p oodhive-monolith
```

Build and run development tests natively on the host. The checked-in
[Cargo configuration](.cargo/config.toml) defaults to
`E:\cargo-target\AIForAll` for Windows; set `CARGO_TARGET_DIR` to a writable
local path if that drive is unavailable or you are on a different platform.

To run the backend locally on Windows, use the
[monolith host guide](runtime/monolith/README.md): start the dependencies
with `just up-infra`, run `just monolith`, then start `just sentinel-sync`
in another terminal. The monolith serves `/health` and `/ready` on port
`8080` by default. For a local Kubernetes environment, follow the
[Kind operator guide](ops/deploy/kind-minimal.md) instead of starting
the entire Compose stack.

## Test and contribute

```sh
cargo test --locked -p hive-domain --lib
cargo fmt --all -- --check
```

[Integration tests](docs/guides/tests-integration.md) run the Rust binaries
on the host and use testcontainers for the dependencies they need. The
[local k6 campaign](ops/tests/k6/native/README.md) exercises HTTPS readiness,
browser OAuth, and Envoy certificate identity. The
[published CI E2E report](https://djoe-denne.github.io/IAMRusty/) identifies
the commit it tested and is separate from the local campaign and code coverage.

- [Handbook](docs/README.md) and [architecture decisions](docs/adr/README.md)
- [Repository security](docs/guides/repository-security.md) and
  [development conventions](AGENTS.md)
- [Product story workflow](docs/guides/product-story-workflow.md): work through
  examples before inspecting implementation, then capture approved stories in
  Backlog.md. The same story files are readable through Obsidian and QMD.

## Acknowledgements

The agent QA workflow draws on [qa-skills](https://github.com/petrkindlmann/qa-skills),
[awesome-qa-skills](https://github.com/naodeng/awesome-qa-skills),
[agentic-qe](https://github.com/proffesor-for-testing/agentic-qe),
[OpenTestingAI](https://github.com/AgenticTesting/OpenTestingAI),
and [Playwright Test Agents](https://playwright.dev/docs/test-agents).
