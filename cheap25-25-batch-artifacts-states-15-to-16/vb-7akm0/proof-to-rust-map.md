---
bead_id: vb-7akm0
bead_title: "Lint: remove #[allow(unreachable_pub)] suppressions by narrowing visibility (P1 bug)"
phase: 7
updated_at: 2026-07-02T00:10:00Z
attempt: 1
---

# Proof-to-Rust Map — vb-7akm0

## Overview

This bridge maps the 6 planned proof obligations to their Rust source targets and
independent verification lanes. vb-7akm0 is a **visibility-narrowing refactor**: it
removes `#[allow(unreachable_pub)]` suppressions across 25 files by narrowing visibility
(`pub fn` → `fn`, `pub` → `pub(crate)`, and file-level attribute deletion). No new
production code is introduced and **no production symbol changes its semantics**
(`behavior_affecting=false` for all 30 proof seeds, confirmed in `delivery-scope.jsonl`
and `proof-review.md` §11).

## Refinement Posture: NO RUST REFINEMENTS

There are **zero Rust refinement obligations** for this bead:

- All 6 obligations (`proof-obligations.planned.jsonl`) are `behavior_affecting=false`.
- No formal-verifier lane (Verus/Kani/Flux/Loom/proptest/fuzz/Miri/TLA+) carries a
  behavior-affecting proof claim. `proof-review.md` §11 records the 8 formal-verifier
  lanes as `not_applicable` with concrete evidence refs, `trusted-base-ledger.jsonl` as
  correctly empty, and the disposition as `APPROVED — NO_PROOF_WORK`.
- Each obligation resolves to **gate-execution evidence** (lint / compile / test / grep /
  decision-ack) owned by State 11 (formal-verifier), not to a Rust `requires`/`ensures`
  refinement that a refinement obligation would track.

Accordingly, `rust-refinement-obligations.jsonl` is **empty** by construction. The bridge
is a structural pass-through: it confirms each proof obligation has a concrete Rust target
and an independent gate, with no refinement claim to bind.

## Obligation → Rust Target Mapping

| Obligation | Verifier Lane | Rust Target(s) | Behavior-Affecting | Refinement |
|---|---|---|---|---|
| PO-LINT-001 | `moon-lint-src` | All 25 files (categories A/B/C/D/E/F/G.touch/G.1/G.2); attribute deletion + visibility metadata | false | none |
| PO-COMPILE-001 | `cargo-check` | 18 files with `pub fn` → `fn` / `pub` → `pub(crate)` narrowings | false | none |
| PO-TEST-001 | `cargo-test` | Sibling `#[cfg(test)]` + integration consumers of narrowed items | false | none |
| PO-EXTERN-001 | `grep-externality` + `check-verus-production-binding` + `check-production-inner-drift` | `vb_validate::diag::diag_codes::CODE_*`, `vb_validate::diagnostic::*`, `vb_cli::lifecycle::test_helpers::create_run_header` | false | none |
| PO-DECISION-001 | `decision-ack` | `.beads/vb-7akm0/decision-ack.md` (orphan-test disposition, category G) | false | none |
| PO-DECISION-GREP-001 | `grep` | `verification/verus/production_inner/` (IncidentReport independence) | false | none |

## Source-Ref Detail (per proof-to-implementation-input.md)

### PO-LINT-001 — visibility policy fires cleanly across 25 files

- **Targets**: `xtask/src/main.rs:2`; `crates/vb_validate/src/{diag/diag_tests.rs,
  schema_support/schema_tests.rs, fact_table.rs, gate_07_stack.rs, gate_08_accessor.rs,
  gate_09_slots.rs, gate_10_node.rs, gate_11_loop.rs, gate_12_14_15.rs, gate_13_cycles.rs,
  taint_prop.rs, type_check.rs, secret_leak.rs, type_sigs.rs, schema_support/schema_doc.rs,
  schema_support/schema_id.rs, schema_support/schema_fields.rs, diag/diag_codes.rs,
  diag/diag_convert.rs, diag/diag_render.rs, diagnostic.rs}`;
  `crates/vb_cli/src/{commands_diff.rs, commands_incident.rs, lifecycle.rs:471}`.
- **Lane**: `moon run :lint-src` (workspace clippy; `unreachable_pub = "deny"`).
- **Evidence**: `.evidence/lint-src/run-001/{exit-code.txt, clippy-output.log}`.
- **Refinement**: none — lint policy is a compile-time gate, not a proof refinement.

### PO-COMPILE-001 — narrowings preserve workspace compilation

- **Targets**: the 18 narrowing files (PO-LINT-001 set minus `xtask/main.rs` and the 3
  file-only attribute deletions).
- **Lane**: `cargo check --workspace --all-features`.
- **Evidence**: `.evidence/cargo-check/run-001/{exit-code.txt, cargo-output.log}`.
- **Refinement**: none — relies on the Rust 2021+ sibling-module direct-path visibility
  rule, which is a language guarantee, not a bead-specific proof.

### PO-TEST-001 — test count and outcomes match pre-change baseline

- **Targets**: sibling `#[cfg(test)]` modules and integration consumers
  (`gate_tests.rs`, `type_taint_tests.rs`, `secret_leak/tests.rs`,
  `schema_support/schema_tests.rs`, `schema_support/schema_fields/*.rs`,
  `diag/diag_tests.rs`, `diag/diag_render/render_tests.rs`,
  `crates/vb_cli/tests/lifecycle_integration.rs`,
  `crates/workspace_tests/tests/derived_status_replay_timeline_tests.rs:29`).
- **Lane**: `cargo test --workspace`.
- **Evidence**: `.evidence/cargo-test/run-001/{exit-code.txt, cargo-test-output.log, test-count.txt}`.
- **Refinement**: none — behavioral parity is confirmed by re-running the pre-existing
  suite; no new test and no proof harness.

### PO-EXTERN-001 — externally-reachable items are NOT narrowed

- **Targets (grep guard)**: `vb_validate::diag::diag_codes::CODE_*`,
  `vb_validate::diagnostic::*`, `vb_cli::lifecycle::test_helpers::create_run_header`.
- **Targets (binding gates)**: `scripts/check-verus-production-binding.sh`,
  `scripts/check-production-inner-drift.sh` (pre-existing trusted infrastructure).
- **Evidence**: `.evidence/grep-externality/run-001/*` and `.evidence/production-binding/run-001/*`.
- **Refinement**: none — this is a structural externality guard, not a refinement claim.

### PO-DECISION-001 / PO-DECISION-GREP-001 — orphan-test disposition (category G)

- **Targets**: `.beads/vb-7akm0/decision-ack.md` (owner-created, State 5) and the
  `verification/verus/production_inner/` mirror-independence grep.
- **Evidence**: `.evidence/decision-ack/run-001/*` and `.evidence/grep-precondition/run-001/*`.
- **Refinement**: none — pre-condition gates, resolved before ApplyTreatment for
  `commands_diff.rs` and `commands_incident.rs`.

## Intentionally Outside Rust Refinement Boundary

- **All 6 obligations** — gate-execution evidence (lint/compile/test/grep/decision-ack)
  owned by State 11, not refinement-bindable proof claims.
- **Trusted-base files** — `verification/verus/extern_*.rs`,
  `verification/verus/production_inner/*.rs`, `kani/`, `xtask/src/main.rs`
  (except its inner-attribute deletion), and the moon `lint-src` task are trusted
  infrastructure (TBP-001..TBP-012); the bead forbids modifying them.

## Waivers

- None required. `waiver-candidates.jsonl` carries a single `W-NONE-001` sentinel with
  `behavior_affecting=false`; no behavior-affecting proof claim exists that would need a
  Rust-evidence waiver.
