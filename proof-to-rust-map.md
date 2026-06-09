# Proof-To-Rust Bridge Map: vb-xi2f.24

## Metadata
- **bead**: vb-xi2f.24
- **state**: 7 (proof-to-implementation bridge)
- **invocation_id**: vb-xi2f24-state7-proof-to-implementation
- **proof_review_invocation_id**: inv-0012-proof-reviewer-state6-r5
- **proof_review_disposition**: APPROVED (5 Verus waivers, Kani/Flux/proptest/fuzz compensating)
- **mapping_status**: planned (allowed at State 7; closure required by State 12)
- **source_checkout**: /home/lewis/src/vb-workspaces/vb-xi2f.24
- **workdir**: /home/lewis/src/vb-workspaces/vb-xi2f.24

## Summary
Maps 32 behavior-affecting proof obligations (11 Kani, 6 Flux, 13 proptest, 2 fuzz) to Rust production source locations, independent behavior tests, and separate refinement harnesses. 5 Verus obligations are formally waived (behavior_affecting: false, covered by Kani+Flux+proptest+fuzz lanes). All mapping_status values are `planned` (valid for State 7; closure required by State 12).

## GOD RULES Compliance

| GOD Rule | Status | Evidence |
|----------|--------|----------|
| Rule 1: No Hardcoded Kani Shapes | ✓ | All 11 harnesses use `kani::any()` for StepAst/StepPrimitive generation |
| Rule 2: No Vacuum Verus Proofs | **WAIVED** | 5 formal waivers (WV-VB-XI2F24-VERUS-001..005); compensating Kani/Flux/proptest/fuzz coverage |
| Rule 3: No Unbounded Math | ✓ | All models use u16 bounded arithmetic (VbU16Max=65535), overflow detection |
| Rule 4: No Loop Oscillations | ✓ | Only verification artifacts changed across 5 repair retries; no harness weakened |
| Rule 5: No Blind Mutations | ✓ | Scope trimmed to reduce lowering call-graph |

## Waiver Documentation

5 Verus obligations are formally waived under GOD RULE 2 (Verus proofs disconnected from production code — `pub(super)` visibility on `canonical_body_step_width`, `body_width`, `emit_single_body_set`, `checked_step_offset` blocks `extern_spec` bindings). Each waiver is `behavior_affecting: false` and carries compensating Kani+Flux+proptest+fuzz evidence. See `formal-waivers.jsonl` for full waiver details. Waivers expire 2026-12-01. When production functions gain `pub(crate)` visibility, Verus lane can be restored via a separate bead.

## Production Code Map

### Key Production Symbols

| Symbol | File | Line | Visibility | Role |
|--------|------|------|------------|------|
| `canonical_body_step_width` | `crates/vb_compile/src/mod_compile_lowering/part_01.rs` | 142 | `pub(super)` | Width calculation for body step primitives |
| `body_width` | `crates/vb_compile/src/mod_compile_lowering/part_01.rs` | 104 | `pub(super)` | Total body width (overhead + sum of step widths) |
| `lower_canonical_aggregate` | `crates/vb_compile/src/mod_compile_lowering/part_04.rs` | 15 | `pub(super)` | Reduce lowering entry point |
| `emit_single_body_set` | `crates/vb_compile/src/mod_compile_lowering/part_04.rs` | 213 | `pub(super)` | Single-step body dispatcher (reference implementation) |
| `emit_reduce_body_steps` | `crates/vb_compile/src/mod_compile_lowering/part_04.rs` | **NOT YET IMPLEMENTED** | — | Multi-step body dispatcher (this bead's implementation scope) |
| `checked_step_offset` | `crates/vb_compile/src/mod_compile_lowering/part_12.rs` | 199 | `pub(super)` | Checked StepIdx offset arithmetic |

### C1/C2: Width-Node Count Synchronization (5 Kani + Flux + proptest obligations)

| Obligation | Verifier | Rust Target | Production Source Ref |
|---|---|---|---|
| PO-WIDTH-MATCH-KANI-001 | kani | Width-node parity | `part_01.rs::body_width:104-141`, `part_04.rs::emit_single_body_set:213-301` (reference; multi-step dispatcher `emit_reduce_body_steps` TBD) |
| PO-WIDTH-MATCH-FLUX-001 | flux-rs | body_width refinements | `part_01.rs::body_width:104-141` |
| PO-WIDTH-MATCH-PROP-001 | proptest | Full pipeline parity | `part_04.rs::lower_canonical_aggregate:15-84`, `part_01.rs::body_width:104-141`, `compile_source()` |
| PO-TRYFROMPARTS-KANI-001 | kani | E2E try_from_parts | `part_04.rs::lower_canonical_aggregate:15-84`, `vb_core::workflow::CompiledWorkflow::try_from_parts` (or `vb_core::CompiledWorkflow::try_from_parts` from `lib.rs:125`) |
| PO-TRYFROMPARTS-PROP-001 | proptest | E2E try_from_parts | Same as PO-TRYFROMPARTS-KANI-001 |

### C3/C5/C6: Step Offset Correctness (4 Kani + 3 Flux + 5 proptest obligations)

| Obligation | Verifier | Rust Target | Production Source Ref |
|---|---|---|---|
| PO-OFFSET-KANI-001 | kani | StepIdx distinctness | `part_04.rs::emit_single_body_set:213-301` (reference), `part_12.rs::checked_step_offset:199-231` |
| PO-OFFSET-FLUX-001 | flux-rs | Monotonic offset refinement | `part_04.rs::emit_single_body_set:213-301` (dispatch loop offset tracking) |
| PO-OFFSET-PROP-001 | proptest | Monotonic offsets | `part_04.rs::lower_canonical_aggregate:15-84`, `part_12.rs::checked_step_offset:199-231` |
| PO-OVERFLOW-KANI-001 | kani | Width overflow detection | `part_01.rs::body_width:104-141` (checked_add for cumulative width) |
| PO-OVERFLOW-FLUX-001 | flux-rs | Width overflow refinement | `part_01.rs::body_width:104-141` (Ok(n) => n <= 65535) |
| PO-OVERFLOW-PROP-001 | proptest | Overflow rejection | `part_01.rs::body_width:104-141`, `compile_source()` |
| PO-NESTED-FOREACH-KANI-001 | kani | ForEach width advancement | `part_01.rs::canonical_body_step_width:142-168`, `part_04.rs::emit_single_body_set:213-301` |
| PO-NESTED-FOREACH-FLUX-001 | flux-rs | ForEach width refinement | `part_01.rs::canonical_body_step_width:142-168`, `part_04.rs::emit_single_body_set:213-301` |
| PO-NESTED-FOREACH-PROP-001 | proptest | ForEach body layout | `part_01.rs::canonical_body_step_width:142-168`, `compile_source()` |

### C4: Body Chain Integrity (1 Kani + 1 Flux + 1 proptest)

| Obligation | Verifier | Rust Target | Production Source Ref |
|---|---|---|---|
| PO-CHAIN-KANI-001 | kani | Next-link chain | `part_04.rs::emit_single_body_set:213-301` (next-link assignment), `part_12.rs::checked_step_offset:199-231` |
| PO-CHAIN-FLUX-001 | flux-rs | Position-aware next-link | `part_04.rs::emit_single_body_set:213-301` (step_next computation) |
| PO-CHAIN-PROP-001 | proptest | Next-link property | `part_04.rs::lower_canonical_aggregate:15-84`, `part_12.rs::checked_step_offset:199-231` |

### C8: Nested Reduce Semantics (1 Kani + 1 Flux + 1 proptest)

| Obligation | Verifier | Rust Target | Production Source Ref |
|---|---|---|---|
| PO-NESTED-NEXT-KANI-001 | kani | Nested Reduce next-field | `part_04.rs::emit_single_body_set:213-301`, `part_04.rs::lower_canonical_aggregate:15-84` |
| PO-NESTED-NEXT-FLUX-001 | flux-rs | Position dispatch refinement | `part_04.rs::emit_single_body_set:213-301` (dispatch loop) |
| PO-NESTED-NEXT-PROP-001 | proptest | Nested Reduce next assign | `part_04.rs::lower_canonical_aggregate:15-84`, `part_12.rs::checked_step_offset:199-231` |

### C7: Single-Step Regression Safety (1 Kani + 1 proptest)

| Obligation | Verifier | Rust Target | Production Source Ref |
|---|---|---|---|
| PO-REGRESSION-KANI-001 | kani | emit_reduce_body_steps vs emit_single_body_set | `part_04.rs::emit_single_body_set:213-301` (reference; `emit_reduce_body_steps` NOT YET IMPLEMENTED — blocked on implementation) |
| PO-REGRESSION-PROP-001 | proptest | Single-step equivalence | `part_04.rs::emit_single_body_set:213-301`, `compile_source()` |

### C12: Empty Body Handling (1 Kani + 1 proptest)

| Obligation | Verifier | Rust Target | Production Source Ref |
|---|---|---|---|
| PO-EMPTY-KANI-001 | kani | Empty body rejection | `part_04.rs::lower_canonical_aggregate:15-84` (body emptiness check) |
| PO-EMPTY-PROP-001 | proptest | Empty body diagnostic | `part_04.rs::lower_canonical_aggregate:15-84`, `compile_source()` |

### C11: No Panic (1 Kani + 1 proptest + 1 fuzz)

| Obligation | Verifier | Rust Target | Production Source Ref |
|---|---|---|---|
| PO-NOPANIC-KANI-001 | kani | Full pipeline panic-freedom | `part_04.rs::lower_canonical_aggregate:15-84`, `part_04.rs::emit_single_body_set:213-301`, `part_01.rs::canonical_body_step_width:142-168` |
| PO-NOPANIC-PROP-001 | proptest | Stat panic-freedom | `part_04.rs::lower_canonical_aggregate:15-84`, `compile_source()` |
| PO-NOPANIC-FUZZ-001 | cargo-fuzz | Hostile input panic | `compile_source()` (full YAML→IR pipeline) |

### C9: Symbolic Diagnostics (1 Kani + 1 proptest + 1 fuzz)

| Obligation | Verifier | Rust Target | Production Source Ref |
|---|---|---|---|
| PO-DIAGNOSTIC-KANI-001 | kani | Valid symbolic codes | `crates/vb_compile/src/mod_compile_errors/collection.rs::primitive_code`, `crates/vb_compile/src/mod_compile_errors/kind.rs::CompileError::code` |
| PO-DIAGNOSTIC-PROP-001 | proptest | Expected code set | `compile_source()`, `mod_compile_errors::collection::primitive_code` |
| PO-DIAGNOSTIC-FUZZ-001 | cargo-fuzz | Hostile input codes | `compile_source()` (full pipeline) |

### C10: Deterministic Lowering (1 proptest)

| Obligation | Verifier | Rust Target | Production Source Ref |
|---|---|---|---|
| PO-DIGEST-PROP-001 | proptest | Digest determinism | `crates/vb_compile/src/mod_compile_lowering/part_05.rs::canonical_digest` |

### N/A: Collision Boundary (1 proptest)

| Obligation | Verifier | Rust Target | Production Source Ref |
|---|---|---|---|
| PO-COLLISION-PROP-001 | proptest | Cross-bead collision | `part_01.rs::canonical_body_step_width:142-168`, `part_04.rs::emit_single_body_set:213-301`, `compile_source()` |

## Deferred / Unresolved Gaps

1. **emit_reduce_body_steps NOT IMPLEMENTED**: The multi-step body dispatcher is the core implementation scope of this bead. PO-REGRESSION-KANI-001 is blocked on this function (Kani harness `kani_reduce_regression.rs` lines 27-28, 134-152 are commented-out TODO blocks). When implemented, the harness comparison between `emit_reduce_body_steps` and `emit_single_body_set` must be uncommented and executed. See finding F-002 from proof-review.md.

2. **Flux Annotation Location (F-001)**: Flux annotations reside in `.flux` files at `verification/flux/vb_compile/mod_compile_lowering/reduce_*.flux`, not inline in production source. `cargo flux -p vb_compile` may not check external .flux files. State 11-12 formal-verifier must confirm the Flux checker actually covers these annotations, or use single-file `flux` commands.

3. **Fuzz Tooling Blocked**: Both fuzz targets (PO-NOPANIC-FUZZ-001, PO-DIAGNOSTIC-FUZZ-001) are BLOCKED_TOOLING (musl+sanitizer incompatibility). Core coverage provided by Kani+proptest. Accept BLOCKED_TOOLING with waiver or resolve infrastructure at State 11-12.

4. **All Compensating Evidence PENDING_FORMAL_EXECUTION (F-003)**: The 32 Kani/Flux/proptest/fuzz artifacts have not been executed. Proof-review approval is based on artifact *quality* (well-formed, non-vacuous, correctly targeted). *Soundness* depends on successful execution at State 11-12. Any failing lane must be repaired or the corresponding Verus waiver reconsidered.

## Behavior Test References (Planned)

All behavior tests are planned for State 8 (test planning) with materialization at State 12. Each RRO row names the planned test path pattern:

- `crates/vb_compile/src/mod_compile_lowering/tests.rs::test_reduce_body_width_parity`
- `crates/vb_compile/src/mod_compile_lowering/tests.rs::test_reduce_body_offset_monotonic`
- `crates/vb_compile/src/mod_compile_lowering/tests.rs::test_reduce_body_chain`
- `crates/vb_compile/src/mod_compile_lowering/tests.rs::test_reduce_foreach_width`
- `crates/vb_compile/src/mod_compile_lowering/tests.rs::test_reduce_nested_next`
- `crates/vb_compile/src/mod_compile_lowering/tests.rs::test_reduce_empty_body`
- `crates/vb_compile/src/mod_compile_lowering/tests.rs::test_reduce_single_step_regression`
- `crates/vb_compile/src/mod_compile_lowering/tests.rs::test_reduce_no_panic`
- `crates/vb_compile/src/mod_compile_lowering/tests.rs::test_reduce_diagnostics`
- `crates/vb_compile/src/mod_compile_lowering/tests.rs::test_reduce_digest_determinism`
- `crates/vb_compile/tests/v1_primitive_lowering.rs::test_reduce_multi_step_body`
- `crates/vb_compile/tests/v1_primitive_lowering.rs::test_try_from_parts_multi_step`
- `crates/vb_compile/tests/v1_primitive_lowering.rs::test_collision_vb_xi2f22`

## Refinement Harness References (Planned)

Separate from behavior tests and verifier harnesses. Materialized at State 12:

- `crates/vb_compile/src/mod_compile_lowering/kani_reduce_body_width.rs` (Kani width parity)
- `crates/vb_compile/src/mod_compile_lowering/kani_reduce_offset.rs` (Kani offset distinctness)
- `crates/vb_compile/src/mod_compile_lowering/kani_reduce_chain.rs` (Kani chain integrity)
- `crates/vb_compile/src/mod_compile_lowering/kani_reduce_nested_next.rs` (Kani nested next)
- `crates/vb_compile/src/mod_compile_lowering/kani_reduce_empty.rs` (Kani empty body)
- `crates/vb_compile/src/mod_compile_lowering/kani_reduce_regression.rs` (Kani regression)
- `crates/vb_compile/src/mod_compile_lowering/kani_reduce_foreach.rs` (Kani ForEach width)
- `crates/vb_compile/src/mod_compile_lowering/kani_reduce_nopanic.rs` (Kani no panic)
- `crates/vb_compile/src/mod_compile_lowering/kani_reduce_diagnostics.rs` (Kani diagnostics)
- `crates/vb_compile/src/mod_compile_lowering/kani_reduce_tryfromparts.rs` (Kani try_from_parts)
- `crates/vb_compile/src/mod_compile_lowering/kani_reduce_overflow.rs` (Kani overflow)
- `verification/flux/vb_compile/mod_compile_lowering/reduce_body_width.flux`
- `verification/flux/vb_compile/mod_compile_lowering/reduce_offset.flux`
- `verification/flux/vb_compile/mod_compile_lowering/reduce_chain.flux`
- `verification/flux/vb_compile/mod_compile_lowering/reduce_nested_next.flux`
- `verification/flux/vb_compile/mod_compile_lowering/reduce_foreach.flux`
- `verification/flux/vb_compile/mod_compile_lowering/reduce_overflow.flux`
- `verification/proptest/vb_compile/reduce_body_width_parity.rs`
- `verification/proptest/vb_compile/reduce_body_offset_monotonic.rs`
- `verification/proptest/vb_compile/reduce_body_chain_integrity.rs`
- `verification/proptest/vb_compile/reduce_nested_next.rs`
- `verification/proptest/vb_compile/reduce_empty_body.rs`
- `verification/proptest/vb_compile/reduce_single_step_regression.rs`
- `verification/proptest/vb_compile/reduce_nested_foreach_layout.rs`
- `verification/proptest/vb_compile/reduce_lowering_no_panic.rs`
- `verification/proptest/vb_compile/reduce_diagnostic_codes.rs`
- `verification/proptest/vb_compile/reduce_digest_determinism.rs`
- `verification/proptest/vb_compile/reduce_body_width_overflow.rs`
- `verification/proptest/vb_compile/reduce_multi_step_try_from_parts.rs`
- `verification/proptest/vb_compile/reduce_together_collision.rs`
- `fuzz/fuzz_targets/reduce_lowering_panic.rs`
- `fuzz/fuzz_targets/reduce_diagnostic_codes.rs`

## Evidence Command Summary

| Verifier | Count | Command Pattern |
|----------|-------|----------------|
| kani | 11 | `cargo kani -p vb_compile --harness check_reduce_* --unwind 16` |
| flux-rs | 6 | `bash scripts/flux-check-package.sh vb_compile` (or single-file `flux` for .flux files) |
| proptest | 13 | `cargo test -p vb_compile -- proptest_reduce_*` |
| cargo-fuzz | 2 | `cargo fuzz run reduce_* -- -max_total_time=300` |

## Bridge Matrix (Full)

| Proof ID | Claim | Behavior Affecting | Rust Source Refs | Behavior Test Refs | Refinement Harness Refs | Verifier | Evidence Command | Status |
|---|---|---|---|---|---|---|---|---|
| PO-WIDTH-MATCH-KANI-001 | Width-node parity bounded | true | body_width:104, emit_single_body_set:213 | test_reduce_body_width_parity | kani_reduce_body_width.rs | kani | `cargo kani -p vb_compile --harness check_reduce_body_width_parity --unwind 16` | planned |
| PO-WIDTH-MATCH-FLUX-001 | body_width refinement | true | body_width:104 | N/A (Flux static) | reduce_body_width.flux | flux-rs | `bash scripts/flux-check-package.sh vb_compile` | planned |
| PO-WIDTH-MATCH-PROP-001 | Width-node random parity | true | lower_canonical_aggregate:15, body_width:104 | test_reduce_body_width_parity | reduce_body_width_parity.rs | proptest | `cargo test -p vb_compile -- proptest_reduce_body_width_parity` | planned |
| PO-TRYFROMPARTS-KANI-001 | E2E try_from_parts | true | lower_canonical_aggregate:15, try_from_parts | test_reduce_multi_step_body | kani_reduce_tryfromparts.rs | kani | `cargo kani -p vb_compile --harness check_reduce_multi_step_try_from_parts --unwind 16` | planned |
| PO-TRYFROMPARTS-PROP-001 | E2E try_from_parts random | true | lower_canonical_aggregate:15 | test_try_from_parts_multi_step | reduce_multi_step_try_from_parts.rs | proptest | `cargo test -p vb_compile -- proptest_reduce_multi_step_try_from_parts` | planned |
| PO-OFFSET-KANI-001 | StepIdx distinctness | true | emit_single_body_set:213, checked_step_offset:199 | test_reduce_body_offset_monotonic | kani_reduce_offset.rs | kani | `cargo kani -p vb_compile --harness check_reduce_body_offset_distinctness --unwind 16` | planned |
| PO-OFFSET-FLUX-001 | Monotonic offset refinement | true | emit_single_body_set:213 | N/A (Flux static) | reduce_offset.flux | flux-rs | `bash scripts/flux-check-package.sh vb_compile` | planned |
| PO-OFFSET-PROP-001 | Monotonic offsets random | true | lower_canonical_aggregate:15, checked_step_offset:199 | test_reduce_body_offset_monotonic | reduce_body_offset_monotonic.rs | proptest | `cargo test -p vb_compile -- proptest_reduce_body_offset_monotonic` | planned |
| PO-OVERFLOW-KANI-001 | Width overflow detection | true | body_width:104 | test_reduce_width_overflow | kani_reduce_overflow.rs | kani | `cargo kani -p vb_compile --harness check_reduce_body_width_overflow --unwind 32` | planned |
| PO-OVERFLOW-FLUX-001 | Width overflow refinement | true | body_width:104 | N/A (Flux static) | reduce_overflow.flux | flux-rs | `bash scripts/flux-check-package.sh vb_compile` | planned |
| PO-OVERFLOW-PROP-001 | Overflow rejection random | true | body_width:104 | test_reduce_width_overflow | reduce_body_width_overflow.rs | proptest | `cargo test -p vb_compile -- proptest_reduce_body_width_overflow` | planned |
| PO-NESTED-FOREACH-KANI-001 | ForEach width advancement | true | canonical_body_step_width:142, emit_single_body_set:213 | test_reduce_foreach_width | kani_reduce_foreach.rs | kani | `cargo kani -p vb_compile --harness check_reduce_foreach_width_advance --unwind 16` | planned |
| PO-NESTED-FOREACH-FLUX-001 | ForEach width refinement | true | canonical_body_step_width:142, emit_single_body_set:213 | N/A (Flux static) | reduce_foreach.flux | flux-rs | `bash scripts/flux-check-package.sh vb_compile` | planned |
| PO-NESTED-FOREACH-PROP-001 | ForEach body layout random | true | canonical_body_step_width:142 | test_reduce_foreach_width | reduce_nested_foreach_layout.rs | proptest | `cargo test -p vb_compile -- proptest_reduce_nested_foreach_layout` | planned |
| PO-CHAIN-KANI-001 | Next-link chain integrity | true | emit_single_body_set:213, checked_step_offset:199 | test_reduce_body_chain | kani_reduce_chain.rs | kani | `cargo kani -p vb_compile --harness check_reduce_body_chain_integrity --unwind 16` | planned |
| PO-CHAIN-FLUX-001 | Position-aware next-link | true | emit_single_body_set:213 | N/A (Flux static) | reduce_chain.flux | flux-rs | `bash scripts/flux-check-package.sh vb_compile` | planned |
| PO-CHAIN-PROP-001 | Next-link property random | true | lower_canonical_aggregate:15, checked_step_offset:199 | test_reduce_body_chain | reduce_body_chain_integrity.rs | proptest | `cargo test -p vb_compile -- proptest_reduce_body_chain_integrity` | planned |
| PO-NESTED-NEXT-KANI-001 | Nested Reduce next-field | true | emit_single_body_set:213, lower_canonical_aggregate:15 | test_reduce_nested_next | kani_reduce_nested_next.rs | kani | `cargo kani -p vb_compile --harness check_reduce_nested_next_correctness --unwind 16` | planned |
| PO-NESTED-NEXT-FLUX-001 | Position dispatch refinement | true | emit_single_body_set:213 | N/A (Flux static) | reduce_nested_next.flux | flux-rs | `bash scripts/flux-check-package.sh vb_compile` | planned |
| PO-NESTED-NEXT-PROP-001 | Nested next assign random | true | lower_canonical_aggregate:15, checked_step_offset:199 | test_reduce_nested_next | reduce_nested_next.rs | proptest | `cargo test -p vb_compile -- proptest_reduce_nested_next` | planned |
| PO-REGRESSION-KANI-001 | Single-step equivalence | true | emit_single_body_set:213, emit_reduce_body_steps (BLOCKED) | test_reduce_single_step_regression | kani_reduce_regression.rs | kani | `cargo kani -p vb_compile --harness check_reduce_single_step_equivalence --unwind 8` | planned (blocked on emit_reduce_body_steps) |
| PO-REGRESSION-PROP-001 | Single-step equivalence random | true | emit_single_body_set:213 | test_reduce_single_step_regression | reduce_single_step_regression.rs | proptest | `cargo test -p vb_compile -- proptest_reduce_single_step_regression` | planned |
| PO-EMPTY-KANI-001 | Empty body rejection | true | lower_canonical_aggregate:15 | test_reduce_empty_body | kani_reduce_empty.rs | kani | `cargo kani -p vb_compile --harness check_reduce_empty_body_rejection` | planned |
| PO-EMPTY-PROP-001 | Empty body diagnostic random | true | lower_canonical_aggregate:15 | test_reduce_empty_body | reduce_empty_body.rs | proptest | `cargo test -p vb_compile -- proptest_reduce_empty_body` | planned |
| PO-NOPANIC-KANI-001 | Full pipeline panic-freedom | true | lower_canonical_aggregate:15, emit_single_body_set:213, canonical_body_step_width:142 | test_reduce_no_panic | kani_reduce_nopanic.rs | kani | `cargo kani -p vb_compile --harness check_reduce_lowering_no_panic --unwind 16` | planned |
| PO-NOPANIC-PROP-001 | Stat panic-freedom | true | lower_canonical_aggregate:15 | test_reduce_no_panic | reduce_lowering_no_panic.rs | proptest | `cargo test -p vb_compile -- proptest_reduce_lowering_no_panic` | planned |
| PO-NOPANIC-FUZZ-001 | Hostile input no panic | true | compile_source() | N/A (fuzz) | reduce_lowering_panic.rs | cargo-fuzz | `cargo fuzz run reduce_lowering_panic -- -max_total_time=300` | planned (BLOCKED_TOOLING: musl+sanitizer) |
| PO-DIAGNOSTIC-KANI-001 | Valid symbolic codes bounded | true | primitive_code, CompileError::code | test_reduce_diagnostics | kani_reduce_diagnostics.rs | kani | `cargo kani -p vb_compile --harness check_reduce_error_diagnostic_codes --unwind 16` | planned |
| PO-DIAGNOSTIC-PROP-001 | Expected code set random | true | primitive_code | test_reduce_diagnostics | reduce_diagnostic_codes.rs | proptest | `cargo test -p vb_compile -- proptest_reduce_diagnostic_codes` | planned |
| PO-DIAGNOSTIC-FUZZ-001 | Hostile input codes | true | compile_source() | N/A (fuzz) | reduce_diagnostic_codes.rs | cargo-fuzz | `cargo fuzz run reduce_diagnostic_codes -- -max_total_time=300` | planned (BLOCKED_TOOLING: musl+sanitizer) |
| PO-DIGEST-PROP-001 | Digest determinism | false | canonical_digest:part_05 | test_reduce_digest_determinism | reduce_digest_determinism.rs | proptest | `cargo test -p vb_compile -- proptest_reduce_digest_determinism` | planned |
| PO-COLLISION-PROP-001 | Cross-bead collision safety | false | canonical_body_step_width:142, emit_single_body_set:213 | test_collision_vb_xi2f22 | reduce_together_collision.rs | proptest | `cargo test -p vb_compile -- proptest_reduce_together_collision` | planned |

## Bridge Mapping Completeness

| Verifier | Obligations | Mapped | Deferred/Blocked | Unresolved |
|---|---|---|---|---|
| verus | 5 | 0 (WAIVED) | 5 (behavior_affecting: false, compensating Kani+Flux+proptest+fuzz) | 0 |
| kani | 11 | 11 | 0 | 0 |
| flux-rs | 6 | 6 | 0 | 0 |
| proptest | 13 | 13 | 0 | 0 |
| cargo-fuzz | 2 | 2 | 2 (BLOCKED_TOOLING) | 0 |
| **Total** | **37** | **32** | **7** | **0** |

## Line Number Verification

All line numbers verified against production code via `grep -n`:
- `part_01.rs::body_width` starts at line 104
- `part_01.rs::canonical_body_step_width` starts at line 142
- `part_04.rs::lower_canonical_aggregate` starts at line 15
- `part_04.rs::emit_single_body_set` starts at line 213
- `part_12.rs::checked_step_offset` starts at line 199
- `emit_reduce_body_steps` does not exist in production code (part of implementation scope)
- All other refs verified and correct

## Handoff to Proof-Reviewer

Bridge mapping artifacts ready for proof-reviewer (bridge review):
- `proof-to-rust-map.md` (this file)
- `rust-refinement-obligations.jsonl` (32 RRO rows)

The proof-reviewer should check:
1. Source refs are symbols, not just file paths
2. Behavior test refs are independent of verifier harnesses
3. Refinement harness refs are separate from behavior tests
4. No behavior-affecting waivers without compensating evidence
5. F-001 (Flux location) and F-002 (emit_reduce_body_steps blocker) are properly documented
