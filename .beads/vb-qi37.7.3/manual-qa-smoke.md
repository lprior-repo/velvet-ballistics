# Manual QA Smoke: vb-qi37.7.3

STATUS: PASS

## Scope

Hands-on/manual QA smoke test for Rust API validation behavior implemented in `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25`.

Required artifacts read before execution:

- `.beads/vb-qi37.7.3/STATE.md`
- `.beads/vb-qi37.7.3/contract.md`
- `.beads/vb-qi37.7.3/test-plan.md`
- `.beads/vb-qi37.7.3/implementation.md`
- `crates/vb_core/tests/vb_qi37_7_3_red.rs`
- `crates/vb_validate/tests/vb_qi37_7_3_red.rs`

## Commands Run With Outcomes

### 1. Full focused nextest red suite

Command:

```bash
cargo nextest run -p vb_core -p vb_validate --test vb_qi37_7_3_red; printf '\nexit: %s\n' "$?"
```

Outcome: PASS, exit code 0.

Captured stdout/stderr:

```text
warning: skipping duplicate package `bitflags v2.10.0 (https://github.com/makepad/makepad?branch=dev#20b6c53b)`:
  /cache/cargo-shared/git/checkouts/makepad-ec2f134f34cd9f98/20b6c53/libs/vulkan/bitflags/Cargo.toml
in favor of /cache/cargo-shared/git/checkouts/makepad-ec2f134f34cd9f98/20b6c53/libs/bitflags/Cargo.toml

warning: /home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/velvet_ballastics/Cargo.toml: file `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/velvet_ballastics/src/main.rs` found to be present in multiple build targets:
  * `bin` target `vb`
  * `bin` target `velvet-ballistics`
    Blocking waiting for file lock on package cache
warning: skipping duplicate package `bitflags v2.10.0 (https://github.com/makepad/makepad?branch=dev#20b6c53b)`:
  /cache/cargo-shared/git/checkouts/makepad-ec2f134f34cd9f98/20b6c53/libs/vulkan/bitflags/Cargo.toml
in favor of /cache/cargo-shared/git/checkouts/makepad-ec2f134f34cd9f98/20b6c53/libs/bitflags/Cargo.toml

    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on build directory
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.10s
────────────
 Nextest run ID e196dcf8-00e6-4e05-b365-fbbcc31f0749 with nextest profile: default
    Starting 16 tests across 2 binaries
        PASS [   0.002s] ( 1/16) vb_core::vb_qi37_7_3_red symbol_constant_equal_to_symbols_count_returns_symbol_out_of_bounds
        PASS [   0.002s] ( 2/16) vb_core::vb_qi37_7_3_red zero_symbols_rejects_build_object_symbol_zero
        PASS [   0.002s] ( 3/16) vb_validate::vb_qi37_7_3_red validate_action_references_returns_orphan_contract_when_contract_id_is_unreferenced
        PASS [   0.002s] ( 4/16) vb_core::vb_qi37_7_3_red validate_resource_references_returns_resource_contract_too_large_when_declared_max_steps_exceeds_hard_limit
        PASS [   0.002s] ( 5/16) vb_core::vb_qi37_7_3_red zero_symbols_rejects_accessor_symbol_zero
        PASS [   0.002s] ( 6/16) vb_core::vb_qi37_7_3_red build_object_field_symbol_equal_to_symbols_count_returns_symbol_out_of_bounds
        PASS [   0.002s] ( 7/16) vb_validate::vb_qi37_7_3_red orphan_action_contract_reports_first_orphan_in_supplied_contract_order
        PASS [   0.002s] ( 8/16) vb_validate::vb_qi37_7_3_red validate_returns_success_for_do_action_without_contract_when_non_action_gates_pass
        PASS [   0.002s] ( 9/16) vb_core::vb_qi37_7_3_red zero_symbols_rejects_constant_symbol_zero
        PASS [   0.021s] (10/16) vb_core::vb_qi37_7_3_red validate_symbol_references_returns_unit_when_all_symbol_carriers_are_in_bounds
        PASS [   0.021s] (11/16) vb_core::vb_qi37_7_3_red validate_resource_references_returns_resource_contract_exceeded_when_node_count_exceeds_max_steps
        PASS [   0.021s] (12/16) vb_validate::vb_qi37_7_3_red validate_action_references_returns_unit_when_unique_do_actions_equal_unique_contract_ids
        PASS [   0.022s] (13/16) vb_core::vb_qi37_7_3_red validate_resource_references_returns_unit_when_declared_and_actual_resources_are_within_limits
        PASS [   0.022s] (14/16) vb_core::vb_qi37_7_3_red validate_symbol_references_returns_symbol_out_of_bounds_when_accessor_field_equals_symbols_count
        PASS [   0.022s] (15/16) vb_validate::vb_qi37_7_3_red validate_with_contracts_returns_missing_contract_when_do_action_has_no_contract
        PASS [   0.022s] (16/16) vb_validate::vb_qi37_7_3_red validate_action_references_returns_missing_contract_when_do_action_has_no_contract
────────────
     Summary [   0.022s] 16 tests run: 16 passed, 0 skipped

exit: 0
```

### 2. Core symbol helper happy path

Command:

```bash
rtk cargo test -p vb_core --test vb_qi37_7_3_red validate_symbol_references_returns_unit_when_all_symbol_carriers_are_in_bounds -- --nocapture; printf '\nexit: %s\n' "$?"
```

Outcome: PASS, exit code 0. This compiles and executes the public API `vb_core::workflow::validate_symbol_references` and verifies exact `Ok(())` plus input non-mutation in the test harness.

Captured stdout/stderr:

```text
warning: /home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/velvet_ballastics/Cargo.toml: file `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/velvet_ballastics/src/main.rs` found to be present in multiple build targets:
  * `bin` target `vb`
  * `bin` target `velvet-ballistics`
warning: skipping duplicate package `bitflags v2.10.0 (https://github.com/makepad/makepad?branch=dev#20b6c53b)`:
  /cache/cargo-shared/git/checkouts/makepad-ec2f134f34cd9f98/20b6c53/libs/vulkan/bitflags/Cargo.toml
in favor of /cache/cargo-shared/git/checkouts/makepad-ec2f134f34cd9f98/20b6c53/libs/bitflags/Cargo.toml

    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on build directory
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.13s
     Running tests/vb_qi37_7_3_red.rs (target/debug/deps/vb_qi37_7_3_red-6c5cfff6e9ae7947)
cargo test: 1 passed, 9 filtered out (1 suite, 0.00s)

exit: 0
```

### 3. Validator action helper happy path

Command:

```bash
rtk cargo test -p vb_validate --test vb_qi37_7_3_red validate_action_references_returns_unit_when_unique_do_actions_equal_unique_contract_ids -- --nocapture; printf '\nexit: %s\n' "$?"
```

Outcome: PASS, exit code 0. This compiles and executes the public API `vb_validate::shared::validate_action_references` and verifies exact `Ok(())` for matching unique action IDs and contracts.

Captured stdout/stderr:

```text
warning: /home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/velvet_ballastics/Cargo.toml: file `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/velvet_ballastics/src/main.rs` found to be present in multiple build targets:
  * `bin` target `vb`
  * `bin` target `velvet-ballistics`
    Blocking waiting for file lock on package cache
warning: skipping duplicate package `bitflags v2.10.0 (https://github.com/makepad/makepad?branch=dev#20b6c53b)`:
  /cache/cargo-shared/git/checkouts/makepad-ec2f134f34cd9f98/20b6c53/libs/vulkan/bitflags/Cargo.toml
in favor of /cache/cargo-shared/git/checkouts/makepad-ec2f134f34cd9f98/20b6c53/libs/bitflags/Cargo.toml

    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.10s
     Running tests/vb_qi37_7_3_red.rs (target/debug/deps/vb_qi37_7_3_red-22ca40fc3c3bb902)
cargo test: 1 passed, 5 filtered out (1 suite, 0.00s)

exit: 0
```

### 4. Core symbol out-of-bounds failure path

Command:

```bash
rtk cargo test -p vb_core --test vb_qi37_7_3_red validate_symbol_references_returns_symbol_out_of_bounds_when_accessor_field_equals_symbols_count -- --nocapture; printf '\nexit: %s\n' "$?"
```

Outcome: PASS, exit code 0. The harness asserts exact typed error `WorkflowError::SymbolOutOfBounds { symbol: SymbolId::new(1) }`.

Captured stdout/stderr:

```text
warning: /home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/velvet_ballastics/Cargo.toml: file `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/velvet_ballastics/src/main.rs` found to be present in multiple build targets:
  * `bin` target `vb`
  * `bin` target `velvet-ballistics`
warning: skipping duplicate package `bitflags v2.10.0 (https://github.com/makepad/makepad?branch=dev#20b6c53b)`:
  /cache/cargo-shared/git/checkouts/makepad-ec2f134f34cd9f98/20b6c53/libs/vulkan/bitflags/Cargo.toml
in favor of /cache/cargo-shared/git/checkouts/makepad-ec2f134f34cd9f98/20b6c53/libs/bitflags/Cargo.toml

    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on build directory
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.20s
     Running tests/vb_qi37_7_3_red.rs (target/debug/deps/vb_qi37_7_3_red-6c5cfff6e9ae7947)
cargo test: 1 passed, 9 filtered out (1 suite, 0.00s)

exit: 0
```

### 5. Resource declared-too-large failure path

Command:

```bash
rtk cargo test -p vb_core --test vb_qi37_7_3_red validate_resource_references_returns_resource_contract_too_large_when_declared_max_steps_exceeds_hard_limit -- --nocapture; printf '\nexit: %s\n' "$?"
```

Outcome: PASS, exit code 0. The harness asserts exact typed error `WorkflowError::ResourceContractTooLarge { resource: "max_steps" }`.

Captured stdout/stderr:

```text
warning: /home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/velvet_ballastics/Cargo.toml: file `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/velvet_ballastics/src/main.rs` found to be present in multiple build targets:
  * `bin` target `vb`
  * `bin` target `velvet-ballistics`
    Blocking waiting for file lock on package cache
warning: skipping duplicate package `bitflags v2.10.0 (https://github.com/makepad/makepad?branch=dev#20b6c53b)`:
  /cache/cargo-shared/git/checkouts/makepad-ec2f134f34cd9f98/20b6c53/libs/vulkan/bitflags/Cargo.toml
in favor of /cache/cargo-shared/git/checkouts/makepad-ec2f134f34cd9f98/20b6c53/libs/bitflags/Cargo.toml

    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on build directory
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.20s
     Running tests/vb_qi37_7_3_red.rs (target/debug/deps/vb_qi37_7_3_red-6c5cfff6e9ae7947)
cargo test: 1 passed, 9 filtered out (1 suite, 0.00s)

exit: 0
```

### 6. Resource actual-usage-exceeded failure path

Command:

```bash
rtk cargo test -p vb_core --test vb_qi37_7_3_red validate_resource_references_returns_resource_contract_exceeded_when_node_count_exceeds_max_steps -- --nocapture; printf '\nexit: %s\n' "$?"
```

Outcome: PASS, exit code 0. The harness asserts exact typed error `WorkflowError::ResourceContractExceeded { resource: "max_steps" }`.

Captured stdout/stderr:

```text
warning: /home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/velvet_ballastics/Cargo.toml: file `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/velvet_ballastics/src/main.rs` found to be present in multiple build targets:
  * `bin` target `vb`
  * `bin` target `velvet-ballistics`
    Blocking waiting for file lock on package cache
warning: skipping duplicate package `bitflags v2.10.0 (https://github.com/makepad/makepad?branch=dev#20b6c53b)`:
  /cache/cargo-shared/git/checkouts/makepad-ec2f134f34cd9f98/20b6c53/libs/vulkan/bitflags/Cargo.toml
in favor of /cache/cargo-shared/git/checkouts/makepad-ec2f134f34cd9f98/20b6c53/libs/bitflags/Cargo.toml

    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on build directory
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.19s
     Running tests/vb_qi37_7_3_red.rs (target/debug/deps/vb_qi37_7_3_red-6c5cfff6e9ae7947)
cargo test: 1 passed, 9 filtered out (1 suite, 0.00s)

exit: 0
```

### 7. Missing action contract failure path

Command:

```bash
rtk cargo test -p vb_validate --test vb_qi37_7_3_red validate_action_references_returns_missing_contract_when_do_action_has_no_contract -- --nocapture; printf '\nexit: %s\n' "$?"
```

Outcome: PASS, exit code 0. The harness asserts exact typed error `ValidationError::ActionContractMissing { action_id: 7, node_index: 2 }`.

Captured stdout/stderr:

```text
warning: /home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/velvet_ballastics/Cargo.toml: file `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/velvet_ballastics/src/main.rs` found to be present in multiple build targets:
  * `bin` target `vb`
  * `bin` target `velvet-ballistics`
    Blocking waiting for file lock on package cache
warning: skipping duplicate package `bitflags v2.10.0 (https://github.com/makepad/makepad?branch=dev#20b6c53b)`:
  /cache/cargo-shared/git/checkouts/makepad-ec2f134f34cd9f98/20b6c53/libs/vulkan/bitflags/Cargo.toml
in favor of /cache/cargo-shared/git/checkouts/makepad-ec2f134f34cd9f98/20b6c53/libs/bitflags/Cargo.toml

    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on build directory
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.19s
     Running tests/vb_qi37_7_3_red.rs (target/debug/deps/vb_qi37_7_3_red-22ca40fc3c3bb902)
cargo test: 1 passed, 5 filtered out (1 suite, 0.00s)

exit: 0
```

### 8. Orphan action contract failure path

Command:

```bash
rtk cargo test -p vb_validate --test vb_qi37_7_3_red validate_action_references_returns_orphan_contract_when_contract_id_is_unreferenced -- --nocapture; printf '\nexit: %s\n' "$?"
```

Outcome: PASS, exit code 0. The harness asserts exact typed error `ValidationError::ActionContractOrphan { action_id: 9 }`.

Captured stdout/stderr:

```text
warning: /home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/velvet_ballastics/Cargo.toml: file `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/velvet_ballastics/src/main.rs` found to be present in multiple build targets:
  * `bin` target `vb`
  * `bin` target `velvet-ballistics`
    Blocking waiting for file lock on package cache
warning: skipping duplicate package `bitflags v2.10.0 (https://github.com/makepad/makepad?branch=dev#20b6c53b)`:
  /cache/cargo-shared/git/checkouts/makepad-ec2f134f34cd9f98/20b6c53/libs/vulkan/bitflags/Cargo.toml
in favor of /cache/cargo-shared/git/checkouts/makepad-ec2f134f34cd9f98/20b6c53/libs/bitflags/Cargo.toml

    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.18s
     Running tests/vb_qi37_7_3_red.rs (target/debug/deps/vb_qi37_7_3_red-22ca40fc3c3bb902)
cargo test: 1 passed, 5 filtered out (1 suite, 0.00s)

exit: 0
```

## Happy Path Evidence

- `cargo nextest run -p vb_core -p vb_validate --test vb_qi37_7_3_red` ran 16 tests and passed all 16.
- `validate_symbol_references_returns_unit_when_all_symbol_carriers_are_in_bounds` passed, proving the public core symbol helper accepts in-range accessor, constant, and build-object symbol carriers and preserves borrowed input.
- `validate_resource_references_returns_unit_when_declared_and_actual_resources_are_within_limits` passed in the full nextest run, proving the public core resource helper accepts declared and actual resource usage within limits.
- `validate_action_references_returns_unit_when_unique_do_actions_equal_unique_contract_ids` passed, proving the public validator action helper accepts matching unique `Do.action` IDs and supplied `ActionContract.id` values.
- `validate_returns_success_for_do_action_without_contract_when_non_action_gates_pass` passed in the full nextest run, proving default `validate(parts)` still skips action-contract completeness as contracted.

## Failure Path Evidence

- Symbol reference failure path passed with exact typed assertion: `WorkflowError::SymbolOutOfBounds { symbol: SymbolId::new(1) }` for accessor field equal to `symbols_count`.
- Additional symbol carrier failure tests passed in nextest: symbol constant equal to `symbols_count`, build-object field equal to `symbols_count`, and zero-symbol accessor/constant/build-object carriers.
- Resource declared-too-large path passed with exact typed assertion: `WorkflowError::ResourceContractTooLarge { resource: "max_steps" }`.
- Resource actual-usage-exceeded path passed with exact typed assertion: `WorkflowError::ResourceContractExceeded { resource: "max_steps" }`.
- Action missing-contract path passed with exact typed assertion: `ValidationError::ActionContractMissing { action_id: 7, node_index: 2 }` for direct helper and `{ action_id: 7, node_index: 0 }` through `validate_with_contracts` in the full nextest run.
- Action orphan-contract path passed with exact typed assertion: `ValidationError::ActionContractOrphan { action_id: 9 }`.

## Public API Surface Verified

The red test harness imports and executes these public APIs successfully:

- `vb_core::workflow::validate_symbol_references`
- `vb_core::workflow::validate_resource_references`
- `vb_validate::shared::validate_action_references`
- `vb_validate::shared::validate_with_contracts`
- `vb_validate::shared::validate`
- `vb_core::workflow::CompiledWorkflow::try_from_parts`

Compilation plus focused test execution verifies those APIs are available from downstream test crates and return the typed `Result` variants asserted by the harness.

## Blockers / Caveats

- No QA-blocking failure found.
- Cargo emits pre-existing warnings about duplicate `bitflags` package and duplicate binary target source for `vb` / `velvet-ballistics`; these did not affect the focused validation behavior smoke test.
- This was a Rust API/test-harness smoke test, not an end-user binary CLI test; no production code or tests were modified.
