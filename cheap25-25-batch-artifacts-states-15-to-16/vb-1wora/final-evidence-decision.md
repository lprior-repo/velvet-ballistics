# Final Evidence Decision — vb-1wora

## Status

**STATUS: APPROVED**

## Decision

`vb-1wora` satisfies the bead-local evidence contract for the trailing-bytes rejection invariant in `decode_record_payload` and `decode_envelope_only`. The 7 POBs close at State 12: 5 PASS, 1 PASS+BLOCKED_TOOLING (Verus smoke + binding gate pass; drift gate BLOCKED_TOOLING), 1 BLOCKED_TOOLING+SMOKE_PASS (full Kani BLOCKED_TOOLING; Kani H6 syntax SMOKE_PASS). The active-context truth-serum audit reproduced the required scoped gates (1678 cargo tests, 8 proptest tests, 6 cargo-test trailing-bytes tests, cargo-fuzz 60s wallclock with 0 crashes, Verus bridge 25 verified, production-binding gate 0 VACUUM, strict source lint 0 issues, rustfmt clean) and found no remaining bead-local blocker.

The bead is approved for scoped landing consideration because the active-context truth-serum audit reproduced the required scoped gates and found no remaining bead-local blocker. The 2 BLOCKED_TOOLING items (full Kani run + production-inner drift gate) are pre-existing workspace-level or unowned issues, not vb-1wora regressions. They are documented in `trusted-base-ledger.jsonl:TL-vb-1wora-002,TL-vb-1wora-003` with explicit ownership routing (vb_core maintainer for the Kani blocker; femdation for the workspace tooling blocker).

## Approved Claims

- Executable runtime behavior for `decode_record_payload` and `decode_envelope_only` returning `Err(JournalError::TrailingBytes { trailing })` iff `bytes.len() > payload_end` (1678 cargo tests + 8 proptest tests + 6 cargo-test trailing-bytes tests).
- Property-based behavior for round-trip preservation (no false positive on well-formed records) via `ps003_exact_boundary_roundtrips` (1024 cases).
- Property-based behavior for trailing-bytes rejection (random trailing bytes in 1..=8) via `ps003_trailing_bytes_are_rejected` (1024 cases).
- Hostile-input behavior for attacker-shaped inputs via cargo-fuzz 60s wallclock (37,080,025 runs, 0 crashes, 0 ooms, 162 coverage points + 165 features).
- Verus bridge arm `Err(SpecJournalError::TrailingBytes { trailing })` is reachable and verified (25 verified, 0 errors; new `wrapper_decode_record_trailing_bytes` exec wrapper exercises the arm).
- Kani H6 syntax is correct under `cfg(kani)` gate (H6 uses `kani::any()` for all symbolic inputs per GOD RULE 1).
- Diagnostic code wiring: `TRAILING_BYTES_CODE = 0x4042`; `diagnostic_code()` and `symbolic_code()` arms are wired (cargo test `trailing_bytes_error_has_correct_code` passes; `trailing_bytes_error_code` passes).
- Strict production source lint: zero `unsafe`, zero `unwrap()/expect()/panic!/todo!/unimplemented!/dbg!`, zero `assert!/assert_eq!/assert_ne!/unreachable!` in production code paths.
- Honest BLOCKED_TOOLING accounting: 2 BLOCKED_TOOLING items (full Kani + drift gate) are pre-existing workspace-level or unowned issues, not vb-1wora regressions.

## Rejected Claims

- No global `moon ci` pass claim.
- No final release-confidence claim.
- No formal proof, theorem proof, Kani full verification, TLA+, Lean, Aeneas, Hax, mutation, or performance claim.
- No claim that the `SymbolicCode::JOURNAL_TRAILING_BYTES` is registered in `CODE_REGISTRY` (it is not; falls back to `INTERNAL_INVARIANT` per `contracts/error-taxonomy.md §2.5`).
- No claim that the production-inner drift gate is mechanically verified (BLOCKED_TOOLING; mirror change is structurally sound per manual review only).
- No claim that the full Kani H6 verification is mechanically verified (BLOCKED_TOOLING; H6 syntax is verified under `cfg(kani)` gate only).
- No claim that the workspace-wide fmt failures (vb_core/src/lib.rs:26, vb_core/src/time.rs:71, vb_runtime/src/frame_pool/tests.rs:114,139) are fixed (pre-existing, unrelated to vb-1wora).

## Current Direct Evidence

- `.beads/vb-1wora/formal-verification-report.md`: `STATUS: APPROVED_WITH_BLOCKED_TOOLING` (the BLOCKED_TOOLING items are pre-existing, not vb-1wora regressions). The 7 POBs are classified as PASS / PASS+BLOCKED_TOOLING / BLOCKED_TOOLING+SMOKE_PASS with raw command evidence in `.beads/vb-1wora/evidence/po-00X-*`.
- `.beads/vb-1wora/verification-ledger.jsonl`: 7 rows (one per POB), all with raw command evidence and exit status 0.
- `.beads/vb-1wora/formal-waivers.jsonl`: 5 rows, all `behavior_affecting: false` and `applicability: not_applicable` (Loom, Miri, Flux, TLA+, CODE_REGISTRY registration). 0 behavior-affecting waivers.
- `.beads/vb-1wora/black-hat-review.md`: `STATUS: APPROVED`. 2 LOW findings, both accepted (`owner_approved_no_action`).
- `.beads/vb-1wora/assurance-bundle.md`: `STATUS: APPROVED`. Requirement-to-evidence map and residual risk table. 9 INV-CODEC-TB-* + HOSTILE-INPUT-001 requirements covered.
- `.beads/vb-1wora/truth-serum-report.md`: `STATUS: APPROVED`. Active-context audit evidence and verdict.
- `.beads/vb-1wora/proof-review.md`: `STATUS: APPROVED` (line 227). 5 fixed_with_evidence findings; 0 blockers; 0 VACUUM.
- `.beads/vb-1wora/proof-plan-review.md`: `STATUS: APPROVED` (line 138). 0 blockers.
- `.beads/vb-1wora/proof-to-rust-review.md`: `STATUS: APPROVED` (line 267). 5 owner_approved_no_action findings.

## Required Follow-Up Before Release Confidence

- Re-run `bash scripts/check-production-inner-drift.sh` in a git-initialized checkout post-fix to confirm zero drift between the new mirror and the post-fix production source. The mirror change is structurally sound; this is a workspace-tooling follow-up.
- Re-run `cargo kani -p vb_storage --harness kani_harness_rejects_trailing_bytes --output-format=json` after the pre-existing `vb_core/src/frame/parts/kani_helpers.rs:22` compile error is fixed by the vb_core maintainer. The H6 harness is correctly authored and is ready to run.
- Optional: Register `JOURNAL_TRAILING_BYTES` in `crates/vb_core/src/diagnostic.rs::CODE_REGISTRY` to upgrade the symbolic observability from `INTERNAL_INVARIANT` fallback to the proper symbolic name. The numeric code (0x4042) and the diagnostic_code() arm are mandatory and are already wired.
- Pre-existing fmt repairs: Fix the 4 pre-existing workspace-wide fmt failures in `vb_core`/`vb_runtime` separately as a `BLOCK_GLOBAL` gate before final landing.
- Do not convert formal waivers (Loom, Miri, Flux, TLA+, CODE_REGISTRY registration) into pass claims without new reviewed evidence.
- Track any future native runtime terminal-event exposure or production-side repair to `vb_core/src/frame/parts/kani_helpers.rs` separately from this bead.

## Final Disposition

Proceed to landing-skill / bead closure flow for scoped delivery. The 2 BLOCKED_TOOLING items are pre-existing workspace-level or unowned issues, not vb-1wora regressions; they are documented with explicit ownership routing in `trusted-base-ledger.jsonl:TL-vb-1wora-002,TL-vb-1wora-003` and in `assurance-bundle.md:BLOCKED_TOOLING`. The bead is ready for scoped landing.
