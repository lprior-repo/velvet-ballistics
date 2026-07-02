# Proof Plan Review Input — vb-qi37.1.4

## Bead
- **ID**: vb-qi37.1.4
- **Title**: runtime/recovery: Fail closed on incomplete recovery
- **State**: 4 (proof-planning)

---

## Contract Clauses and Proof Status

| Clause | Description | Primary Verifier | Status |
|---|---|---|---|
| INV-RC-001 | slot_values=true → InvalidRecoveryHydration | Verus | pending |
| INV-RC-002 | slot_taint=true → InvalidRecoveryHydration | Verus | pending |
| INV-RC-003 | action_payloads=true → InvalidRecoveryHydration | Verus | **gap — missing check** |
| INV-RC-004 | pending_actions non-empty + flag=true → InvalidRecoveryHydration | Verus | pending |
| INV-RC-005 | action_payloads=true → no action results consumed | Verus | pending |
| INV-RC-006 | action_payloads not consumed when unsupported | Verus | pending |
| INV-RC-007 | RunResumed/RunRetried/RunAnswered not silently dropped | TLA+ | **spec not written** |
| INV-RC-008 | ActionAbiMismatch on digest mismatch | Verus | pending |
| INV-RC-009 | PolicyDigestMismatch on digest mismatch | Verus | pending |
| POST-RC-001 | Ok iff all 4 unsupported flags false | Verus | pending |
| POST-RC-002 | verify_digests Full → all digests match | Verus | pending |
| POST-RC-003 | replay_events includes lifecycle events | TLA+ | pending |
| POST-RC-004 | action_payloads in same conditional as slot_values | Verus | pending |

---

## Discovery Evidence

**All 11 recovery source files use `#![forbid(unsafe_code)]`**: No unsafe Rust in proof boundary.

**Risk patterns found**:
- `unwrap_or`, `panic!` found in test code only (not in `reject_unsupported_live_frame_state` boundary function)
- `state` keyword appears in function names and types (expected — recovery state machine)
- `retry`, `queue` appear in comments and struct names (expected — recovery FSM)

**No TLA+ or Verus annotations found in source** — proof obligations will be written in separate spec files.

**RecoveryReplay.tla spec exists only in tla-spec.md** — not yet written as actual `.tla` file.

---

## Open Questions

| ID | Question | Impact | Resolution |
|---|---|---|---|
| Q1 | Is `action_payloads: true` ever set by storage replay today? | If never set, the fix has no runtime effect | Contract specifies behavior regardless |
| Q2 | Should `action_payloads: true` cause immediate rejection or partial hydration? | Contract says immediate rejection (fail-closed) | Resolved by INV-RC-003 |
| Q3 | Is `DigestCheck::Full` intentionally deferred for action/policy digests? | Gap identified; contract wires it in | Resolved by INV-RC-008/009 |

---

## Verifier Availability

| Verifier | Available | Command |
|---|---|---|
| verus | Available | `verus verification/verus/recovery_verification.rs` |
| tlc | Available (workspace) | `tlc -config RecoveryReplay.cfg RecoveryReplay.tla` |
| cargo test | Available | `cargo test -p vb_storage --test recovery_integration` |
| cargo kani | Available | `cargo kani --workspace` |
| loom | Available | `cargo test --test loom` |

---

## Waiver Review Triggers

| Lane | Waiver | Trigger Condition |
|---|---|---|
| TLA+ (INV-RC-007) | WAIVER-INV-RC-007-TLA | Spec file not written by proof-writer |
| Lean theorem | WAIVER-LEAN | All clauses remain Verus-expressible |
| Loom | implicit | ActionReplayTracker non-critical path |

---

## Reviewer Attention Points

1. **INV-RC-003 is the primary gap** — the `action_payloads` check is missing from `reject_unsupported_live_frame_state`. All 4 unsupported flags must be checked.

2. **TLA+ spec must be written first** — `specs/RecoveryReplay.tla` and `specs/RecoveryReplay.cfg` do not exist as files. Proof-writer must create them.

3. **Integration tests are gap-detection** — INTEG-RC-GAP-001/002/003 are expected to FAIL on current source (pre-fix). This is intentional to demonstrate the gap exists.

4. **Verus and TLA+ have overlapping scope** — POST-RC-001 is proven by both Verus (VERUS-POST-RC-001) and TLA+ (TLA-RC-SAFE). This is intentional redundancy.

5. **No unsafe code in proof boundary** — all recovery source files have `#![forbid(unsafe_code)]`, eliminating UB verification concern.

---

## Anti-Hallucination Attestation

- [x] All verifier commands checked against workspace toolchain
- [x] All source file paths verified to exist
- [x] All spec file paths verified (or marked MISSING)
- [x] No verifier pass results claimed
- [x] No tool availability invented
- [x] Waivers documented for skipped lanes
