# Proof Strategy — vb-0sps Generated-vs-IR BDD Parity (State 4, attempt 3-of-7)

## Authority and scope

- Bead: `vb-0sps` / `VB-BDD-CATALOG-007`.
- State: 4 proof planning after State 3 canonical ledger repair.
- Dispatch manifest: `.beads/vb-0sps/dispatch-manifest-state4-after-contract-repair-attempt3.json`.
- Delegate: `proof-planner`.
- Source scope: isolated workspace only, `/home/lewis/src/bd-vb-0sps-bdd`.
- Non-goal preserved: this plan does **not** reactivate generated/maxperf release scope, PGO, benchmark ratios, `compile --emit rust` release readiness, or whole-fleet verification.
- Contract repair input: State 3 canonical ledger now has exact rows for all previously missing clauses (`PRE-003`, `PRE-004`, `PRE-005`, `INV-001`, `INV-004`, `INV-005`, `INV-006`); State 6 identified TLA vacuity and missing split configs as remaining blockers.

## Strategy summary

The proof strategy is a layered parity argument across five lanes:

1. **Executable BDD acceptance (State 6, cargo test):** The workspace BDD target exercises public APIs from `vb_core`, `vb_codegen`, and the acceptance catalog. It compares structured observations from IR oracle and generated candidate using exact typed fields, never Debug/display strings.
2. **Focused crate regression gates (State 6, cargo test):** `vb_codegen` tests verify fail-closed generation, emitted runtime constraints, and unsupported diagnostics. `vb_core` tests run conditionally if implementation touches core fixture helpers.
3. **TLA+ temporal model (State 5, tlc):** Owns PRE-004, POST-003/004/005, INV-004/005/006, and divergence sanity. Requires a non-vacuous model with separate IR/generated transition relations and 5 split configs. TLC must complete (exit 0) for each bounded scenario config. No TLA waiver is granted by this bead.
4. **Verus adapter layer (waived, future binding):** Owns PRE-003, POST-001/002, INV-002/003 after concrete parity adapters exist. Currently waived per `WAIVER-VERUS-ADAPTERS-001` with full metadata; waiver expires when adapters exist or before State 6 if they already exist.
5. **NON-GOAL enforcement (State 4, contract-verification-reviewer):** Manual review confirms no maxperf/release scope creep in any vb-0sps artifact.

## Risk classification

| Risk tag | Description | Affected obligations |
|---|---|---|
| `semantic-parity` | Terminal result/status/PC differ between IR and generated | POST-001 (BDD), INV-004 (TLA) |
| `taint-parity` | Slot taints or result taint differ | POST-001 (BDD), INV-003 (Verus waiver) |
| `journal-parity` | Event sequence order/fields differ | POST-005 (TLA+BDD), TLA-DIVERGENCE-SANITY |
| `suspension-semantics` | Suspension kind/metadata differ or mode advances past boundary | POST-003 (TLA), INV-005 (TLA) |
| `resume-semantics` | Post-resume output/event/PC differ | POST-004 (TLA), PRE-004 (TLA) |
| `typed-error-parity` | Error variant/fields differ or wrong adapter mapping | POST-002 (Verus waiver+BDD) |
| `unsupported-ir` | Unsupported IR silently emits/falls back instead of failing closed | PRE-005 (TLA+BDD), POST-006 (BDD), INV-006 (TLA) |
| `release-scope-creep` | Evidence misrepresented as maxperf/release readiness | INV-007 (NON-GOAL lane) |
| `adapter-vacuum` | Verus/TLA+ targets created without binding to real code | VERUS lanes (waived), TLA lanes (planned for State 5) |
| `tla-vacuity` | TLA model proves parity-by-construction rather than IR-vs-generated refinement | All TLA lanes (State 5 must repair) |
| `concurrency` | Concurrent IR/generated execution introduces non-determinism | TLA lanes (addressed via separate transition relations) |

## Lane map (layer → verifier → obligations)

| Layer | Verifier | Owner | Obligations |
|---|---|---|---|
| `manual-qa` | cargo test | State 6 | PRE-001, PRE-002, POST-001, POST-002 (BDD portion), POST-006, POST-007, INV-001, CODEGEN-REG-001, CORE-REG-001 (conditional) |
| `tla-plus` | tlc | State 5 | PRE-004, PRE-005, POST-003, POST-004, POST-005, INV-004, INV-005, INV-006, TLA-DIVERGENCE-SANITY |
| `waiver` | waiver | waived | PRE-003, POST-001 (Verus portion), POST-002 (Verus portion), INV-002, INV-003 |
| `review` | contract-verification-reviewer | State 4 | INV-007 (NON-GOAL lane) |

## Required executable lanes (State 6 evidence-gated)

### Lane BDD-1: generated-vs-IR acceptance target

- **Verifier:** cargo test (BDD)
- **Command:** `TMPDIR=/tmp/opencode cargo test -p velvet-ballastics-workspace-tests --test vb_0sps_generated_ir_parity_bdd`
- **Artifact:** `crates/workspace_tests/tests/vb_0sps_generated_ir_parity_bdd.rs` (created in State 6)
- **Owner state:** 6
- **Required evidence:** all 6 scenario families pass with structured field assertions; see `verification-layers.md` for exact evidence fields.
- **Rerun from:** 6

### Lane BDD-2: acceptance catalog closure

- **Verifier:** cargo test (BDD catalog)
- **Command:** `TMPDIR=/tmp/opencode cargo test -p velvet-ballastics-workspace-tests --test vb_hxm0_acceptance_catalog`
- **Artifact:** `crates/workspace_tests/src/acceptance_catalog.rs`
- **Owner state:** 6
- **Required evidence:** `VB-BDD-CATALOG-007` executable_evidence_target is `Some(...)` and deferred_follow_up_bead is None.
- **Rerun from:** 6

### Lane CODEGEN-REG-001: focused codegen regression gate

- **Verifier:** cargo test
- **Command:** `TMPDIR=/tmp/opencode cargo test -p vb_codegen --all-features`
- **Artifact:** `crates/vb_codegen/src/lib.rs`
- **Owner state:** 6
- **Required evidence:** `validate_generated_subset`, `emit_rust_workflow`, unsupported diagnostics, generated runtime API tests green.
- **Rerun from:** 6

### Lane CORE-REG-001: conditional core regression gate

- **Verifier:** cargo test (conditional)
- **Command:** `TMPDIR=/tmp/opencode cargo test -p vb_core --all-features`
- **Artifact:** `crates/vb_core/src/engine.rs`
- **Condition:** Required only if State 6 implementation touches core engine fixture helpers.
- **Owner state:** 6
- **Required evidence:** core tests pass with touched semantics.
- **Rerun from:** 6

## Formal proof lanes — TLA+ (State 5, planned)

### TLA obligations — plan status and blockers

The TLA model exists at `verification/tla/generated_ir_parity/GeneratedIrParity.tla` and a monolithic `.cfg` exists. State 6 review rejected the model for two independent reasons:

1. **Vacuity (LETHAL):** The model writes identical state to both IR and generated sides in the same action (`LockstepDo`). Separate IR/generated transition relations must replace lockstep actions, plus an explicit `ObservationRefinesOracle` invariant that can fail.
2. **Missing split configs (LETHAL):** The contract and tla-spec.md require 5 split config files. Only a monolithic `.cfg` exists. State 5 must author all 5 split configs.

**No TLA waiver is granted.** The model must be repaired and TLC must complete (exit 0) for each of the 5 bounded scenario configs, or a separately reviewed waiver with full metadata must be approved before State 6.

### TLA-PRE-004 / TLA-POST-003-005 / TLA-INV-004-006 / TLA-DIVERGENCE-SANITY

- **Verifier:** tlc
- **Artifact:** `verification/tla/generated_ir_parity/GeneratedIrParity.tla`
- **Owner state:** 5
- **Commands** (exact from tla-spec.md, split config per scenario family):

  ```
  timeout 120 tlc -config verification/tla/generated_ir_parity/GeneratedIrParity_success.cfg -workers 1 verification/tla/generated_ir_parity/GeneratedIrParity.tla
  timeout 120 tlc -config verification/tla/generated_ir_parity/GeneratedIrParity_suspension_resume.cfg -workers 1 verification/tla/generated_ir_parity/GeneratedIrParity.tla
  timeout 120 tlc -config verification/tla/generated_ir_parity/GeneratedIrParity_typed_error.cfg -workers 1 verification/tla/generated_ir_parity/GeneratedIrParity.tla
  timeout 120 tlc -config verification/tla/generated_ir_parity/GeneratedIrParity_unsupported_reject.cfg -workers 1 verification/tla/generated_ir_parity/GeneratedIrParity.tla
  timeout 120 tlc -config verification/tla/generated_ir_parity/GeneratedIrParity_divergence_sanity.cfg -workers 1 verification/tla/generated_ir_parity/GeneratedIrParity.tla
  ```

- **Expected evidence for success configs (first 4):** TLC exits 0 with no invariant/temporal/deadlock violations.
- **Expected evidence for divergence sanity config (5th):** TLC exits non-zero with `ObservationRefinesOracle` or `SameJournalPrefix` violation under injected candidate fault (negative sanity proving equality is not by construction).
- **Blocker note:** vacuity repair and 5 split configs are State 5 responsibility before TLC can pass.
- **Rerun from:** 5

## Formal proof lanes — Verus (waived, future binding)

### WAIVER-VERUS-ADAPTERS-001 — applies to PRE-003, POST-001 (Verus portion), POST-002 (Verus portion), INV-002, INV-003

- **Status:** `waived`
- **Waiver owner:** State 5 proof-writer plus State 6 contract-verification reviewer
- **Waiver reason:** no concrete adapter exec functions exist at State 3; creating proof-only models violates no-vacuum Verus rule
- **Waiver limitation:** does not formally prove Rust-local comparator correctness
- **Waiver expiry:** expires when `compare_observed_runs`, `normalize_error`, or event-sequence adapters exist, or before State 6 approval if adapters already exist
- **Waiver follow-up:** bind Verus `spec_observed_run_equal` / `proof_compare_observed_runs_sound` and `spec_normalized_error_equal` / `proof_normalized_error_mapping_total` to real adapter exec functions
- **Compensating evidence:** BDD exact structured assertions, single-field mismatch proptest/focused cases, TLA+ temporal refinement for sequence behavior, static review forbidding Debug/display string equality

## NON-GOAL enforcement lane

### INV-007 / NON-GOAL-001 — no maxperf/release scope creep

- **Verifier:** contract-verification-reviewer (manual)
- **Artifact:** all vb-0sps artifacts
- **Owner state:** 4
- **Required evidence:** no claim of maxperf, PGO, generated-vs-IR speed ratio, `compile --emit rust` readiness, or generated execution as current milestone gate appears anywhere in vb-0sps artifacts.
- **Rerun from:** 4

## Rerun policy

| Obligation | Rerun_from | Condition |
|---|---|---|
| BDD-POST-001, BDD-POST-002, BDD-POST-003, BDD-POST-004, BDD-POST-005, BDD-POST-006 | 6 | After State 6 implementation |
| BDD-PRE-001, BDD-PRE-002 | 6 | After State 6 implementation |
| CAT-007-001 | 6 | After acceptance_catalog.rs update |
| CODEGEN-REG-001 | 6 | Always |
| CORE-REG-001 | 6 | Only if core touched |
| TLA-PRE-004, TLA-POST-003, TLA-POST-004, TLA-POST-005, TLA-INV-004, TLA-INV-005, TLA-INV-006 | 5 | After vacuity repair + split config authoring |
| TLA-DIVERGENCE-SANITY | 5 | After divergence sanity config authored and model repaired |
| VERUS-CMP-001, VERUS-ERR-001 | 6 | After adapter exists; waiver expires |
| NON-GOAL-001 | 4 | Always |

## Blockers forwarded to downstream states

1. **TLA vacuity repair (State 5):** The existing model must be redesigned with separate IR/generated transition relations; `LockstepDo` must be replaced; `ObservationRefinesOracle` must be capable of failing.
2. **TLA split configs (State 5):** All 5 config files must be authored; monolithic `.cfg` is insufficient.
3. **TLA TLC completion (State 5):** All 5 TLC runs must exit 0 (first 4) or exit non-zero with expected violation (divergence sanity).
4. **Verus adapter existence (State 5/6):** Waiver `WAIVER-VERUS-ADAPTERS-001` expires when `compare_observed_runs`, `normalize_error`, or event-sequence adapters are added; Verus obligations bind to real exec functions.
5. **Catalog closure (State 6):** `VB-BDD-CATALOG-007` must be updated to point to executable BDD target before `deferred_follow_up_bead` is cleared.

## Validation

```text
python3 -m json.tool .beads/vb-0sps/proof-obligations.planned.jsonl >/dev/null && echo "JSONL: VALID"
```
