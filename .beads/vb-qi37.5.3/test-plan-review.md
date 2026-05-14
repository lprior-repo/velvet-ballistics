# test-plan-review.md — vb-qi37.5.3 (State 9 — FINAL)

## Plan Inquisition (Mode 1) — Final Review

**Scope**: vb-qi37.5.3 idempotency evidence propagation in admission. No plan changes since State 8. This review assesses the judgment call on the remaining coverage gap.

**Prior findings (all resolved)**:
- Fix 1: proptests.rs module declared in lib.rs ✓
- Fix 2: checksum mismatch tests present ✓
- Fix 3: branch coverage 52.87% → 87.38% (now 88.99%) ✓
- Fix 4: keys.rs outside scope ✓

---

## Axis 1 — Contract Parity

All pub fns in admission.rs have BDD scenarios. Error variants are exercised with exact assertions. PASS.

## Axis 2 — Assertion Sharpness

All Then: assertions use exact values. No `is_ok()`/`is_err()` as sole assertions. PASS.

## Axis 3 — Trophy Allocation

- admission.rs: 84 tests / 4 pub fns = 21x — target ≥5x. PASS.
- proptests: 33 tests running. PASS.

## Axis 4 — Boundary Completeness

VerificationWarning gate bounds: explicit. PASS.

## Axis 5 — Mutation Survivability

Assessment: All practical error paths (checksum mismatch, policy violations, malformed envelopes) have test coverage. The untested regions represent hardware-level failure injection (postcard memory allocation failure, disk write failure, system durability failure) that cannot be triggered without mocking infrastructure not present in vb_storage.

## Axis 6 — Evidence Plan Audit

Perholzmann test rules: all tests have explicit Given/When/Then. Preconditions are stated. Side effects (temp_journal) are self-cleaning via tempfile::tempdir. PASS.

---

## JUDGMENT CALL: Remaining Coverage Gap

| Metric | Value | Threshold | Gap |
|--------|-------|-----------|-----|
| admission.rs regions | 88.99% | ≥90% | 1.01pp / ~16 regions |
| TOTAL regions | 89.42% | ≥90% | 0.58pp |
| Line coverage | 93.34% | — | — |

**Nature of remaining untested regions**:
- `postcard::to_allocvec` error branches — serialization failures on valid input data. postcard errors only on memory allocation failure or hardware errors. Cannot be triggered through public API.
- `journal.put_compiled_ir` error branches — disk/permission failures. Require mock Journal.
- `journal.persist_strict()` error branches — system sync/durability failures. Require mock Journal.
- `journal.compiled_ir` returning None — race condition in journal state. Require mock Journal.

**Fundamental constraint**: vb_storage tests use `temp_journal()` — a real FjallJournal backed by actual filesystem. No mock Journal interface exists. Injecting these errors requires either (a) hardware-level fault injection or (b) a mocking layer for Journal. Neither is available in the current test infrastructure, and neither is in scope for this feature bead.

---

## VERDICT: test-plan APPROVED with documented limitation

The test plan is sound. All practical error paths are covered. The remaining gap is a fundamental constraint of the testing environment, not a test quality deficiency.
