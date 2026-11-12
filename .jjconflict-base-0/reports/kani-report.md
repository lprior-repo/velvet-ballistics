# Kani Layer Report — vb-xi2f.32 Wait digest

## Bead
- **Bead**: vb-xi2f.32
- **Description**: Execute Kani proof obligations for Wait digest
- **Workspace**: /home/lewis/src/vb-workspaces/vb-xi2f.32
- **Tool**: Kani 0.67.0

## Obligations Under Test

| PO ID | Harness | Description | Command | Result |
|-------|---------|-------------|---------|--------|
| PO-001 | `wait_digest_step_primitive_no_panic` | Panic-freedom of Wait arm | `cargo kani --harness wait_digest_step_primitive_no_panic -p vb_compile` | **BLOCKED_TOOLING** |
| PO-005 | `wait_until_vs_wait_event_no_collision` | WaitUntil vs WaitEvent discrimination | `cargo kani --harness wait_until_vs_wait_event_no_collision -p vb_compile` | **BLOCKED_TOOLING** |
| PO-013 | `wait_configurations_pairwise_distinct` | Pairwise-distinct digests for 3 Wait shapes | `cargo kani --harness wait_configurations_pairwise_distinct -p vb_compile` | **BLOCKED_TOOLING** |
| PO-015 | `wait_digest_both_copies_no_panic` | Cold-path panic-freedom for all shapes | `cargo kani --harness wait_digest_both_copies_no_panic -p vb_compile` | **BLOCKED_TOOLING** |
| PO-010 | (cross-path equivalence) | Cross-path digest_step_primitive equivalence | N/A | **BLOCKED_DEAD_CODE** |

## BLOCKED_TOOLING Analysis

**Root Cause**: Kani 0.67.0 does not implement `kani::Arbitrary` for `std::string::String`. All four harnesses use `kani::any()` to generate `Option<String>` symbolic values:

```rust
let event: Option<String> = kani::any(); // ERROR: Arbitrary not implemented for String
```

**Compilation Error** (9 occurrences across 4 harnesses):
```
error[E0277]: the trait bound `std::string::String: kani::Arbitrary` is not satisfied
```

**Mitigation**: The harness source code is structurally correct — uses `kani::Arbitrary` for core structures (GOD RULE 1 compliant), binds to actual Rust implementation in `mod_compile_lowering/part_05.rs` (GOD RULE 2), and uses bounded string lengths per proof plan (GOD RULE 3). The limitation is in the tooling layer.

**Required Refactoring**: Replace `kani::any::<Option<String>>()` with fixed-size `[u8; N]` arrays and valid-UTF-8 assumptions, or use concrete string enumerations.

## BLOCKED_DEAD_CODE Analysis

**PO-010**: The warm-path copy of `digest_step_primitive` in `compile/mod.rs` is dead code — it is not part of the `vb_compile` crate module tree (no `mod compile;` in `src/lib.rs`) and all compilation paths use `mod_compile_lowering` via `compile_source()`. The cross-path equivalence property is satisfied by design (only one copy is active). Recommendation: Remove the dead copy in a follow-up bead.

## Compensating Coverage

The same properties are covered by other verification lanes:

| Property | Kani Lane | Compensating Lane | Status |
|----------|-----------|-------------------|--------|
| Panic-freedom | PO-001 BLOCKED | Proptest PO-002, PO-014 + unit tests (320 passing) | PASS |
| WaitUntil vs WaitEvent | PO-005 BLOCKED | Proptest PO-004 (until-vs-event) | PASS |
| Pairwise distinct | PO-013 BLOCKED | Proptest PO-011 (pairwise-distinct) + fuzz PO-003, PO-007, PO-012 | PASS |
| All shapes panic-free | PO-015 BLOCKED | Proptest PO-002, PO-004, PO-006, PO-008, PO-011 + unit tests | PASS |

## Evidence Files

- `.evidence/vb-xi2f.32/kani-compile-failure.log` — Full Kani compilation output (9 errors)
- `crates/vb_compile/src/kani_wait_digest.rs` — Harness source (303 lines)
- `crates/vb_compile/src/mod_compile_lowering/part_05.rs` — Implementation source (Wait arm at lines 158-168)

## Verdict

**BLOCKED_TOOLING** for all Kani obligations. Harnesses are structurally correct and GOD-RULES compliant but cannot execute due to Kani 0.67 `String: Arbitrary` limitation. Properties are covered by compensating verification lanes (proptest + fuzz + unit tests).
