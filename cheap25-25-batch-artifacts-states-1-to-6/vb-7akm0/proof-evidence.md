# Proof Evidence — vb-7akm0

**Bead:** vb-7akm0
**Title:** Lint: remove `#[allow(unreachable_pub)]` suppressions by narrowing visibility (P1 bug)
**State:** Go-skill State 5 (Proof Evidence Scaffold)
**Workspace:** `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-7akm0`
**Source checkout:** `/home/lewis/src/velvet-ballistics` (coordination only)
**Generated:** 2026-07-01
**Owner:** proof-writer (State 5)

---

## STATUS

**PENDING_FORMAL_EXECUTION** — this file is the State 5 evidence scaffold. All 6
proof obligations are deferred to State 11 (formal-verifier) because:
- The bead is `behavior_affecting=false` for every Suppression row.
- No formal verifier (Verus/Kani/Flux/Loom/proptest/fuzz/Miri/TLA+) is required.
- All obligations resolve to Rust-local gate executions that belong to the formal-verifier's
  per-gate evidence capture routine, NOT to proof-writer's per-obligation artifact
  authoring routine.

This file does **NOT** claim any post-implementation gate has passed. It records:
1. The 6 obligation IDs and their lane bindings.
2. The 2 gates explicitly named by the task spec as `PENDING_FORMAL_EXECUTION`:
   `moon run :lint-src` and `cargo test --workspace`.
3. The PENDING_FORMAL_EXECUTION command list per obligation, for State 11 execution.
4. The non-applicability evidence for the 8 formal verifier lanes.

---

## 1. Obligations And Bindings

Per `proof-obligations.planned.jsonl` (6 rows; sha-256 `a5a03321ca16dca48e0e4d72a763fc5ec331b3e3e94071e849f1f1e3a0787334`):

### 1.1 Obligation Schedule

| ID | Verifier | Mode | Required | Behavior-Affecting | Status |
|----|----------|------|----------|--------------------|--------|
| `PO-LINT-001`         | `moon-lint-src`     | `verify-standard`        | true | false | PENDING_FORMAL_EXECUTION |
| `PO-COMPILE-001`      | `cargo-check`       | `verify-standard`        | true | false | PENDING_FORMAL_EXECUTION |
| `PO-TEST-001`         | `cargo-test`        | `verify-standard`        | true | false | PENDING_FORMAL_EXECUTION |
| `PO-EXTERN-001`       | `grep` + binding    | `verify-formal-closure`  | true | false | PENDING_FORMAL_EXECUTION |
| `PO-DECISION-001`     | `decision-ack`      | `pre-condition`          | true | false | PENDING_FORMAL_EXECUTION (State 4/7) |
| `PO-DECISION-GREP-001`| `grep`              | `pre-condition`          | true | false | PENDING_FORMAL_EXECUTION (State 4/7) |

### 1.2 Owner State and Rerun-From

All 6 obligations have `owner_state=5` and `rerun_from=5` per `proof-obligations.planned.jsonl`.
The two pre-condition gates (PO-DECISION-001, PO-DECISION-GREP-001) have effective
`owner_state=4` in addition because they MUST resolve before `ApplyTreatment` runs
on categories G.1/G.2.

---

## 2. PENDING_FORMAL_EXECUTION — moon run :lint-src (PO-LINT-001)

### 2.1 Obligation Detail

| Field | Value |
|-------|-------|
| Obligation ID | `PO-LINT-001` |
| Requirement ID | `R-vb-7akm0-001..004,R-vb-7akm0-005..014,R-vb-7akm0-015..018,R-vb-7akm0-019,R-vb-7akm0-020,R-vb-7akm0-021,R-vb-7akm0-022,R-vb-7akm0-023,R-vb-7akm0-024,R-vb-7akm0-025,R-vb-7akm0-026,R-vb-7akm0-028` |
| Contract clause | `LS-VESTIGIAL.1..4,LS-INTERNAL.1..7,LS-TAINT.1..3,LS-SCHEMA.1..4,LS-DIAG.1..3,LS-REEXPORT.1,LS-ORPHAN.1..2,LS-LIFECYCLE.1,LS-INVARIANT.1,LS-VERIFY.1` |
| Risk | `low` |
| Behavior-affecting | `false` |

### 2.2 Command

```bash
moon run :lint-src 2>&1 | tee .evidence/lint-src/run-001/exit-code.txt
```

Underlying command (per `.moon/tasks/all.yml:46-62`):

```bash
cargo clippy --workspace --lib --bins --examples --all-features
```

### 2.3 Expected Outcome

- **Exit code:** `0`
- **`unreachable_pub` diagnostics count:** `0`
- **Surviving `#[allow(unreachable_pub)]` attributes:** `0`
- **Raw log:** `.evidence/lint-src/run-001/clippy-output.log`

### 2.4 Status

**`PENDING_FORMAL_EXECUTION`** — gated on State 7 implementation-owner completing
the 25 attribute changes and State 11 formal-verifier executing the command from
the isolated workspace.

### 2.5 Why PENDING_FORMAL_EXECUTION (Not Yet Executed)

Proof-writer (State 5) is restricted to writing proof/model/harness artifacts, not
executing machine gates. PO-LINT-001 depends on:

1. The 25 attribute changes being applied by the implementation owner.
2. `moon run :lint-src` being executed from a clean isolated workspace on the post-change tree.

Neither step has been performed yet in the State 5 cycle.

---

## 3. PENDING_FORMAL_EXECUTION — cargo test --workspace (PO-TEST-001)

### 3.1 Obligation Detail

| Field | Value |
|-------|-------|
| Obligation ID | `PO-TEST-001` |
| Requirement ID | `R-vb-7akm0-002,R-vb-7akm0-003,R-vb-7akm0-004,R-vb-7akm0-005..014,R-vb-7akm0-015..018,R-vb-7akm0-020,R-vb-7akm0-021,R-vb-7akm0-025,R-vb-7akm0-027,R-vb-7akm0-029` |
| Contract clause | `LS-VESTIGIAL.2..4,LS-INTERNAL.1..7,LS-TAINT.1..3,LS-SCHEMA.1..4,LS-DIAG.2..3,LS-LIFECYCLE.1,LS-INVARIANT.2,LS-VERIFY.2` |
| Risk | `medium` |
| Behavior-affecting | `false` |

### 3.2 Command

```bash
cargo test --workspace 2>&1 | tee .evidence/cargo-test/run-001/exit-code.txt
```

### 3.3 Expected Outcome

- **Exit code:** `0`
- **Test count delta:** `0` (same as pre-change baseline captured in `baseline-report.md`)
- **Specific crates passing:** `vb_validate` (`--lib`), `vb_cli` (`--lib`), `workspace_tests` (`--tests`)
- **Integration tests passing:** `crates/workspace_tests/tests/derived_status_replay_timeline_tests.rs`, `crates/vb_cli/tests/lifecycle_integration.rs`
- **Raw log:** `.evidence/cargo-test/run-001/cargo-test-output.log`
- **Test count summary:** `.evidence/cargo-test/run-001/test-count.txt`

### 3.4 Status

**`PENDING_FORMAL_EXECUTION`** — gated on State 7 implementation-owner applying
attribute changes and State 11 formal-verifier executing the command from the
isolated workspace.

### 3.5 Why PENDING_FORMAL_EXECUTION

Proof-writer (State 5) is restricted to writing proof/model/harness artifacts, not
executing `cargo test`. PO-TEST-001 also depends on the implementation owner
populating `.beads/vb-7akm0/decision-ack.md` (PO-DECISION-001) and applying the
correct decisions to categories G.1/G.2 before category-G narrowing.

---

## 4. PENDING_FORMAL_EXECUTION — Other Four Obligations

### 4.1 PO-COMPILE-001 (`cargo check`)

**Command:**

```bash
cargo check --workspace --all-features 2>&1 | tee .evidence/cargo-check/run-001/exit-code.txt
```

**Expected outcome:** exit code 0; no compile errors after `pub fn → fn` and `pub → pub(crate)`
narrowings; cargo reports `Finished dev` profile.

**Status:** `PENDING_FORMAL_EXECUTION` (State 7 implementation-owner + State 11 formal-verifier).

### 4.2 PO-EXTERN-001 (grep + Verus binding gates)

**Command:**

```bash
mkdir -p .evidence/grep-externality/run-001 && \
  grep -R 'vb_validate::diag::diag_codes::CODE_' . --exclude-dir=.git --exclude-dir=.evidence \
    > .evidence/grep-externality/run-001/diag-codes-CODE_.txt 2>&1 ; \
  grep -R 'diagnostic_from_error\|error_code' . --exclude-dir=.git --exclude-dir=.evidence \
    > .evidence/grep-externality/run-001/diagnostic-render.txt 2>&1 ; \
  grep -R 'vb_validate::diagnostic::' . --exclude-dir=.git --exclude-dir=.evidence \
    > .evidence/grep-externality/run-001/diagnostic-reexport.txt 2>&1 ; \
  grep -R 'vb_cli::lifecycle::test_helpers::create_run_header' . --exclude-dir=.git --exclude-dir=.evidence \
    > .evidence/grep-externality/run-001/lifecycle-create-run-header.txt 2>&1 ; \
  bash scripts/check-verus-production-binding.sh \
    > .evidence/production-binding/run-001/check-verus-prod-binding.txt 2>&1 ; \
  echo $? > .evidence/production-binding/run-001/check-verus-prod-binding-exit.txt ; \
  bash scripts/check-production-inner-drift.sh \
    > .evidence/production-binding/run-001/check-prod-inner-drift.txt 2>&1 ; \
  echo $? > .evidence/production-binding/run-001/check-prod-inner-drift-exit.txt
```

**Expected outcome:**
- `scripts/check-verus-production-binding.sh` exit code 0 — Verus production-bound specs
  continue to bind via STRONG (`#[path]`) or WEAK (production_inner mirror); the bead
  does not break the binding by altering `vb_cli::commands_incident::IncidentReport` visibility.
- `scripts/check-production-inner-drift.sh` exit code 0 — production_inner mirror drift = 0.
- Grep evidence files show:
  - (1) `diag_codes` grep returns expected internal hits
    (`diag/diag_render/parts.rs`, `diag/diag_render/parts/contract.rs`, `diag/tests.rs`)
    and 0 unexpected external hits if option (b) PubToPubCrate is chosen.
  - (2) `diagnostic_from_error`/`error_code` show downstream consumers in
    `workspace_tests/tests/capability_contract_schema.rs`,
    `diagnostic_code_ranges_test.rs`, `e2e_diagnostic_chain.rs`,
    `vb_test_validate_diagnostic_behavior.rs`, diagnostic_chain tests.
  - (3) `create_run_header` shows consumers in
    `workspace_tests/tests/derived_status_replay_timeline_tests.rs` and
    `vb_cli/tests/lifecycle_integration.rs`.

**Status:** `PENDING_FORMAL_EXECUTION`.

### 4.3 PO-DECISION-001 (`decision-ack`)

**Gate command:**

```bash
test -f .beads/vb-7akm0/decision-ack.md && \
  grep -E '^Decision: (RetireOrphanTest|RegisterOrphanTest)$' .beads/vb-7akm0/decision-ack.md > /dev/null && \
  echo 'decision-ack OK' && tee .evidence/decision-ack/run-001/decision-exit.txt
```

**Expected outcome:**
- `.beads/vb-7akm0/decision-ack.md` exists with exactly one `Decision:` line.
- Decision value is either `RetireOrphanTest` (default per
  `codebase-map.md` Open Questions recommendation 1 and `contract.md §2.7 LS-ORPHAN.1`)
  or `RegisterOrphanTest`.
- Rationale block present.

**Status:** `PENDING_FORMAL_EXECUTION` (State 4/7 — implementation owner writes
`decision-ack.md` before ApplyTreatment runs on categories G.1/G.2).

### 4.4 PO-DECISION-GREP-001 (`grep IncidentReport` precondition)

**Gate command:**

```bash
grep -R 'IncidentReport' verification/verus/production_inner/ \
  > .evidence/grep-precondition/run-001/incident-report-production-inner.txt 2>&1 ; \
if [ -s .evidence/grep-precondition/run-001/incident-report-production-inner.txt ] ; then \
  echo 'PRECONDITION_FAILED' > .evidence/grep-precondition/run-001/incident-report-precondition-exit.txt ; \
  exit 1 ; \
else \
  echo 'PRECONDITION_OK' > .evidence/grep-precondition/run-001/incident-report-precondition-exit.txt ; \
fi
```

**Expected outcome:**
- `incident-report-production-inner.txt` is empty (0 bytes or no content lines).
- `incident-report-precondition-exit.txt` contains `PRECONDITION_OK`.

**Status:** `PENDING_FORMAL_EXECUTION` (State 4/7 — must run BEFORE ApplyTreatment
on `commands_incident.rs`).

---

## 5. Non-Applicable Formal Verifier Lanes

Per `proof-strategy.md` §3.7 and `proof-plan-review.md` rows 9-16:

| Lane | Verdict | Non-Applicability Evidence |
|------|---------|----------------------------|
| `verus` | `NOT_APPLICABLE` | No spec/proof fn changes; the bead is a Rust-local visibility refactor with no refinement types. The existing Verus proofs at `verification/verus/extern_vb_ahfl_bounds_production.rs` bind to `production_inner` mirrors and do not consume `vb_cli::commands_incident::IncidentReport` directly (delivery-scope.jsonl:32). |
| `kani` | `NOT_APPLICABLE` | No new `#[kani::proof]` harnesses; no unsafe code is introduced or removed. Existing kani harnesses consume canonical `vb_validate::gates::*` (delivery-scope.jsonl:31), NOT the duplicates in `gate_07_stack.rs`…`gate_13_cycles.rs`. |
| `flux-rs` | `NOT_APPLICABLE` | No refinement types in scope; the touched files contain no `#[flux::*]` annotations; `cargo flux -p vb_validate --message-format human` is unaffected. |
| `loom` | `NOT_APPLICABLE` | No concurrent actors introduced; the touched files are all `#[cfg(test)] mod` or single-threaded production code. |
| `proptest` | `NOT_APPLICABLE` | No new property-based tests; the bead is a refactor of existing test infrastructure, not a new test surface. |
| `cargo-fuzz` | `NOT_APPLICABLE` | No fuzz targets introduced; the touched files are not parser/compiler code. |
| `miri` | `NOT_APPLICABLE` | No `unsafe` blocks in scope (Holzman Rust §"No unsafe"); Miri cannot add value to a visibility refactor. |
| `tla-plus` | `NOT_APPLICABLE` (globally removed) | TLA+ removed from repo per proof-planner skill §"TLA+ removed". No temporal/workflow behavior changes. |

No `cargo verus`, `cargo kani`, `cargo flux`, `RUSTFLAGS=--cfg loom cargo test`,
`cargo test <proptest>`, `cargo fuzz run`, `cargo miri`, or `tla` commands
are planned for this bead.

---

## 6. State-5 Evidence Summary

| Artifact | Required | Created by State 5 | Held in State 7 | Executed by State 11 |
|----------|----------|--------------------|-----------------|----------------------|
| Verus spec file | NO (NOT_APPLICABLE) | n/a | n/a | n/a |
| Kani harness file | NO (NOT_APPLICABLE) | n/a | n/a | n/a |
| Flux refinement file | NO (NOT_APPLICABLE) | n/a | n/a | n/a |
| Loom model file | NO (NOT_APPLICABLE) | n/a | n/a | n/a |
| proptest property file | NO (NOT_APPLICABLE) | n/a | n/a | n/a |
| fuzz target file | NO (NOT_APPLICABLE) | n/a | n/a | n/a |
| Miri harness file | NO (NOT_APPLICABLE) | n/a | n/a | n/a |
| TLA+ spec file | NO (NOT_APPLICABLE) | n/a | n/a | n/a |
| `proof-writer-report.md` | YES | yes (this bead, this file) | n/a | n/a |
| `proof-evidence.md` | YES | yes (this file) | n/a | n/a |
| `trusted-base-ledger.jsonl` | YES (empty) | yes (0 bytes) | n/a | n/a |
| `state5` ledger row | YES | yes (next row appended) | n/a | n/a |
| `.evidence/lint-src/run-001/exit-code.txt` | YES | n/a | n/a | yes |
| `.evidence/cargo-check/run-001/exit-code.txt` | YES | n/a | n/a | yes |
| `.evidence/cargo-test/run-001/exit-code.txt` | YES | n/a | n/a | yes |
| `.evidence/grep-externality/run-001/*.txt` | YES | n/a | n/a | yes |
| `.evidence/production-binding/run-001/exit.txt` | YES | n/a | n/a | yes |
| `.evidence/decision-ack/run-001/decision-exit.txt` | YES (pre-condition) | n/a | yes (State 7) | n/a |
| `.evidence/grep-precondition/run-001/incident-report-precondition-exit.txt` | YES (pre-condition) | n/a | yes (State 7) | n/a |

---

## 7. PENDING_FORMAL_EXECUTION — Final Status

**ALL 6 obligations are PENDING_FORMAL_EXECUTION.**

Specifically named by the task spec:

1. `moon run :lint-src` (PO-LINT-001) — PENDING_FORMAL_EXECUTION (State 11).
2. `cargo test --workspace` (PO-TEST-001) — PENDING_FORMAL_EXECUTION (State 11).

The remaining 4 obligations are likewise PENDING_FORMAL_EXECUTION (State 11 or State 4/7
pre-condition). No formal-verifier artifact is required.

---

## 8. Trust Ledger Entries

`trusted-base-ledger.jsonl` is intentionally empty (0 bytes). The 12 trusted items
in `trusted-base-plan.md` (TBP-001..TBP-012) are categorical trusted infrastructure
and require no per-bead ledger entries. The 6 verified items (VBP-001..VBP-006) are
evidence emissions bound to gate executions in State 11, not trust allowances.

---

## 9. Decision

**Status:** `PENDING_FORMAL_EXECUTION` — evidence scaffolded, awaiting State 11
execution and State 7 implementation.

No blockers. No production-code edits. No formal-verifier artifacts required.

---

*Generated by proof-writer skill. State 5. Behavior-affecting: false (every obligation
is `behavior_affecting=false`).*
