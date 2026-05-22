# Test Plan: vb-qi37.26.1 — Fix vb_ipc Typed Handler Compile Errors

## Summary
- **Bead ID:** vb-qi37.26.1
- **Type:** Compile-fix prerequisite bead
- **Test approach:** Green-first (fix already applied in commit `0ebc5270`; current code compiles cleanly)
- **Behaviors identified:** 7
- **Trophy allocation:** 0 unit / 0 integration / 0 e2e / 7 static analysis
  - *Rationale:* This is a compile-fix bead. No runtime behavior changes. All guarantees are enforced through compilation gates (`cargo check`, `cargo clippy`) and static scan (`grep`). Deviating from the standard 60/30/5/5 ratio is justified because the contract is purely about type-correct compilation and safety-preservation.
- **Proptest invariants:** 0 (no new pure functions introduced)
- **Fuzz targets:** 0 (no new parsing/deserialization boundaries)
- **Kani harnesses:** 0 (waived — no algorithmic logic to verify)
- **Mutation testing:** Explicitly forbidden for this lifecycle phase
- **Red Queen:** Explicitly forbidden for this lifecycle phase

---

## 1. Behavior Inventory

| ID | Behavior |
|----|----------|
| B1 | `vb_ipc` crate compiles without errors when checked |
| B2 | `velvet-ballastics-workspace-tests` compiles with `--tests` target |
| B3 | `vb_ipc` crate passes clippy with warnings-as-errors |
| B4 | Changed code in `handlers.rs` introduces no new panicking APIs |
| B5 | `handlers.rs` contains no `unsafe` blocks beyond the `#![forbid(unsafe_code)]` directive |
| B6 | Orphaned handler files in `crates/vb_ipc/src/server/handlers/` remain unreferenced by the module tree |
| B7 | `handlers.rs` uses strongly-typed enum variants instead of string literals for IPC payload fields |

---

## 2. Trophy Allocation

| Behavior | Test Layer | Rationale |
|----------|-----------|-----------|
| B1 | Static Analysis | `cargo check -p vb_ipc` is the canonical compilation gate |
| B2 | Static Analysis | `cargo check -p velvet-ballastics-workspace-tests --tests` validates downstream compilation |
| B3 | Static Analysis | `cargo clippy -p vb_ipc -- -D warnings` enforces source lint zero tolerance |
| B4 | Static Analysis | `grep` diff-scoped scan for `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!` |
| B5 | Static Analysis | `grep` for `unsafe` confirms `#![forbid(unsafe_code)]` is the only match |
| B6 | Static Analysis | `test -f` confirms `handlers/mod.rs` does not exist, preserving orphaned file isolation |
| B7 | Static Analysis | `grep` counts fully-qualified enum variant usage (`EdgeType::`, `PassFail::`, `GateKind::`, `NodeKind::`, `TaintPathStatus::`) |

---

## 3. BDD Scenarios

### Behavior B1: vb_ipc compiles cleanly
```
Given: The workspace checkout is clean on commit 0ebc5270
When:  cargo check -p vb_ipc is executed
Then:  The command exits with code 0
And:   Zero errors or warnings are emitted
```
*Test function name:* `fn test_vb_ipc_compiles_cleanly()`

### Behavior B2: workspace-tests compiles cleanly
```
Given: The workspace checkout is clean on commit 0ebc5270
When:  cargo check -p velvet-ballastics-workspace-tests --tests is executed
Then:  The command exits with code 0
And:   Zero errors or warnings are emitted
```
*Test function name:* `fn test_workspace_tests_compiles_cleanly()`

### Behavior B3: vb_ipc passes clippy with zero warnings
```
Given: The workspace checkout is clean on commit 0ebc5270
When:  cargo clippy -p vb_ipc -- -D warnings is executed
Then:  The command exits with code 0
And:   Zero clippy warnings or errors are emitted
```
*Test function name:* `fn test_vb_ipc_clippy_passes_with_zero_warnings()`

### Behavior B4: No new panicking APIs introduced in handlers.rs
```
Given: A diff from the baseline commit (0ebc5270^) to HEAD
When:  grep is run over only the changed lines for unwrap, expect, panic!, todo!, unimplemented!
Then:  Zero new matches are found in non-test code
```
*Test function name:* `fn test_no_new_unwrap_expect_panic_todo_unimplemented_in_handlers_diff()`

*Note on pre-existing matches:* The file contains pre-existing `.unwrap_or()`, `.expect()`, and `assert!(false, ...)` calls in test code and safe fallbacks (e.g., `u16::try_from(...).unwrap_or(u16::MAX)`). These are grandfathered. The test must scope to the diff only.

### Behavior B5: No unsafe code in handlers.rs
```
Given: The file crates/vb_ipc/src/server/handlers.rs
When:  grep -n 'unsafe' is executed on the file
Then:  Exactly one match is found: line 1 containing #![forbid(unsafe_code)]
And:   No unsafe blocks, functions, or traits exist
```
*Test function name:* `fn test_no_unsafe_in_handlers_rs()`

### Behavior B6: Orphaned handler files remain excluded
```
Given: The directory crates/vb_ipc/src/server/handlers/
When:  test -f crates/vb_ipc/src/server/handlers/mod.rs is executed
Then:  The command returns exit code 1 (file does not exist)
And:   The orphaned files (command.rs, event.rs, query.rs, session.rs) are not wired into the module tree
```
*Test function name:* `fn test_orphaned_handlers_files_do_not_affect_build()`

### Behavior B7: Typed enum variants used in handlers.rs
```
Given: The file crates/vb_ipc/src/server/handlers.rs
When:  grep counts fully-qualified enum variant references
Then:  At least 100 matches are found for EdgeType::, PassFail::, GateKind::, NodeKind::, and TaintPathStatus::
And:   Zero string literal assignments exist where typed enum variants are expected
```
*Test function name:* `fn test_enum_variants_used_instead_of_string_literals()`

---

## 4. Proptest Invariants

*None.* This compile-fix bead introduces no new pure functions with multiple inputs. All existing pure functions (`sanitize_runtime_error`, `sanitize_validation_detail`, `node_kind_label`) are unchanged by the fix.

---

## 5. Fuzz Targets

*None.* This bead changes no parsing or deserialization boundaries. The `decode_payload` function and postcard decoding paths are unchanged.

---

## 6. Kani Harnesses

*None — waived.* Per `verification-layers.md`, all deep verification lanes (Kani, Verus, TLA+, Flux, Loom, Miri) are waived for this compile-fix prerequisite bead. Rationale: no new algorithmic logic, no unsafe code, no concurrent changes, no protocol modifications.

---

## 7. Mutation Testing Checkpoints

*Not applicable.* Mutation testing (`cargo-mutants`) is explicitly forbidden for this lifecycle phase. This bead is a compile-fix prerequisite; the mutation refresh is the responsibility of the downstream bead `vb-qi37.26`.

---

## 8. Combinatorial Coverage Matrix

| Scenario | Input Class | Expected Output | Test Layer | Maps to |
|----------|-------------|-----------------|------------|---------|
| T1: vb_ipc compiles | clean checkout, pinned nightly | exit 0, zero errors | static | B1 / C1 / COMP-001 |
| T2: workspace-tests compiles | clean checkout, `--tests` target | exit 0, zero errors | static | B2 / C2 / COMP-002 |
| T3: clippy passes | `-D warnings` enforced | exit 0, zero warnings | static | B3 / C1 / COMP-003 |
| T4: no new panics in diff | diff-scoped grep | zero new matches | static | B4 / C3 / SAFE-001 |
| T5: no unsafe code | full-file grep for `unsafe` | exactly 1 match (`#![forbid(unsafe_code)]`) | static | B5 / C3 / SAFE-002 |
| T6: no handlers/mod.rs | filesystem check | exit 1 (not found) | static | B6 / C4 / ORPH-001 |
| T7: enum variants used | full-file grep for `::` variants | ≥100 matches, zero string-literal regressions | static | B7 / INV-001 / TYPE-001 |

---

## 9. Contract Clause → Test Case Mapping

| Contract Clause | Description | Test Case(s) | Proof Obligation |
|-----------------|-------------|--------------|------------------|
| **C1** | `vb_ipc` compiles | T1, T3 | COMP-001, COMP-003 |
| **C2** | `workspace-tests` compiles | T2 | COMP-002 |
| **C3** | No safety regressions | T4, T5 | SAFE-001, SAFE-002 |
| **C4** | Orphaned files excluded | T6 | ORPH-001 |
| **INV-001** | Type consistency (enum variants, not strings) | T7 | TYPE-001, COMP-001 |
| **INV-002** | Compilation isolation (orphans unreferenced) | T6 | ORPH-001 |
| **INV-003** | Safety preservation (no unsafe/panics introduced) | T4, T5 | SAFE-001, SAFE-002 |

---

## 10. Test Execution Commands

### T1 — cargo check -p vb_ipc
```bash
cargo check -p vb_ipc
```
*Expected:* Exit code `0`, zero errors.

### T2 — cargo check workspace-tests
```bash
cargo check -p velvet-ballastics-workspace-tests --tests
```
*Expected:* Exit code `0`, zero errors.

### T3 — cargo clippy vb_ipc
```bash
cargo clippy -p vb_ipc -- -D warnings
```
*Expected:* Exit code `0`, zero warnings.

### T4 — diff-scoped panic pattern audit
```bash
# Scope grep to the diff between baseline and fix commit
# If git is unavailable in the workspace, use the pre-generated baseline report
# in .beads/vb-qi37.26.1/baseline-report.md for comparison.
git diff 0ebc5270^..0ebc5270 -- crates/vb_ipc/src/server/handlers.rs | \
  grep -E '^\+.*(unwrap|expect|panic!|todo!|unimplemented!)' | \
  grep -v '^\+.*//\|test\|assert!' || true
```
*Expected:* Zero lines of output (no new panicking APIs in production code).

### T5 — unsafe code audit
```bash
grep -n 'unsafe' crates/vb_ipc/src/server/handlers.rs
```
*Expected:* Exactly one match: `1:#![forbid(unsafe_code)]`.

### T6 — orphaned module exclusion
```bash
test -f crates/vb_ipc/src/server/handlers/mod.rs; echo $?
```
*Expected:* Exit code `1` (file does not exist).

### T7 — enum variant usage confirmation
```bash
/usr/bin/rg -n 'EdgeType::|PassFail::|GateKind::|NodeKind::|TaintPathStatus::' \
  crates/vb_ipc/src/server/handlers.rs | wc -l
```
*Expected:* Count ≥ 100 (baseline: 227 matches at time of fix).

---

## 11. Exit Criteria

- [ ] T1 passes: `cargo check -p vb_ipc` exits 0
- [ ] T2 passes: `cargo check -p velvet-ballastics-workspace-tests --tests` exits 0
- [ ] T3 passes: `cargo clippy -p vb_ipc -- -D warnings` exits 0
- [ ] T4 passes: No new `unwrap`, `expect`, `panic!`, `todo!`, or `unimplemented!` in the diff
- [ ] T5 passes: Only `#![forbid(unsafe_code)]` matches for `unsafe` in `handlers.rs`
- [ ] T6 passes: `handlers/mod.rs` does not exist
- [ ] T7 passes: Enum variant usage count ≥ 100 with zero string-literal regressions
- [ ] All contract clauses (C1–C4, INV-001–INV-003) have at least one mapped passing test
- [ ] Traceability matrix updated with test results

---

## Open Questions

*None.* The fix is already applied and verified. All open questions from the contract review (notably the diff-scoping precision for SAFE-001) have been addressed in the test specification above.

---

*Test plan generated by test-planner subagent for bead vb-qi37.26.1.*
*Status: Ready for test-writer execution.*
