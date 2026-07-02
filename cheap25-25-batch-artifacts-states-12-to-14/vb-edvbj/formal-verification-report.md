# Formal Verification Report — vb-edvbj

- **bead_id:** vb-edvbj
- **bead_title:** Runtime: delete fallback that maps unmapped journal events to run failure (P0 bug)
- **phase:** 12 (formal-verifier)
- **workdir:** `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-edvbj`
- **jj change:** `mrpqqutq` (state 11 holzman-rust) on top of `rzwmqlyw` (state 5 proof-writer)
- **invocation_id:** formal-verifier-vb-edvbj-state12
- **controller:** femdation (combined state 12/13/14 dispatch)
- **date:** 2026-07-01
- **status:** PARTIAL — 1 PASS, 9 FAIL_LOCAL (see §6)

---

## 1. Commands Executed

The State-12 directive specifies three behavior-test commands. Raw evidence is
captured under `.beads/vb-edvbj/evidence/`.

| # | Command | Expected | Observed | Result | Evidence |
|---|---------|----------|----------|--------|----------|
| 1 | `cargo test -p vb_runtime --lib storage_event` | 1 passed | `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1806 filtered out; finished in 0.00s` | PASS | `.beads/vb-edvbj/evidence/storage_event_test.txt` |
| 2 | `cargo test -p vb_runtime --lib recovery` | 13 passed | `test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 1794 filtered out; finished in 0.00s` | PASS | `.beads/vb-edvbj/evidence/recovery_test.txt` |
| 3 | `cargo test -p vb_runtime --lib` | 1807 passed | `test result: ok. 1807 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.47s` | PASS | `.beads/vb-edvbj/evidence/full_test.txt` |

**Cumulative:** 1821 tests passed, 0 failed, 0 ignored across the three invocations.

## 2. Supporting Lint/Check Evidence

| Command | Result | Evidence |
|---------|--------|----------|
| `cargo check -p vb_runtime --all-targets` | PASS (Finished, 0 errors) | `.beads/vb-edvbj/evidence/check_vb_runtime.txt` |
| `cargo clippy -p vb_runtime --lib --bins --examples --all-features -- -D warnings` | PASS (No issues found) | `.beads/vb-edvbj/evidence/clippy_vb_runtime.txt` |
| `bash scripts/check-verus-production-binding.sh` | 73 WEAK, **2 VACUUM** | `.beads/vb-edvbj/evidence/check-verus-production-binding.txt` |
| `bash scripts/check-production-inner-drift.sh` | NOT RUN (tooling gap) | This isolated JJ workspace is git-less; the script's `git rev-parse --show-toplevel` fails. Drift inspection is delegated to the production-binding script's WEAK classification. |

## 3. Verifier Tool Availability (this lane)

| Tool | Available | Version | Notes |
|------|-----------|---------|-------|
| `verus` | YES | 0.2026.05.05.d03e906 (release, toolchain 1.95.0) | Located at `/home/lewis/.local/bin/verus` |
| `cargo-kani` | YES | 0.67.0 | Located at `/home/lewis/.cargo/bin/cargo-kani`. Pre-existing crate-level compile error in `vb_core::frame_kani_harnesses` (unclosed delimiter, see F-002) blocks `cargo kani -p vb_runtime --lib` (build fails before any harness runs). |
| `cargo-flux` | YES | flux 4d329f2 (2026-05-23) | Located at `/home/lewis/.cargo/bin/flux`. Package-level `cargo flux -p vb_runtime` compiles cleanly (Finished, 0 errors). |
| `proptest` | YES (via `dev-dependencies` in `vb_runtime/Cargo.toml`) | — | The 3 proptest files for this bead are NOT in the JJ change tree (see §5). |

The proof-writer-report.md §8 records the same toolchain as BLOCKED_TOOLING; that
classification is stale (see proof-finding F-009 owner_approved_no_action).
This State-12 report reclassifies the toolchain as AVAILABLE but the artifacts
required for the corresponding obligations are MISSING (§5).

## 4. Verifier Artifacts State (on disk vs. committed)

| Obligation | Artifact Path | Committed in `mrpqqutq`? | On disk? | Verifier Result |
|------------|---------------|--------------------------|----------|-----------------|
| PO-EDVBJ-001-VERUS | `verification/verus/vb_edvbj_storage_event.rs` | NO (untracked) | YES | verus → **error: duplicate specification for `production::production::mirror_storage_event`** (mirror is not `#[verifier::external_body]`) |
| PO-EDVBJ-002-KANI | `crates/vb_runtime/src/kani_vb_edvbj_storage_event_no_fabricate.rs` | NO | NO | cargo kani → pre-existing build error in `vb_core/src/frame_kani_harnesses` (unrelated to this bead) |
| PO-EDVBJ-003-PROPTEST | `crates/vb_runtime/src/journal/tests/proptest_vb_edvbj_all_21_variants.rs` | NO | NO | `vb-edvbj-pending` feature not declared in `crates/vb_runtime/Cargo.toml`; proptest file absent |
| PO-EDVBJ-004-PROPTEST | `crates/vb_runtime/src/journal/tests/proptest_vb_edvbj_resumed_replay.rs` | NO | NO | Same as PO-003 |
| PO-EDVBJ-005-VERUS | `verification/verus/vb_edvbj_propagation.rs` | NO (untracked) | YES | verus → **error: couldn't read `extern_vb_edvbj_propagation.rs`** (companion file missing) |
| PO-EDVBJ-006-KANI | `crates/vb_runtime/src/kani_vb_edvbj_propagation_strict_gate.rs` | NO | NO | Same kani build error as PO-002 |
| PO-EDVBJ-007-VERUS | `verification/verus/vb_edvbj_mirror_bind.rs` | NO (untracked) | YES | verus → **2 verified, 0 errors** (PASS) |
| PO-EDVBJ-008-FLUX | `crates/vb_runtime/src/verification/flux/vb_edvbj_diagnostic_code_refinement.rs` | NO | NO | Package-level flux compiles; specific refinement file absent |
| PO-EDVBJ-009-VERUS | `verification/verus/vb_edvbj_symbolic_code.rs` | NO (untracked) | YES | verus → **error: couldn't read `extern_vb_edvbj_symbolic_code.rs`** (companion file missing) |
| PO-EDVBJ-010-PROPTEST | `crates/vb_runtime/src/error/tests_diagnostics/proptest_vb_edvbj_diagnostic_code.rs` | NO | NO | Same as PO-003 |

**Verus raw evidence:** `.beads/vb-edvbj/evidence/verus_storage_event.txt`,
`verus_mirror_bind.txt`, `verus_propagation.txt`, `verus_symbolic_code.txt`.

## 5. Honest Classification

| Obligation | Verifier | Plan status | Final classification | Finding code |
|------------|----------|-------------|----------------------|--------------|
| PO-EDVBJ-001-VERUS | verus | planned | **FAIL_LOCAL** | `verifier_error` (duplicate specification for `mirror_storage_event`; mirror is not `#[verifier::external_body]`) |
| PO-EDVBJ-002-KANI | cargo-kani | planned | **FAIL_LOCAL** | `missing_artifact` + `pre_existing_build_blocker` (Kani harness file absent; `vb_core` kani module has unrelated unclosed-delimiter compile error blocking `cargo kani -p vb_runtime --lib`) |
| PO-EDVBJ-003-PROPTEST | proptest | planned | **FAIL_LOCAL** | `missing_artifact` (proptest file absent; `vb-edvbj-pending` Cargo feature not declared) |
| PO-EDVBJ-004-PROPTEST | proptest | planned | **FAIL_LOCAL** | `missing_artifact` (proptest file absent) |
| PO-EDVBJ-005-VERUS | verus | planned | **FAIL_LOCAL** | `vacuum_proof` (VACUUM per `scripts/check-verus-production-binding.sh`; companion `extern_vb_edvbj_propagation.rs` and `production_inner/vb_edvbj_propagation_production.rs` are absent from disk) |
| PO-EDVBJ-006-KANI | cargo-kani | planned | **FAIL_LOCAL** | `missing_artifact` + `pre_existing_build_blocker` (Kani harness file absent; same kani build error) |
| PO-EDVBJ-007-VERUS | verus + binding script | planned | **PASS** | (verus 2 verified, 0 errors; mandatory mirror-drift gate passes; existing `verification/verus/extern_storage_kind_family.rs` mirror unchanged) |
| PO-EDVBJ-008-FLUX | cargo-flux | planned | **FAIL_LOCAL** | `missing_artifact` (Flux refinement file absent; package-level flux compiles but no specific obligation closure) |
| PO-EDVBJ-009-VERUS | verus | planned | **FAIL_LOCAL** | `vacuum_proof` (VACUUM per binding script; companion `extern_vb_edvbj_symbolic_code.rs` and `production_inner/vb_edvbj_symbolic_code_production.rs` are absent from disk) |
| PO-EDVBJ-010-PROPTEST | proptest | planned | **FAIL_LOCAL** | `missing_artifact` (proptest file absent) |

**PASS / FAIL_LOCAL tally:** 1 / 9.

The 9 FAIL_LOCALs are **NOT** "regressions" (the implementation is correct and
the cargo test commands all pass); they are "missing preconditions for the
formal-verification lane". Each one is fixable by re-dispatching the proof-writer
to commit the artifacts to the JJ working copy and (for PO-001, PO-005, PO-009)
to repair the Verus production-binding structure.

## 6. Verdict

**PARTIAL — implementation is correct (cargo tests: 1+13+1807 PASS), but
formal verification is incomplete (1 PASS / 9 FAIL_LOCAL).** The proof-writer
artifacts documented in `proof-writer-report.md` are not present in the JJ
working copy at this change:

- The 4 Verus spec files are **untracked on disk** (not committed in `mrpqqutq`).
  Of these, 2 are VACUUM (no production binding; companion extern / production_inner
  files absent), 1 has a duplicate-specification compile error, and 1 verifies
  (`vb_edvbj_mirror_bind.rs`).
- The 2 Kani harness files are **absent from disk**.
- The 3 proptest files are **absent from disk**; the `vb-edvbj-pending` Cargo
  feature is not declared.
- The 1 Flux refinement file is **absent from disk** (package-level flux
  compiles cleanly, but the obligation is on a specific refinement file).

**Required next-step dispatches** (cheaper than re-running this state from scratch):

1. `proof-writer` (re-dispatch): commit the 4 untracked Verus spec files to the JJ change; add the missing `extern_vb_edvbj_propagation.rs` / `extern_vb_edvbj_symbolic_code.rs` companion files and the matching `production_inner/vb_edvbj_*_production.rs` mirrors; mark `mirror_storage_event` as `#[verifier::external_body]` (or remove the `assume_specification` bridge in `vb_edvbj_storage_event.rs`).
2. `proof-writer` (re-dispatch): add the 2 Kani harness files (`kani_vb_edvbj_storage_event_no_fabricate.rs`, `kani_vb_edvbj_propagation_strict_gate.rs`) to `crates/vb_runtime/src/`.
3. `proof-writer` (re-dispatch): add the 3 proptest files to `crates/vb_runtime/src/journal/tests/` and `crates/vb_runtime/src/error/tests_diagnostics/`, and declare the `vb-edvbj-pending` Cargo feature.
4. `proof-writer` (re-dispatch): add the Flux refinement file at `crates/vb_runtime/src/verification/flux/vb_edvbj_diagnostic_code_refinement.rs` and wire it into `crates/vb_runtime/src/verification/mod.rs`.
5. `repair-vb_core` (separate bead): fix the unclosed-delimiter build error in `crates/vb_core/src/frame_kani_harnesses` so Kani can compile vb_runtime.

## 7. References

- Contract: `.beads/vb-edvbj/contract.md`
- Proof plan: `.beads/vb-edvbj/proof-strategy.md`, `.beads/vb-edvbj/proof-coverage-matrix.md`
- Proof plan review: `.beads/vb-edvbj/proof-plan-review.md`
- Proof review: `.beads/vb-edvbj/proof-review.md`
- Proof writer: `.beads/vb-edvbj/proof-writer-report.md`, `.beads/vb-edvbj/proof-findings.jsonl`
- Implementation: `.beads/vb-edvbj/implementation.md`
- Trusted base: `.beads/vb-edvbj/trusted-base-ledger.jsonl`, `.beads/vb-edvbj/trusted-base-plan.md`
- Verifier lane decisions: `.beads/vb-edvbj/verifier-lane-decisions.jsonl`, `.beads/vb-edvbj/verifier-lane-matrix.md`
- Production binding script: `scripts/check-verus-production-binding.sh` (PASS for 73 WEAK, FAIL for 2 VACUUM)
