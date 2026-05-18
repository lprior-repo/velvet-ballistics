# Proof Review — vb-vt2f Verus waiver evidence repair

## Findings

No blocking findings for `WAIVER-VERUS-VT2F-002`.

Informational caveat: this review approves only the State 6 Verus-waiver evidence row. It does not close the separate State 11 `PRE-001`/`INV-002` `.beads/vb-vt2f/test-review.md` public-surface audit gap recorded in `.beads/vb-vt2f/formal-verification-report.md:38`.

## Scope

- Bead: `vb-vt2f`
- Sublane: `State 6 proof-review-waiver-evidence-repair-after-state11-rejection`
- Reviewed obligation: `WAIVER-VERUS-VT2F-002`
- Reviewed prerequisite obligations/evidence:
  - `TLA-VT2F-LIFECYCLE-001`
  - `TLA-VT2F-STRICT-ADMISSION-001`
  - `KANI-VT2F-RUNTIME-FACADE-001`
  - `KANI-VT2F-SHARD-LOWER-001`
  - `PROJ-EQ-VT2F-001`
  - BDD direct API nextest
  - acceptance catalog nextest
  - `moon ci`

## Commands Run

```text
pwd -P && test -s ".beads/vb-vt2f/proof-obligations.jsonl" && test -s ".beads/vb-vt2f/proof-obligations.planned.jsonl" && test -s ".beads/vb-vt2f/proof-evidence.md" && test -s ".beads/vb-vt2f/formal-verification-report.md" && test -s ".beads/vb-vt2f/contract-verification-review.md" && test -s ".beads/vb-vt2f/machine-gate-report.md" && python -c '...jsonl validation...'
```

Result: PASS; cwd `/home/lewis/src/bd-vb-vt2f-bdd`; `proof-obligations.jsonl: valid jsonl lines=40`; `proof-obligations.planned.jsonl: valid jsonl lines=40`.

```text
rtk grep -n "WAIVER-VERUS-VT2F-002|TLA-VT2F-LIFECYCLE-001|TLA-VT2F-STRICT-ADMISSION-001|KANI-VT2F-RUNTIME-FACADE-001|KANI-VT2F-SHARD-LOWER-001|PROJ-EQ-VT2F-001|VERIFICATION:- SUCCESSFUL|0 of 489 failed|0 of 122 failed|13 tests run: 13 passed|9015 tests run: 9015 passed|STATUS: APPROVED|non-vacuum|Verus|TLC|No error" ".beads/vb-vt2f/proof-evidence.md" ".beads/vb-vt2f/formal-verification-report.md" ".beads/vb-vt2f/contract-verification-review.md" ".beads/vb-vt2f/machine-gate-report.md" ".beads/vb-vt2f/proof-obligations.jsonl"
```

Result: PASS for prerequisite evidence scan; 50 matches across 5 files.

```text
rtk grep -n "ASSUME|assume|axiom|admit|sorry|trusted|unimplemented|todo|unwind|invariant|PROPERTY|THEOREM|proof fn|requires|ensures|loom::model|fuzz_target|proptest!|kani::|unsafe|bounded_any|stub" "crates/vb_runtime/src/kani_vt2f_runtime_facade.rs" "crates/vb_runtime/src/kani_vt2f_shard_lower_semantics.rs" ".beads/vb-vt2f/proof-evidence.md" ".beads/vb-vt2f/proof-writer-report.md" ".beads/vb-vt2f/proof-architecture-report.md" ".beads/vb-vt2f/verification-layers.md"
```

Result: PASS for vacuity scan. The current Kani projection source files contain `#![forbid(unsafe_code)]`, `#[kani::proof]`, `kani::any`, and `kani::cover!`; no source-level `kani::assume`, stubs, `bounded_any`, or unsafe body was found in the two reviewed projection kernels. Historical `assume(false)` hits in `.beads/vb-vt2f/proof-evidence.md:74-75,112-113` belong to explicitly superseded concrete Kani timeout attempts, not current approval evidence.

## Evidence Mapping

| Obligation / gate | Review decision | Evidence |
|---|---:|---|
| `TLA-VT2F-LIFECYCLE-001` | PASS accepted | `.beads/vb-vt2f/formal-verification-report.md:22` records TLC PASS over `3600 states generated, 1302 distinct states found`, no error. |
| `TLA-VT2F-STRICT-ADMISSION-001` | PASS accepted | `.beads/vb-vt2f/formal-verification-report.md:23` records TLC PASS over `2892 states generated, 1096 distinct states found`, no error; `.beads/vb-vt2f/proof-evidence.md:32-48` includes raw TLC output. |
| `KANI-VT2F-RUNTIME-FACADE-001` | PASS accepted as projection-kernel proof only | `.beads/vb-vt2f/formal-verification-report.md:24` records raw output `/home/lewis/.local/share/opencode/tool-output/tool_e3c075e44001JHnCuU95i322WP`, `0 of 489 failed`, `7 of 7 cover properties satisfied`, `VERIFICATION:- SUCCESSFUL`. |
| `KANI-VT2F-SHARD-LOWER-001` | PASS accepted as projection-kernel proof only | `.beads/vb-vt2f/formal-verification-report.md:25` records raw output `/home/lewis/.local/share/opencode/tool-output/tool_e3c0783ae001tDMG6LaQ9Savko`, `0 of 122 failed`, `8 of 8 cover properties satisfied`, `VERIFICATION:- SUCCESSFUL`. |
| `PROJ-EQ-VT2F-001` | APPROVED as manual trusted-boundary review | `.beads/vb-vt2f/contract-verification-review.md:3,43-50,54` approves projection parity, boundary limits, and non-overclaiming; `.beads/vb-vt2f/proof-evidence.md:161-163` states this is manual trusted projection, not executable concrete refinement. |
| BDD direct API nextest | PASS accepted | `.beads/vb-vt2f/formal-verification-report.md:20` records `13 tests run: 13 passed, 0 skipped`; `.beads/vb-vt2f/machine-gate-report.md:15,25` records the same affected direct API target green after implementation repair. |
| acceptance catalog nextest | PASS accepted | `.beads/vb-vt2f/formal-verification-report.md:21` and `.beads/vb-vt2f/machine-gate-report.md:26` record `13 tests run: 13 passed, 0 skipped`. |
| `moon ci` | PASS accepted | `.beads/vb-vt2f/formal-verification-report.md:26` records raw output `/home/lewis/.local/share/opencode/tool-output/tool_e3c0a1056001aQ0u3hKC4ns2He`, `9015 tests run: 9015 passed (1 slow), 2 skipped`, `Tasks: 20 completed (4 cached)`. |

## Non-vacuum Verus Infeasibility Finding

`WAIVER-VERUS-VT2F-002` is APPROVED for bead `vb-vt2f` only.

Rationale: a non-vacuum Verus proof would need specifications bound to actual `vb_runtime`/`vb_core` executable transition functions with `requires`/`ensures`, not detached ghost models. The current reviewed target is mutable runtime/shard/admission/ask behavior with store selection, scheduler/public shell, trace/journal/counter observations, and Kani-only projection kernels. `.beads/vb-vt2f/verification-layers.md:41-45` explicitly records that no approved non-vacuum Verus proof exists and that executable Verus obligations must replace the waiver if a pure transition kernel is extracted or runtime/core semantics change. `.beads/vb-vt2f/proof-strategy.md:109-125` makes the waiver candidate-only and requires this reviewer finding before approval.

I find direct non-vacuum Verus binding infeasible in this repair without production refactoring or extracting a pure transition kernel. Approving the waiver is not a claim that Verus proved the mutable implementation; it is an explicit risk acceptance backed by passed TLA+ temporal models, passed owner-authorized Kani projection kernels, manual projection-equivalence review, BDD/catalog execution, and green `moon ci`.

## Waiver Decision

- `WAIVER-VERUS-VT2F-002`: APPROVED.
- Scope limit: bead `vb-vt2f` only.
- Non-reuse caveat: not reusable as a concrete-runtime Verus/Kani equivalence proof.
- Expiry: before any runtime, shard, admission, ask, action failure, journal, trace, or accepted-artifact store-selection semantic edit; also expires if a pure transition kernel is extracted, at which point executable Verus obligations must replace the waiver.

## Final Decision

STATUS: APPROVED
