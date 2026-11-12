# Handoff — vb-9tg1c (local3090 final polish)

## Scope

Two bounded fixes: (A) mechanical rustfmt on 7 pinned files; (B) metadata corrections for vb-ynp2k code/test review findings F1, F2, T1.

## Fix A: Rustfmt (pinned nightly-2026-04-28, edition 2024)

Command:
```bash
rustup run nightly-2026-04-28 rustfmt --edition 2024 \
  crates/vb_storage/tests/accepted_artifact_red_phase.rs \
  crates/workspace_tests/tests/vb_79ffw_ticket_authority_probe.rs \
  crates/workspace_tests/tests/vb_core_yaml_e2e_chain_contract.rs \
  crates/workspace_tests/tests/vb_eepg_bdd_tests.rs \
  crates/workspace_tests/tests/vb_kkvb_xtask_command_shell_coverage.rs \
  crates/workspace_tests/tests/vb_lp2v_admission_integration.rs \
  crates/workspace_tests/tests/vb_ssei_verification_admission_acceptance.rs
```

Verification: `--check` on the same 7 files passes (no output).

**Note:** Full `cargo fmt --check` may still fail due to qi37 (handled separately by DeepSeek). This is not a claim of global fmt pass.

## Fix B: Metadata corrections

### F1 (LOW) — drift annotation + stale comments

**File:** `verification/verus/production_inner/try_from_parts_production.rs`

1. Added drift annotation above `validate_input_slots`:
   ```
   // Production crates/vb_core/src/workflow/mod.rs:876-906
   // Drift policy: any change to this function body in production MUST be
   // mirrored here. The spec proofs depend on the faithful implementation.
   ```

2. Corrected stale "all stub Ok" comments in the `try_from_parts` method doc (lines 435-441): now states `validate_parts`/`validate_budget` are stubs while `validate_input_slots` is faithful.

3. Corrected section header (lines 518-527): "VERBATIM PRODUCTION: stub validation helpers" → "PRODUCTION MIRROR: stub validation helpers + faithful input-slot body". Updated line ranges to match production (850-874, 908-914).

### F2 (LOW) — stale variant count

**File:** `verification/verus/try_from_parts.rs:417`

Changed "exactly enumerates these 8 variants" → "exactly enumerates these 11 variants" (matches actual `spec_workflow_error_is_bound_violation_spec` enumeration).

### Spec file production body correction

**File:** `verification/verus/try_from_parts.rs`

1. Added missing `validate_input_slots(&parts)?;` to the shown production body (line 16).
2. Corrected line ranges: `workflow/mod.rs:33-51` → `workflow/mod.rs:35-56` (production body), `validate_parts` `753-777` → `850-874`, `validate_budget` `779-785` → `908-914`.
3. Added third validation step to the production validation sequence documentation (line 37-38).

### T1 (MEDIUM) — false zero-test claim

**File:** `.beads/vb-hrvix/contract.md`

1. Added corrected test row: `cargo test -p vb_core --lib input_slot -- --test-threads=1` → 7 passed.
2. Added "Metadata Correction" section explaining the original command ran 0 tests (wrong package `velvet-ballistics` + `--exact` filter matching nothing).

**Fresh evidence:** `.beads/vb-hp1qq/evidence/actual-input-slot-tests.*` (7 passed, 0 failed, 15.789s).

## Verification

| Check | Command | Result |
|---|---|---|
| Input-slot tests | `cargo test -p vb_core --lib input_slot -- --test-threads=1` | 7 passed, 0 failed |
| Production-inner drift | `bash scripts/check-production-inner-drift.sh` | PASS |
| Verus production binding | `bash scripts/check-verus-production-binding.sh` | PASS |
| Rustfmt --check (7 files) | `rustup run nightly-2026-04-28 rustfmt --edition 2024 --check ...` | PASS |

All checks run through `/tmp/vb-run-gate.py` with immutable evidence.

## Trust boundary note

These are metadata corrections. The Verus proofs remain spec-level, not whole-production verification. The production_inner mirror binding is WEAK (hand-maintained verbatim copy gated by identifier-level drift heuristic), confirmed by the gate (STRONG 0, WEAK 72, VACUUM 0). The drift annotation added here improves maintainability but does not change the binding classification.

## No source changes

No production source files modified. No test strengthening (T2 finding left as-is per user instruction). No full CI or fleet Kani runs.