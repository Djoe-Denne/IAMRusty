# Cargo Fmt Pre-Commit Hook

Use the repository hook in `.githooks/pre-commit` to block commits when Rust
formatting is out of date.

## One-Time Setup

From the repository root, point Git at the versioned hooks directory:

```bash
git config core.hooksPath .githooks
```

This setting is local to your clone.

On macOS or Linux, make the hook executable if Git reports that it was ignored:

```bash
chmod +x .githooks/pre-commit
```

## What The Hook Checks

Before each commit, Git runs:

```bash
cargo fmt --all -- --check
```

If formatting is wrong, the commit stops. Fix it with:

```bash
cargo fmt
```

Then stage the formatted files and commit again.

## Manual Verification

Run the same check any time:

```bash
cargo fmt --all -- --check
```
