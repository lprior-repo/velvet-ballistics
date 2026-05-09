# Manual QA Smoke Report: vb-qi37.2.1

## Bead Info
- **ID**: vb-qi37.2.1
- **Title**: runtime: Define aggregate resource budget model
- **Status**: IN_PROGRESS
- **QA Date**: 2026-05-09

---

## Command Executed

```bash
cargo nextest run -p vb_core --test aggregate_resource_budget_red --no-fail-fast
```

---

## Output

```
warning: skipping duplicate package `bitflags v2.10.0 (https://github.com/makepad/makepad?branch=dev#20b6c53b)`:
  /cache/cargo-shared/git/checkouts/makepad-ec2f134f34cd9f98/20b6c53/libs/vulkan/bitflags/Cargo.toml
  -> /cache/cargo-shared/git/checkouts/makepad-ec2f134f34cd9f98/20b6c53/libs/bitflags/Cargo.toml

warning: /home/lewis/src/Velvet-ballistics/crates/velvet_ballastics/Cargo.toml: file `/home/lewis/src/Velvet-ballistics/crates/velvet_ballastics/src/main.rs` found to be present in multiple build targets:
  * `bin` target `vb`
  * `bin` target `velvet-ballistics`

error: no test target named `aggregate_resource_budget_red` in `vb_core` package

help: available test targets:
    phase1_core_types
    proptest_core_types
    section36_mandatory_coverage
    section38_behavioral_properties

error: command `/home/lewis/.rustup/toolchains/nightly-2026-04-28-x86_64-unknown-linux-gnu/bin/cargo test --no-run --message-format json-render-diagnostics --package vb_core --test aggregate_resource_budget_red` exited with code 101

EXIT_CODE: 101
```

---

## Findings

| Check | Result |
|-------|--------|
| Command exit code (expected 0) | 101 (FAIL) |
| Test target exists | NO |
| stderr output | YES (error message) |
| Stack trace in output | NO |

### Available Test Targets in vb_core
- `phase1_core_types`
- `proptest_core_types`
- `section36_mandatory_coverage`
- `section38_behavioral_properties`

The requested test `aggregate_resource_budget_red` does not exist in the vb_core package.

---

## Notes
- Bead directory `.beads/vb-qi37.2.1/` was not checked out; files (contract.md, test-plan.md, implementation.md) not present
- Test name may have been renamed or not yet implemented

STATUS: FAIL
