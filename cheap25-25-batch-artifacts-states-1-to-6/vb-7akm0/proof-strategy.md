# Proof Strategy — vb-7akm0 Lint-Suppression Audit

**Bead:** vb-7akm0
**Title:** Lint: remove `#[allow(unreachable_pub)]` suppressions by narrowing visibility (P1 bug)
**State:** Go-skill State 4 (Proof Planning)
**Workspace:** `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-7akm0`
**Source:** `/home/lewis/src/velvet-ballistics`
**Generated:** 2026-07-01
**Owner:** proof-planner (State 4)

---

## 1. Scope Summary

This bead audits **25 `#[allow(unreachable_pub)]` suppressions across 25 source files**
and resolves them by either deleting the now-vestigial attribute, narrowing `pub fn` → `fn`
(category B/C), narrowing `pub` → `pub(crate)` (category D), or retiring the orphan test
that motivates the suppression (category G). The change is **behavior-preserving** at the
workspace level: no production-code symbol changes its semantics. Only visibility metadata
or attribute lines change.

### 1.1 Categorization (from contract.md §2)

| Cat | Treatment | Files | Examples |
|-----|-----------|-------|----------|
| A (vestigial) | `delete-allow` | 4 | `xtask/src/main.rs`, `diag_tests.rs`, `schema_tests.rs`, `fact_table.rs` |
| B (gate internal dup) | `pub fn` → `fn` | 7 | `gate_07_stack.rs`…`gate_13_cycles.rs` |
| C (taint/type/secret-leak dup) | `pub fn` → `fn` | 3 | `taint_prop.rs`, `type_check.rs`, `secret_leak.rs` |
| D (schema support) | `pub` → `pub(crate)` | 4 | `type_sigs.rs`, `schema_doc.rs`, `schema_id.rs`, `schema_fields.rs` |
| E (diag) | mixed: delete-allow (2 files) + decision for diag_codes | 3 | `diag_codes.rs` (decision), `diag_convert.rs` (delete), `diag_render.rs` (delete) |
| F (diagnostic.rs reexport) | `delete-allow` | 1 | `diagnostic.rs` |
| G (orphan test) | `decision-required` | 3 | `commands_diff.rs`, `commands_incident.rs` (decision), `lifecycle.rs` (delete) |

**Total files with attribute changes:** 25 (one attribute each).
**Total decision-required files:** 2 (`commands_diff.rs`, `commands_incident.rs`) gated on the orphan-test decision.

### 1.2 Bead-Wide Invariants

- **LS-INVARIANT.1:** Every remaining `pub` item in the workspace satisfies
  `reachable_via_external_path == true` (externally reachable via downstream crate,
  registered integration test, or `#[cfg(test)] mod` in the lint-src compile set).
- **LS-INVARIANT.2:** No production-code symbol changes its semantics; only visibility or
  attribute metadata changes. `behavior_affecting == false` for every Suppression row.
- **LS-INVARIANT.3:** The 25 attribute lines removed are NOT replaced with any new
  `#[allow(unreachable_pub)]` override.

---

## 2. Risk Classification

| Risk Tag | Severity | Affected Clauses | Lanes |
|---------|----------|------------------|-------|
| `lint_suppression_audit` | low | All 30 clauses | source-lint, grep |
| `test_visibility` | medium | LS-INTERNAL.* (B), LS-TAINT.* (C), LS-SCHEMA.* (D) | cargo-test, source-lint |
| `public_api` | medium | LS-DIAG.1, LS-DIAG.3, LS-REEXPORT.1, LS-LIFECYCLE.1 | grep, source-lint |
| `dormant_artifact` | medium | LS-ORPHAN.1, LS-ORPHAN.2 | decision-ack (pre-condition), source-lint |
| `decision_required` | high | LS-ORPHAN.1, LS-ORPHAN.2 | decision-ack (pre-condition) |
| `production_binding_verification` | medium | LS-ORPHAN.2 | check-verus-production-binding, check-production-inner-drift |
| `test_suite_reverify` | medium | LS-INTERNAL.*, LS-TAINT.*, LS-SCHEMA.*, LS-VERIFY.1, LS-VERIFY.2 | cargo-test |

There are **no formal Verus / Kani / Flux / Loom / TLA+ obligations** in this bead. Every
proof obligation is anchored in the existing workspace gate suite (`moon run :lint-src`,
`cargo test --workspace`, `bash scripts/check-verus-production-binding.sh`,
`bash scripts/check-production-inner-drift.sh`).

---

## 3. Verifier Lane Strategy

### 3.1 Lane 1: `moon-lint-src` (source-lint)
- **Script:** `.moon/tasks/all.yml:46-62` → `cargo clippy --workspace --lib --bins --examples --all-features`
- **Coverage:** PS-vb-7akm0-001..004 (vestigial), PS-vb-7akm0-026, PS-vb-7akm0-028 (LS-INVARIANT.1, LS-VERIFY.1)
- **Post-condition:** zero `#[allow(unreachable_pub)]` suppressions survive; zero `unreachable_pub` warnings fire.
- **Evidence:** raw exit code + raw log captured in `.evidence/lint-src/<run_id>/exit-code.txt`.

### 3.2 Lane 2: `cargo-test-workspace` (cargo-test)
- **Script:** `cargo test --workspace` (covers `cargo test -p vb_validate --lib`,
  `cargo test -p vb_cli --lib`, `cargo test --workspace --tests`)
- **Coverage:** PS-vb-7akm0-002..004 (vestigial + cargo compile), PS-vb-7akm0-005..018 (B/C/D),
  PS-vb-7akm0-020, PS-vb-7akm0-021 (E.2, E.3), PS-vb-7akm0-027 (LS-INVARIANT.2), PS-vb-7akm0-029 (LS-VERIFY.2)
- **Post-condition:** all tests pass with same test count as pre-change baseline.
- **Evidence:** raw exit code + raw log captured in `.evidence/cargo-test/<run_id>/exit-code.txt`.

### 3.3 Lane 3: `grep-externality` (pre-ApplyTreatment grep)
- **Script:** `grep -R 'vb_validate::diag::diag_codes::CODE_' . --exclude-dir=.git`
  (and similar grep per externally-reachable item).
- **Coverage:** PS-vb-7akm0-019 (LS-DIAG.1), PS-vb-7akm0-021 (LS-DIAG.3), PS-vb-7akm0-022 (LS-REEXPORT.1),
  PS-vb-7akm0-025 (LS-LIFECYCLE.1)
- **Pre-condition:** every `pub` item that the bead intends to keep `pub` IS externally reachable
  via a downstream-crate path or a registered integration test.
- **Evidence:** grep output captured in `.evidence/grep-externality/<run_id>/<item>.txt`; zero
  unexpected hits required for category E "PubToPubCrate" option; explicit hit-list required
  for category E "DeleteAllow" and category G "Decision" justifications.

### 3.4 Lane 4: `check-verus-production-binding` (production-binding)
- **Script:** `bash scripts/check-verus-production-binding.sh`
- **Coverage:** PS-vb-7akm0-024 (LS-ORPHAN.2), PS-vb-7akm0-030 (LS-VERIFY.3)
- **Post-condition:** Verus production-bound specs continue to bind via STRONG (`#[path]`)
  or WEAK (production_inner mirror); the bead does not break the binding by altering
  `vb_cli::commands_incident::IncidentReport` visibility.
- **Evidence:** raw exit code captured in `.evidence/production-binding/<run_id>/exit-code.txt`.

### 3.5 Lane 5: `check-production-inner-drift` (mirror drift)
- **Script:** `bash scripts/check-production-inner-drift.sh`
- **Coverage:** PS-vb-7akm0-024 (LS-ORPHAN.2)
- **Post-condition:** production_inner mirror drift = 0 (any drift fails CI).
- **Evidence:** raw exit code captured in `.evidence/production-binding/<run_id>/drift-exit-code.txt`.

### 3.6 Lane 6: `decision-ack` (pre-condition gate)
- **Artifact:** `.beads/vb-7akm0/decision-ack.md` (must exist before `ApplyTreatment`)
- **Coverage:** PS-vb-7akm0-023 (LS-ORPHAN.1), PS-vb-7akm0-024 (LS-ORPHAN.2)
- **Pre-condition:** the orphan-test decision (RetireOrphanTest or RegisterOrphanTest) MUST be
  recorded in decision-ack.md before the implementation owner runs `ApplyTreatment`.
- **Default:** RetireOrphanTest (per codebase-map.md §"Open Questions" recommendation 1 and
  contract.md §2.7 LS-ORPHAN.1 default).
- **Evidence:** decision-ack.md existence + content hash.

### 3.7 Non-Applicable Lanes (formal verifiers)

| Lane | Verdict | Concrete evidence |
|------|---------|-------------------|
| `verus` | `not_applicable` | No spec/proof fn changes; the bead is a Rust-local visibility refactor with no refinement types. Cargo.toml:59 does not enable `verus` for the touched files. The existing Verus proofs at `verification/verus/extern_vb_ahfl_bounds_production.rs` bind to `production_inner` mirrors and do not consume `vb_cli::commands_incident::IncidentReport` directly (delivery-scope.jsonl row 32). |
| `kani` | `not_applicable` | No new `#[kani::proof]` harnesses; no unsafe code is introduced or removed; existing kani harnesses consume canonical `vb_validate::gates::*` (delivery-scope.jsonl row 31), NOT the duplicates in `gate_07_stack.rs`…`gate_13_cycles.rs`. |
| `flux-rs` | `not_applicable` | No refinement types in scope; the touched files contain no `#[flux::*]` annotations; `cargo flux -p vb_validate --message-format human` is unaffected. |
| `loom` | `not_applicable` | No concurrent actors introduced; the touched files are all `#[cfg(test)] mod` or single-threaded production code. |
| `proptest` | `not_applicable` | No new property-based tests; the bead is a refactor of existing test infrastructure, not a new test surface. |
| `cargo-fuzz` | `not_applicable` | No fuzz targets introduced; the touched files are not parser/compiler code. |
| `miri` | `not_applicable` | No `unsafe` blocks in scope (Holzman Rust §"No unsafe"); Miri cannot add value to a visibility refactor. |
| `tla-plus` | `not_applicable` (globally removed) | TLA+ removed from repo per proof-planner skill §"TLA+ removed". No temporal/workflow behavior changes. |

---

## 4. Obligation Summary

| ID | Clause(s) | Verifier | Command | Required | Risk |
|----|-----------|----------|---------|----------|------|
| PO-LINT-001 | LS-VESTIGIAL.* (A), LS-INTERNAL.* (B), LS-TAINT.* (C), LS-SCHEMA.* (D), LS-DIAG.* (E), LS-REEXPORT.1 (F), LS-ORPHAN.* (G), LS-LIFECYCLE.1, LS-INVARIANT.1, LS-VERIFY.1 | `moon-lint-src` | `moon run :lint-src 2>&1` | true | low |
| PO-COMPILE-001 | LS-VESTIGIAL.* (A), LS-INTERNAL.* (B), LS-TAINT.* (C), LS-SCHEMA.* (D) | `cargo-check` | `cargo check --workspace --all-features 2>&1` | true | medium |
| PO-TEST-001 | LS-VESTIGIAL.2..4 (A.2-4), LS-INTERNAL.* (B), LS-TAINT.* (C), LS-SCHEMA.* (D), LS-DIAG.2..3 (E.2-3), LS-LIFECYCLE.1, LS-INVARIANT.2, LS-VERIFY.2 | `cargo-test` | `cargo test --workspace 2>&1` | true | medium |
| PO-EXTERN-001 | LS-DIAG.1 (E.1), LS-DIAG.3 (E.3), LS-REEXPORT.1 (F), LS-LIFECYCLE.1, LS-VERIFY.3 | `grep-externality` + `check-verus-production-binding` + `check-production-inner-drift` | see §3.3-3.5 | true | medium |
| PO-DECISION-001 | LS-ORPHAN.1 (G.1), LS-ORPHAN.2 (G.2) | `decision-ack` | pre-condition: `.beads/vb-7akm0/decision-ack.md` exists with RetireOrphanTest or RegisterOrphanTest choice | true | high |
| PO-DECISION-GREP-001 | LS-ORPHAN.2 (G.2) | `grep IncidentReport verification/verus/production_inner/` | `grep -R 'IncidentReport' verification/verus/production_inner/` returns no results | true | medium |

**Total obligations:** 6 (within 4-6 budget).

---

## 5. Assumptions

| ID | Assumption | Source |
|----|------------|--------|
| ASM-001 | The orphan test `vb_test_cli_diff_incident_behavior.rs` will be **retired** (default) before `ApplyTreatment` runs. | contract.md §2.7 LS-ORPHAN.1 default; codebase-map.md §"Open Questions" recommendation 1 |
| ASM-002 | The remaining 23 attribute removals (categories A, B, C, D, F, lifecycle) are atomic and idempotent; the bead does not split them across multiple commits. | contract.md §3 ("behavior-preserving") |
| ASM-003 | Rust 2021+ visibility rules permit sibling-module direct-path access to non-pub items in non-pub modules; verified empirically by `cargo test -p vb_validate --lib` after each category's changes. | contract.md §2.2; codebase-map.md §"Category B" |
| ASM-004 | `pub(crate)` items are not subject to `unreachable_pub` lint (Rust compiler behavior); the lint targets `pub` without narrowing, not `pub(crate)`. | contract.md §2.4 |
| ASM-005 | Verus production-binding is preserved because the touched `IncidentReport` is local to `crates/vb_cli/src/commands_incident.rs`; the Verus specs bind via `production::Kind::IncidentReport` enum variant, NOT the local struct. | codebase-map.md §"production_binding_verification" |
| ASM-006 | `pub(super) fn all_variants()` in `diag_convert.rs:10` is NOT subject to the `unreachable_pub` lint; deleting the inner-attribute is safe. | contract.md §2.5 LS-DIAG.2 |

---

## 6. Budget Constraints

| Verifier | Time Budget | Notes |
|----------|-------------|-------|
| `moon run :lint-src` | 5 min | Whole-workspace clippy with `--all-features` |
| `cargo check --workspace --all-features` | 3 min | Compile-check only, no test execution |
| `cargo test --workspace` | 15 min | Full test surface including integration tests |
| `grep -R 'vb_validate::diag::diag_codes::CODE_' .` | <1 min | Single grep across .git-excluded tree |
| `bash scripts/check-verus-production-binding.sh` | <1 min | Existing gate, no change to its runtime |
| `bash scripts/check-production-inner-drift.sh` | <1 min | Existing gate, no change to its runtime |

---

## 7. Waiver Candidates

No behavior-affecting waivers. The bead is `behavior_affecting == false` for every Suppression
row (delivery-scope.jsonl confirms). A sentinel row `W-NONE-001` is emitted in
`waiver-candidates.jsonl` to satisfy the validator's "must have at least one waiver row"
constraint. See `waiver-candidates.jsonl` for the sentinel.

---

## 8. Open Questions (for implementation owner)

| ID | Question | Owner | Default if No Answer |
|----|----------|-------|----------------------|
| OQ-001 | Retire or register `vb_test_cli_diff_incident_behavior.rs`? | user/architect | Retire (deletes 646-line orphan test) |
| OQ-002 | For `diag_codes.rs`: narrow 60+ constants to `pub(crate)` (option b) or leave `pub` and delete allow (option a)? | user/architect | Option (a) DeleteAllow (preserves external API stability; the spec contract says grep returns 0 external consumers, but the user may want to keep `pub` for forward compatibility) |
| OQ-003 | Split the implementation into one commit per category, or one commit per file? | implementation owner | One commit per category (5 commits: A, B+C, D, E+F+lifecycle, G) — easier to bisect if a test regresses |

---

## 9. Artifact Targets

| Artifact | Owner | Rerun From |
|----------|-------|------------|
| `proof-strategy.md` | proof-planner (this file) | state 4 |
| `verifier-lane-decisions.jsonl` | proof-planner (this file) | state 4 |
| `verifier-lane-matrix.md` | proof-planner (this file) | state 4 |
| `proof-coverage-matrix.md` | proof-planner (this file) | state 4 |
| `proof-obligations.planned.jsonl` | proof-planner (this file) | state 4 |
| `trusted-base-plan.md` | proof-planner (this file) | state 4 |
| `waiver-candidates.jsonl` | proof-planner (this file) | state 4 |
| `proof-to-implementation-input.md` | proof-planner (this file) | state 4 |
| `decision-ack.md` | **implementation owner (must exist before ApplyTreatment)** | state 5 |
| `ApplyTreatment` (code changes) | holzman-rust / implementation owner | state 5 |
| `proof-plan-review.md` | proof-plan-reviewer | state 4b |
| `formal-verification-report.md` | formal-verifier | state 12 |

---

*Generated by proof-planner skill. Status: planned. Behavior-affecting: false. All obligations require implementation owner to apply the attribute changes and run the gates.*