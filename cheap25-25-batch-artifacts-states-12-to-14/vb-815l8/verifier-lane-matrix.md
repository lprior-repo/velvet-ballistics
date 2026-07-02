# Verifier Lane Matrix — vb-815l8

Maps each proof seed from `.beads/vb-815l8/proof-seeds.jsonl` to its assigned verifier lanes and obligation rows.

## Matrix (proof-seed × verifier-lane)

| Proof Seed | Requirement | cargo-test | source-lint | verus | kani | flux | proptest | loom | miri | tla+ | fuzz |
|---|---|---|---|---|---|---|---|---|---|---|---|
| ps-vb815l8-001 | C-1 (Runtime frame hydration rejects every seed) | ✅ PO-001/PO-002 | — | — | — | — | — | — | — | — | — |
| ps-vb815l8-002 | C-2 (Boundary seed validation is invariant, not permissive) | — | ✅ PO-003 | — | — | — | — | — | — | — | — |
| ps-vb815l8-003 | C-3 (Test uses typed assertion, not tautological) | ✅ PO-001 | ✅ PO-003 | — | — | — | — | — | — | — | — |
| ps-vb815l8-004 | C-4 (Import is added at lines 7-13) | ✅ PO-001 | ✅ PO-003 | — | — | — | — | — | — | — | — |
| ps-vb815l8-005 | C-1 (Discrimination safety of PartialEq on RuntimeError) | — | — | — | — | — | — | — | — | — | — |
| ps-vb815l8-006 | C-1 (Storage-layer from_seed unconditionally marks all missing flags true) | — | — | — | — | — | — | — | — | — | — |
| ps-vb815l8-007 | C-1 (Secondary gate: RunFrame::new rejects step_count==0) | — | — | — | — | — | — | — | — | — | — |

## Non-Applicable Lanes (with concrete evidence per EARS contract)

| Lane | Proof Seed | Reason |
|---|---|---|
| **verus** | ps-vb815l8-001/002/003/004/005/006/007 | TEST-ONLY bead; production code is forbidden to mutate. Verus requires spec/proof artifacts that bind to production code via `#[path = "..."]` (STRONG) or production_inner mirror (WEAK); neither is in scope for a test-only edit. The 8 existing unit tests at `crates/vb_runtime/src/recovery/tests.rs:55-57, 119-122, 170-173, 212-215, 269-272, 294-297, 359-362, 489-492` lock the same contract with typed assertions. Per `proof-seeds.jsonl ps-vb815l8-001`: "Verus/Kani/Flux are NOT in scope per bead scope." |
| **kani** | ps-vb815l8-001/002/003/004/005/006/007 | TEST-ONLY bead; no new production harness introduced. The Kani production-binding gate is N/A since no Verus-style production spec is in scope. The 8 existing unit tests cover the contract surface; a Kani proof of unit-variant equality adds no coverage beyond `equality.rs:3-28`. |
| **flux** | ps-vb815l8-001/002/003/004/005/006/007 | TEST-ONLY bead; no refinement types in the changed surface. The `assert_eq!` is the most-refined form for unit-variant equality. No new refinement obligation can be stated. |
| **proptest** | ps-vb815l8-001/002/003/004/005/006/007 | Per `proof-seeds.jsonl ps-vb815l8-001`: "no new proptest required; existing canonical coverage is sufficient." The 8 unit tests already cover the seed-shape space. The workspace_tests witness is a single seed (line 50-72); property-based generation is not needed for a one-line replacement. |
| **loom** | ps-vb815l8-001/002/003/004/005/006/007 | The test is single-threaded, no `async`, no `tokio`, no `Mutex`, no `RwLock`. The runtime boundary is synchronous. |
| **miri** | ps-vb815l8-001/002/003/004/005/006/007 | Both `crates/vb_runtime/src/recovery.rs:1` and the target test file have `#![forbid(unsafe_code)]`. No UB paths exist. |
| **tla+** | ps-vb815l8-001/002/003/004/005/006/007 | No state machine, no scheduling, no temporal property. Per `proof-planner` SKILL.md: "TLA+ removed; temporal workflows are covered by loom + proptest." This bead has no temporal surface. |
| **fuzz / cargo-fuzz** | ps-vb815l8-001/002/003/004/005/006/007 | The test exercises a single manually-constructed seed. The 8 unit tests already cover the contract surface across 8 seed shapes. Fuzz would not add coverage to a single-typed-error contract. |
| **code-review** | (subsumed) | Code review is owned by `test-reviewer` and `black-hat-reviewer` downstream, not by proof-planner. No code-review obligation row. |
| **cargo-mutants** | (out of scope) | Bead is TEST-ONLY and adds zero new branches. Mutation testing is not a required lane per the controller directive ("2-3 obligations"). |

## Legend

- ✅ = Active lane with at least one obligation row
- — = Not applicable (with concrete evidence above)
- subsumed = owned by a downstream agent

## Lane Profile Summary

| Selected lane | Obligation count | Behavior-affecting rows |
|---|---|---|
| cargo-test | 2 (PO-001, PO-002) | 0 (false) |
| source-lint | 2 (PO-003, PO-004) | 0 (false) |
| **Total** | **4** | **0** |

Within controller budget of 2-3 obligations per lane (cargo-test has 2 test-runs, source-lint has 1 main + 1 sub-gate; reviewer may collapse to 3 by merging PO-002 into PO-001 if strictly required).
