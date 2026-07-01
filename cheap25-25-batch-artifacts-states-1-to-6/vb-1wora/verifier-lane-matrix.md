# Verifier Lane Matrix — vb-1wora

Maps each proof seed (from `contracts/proof-seeds.jsonl`) to its assigned verifier lanes for this bead. Built from `verifier-lane-decisions.jsonl`.

## 1. Matrix

| Proof Seed ID | Description | rust-local | cargo-test | proptest | kani | verus | cargo-fuzz |
|---|---|---|---|---|---|---|---|
| PS-VB-1WORA-001 | TrailingBytes iff bytes.len() > payload_end (canonical) | — | ✅ | — | ✅ | — | ✅ |
| PS-VB-1WORA-002 | Ok only if bytes.len() == payload_end | — | ✅ | ✅ | — | — | — |
| PS-VB-1WORA-003 | Trailing check precedes verify_digest_match | ✅ | — | — | ✅ | — | — |
| PS-VB-1WORA-004 | decode_envelope_only obeys same invariant (mirror) | — | ✅ | ✅ | — | — | — |
| PS-VB-1WORA-005 | TrailingBytes reachable only when trailing > 0 | — | ✅ | ✅ | ✅ | — | — |
| PS-VB-1WORA-006 | TRAILING_BYTES_CODE == 0x4042 and variant maps to it | ✅ | ✅ | — | — | — | — |
| PS-VB-1WORA-007 | Verus PS-003 bridge enumerates TrailingBytes arm | — | — | — | — | ✅ | — |
| PS-VB-1WORA-008 | Fuzz target exercises trailing-bytes path | — | — | — | — | — | ✅ |
| PS-VB-1WORA-009 | TrailingBytes / UnexpectedEof mutual exclusion | — | ✅ | ✅ | — | — | — |
| PS-VB-1WORA-010 | encode + decode round-trip unchanged | — | ✅ | ✅ | — | — | — |

## 2. Non-Applicable Lanes (Cross-Cutting)

These lanes are not invoked for **any** proof seed of this bead. The decision for each is recorded in `verifier-lane-decisions.jsonl` rows `VLD-vb-1wora-007` (loom), `VLD-vb-1wora-008` (miri), `VLD-vb-1wora-009` (flux), `VLD-vb-1wora-010` (tla-plus).

| Lane | Proof Seeds | Reason | Evidence ref |
|---|---|---|---|
| Loom | ALL | Single-threaded pure parser; no concurrent memory ordering, no lock-free structures, no Send/Sync shared-state across the decode boundary. | `crates/vb_storage/src/codec/payload.rs:56-82` (no shared state in decode_record_payload body or callees); `vb_storage` crate-level `#![forbid(unsafe_code)]` + absence of `std::sync` imports in `codec/*` modules; `contracts/hazard-analysis.md §2.5` |
| Miri | ALL | `vb_storage` is `#![forbid(unsafe_code)]`; the new check is a pure `usize` compare + subtraction with no raw pointers, no `MaybeUninit`, no aliasing. | crate-level forbid-unsafe attribute; the post-fix trailing-bytes check is a pure arithmetic op; the new variant has no raw pointer fields; `contracts/hazard-analysis.md §2.6` |
| Flux | ALL | No refinement type / indexed type / constraint refinement is introduced. The `trailing > 0` invariant is enforced structurally at the producer site, not at the type level. The Verus mirror handles the refinement claim. | `TrailingBytes { trailing: usize }` is a plain enum variant with a primitive field; `contracts/type-contracts.md §5` ("Type-state / typestate considerations: None"); `contracts/hazard-analysis.md §2.4` (REFINE row pinned to Verus mirror) |
| TLA+ | ALL | Decode pipeline is single-pass synchronous; no temporal / state-machine / distributed-protocol behavior. TLA+ was explicitly removed from the proof-planner skill; temporal workflows use loom + proptest. | proof-planner skill doctrine ("TLA+ removed"); `decode_record_payload` is a pure function over `&[u8]` with no observable state across calls; `contracts/hazard-analysis.md §2.1` |

## 3. Lane Decision Counts

| Lane | Required | Not Applicable | Total |
|---|---|---|---|
| rust-local | 1 (POB-vb-1wora-001) | 0 | 1 |
| cargo-test | 2 (POB-vb-1wora-002, POB-vb-1wora-003) | 0 | 2 |
| proptest | 2 (POB-vb-1wora-003, POB-vb-1wora-005) | 0 | 2 |
| kani | 1 (POB-vb-1wora-004) | 0 | 1 |
| verus | 1 (POB-vb-1wora-006) | 0 | 1 |
| cargo-fuzz | 1 (POB-vb-1wora-007) | 0 | 1 |
| loom | 0 | 1 | 1 |
| miri | 0 | 1 | 1 |
| flux | 0 | 1 | 1 |
| tla-plus | 0 | 1 | 1 |

## 4. Legend

- ✅ = Active lane (required, has planned obligation)
- — = Not applicable for this proof seed (intentionally omitted; cross-cutting non-applicability is recorded separately in §2)
- Required = at least one planned obligation is bound to this verifier
- Not Applicable = the verifier does not apply to the proof seed; concrete evidence cited in `non_applicability_evidence_refs` of the matching `verifier-lane-decisions.jsonl` row