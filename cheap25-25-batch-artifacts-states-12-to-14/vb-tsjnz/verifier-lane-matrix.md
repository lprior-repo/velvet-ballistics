# Verifier Lane Matrix — vb-tsjnz

STATUS: PLANNED (proof-planner State 4). No disposition is claimed.
Proof-plan-reviewer owns `verifier-lane-review.jsonl`.

## Matrix: Proof Seeds × Verifier Lanes

Rows enumerated per `(proof_seed_id, verifier)` tuple. The cargo-*
lanes and `diff-audit` are required by PS risk; the formal proof lanes
(kani, verus, flux-rs, loom, miri, proptest, cargo-fuzz) are recorded
`not_applicable` per EARS with concrete source evidence.

| proof_seed_id | Lane | Applicable | Decision |
|---|---|---|---|
| PS-VBTSJNZ-001 | cargo-metadata | YES | required — confirm `[workspace.package].version` resolves to `vb_queue_semantics` |
| PS-VBTSJNZ-001 | cargo-check | YES | required — cargo check exercises the version inheritance resolution |
| PS-VBTSJNZ-001 | cargo-clippy | NO | no clippy lint attaches to a Cargo.toml line replacement |
| PS-VBTSJNZ-001 | cargo-test | NO | workspace_tests does not assert version inheritance |
| PS-VBTSJNZ-001 | diff-audit | YES | required — diff-audit confirms line 3 replacement shape and held invariants |
| PS-VBTSJNZ-001 | kani | NO | Cargo metadata-only patch; no bounded Rust surface (`codebase-map.md` §Source Inventory) |
| PS-VBTSJNZ-001 | verus | NO | no production-bound Verus seam for a Cargo.toml patch (AGENTS.md formal mandates) |
| PS-VBTSJNZ-001 | flux-rs | NO | no refinement types in scope; the patch is a TOML edit |
| PS-VBTSJNZ-001 | loom | NO | no concurrency introduced |
| PS-VBTSJNZ-001 | miri | NO | no unsafe / no raw pointers introduced |
| PS-VBTSJNZ-001 | proptest | NO | no behavioral property to property-check; stub has no runtime |
| PS-VBTSJNZ-001 | cargo-fuzz | NO | no parser / codec / hostile input in the patch |
| PS-VBTSJNZ-002 | cargo-metadata | YES | required — confirm `[lints]` block is recognized by cargo metadata graph |
| PS-VBTSJNZ-002 | cargo-check | YES | required — primary check that the workspace lints accept the existing `src/lib.rs` |
| PS-VBTSJNZ-002 | cargo-clippy | YES | required — clippy enforce group correctness/suspicious/perf/complexity + clippy forbid set |
| PS-VBTSJNZ-002 | cargo-test | NO | no test in `workspace_tests` exercises the `[lints]` block |
| PS-VBTSJNZ-002 | diff-audit | YES | required — confirm `[lints]` block is terminal and shape-identical to siblings |
| PS-VBTSJNZ-002 | kani | NO | no bounded panic-freedom surface (the affected `lib.rs` is out-of-scope for vb-tsjnz) |
| PS-VBTSJNZ-002 | verus | NO | no production-bound Verus seam (`codebase-map.md` §Source Inventory) |
| PS-VBTSJNZ-002 | flux-rs | NO | no scoped Flux annotations |
| PS-VBTSJNZ-002 | loom | NO | no concurrency |
| PS-VBTSJNZ-002 | miri | NO | no unsafe |
| PS-VBTSJNZ-002 | proptest | NO | no behavioral property in the patch |
| PS-VBTSJNZ-002 | cargo-fuzz | NO | no parser/codec |
| PS-VBTSJNZ-003 | cargo-check | YES | required — REQ-VBTSJNZ-005; this is the primary build-acceptance lane |
| PS-VBTSJNZ-003 | cargo-clippy | YES | required — clippy's forbid set is required-by and enforced-by for the same source surface |
| PS-VBTSJNZ-003 | cargo-metadata | NO | cargo-metadata is read-only and does not compile |
| PS-VBTSJNZ-003 | cargo-test | NO | this seed is about build acceptance, not test execution |
| PS-VBTSJNZ-003 | diff-audit | NO | diff-audit does not validate build acceptance |
| PS-VBTSJNZ-003 | kani | NO | Kani would require a runtime harness; the patch is metadata-only |
| PS-VBTSJNZ-003 | verus | NO | no production-bound Verus seam for the lint-enabling Edit |
| PS-VBTSJNZ-003 | flux-rs | NO | no scoped Flux annotations |
| PS-VBTSJNZ-003 | loom | NO | no concurrency |
| PS-VBTSJNZ-003 | miri | NO | no unsafe |
| PS-VBTSJNZ-003 | proptest | NO | no behavioral property |
| PS-VBTSJNZ-003 | cargo-fuzz | NO | no parser/codec |
| PS-VBTSJNZ-004 | cargo-clippy | YES | required — REQ-VBTSJNZ-006; `-D warnings` is the canonical zero-warning gate |
| PS-VBTSJNZ-004 | cargo-check | NO | cargo check does not promote `warn`-level lints to errors |
| PS-VBTSJNZ-004 | cargo-metadata | NO | cargo metadata is read-only |
| PS-VBTSJNZ-004 | cargo-test | NO | not a test lane |
| PS-VBTSJNZ-004 | diff-audit | NO | not a build lane |
| PS-VBTSJNZ-004 | kani | NO | no bounded model surface (out-of-scope `src/lib.rs`) |
| PS-VBTSJNZ-004 | verus | NO | no production-bound Verus seam |
| PS-VBTSJNZ-004 | flux-rs | NO | no refinement type encoding |
| PS-VBTSJNZ-004 | loom | NO | no concurrency |
| PS-VBTSJNZ-004 | miri | NO | no unsafe |
| PS-VBTSJNZ-004 | proptest | NO | no behavioral property |
| PS-VBTSJNZ-004 | cargo-fuzz | NO | no parser/codec |
| PS-VBTSJNZ-005 | cargo-test | YES | required — REQ-VBTSJNZ-007; both `vb_8ma2_workspace_assertions` and `vb_qi37_25_quality_gates` are required |
| PS-VBTSJNZ-005 | cargo-check | NO | not a test lane |
| PS-VBTSJNZ-005 | cargo-clippy | NO | not a test lane |
| PS-VBTSJNZ-005 | cargo-metadata | NO | not a test lane |
| PS-VBTSJNZ-005 | diff-audit | NO | not a test lane |
| PS-VBTSJNZ-005 | kani | NO | workspace_tests does not use Kani harnesses |
| PS-VBTSJNZ-005 | verus | NO | no Verus seam |
| PS-VBTSJNZ-005 | flux-rs | NO | no refinement type |
| PS-VBTSJNZ-005 | loom | NO | no concurrency |
| PS-VBTSJNZ-005 | miri | NO | no unsafe |
| PS-VBTSJNZ-005 | proptest | NO | the assertion tests are not property tests |
| PS-VBTSJNZ-005 | cargo-fuzz | NO | not a fuzz lane |
| PS-VBTSJNZ-006 | diff-audit | YES | required — confirm `.config/source-length-exceptions.txt` line 323 unchanged and `src/lib.rs` not modified |
| PS-VBTSJNZ-006 | cargo-check | NO | not the gating lane for file-preservation |
| PS-VBTSJNZ-006 | cargo-clippy | NO | not the gating lane |
| PS-VBTSJNZ-006 | cargo-metadata | NO | cargo metadata does not surface source-length exceptions |
| PS-VBTSJNZ-006 | cargo-test | NO | not a test lane |
| PS-VBTSJNZ-006 | kani | NO | no bounded model surface |
| PS-VBTSJNZ-006 | verus | NO | no production-bound Verus seam |
| PS-VBTSJNZ-006 | flux-rs | NO | no refinement type |
| PS-VBTSJNZ-006 | loom | NO | no concurrency |
| PS-VBTSJNZ-006 | miri | NO | no unsafe |
| PS-VBTSJNZ-006 | proptest | NO | no property test |
| PS-VBTSJNZ-006 | cargo-fuzz | NO | not a fuzz lane |
| PS-VBTSJNZ-007 | cargo-metadata | YES | required — REQ-VBTSJNZ-011; parse JSON and assert version equality |
| PS-VBTSJNZ-007 | cargo-check | NO | cargo check does not surface resolved package version |
| PS-VBTSJNZ-007 | cargo-clippy | NO | not a clippy lane |
| PS-VBTSJNZ-007 | cargo-test | NO | not a test lane (workspace_tests does not assert version equality) |
| PS-VBTSJNZ-007 | diff-audit | NO | diff-audit does not validate resolution |
| PS-VBTSJNZ-007 | kani | NO | no bounded model surface |
| PS-VBTSJNZ-007 | verus | NO | no production-bound Verus seam |
| PS-VBTSJNZ-007 | flux-rs | NO | no refinement type |
| PS-VBTSJNZ-007 | loom | NO | no concurrency |
| PS-VBTSJNZ-007 | miri | NO | no unsafe |
| PS-VBTSJNZ-007 | proptest | NO | no property test |
| PS-VBTSJNZ-007 | cargo-fuzz | NO | not a fuzz lane |
| PS-VBTSJNZ-008 | diff-audit | YES | required — confirm `jj diff` shows exactly one file modified with two hunks |
| PS-VBTSJNZ-008 | cargo-check | NO | diff-audit is not gated by build |
| PS-VBTSJNZ-008 | cargo-clippy | NO | not a clippy lane |
| PS-VBTSJNZ-008 | cargo-metadata | NO | not a metadata lane |
| PS-VBTSJNZ-008 | cargo-test | NO | not a test lane |
| PS-VBTSJNZ-008 | kani | NO | no bounded model surface |
| PS-VBTSJNZ-008 | verus | NO | no Verus seam |
| PS-VBTSJNZ-008 | flux-rs | NO | no refinement type |
| PS-VBTSJNZ-008 | loom | NO | no concurrency |
| PS-VBTSJNZ-008 | miri | NO | no unsafe |
| PS-VBTSJNZ-008 | proptest | NO | no property test |
| PS-VBTSJNZ-008 | cargo-fuzz | NO | not a fuzz lane |

## Summary by lane

| Lane | Required rows | Not applicable rows | Reason for non-applicability |
|---|---|---|---|
| cargo-metadata | 3 (PS-001, PS-002, PS-007) | 5 | not required for PS-003/004/005/006/008 |
| cargo-check | 2 (PS-002, PS-003) | 6 | PS-001=meta, PS-004/005/007/008 do not surface compile errors |
| cargo-clippy | 3 (PS-002, PS-003, PS-004) | 5 | clippy is not the gate for PS-001/005/006/007/008 |
| cargo-test | 1 (PS-005) | 7 | not a test lane for the other seeds |
| diff-audit | 5 (PS-001, PS-002, PS-006, PS-008 — plus PS-003/004 supporting) | 3 | diff-audit does not gate PS-005/007 |
| kani | 0 | 8 | Cargo metadata-only patch; no bounded Rust surface |
| verus | 0 | 8 | no production-bound Verus seam exists for the patch |
| flux-rs | 0 | 8 | no scoped Flux annotations or extern specs |
| loom | 0 | 8 | no concurrency introduced |
| miri | 0 | 8 | no unsafe introduced |
| proptest | 0 | 8 | no behavioral property in the patch |
| cargo-fuzz | 0 | 8 | no parser/codec/hostile-input surface |
