---
name: rustycog-submodule
description: >-
  Pin, bump, and consume the Djoe-Denne/rustycog git submodule so this
  workspace and GitHub CI resolve the same rustycog-framework commit. Use when
  cloning the repo, cargo fails to read rustycog/Cargo.toml, the user mentions
  rustycog submodule, pin, bump, gitignored rustycog copy, path patch
  rustycog-framework, sibling repos/rustycog, submodules: true, recurse-submodules,
  or a local-vs-CI rustycog discrepancy.
---

# RustyCog submodule

This repo does **not** vendor rustycog and does **not** keep a gitignored fork.
`rustycog/` is a git submodule of https://github.com/Djoe-Denne/rustycog
(`160000` gitlink). Cargo consumes it via the workspace patch:

```toml
[patch.crates-io]
rustycog-framework = { path = "rustycog" }
```

Service crates still depend on crates.io `rustycog-framework` `0.1.1`. The
patch overrides that with the submodule tree (needed because crates.io still
ties `rdkafka` to `events`/`full`).

For crate APIs and service wiring, use `.agents/skills/rustycog/SKILL.md`.
This skill is only the pin.

## When to use

- Init / clone / CI cannot find `rustycog/Cargo.toml`
- Bump the SDK after a rustycog push
- Someone recreated a local copy under `rustycog/` or pointed the patch at `../rustycog`
- Local build and GitHub CI disagree on rustycog behavior

## Two working trees

| Tree | Role |
|---|---|
| Sibling `../rustycog` (usually `C:/Users/djden/source/repos/rustycog`) | Develop and publish the SDK. Own git remote `Djoe-Denne/rustycog`. |
| This repo's `rustycog/` | **Pinned checkout** of a rustycog commit. Detached HEAD is normal. Serena/GrepAI index it. |

Never treat `AIForAll/rustycog` as a second fork. Edits there are lost on the
next `submodule update` unless they are committed **in the rustycog repo** and
the gitlink here is bumped.

## Clone and init

```bash
git clone --recurse-submodules <this-repo-url>
# already cloned:
git submodule update --init rustycog
```

Do not `git clone` rustycog into `rustycog/` by hand and do not add `rustycog/`
back to `.gitignore`.

## Develop rustycog, then pin

1. Work in the **sibling** rustycog repo (not the submodule).
2. Commit and push to `Djoe-Denne/rustycog` (`main`).
3. In **this** repo, move the gitlink and commit it:

```bash
git submodule update --remote rustycog
git add rustycog
git status   # rustycog must show as a gitlink (mode 160000), not as regular files
git commit -m "chore: bump rustycog submodule"
```

`git ls-files -s rustycog` must start with `160000`. If you see thousands of
new files under `rustycog/`, you added a copy instead of a submodule — stop
and fix.

Do not bump the pin until the rustycog commit is on GitHub. CI checks out
the recorded SHA.

## CI contract

`.github/workflows/ci.yml` uses a single checkout:

```yaml
- uses: actions/checkout@v4
  with:
    submodules: true
```

Sonar also sets `fetch-depth: 0`. Do **not** add a second
`actions/checkout` of `Djoe-Denne/rustycog` into `path: rustycog` — that
fights the submodule.

## Forbidden

- Gitignored or ad-hoc copy at `rustycog/`
- `[patch.crates-io]` path `../rustycog` (breaks CI)
- Dropping the path patch without a crates.io release that keeps `rdkafka` off `full`
- Committing submodule file contents as normal files
- `git submodule add` while a leftover `rustycog/` directory still exists — delete the copy first

## Recover a missing checkout

```bash
git submodule update --init rustycog
test -f rustycog/Cargo.toml
```

If `git submodule status` is empty, restore `.gitmodules`:

```
[submodule "rustycog"]
	path = rustycog
	url = https://github.com/Djoe-Denne/rustycog.git
```

then `git submodule update --init rustycog`.
