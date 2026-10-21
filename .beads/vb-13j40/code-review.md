# Independent artifact repair re-review

**Bead**: vb-13j40 (discovered-from:vb-9agmk)  
**State**: renewed independent CODE review  
**Reviewer**: black-hat-reviewer / independent Codex reviewer  
**Source checkout**: /home/lewis/src/isoloated/velvet-ballistics-vb-9agmk-code-review-sol-20261021  
**Attempt**: 2, including narrow helper-size follow-up  
**Functional repair source**: 57f307dac5dd59153f46f1d3e7b92ea73e2de35b  
**Final approved source**: 6311991293c539d3016419acea3bb5c2e1f4f843 (codex/vb-9agmk-execution-binding-split)  
**Prior rejection**: report832840178a4de0a8d2dc391ad2bdc2ec7607da6b, exact948f19cb source.  
**Contract**: candidate .beads/vb-9agmk/contract.md C1-C8; C7 registry parity and C8 actual execution-tuple binding address the prior findings.

## Gate Result

**STATUS: APPROVED** for scoped artifact repair source6311991293c539d3016419acea3bb5c2e1f4f843.

Both original functional findings are closed with independent exact-error public oracles. The new42-line binding helper in57f was mechanically extracted in6311; final helpers are21/16/19 physical lines. This approval does not cover CLI, retained-input recovery, full v1, full CI, unrelated style debt, or independent TEST approval.

## PHASE 1: Contract & Bead Parity

| Requirement / prior finding | Status | Source and executable evidence |
|---|---|---|
| FINDING-001: bind submitted IR | Closed | execution_binding::validate independently verifies the stored artifact for policy; compare_execution_tuple serializes actual workflow.to_parts and compares exact artifact.ir. Altered constant99 with retained claimed digest rejects with exact AdmissionArtifactInvalid. |
| FINDING-001: bind supplied contracts | Closed | Entire supplied slice compared to artifact.action_contracts. Strict Writes/AtLeastOnceExternal/Unsafe substitution rejects through both public contract APIs and direct queued processing. Matching dense pure contract submits and ticks successfully. |
| FINDING-002 / C7: sparse and duplicate registry rejection | Closed | Core validate_action_registry enforces checked u16 index==ID and reconstructs ActionName. Compiler/storage invoke it. Oracles assert exact IndexMismatch fields for sparse[7] and duplicate[0,0], plus dense controls. |
| C8 processing before allocation/state/events | Pass | Common chunk_001 path compares tuple before prepare_run_slots, frame construction, trace and acceptance events. Queued IR/contract substitutions fail with0active runs and no acceptance events. |
| Existing runtime validation precedence | Pass in scope | Existing duplicate/empty/dense/capability/workflow-binding checks retain order; tuple check follows them.13 validate_contracts_ tests pass, including duplicate-before-index-mismatch. |
| Verifier digest includes new registry source | Pass in inspected diff | verifier_digest includes action/registry.rs;20-test suite checks independent provenance/closure digests. |
| Claim honesty | Preserved | Repair introduces no compiler durability, executor conformance, formal proof or speed claim. |

Runtime retains its existing duplicate-first diagnostic passes. Strict/Journaled also verify through the core-backed artifact verifier, so checked registry shape agrees without changing public runtime diagnostic precedence. All four Runtime submission shapes route to the new binding guard, and the common processing path covers queued submissions. Relaxed deliberately retains its existing policy behavior.

## PHASE 2: Farley Engineering Rigor

| Repair function | Physical lines | Limit | Status |
|---|---|---|---|
| core/action/registry.rs::validate_action_registry | 21 (29-49) | 25 | Pass |
| execution_binding.rs::validate | 21 (7-27) | 25 | Pass |
| execution_binding.rs::load_artifact | 16 (29-44) | 25 | Pass |
| execution_binding.rs::compare_execution_tuple | 19 (46-64) | 25 | Pass |
| Introduced helper parameters | At most4 | 5 | Pass |

This re-review assesses the repair functions and previously reported defects. It does not certify repository-wide function-size compliance or expand into unchanged initial verifier/ledger functions. Production diff57f..6311 contains only execution_binding.rs; extraction preserves policy, error mapping and comparison order. The intervening child adds fixture/evidence work rather than production behavior.

## PHASE 3: Holzman Rust (The Big 6)

| Rule | Status |
|---|---|
| No unsafe/unwrap/expect/panic/todo/unimplemented/dbg in introduced repair modules | No occurrence observed in inspected modules; no whole-repository lint claim. |
| Checked identifiers / safe access | u16::try_from(index), iterators and typed registry errors; no unchecked indexing introduced. |
| Verify before run allocation | Common processing binding precedes prepare_run_slots and frame construction. |
| Guard preservation | Semantic registry/capability guards remain ahead of new tuple comparison;13 focused tests pass. |
| Evidence honesty | No inherited author result counted as fresh reviewer verification. |

Applicable Holzmann references read in this review session: references/nasa-jpl-standards.md and references/runtime-performance-architecture.md under /home/lewis/.agents/skills/holzman-rust/. Master and repository pinned governance remain controlling. Broad zero-slippage/CI gates are outside the explicit focused re-review scope.

## PHASE 4: Ruthless Simplicity & DDD

| Check | Status |
|---|---|
| Trusted execution product at consumer boundary | Actual workflow bytes and full contract registry checked together; claimed digest alone no longer authorizes a different executable tuple. |
| Shared registry invariant | Compiler/storage use the core validator; runtime public precedence remains intact. |
| New nullable lifecycle / bool state machine | None introduced by this repair. |
| Mechanical helper extraction | Bounded load/map and compare helpers, no new generic framework or behavioral branch. |

## PHASE 5: The Bitter Truth

The original counterexample observers were reversed into regressions expecting exact typed rejection. They cover public Runtime admission and direct queued processing, with valid controls that actually tick. The revised runtime rejects caller-owned executable data that differs from the checked artifact. This scoped result is not a recovery theorem or full v1 acceptance.

## Findings (Ordered by Severity)

| Finding | Severity | Location | Status |
|---|---|---|---|
| Original FINDING-001: executable tuple substitution | HIGH/P1 | execution_binding.rs:23,58-61 and call sites | Closed on57f; independently confirmed on6311 |
| Original FINDING-002: registry parity | MEDIUM/P2 | registry.rs:29; compiler artifact.rs:175; storage verification.rs:134 | Closed on57f; independently confirmed on6311 |
| New helper exceeded25 lines in57f | LOW/P3 | old execution_binding.rs:6-47 | Closed by6311 extraction |

No open actionable finding remains within this repair re-review scope.

## Quality Gates

| Source / gate | Actual result | Evidence |
|---|---|---|
| 57f independent reversed counterexamples | Exit0;3pass,0fail,0ignore | reversed-probes.log / command JSON |
| 57f public artifact suite | Exit0;20pass,0fail,0ignore,0filtered | candidate-focused.log / command JSON |
| 6311 independent reversed counterexamples | Exit0;3pass,0fail,0ignore | 6311-reversed-probes.log / command JSON |
| 6311 public artifact suite | Exit0;20pass,0fail,0ignore,0filtered | 6311-candidate-focused.log / command JSON |
| 6311 runtime validate_contracts_ lane | Exit0;13pass,0fail,0ignore,1901filtered | 6311-validation-precedence.log / command JSON |
| Initial reviewer probe compile | Exit101; corrected reviewer API-shape mistakes | reversed-probes-attempt1.log; snapshot is not Result; tick returns Result<bool>. Not candidate defects. |
| Full1596storage/full1914runtime/strictclippy | Author-reported only, not rerun by reviewer | Candidate review-repair-evidence.md and raw author logs. |
| Broad moon ci / full v1 / independent TEST review | Not supplied | Separate owners and gates. |

Final source has36 independently executed focused tests (3+20+13), all passing. Earlier23 executions on57f are retained separately. Commands use user systemd scopes, MemoryHigh12G/MemoryMax16G/MemorySwapMax0, Cargo jobs2, timeout180s and RUSTC_WRAPPER cleared. JSON retains exact argv, actual exits and source pins.

Probe source is archived beside this report. It was temporarily copied under crates/workspace_tests/tests/vb_13j40_review_regressions.rs and moved back after execution; no production or executable test source is published by the reviewer. To reproduce, copy it to that path at the logged source and run the command. Unlike the first review's observers, this source asserts rejection and should pass after repair. Its helper-module unused-function warning is retained and is not a strict lint pass.

## Verdict

**STATUS: APPROVED** for exact6311991293c539d3016419acea3bb5c2e1f4f843 within the stated scope. Both functional findings and the new helper-size issue are closed. Original948f remains a historical rejection, superseded only by this exact-source scoped re-review.

## Required Repair Actions

None in the reviewed repair slice. CLI caller migration, atomic retained-input admission, full production recovery, independent TEST approval and canonical CI retain their separate owners/gates. Parent closure belongs to the coordinating owner; this reviewer closes only vb-13j40.
