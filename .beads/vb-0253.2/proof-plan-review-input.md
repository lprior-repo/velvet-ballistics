# Proof Plan Review Input: vb-0253.2

## Bead

- **id**: vb-0253.2
- **isolated workspace**: /tmp/vb-ws/vb-0253.2
- **state**: 3 → 4 (Proof Planning)
- **skill**: proof-planner

---

## What This Bead Does

Facade conversion of `vb_ipc` crate: remove 320-line duplicate block (lib.rs lines 641–960) containing verbatim copies of `bounded.rs`, `ingress.rs`, `error.rs` definitions. Add `pub mod bounded; pub mod ingress; pub mod error;` declarations and re-export canonical types from `lib.rs` facade. Update `tests.rs` imports. Promote `codec.rs` re-exports too. No behavioral changes.

---

## Proof Obligations Summary

**16 total obligations**, all in `verify-standard` mode (cargo test + clippy).

| ID | Clause | Layer | Risk |
|---|---|---|---|
| SRC-001 | INV-001 | static-scan | medium |
| SRC-002 | INV-002 | static-scan | medium |
| SRC-003 | INV-003 | static-scan | medium |
| SRC-004 | INV-004 | static-scan | medium |
| SRC-005 | INV-005 | static-scan | medium |
| SRC-006 | INV-009 | static-scan | medium |
| SRC-007 | POST-004 | static-scan | low |
| SRC-008 | POST-006 | static-scan | low |
| SRC-009 | POST-001 | static-scan | low |
| BUILD-001 | POST-002 | compile-check | high |
| BUILD-002 | INV-006 | compile-check | high |
| BUILD-003 | INV-006 | compile-check | medium |
| TEST-001 | INV-001–INV-008 | compile-check | high |
| LINT-001 | INV-010 | static-scan | high |
| MOON-001 | GATE-001 | gauntlet-standard | medium |
| WAIVER-FORMAL-001 | TLA+/Verus/Kani/Loom/Miri | waiver | low |

---

## Behavioral Coverage

The existing test suite covers all behavioral contracts:

- **bounded-memory** (INV-007): `bounded_queue_applies_backpressure`, `try_submit_returns_full_when_at_capacity`, `adversarial_memory_ingress_full_then_drain_then_submit`
- **payload-validation** (INV-008): `oversized_payload_is_rejected`, `bounded_payload_rejects_oversized_with_exact_counts`, `adversarial_encode_payload_exceeding_bound_rejected`
- **canonical-uniqueness** (INV-001–INV-006): covered by static-scan obligations SRC-001–SRC-006
- **stable-re-exports** (INV-006): covered by BUILD-001, BUILD-002, BUILD-003
- **no-unsafe** (INV-010): covered by LINT-001
- **no-concurrency-change** (INV-011): `adversarial_memory_ingress_disconnected_after_sender_drop`, `adversarial_memory_ingress_full_then_drain_then_submit` via TEST-001

---

## Waiver Request

**WAIVER-FORMAL-001** requests explicit waiver of formal proof (TLA+/Verus/Kani/Loom/Miri) on the basis that:
1. This is a pure structural refactor — no behavioral change
2. All behavioral invariants already exercised by Fowler test suite (60+ tests)
3. No temporal properties, protocol state machines, or concurrency patterns introduced
4. crossbeam_channel is the trusted runtime boundary; no new channel usage

Evidence: `contract.md` Section "Non-goals" and `contract.md` Section "Verus-Owned Clauses" (explicit "no Verus proof required").

---

## Verification Lanes Used

- **verify-standard only** — cargo test + clippy + static-scan via rg/cargo
- No TLA+, Verus, Kani, Loom, Miri lanes
- Moon ci `:verify-standard` lane as workspace-wide gauntlet

---

## Reviewer Attention Points

1. **Duplicate removal completeness**: Are SRC-001–SRC-006 grep patterns specific enough to catch all duplicate struct/enum definitions? Could a type alias or re-export confuse them?
2. **Import update coverage**: POST-007 (tests.rs import updates) is covered by TEST-001 only — is compile-check sufficient evidence, or does it need a dedicated static-scan?
3. **Re-export chain**: codec.rs is also being re-exported — should SRC-009 also check for `pub use codec::{encode_payload, decode_payload}` or is BUILD-001 sufficient?
4. **Moon ci coverage**: MOON-001 runs the full moon ci lane. Is `:verify-standard` the correct task name for this bead's scope?

---

## Files in This Artifact

- `.beads/vb-0253.2/contract.md` — authoritative contract
- `.beads/vb-0253.2/proof-obligations.jsonl` — obligation ledger
- `.beads/vb-0253.2/traceability-matrix.jsonl` — clause → test/proof mapping
- `.beads/vb-0253.2/delivery-scope.jsonl` — 23-row delivery scope
