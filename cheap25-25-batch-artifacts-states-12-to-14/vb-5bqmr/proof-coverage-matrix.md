# Proof Coverage Matrix — vb-5bqmr

## Bead

`vb-5bqmr` — SlotExtra: reject unknown VBSE versions instead of legacy downgrade (P1 bug)

## Matrix legend

- **Required lanes**: K = Kani, V = Verus (rust-local), F = Flux-RS, P = Proptest
- **Behavior-affecting**: B (binding to production)
- **Not applicable**: — (no such row exists in this plan; the user-bounded
  four-lane set is complete)
- **Status**: ✅ planned (State 4 — this planner's output)

## Coverage matrix

| # | Proof Obligation | Requirement | Contract Clause | Hazard(s) | Behavior | Verifier | Target Symbol | Status |
|---|---|---|---|---|---|---|---|---|
| 1 | PO-VERUS-001 | `vb-5bqmr-C-DEC-001..004` + `C-ERR-002` | C-DEC-001, C-DEC-002, C-DEC-003, C-DEC-004, C-ERR-002 | H-001, H-014 | B | verus | `crates::vb_storage::slot_extra::decode_slot_written_extra` | ✅ planned |
| 2 | PO-KANI-001 | `vb-5bqmr-C-DEC-002` + `C-NEG-004/005` | C-DEC-002, C-NEG-004, C-NEG-005 | H-001 | B | kani | `crates::vb_storage::slot_extra::decode_slot_written_extra` | ✅ planned |
| 3 | PO-KANI-002 | `vb-5bqmr-C-DEC-004` + `C-NEG-001/002/003/006` | C-DEC-004, C-NEG-001, C-NEG-002, C-NEG-003, C-NEG-006 | H-005, H-006, H-008, H-016 | B | kani | `crates::vb_storage::slot_extra::decode_slot_written_extra` | ✅ planned |
| 4 | PO-FLUX-001 | `vb-5bqmr-C-CON-001/004` + `C-DEC-004` | C-CON-001, C-CON-004, C-DEC-004 | H-007 | B | flux-rs | `crates::vb_storage::slot_extra::{SLOT_WRITTEN_EXTRA_PREFIX, SLOT_WRITTEN_EXTRA_MAGIC, SLOT_WRITTEN_EXTRA_VERSION, decode_slot_written_extra}` | ✅ planned |
| 5 | PO-PROP-001 | `vb-5bqmr-C-DEC-002` + `C-NEG-004/005` | C-DEC-002, C-NEG-004, C-NEG-005 | H-001 | B | proptest | `crates::vb_storage::slot_extra::decode_slot_written_extra` | ✅ planned |
| 6 | PO-PROP-002 | `vb-5bqmr-C-ENC-002` + `C-NEG-001/002/003` | C-ENC-002, C-NEG-001, C-NEG-002, C-NEG-003 | H-005, H-006, H-014 | B | proptest | `crates::vb_storage::slot_extra::{decode_slot_written_extra, encode_slot_written_extra, SlotWrittenExtraError, SlotWrittenExtraEnvelope}` | ✅ planned |
| 7 | PO-PROP-003 | `vb-5bqmr-C-REC-001/002` + `C-RUN-001/002` + `C-FOR-001/002` | C-REC-001, C-REC-002, C-RUN-001, C-RUN-002, C-FOR-001, C-FOR-002 | H-003, H-004, H-009, H-013 | B | proptest | `crates::vb_storage::recovery::replay::summary::hydrate::decoded_slot_taint` + `crates::vb_runtime::primitives::collect::CollectStates::hydrate_slot_written_extra` | ✅ planned |

## Contract-clause → obligation mapping

| Clause | Description | Obligations covering it |
|---|---|---|
| C-DEC-001 | v1 prefix → Envelope or DecodeFailed | PO-VERUS-001, PO-KANI-002, PO-FLUX-001, PO-PROP-002 |
| C-DEC-002 | magic-but-unknown-version → VersionMismatch (the bug fix) | PO-VERUS-001, PO-KANI-001, PO-FLUX-001, PO-PROP-001 |
| C-DEC-003 | no-magic → LegacyFrameExtra (preserved) | PO-VERUS-001, PO-KANI-002, PO-PROP-002 |
| C-DEC-004 | three arms mutually exclusive + exhaustive | PO-VERUS-001, PO-KANI-002, PO-FLUX-001 |
| C-CON-001 | prefix compositionally derived from MAGIC+VERSION | PO-KANI-002, PO-FLUX-001, PO-PROP-002 |
| C-CON-002 | prefix retained | (compile-time `cargo build --all-targets -p vb_storage`) |
| C-CON-003 | MAGIC, VERSION re-exported | (compile-time; `cargo doc` smoke) |
| C-CON-004 | prefix len = MAGIC.len() + 1 = 5 | PO-FLUX-001 |
| C-ERR-001 | VersionMismatch Copy + zero-allocation | PO-PROP-002 |
| C-ERR-002 | VersionMismatch{0x01} unreachable from decoder | PO-VERUS-001, PO-KANI-001, PO-PROP-001 |
| C-ERR-003 | at-most-one-outcome | PO-VERUS-001, PO-KANI-002 |
| C-REC-001 | hydrate match exhaustive, no catch-all | PO-PROP-003 |
| C-REC-002 | hydrate VersionMismatch → CorruptSlotTaint + warn | PO-PROP-003 |
| C-REC-003 | hydrate DecodeFailed → CorruptSlotTaint | PO-PROP-003 |
| C-REC-004 | RecoveryError not widened | (compile-time; `cargo build --all-targets -p vb_storage`) |
| C-RUN-001 | collect match exhaustive, no catch-all | PO-PROP-003 |
| C-RUN-002 | collect VersionMismatch → CollectExtraHydrationFailed{kind=VersionMismatch} + warn | PO-PROP-003 |
| C-RUN-003 | collect DecodeFailed → kind=DecodeFailed | PO-PROP-003 |
| C-RUN-004 | CollectExtraHydrationFailureKind gains VersionMismatch | (compile-time; `cargo build --all-targets -p vb_runtime`) |
| C-ENC-001 | encoder unchanged | (compile-time; round-trip in PO-PROP-002) |
| C-ENC-002 | encode/decode round-trip | PO-PROP-002 |
| C-NEG-001 | legacy short input → LegacyFrameExtra (regression BDD) | PO-KANI-002, PO-PROP-002 |
| C-NEG-002 | 4-byte magic-only → LegacyFrameExtra | PO-KANI-002, PO-PROP-002 |
| C-NEG-003 | corrupt v1 → DecodeFailed | PO-KANI-002, PO-PROP-002 |
| C-NEG-004 | unknown version byte → VersionMismatch{found} | PO-KANI-001, PO-PROP-001 |
| C-NEG-005 | high version byte (0xFF) → VersionMismatch{0xFF} | PO-KANI-001, PO-PROP-001 |
| C-NEG-006 | legacy arm zero allocation | PO-KANI-002 |
| C-FOR-001 | no catch-all Err(_) in vb_storage | PO-PROP-003 |
| C-FOR-002 | no catch-all Err(_) in vb_runtime | PO-PROP-003 |
| C-FOR-003 | forward-compat monotone | (process; tracked in `vb-1rqz7.*`) |

## Hazard coverage

| Hazard | Severity | Lanes | Obligations | Status |
|---|---|---|---|---|
| H-001 silent downgrade | P1 | V, K, F, P | PO-VERUS-001, PO-KANI-001, PO-FLUX-001, PO-PROP-001 | ✅ planned |
| H-005 BDD legacy regression | regression | K, P | PO-KANI-002, PO-PROP-002 | ✅ planned |
| H-006 BDD corrupt-v1 regression | regression | K, P | PO-KANI-002, PO-PROP-002 | ✅ planned |
| H-008 allocation on legacy | N/A | K | PO-KANI-002 | ✅ planned |
| H-009 warn-log emission | negligible | P | PO-PROP-003 | ✅ planned |
| H-013 API-additive enum widening | low | P (compile-time) | PO-PROP-002, PO-PROP-003 | ✅ planned |
| H-014 forward-compat hardener | N/A | V, K, P | PO-VERUS-001, PO-KANI-001, PO-PROP-001, PO-PROP-002 | ✅ planned |
| H-016 master §47 lattice preserve | low | K | PO-KANI-002 (legacy arm classification) | ✅ planned |

## Self-audit checklist

- [x] Every `(requirement_id, contract_clause, proof_seed_id, verifier)` tuple
      in the user-bounded lane set has at least one `verifier-lane-decision/v1`
      row.
- [x] Every `required` lane decision has at least one paired
      `proof-obligation/v1` ID, and the obligation exists in
      `proof-obligations.planned.jsonl`.
- [x] Every `proof-obligation/v1` row references a production source symbol
      via `target` (no file-only).
- [x] The Verus obligation has a `production_binding` field with `mechanism: STRONG`
      pointing at `crates/vb_storage/src/slot_extra.rs`.
- [x] No behavior-affecting waiver row is emitted (waivers are non-behavior).
- [x] No `not_applicable` row is emitted; the user-bounded four-lane set is
      complete for this bead.
- [x] The bead's user-stated forbidden behaviors are covered by the
      anti-invariant clauses in PO-PROP-001/002/003.