# Hive pagination property-test pilot

This standalone Windows-native experiment checks a property of Hive's
`PaginationResponse::new`: filling pages exactly leaves no next page; adding
one item adds exactly one page, unless the `u32` page count is already
saturated. It uses one property, 512 generated cases, and seed `20261124`.

From the repository root in PowerShell, using a clone with full history:

```powershell
.\experiments\hive-pagination-proptest\export-sources.ps1
cargo test --locked --manifest-path experiments/hive-pagination-proptest/Cargo.toml --lib
cargo test --locked --manifest-path experiments/hive-pagination-proptest/Cargo.toml --lib --features historical
```

The exporter copies the DTO and its implementation from two pinned Git blobs
into ignored files. The first test should pass. The second should **fail**:
the historical floating-point calculation drops one page at
9,007,199,254,740,993 items with a page size of 4,194,304, returning
2,147,483,648 pages instead of 2,147,483,649. That counterexample exercises the
DTO's numeric range; it does not establish a failure in normal API use.

This experiment does not test the Hive service or HTTP endpoints. It does not
change the workspace lockfile, require Docker, or add a production dependency.
