# Trusted Base Plan — vb-t6hx (Reduced Scope)

State 4 planning ledger for assumptions, reductions, external semantics, and future trust closures. Reduced scope: proptest, Kani, cargo-fuzz, and behavior tests only. Verus/Flux/TLA+/Loom/Miri excluded. This file is not an approval artifact.

| ID | Obligation | Trusted surface / bound | Reason | Closure expectation |
|---|---|---|---|---|
| TBP-vb-t6hx-R01 | PO-vb-t6hx-R01 | Kani finite max rows/limit (16 each). | Exhaustive only inside declared bound. | Cover zero, one, limit-1, limit, limit+1, max. Use kani::Arbitrary generators. |
| TBP-vb-t6hx-R02 | PO-vb-t6hx-R02 | Output row counting parser in property test. | CLI text can contain headers/legends. | Test parser must count data rows only and assert stable categories separately. |
| TBP-vb-t6hx-R03 | PO-vb-t6hx-R03 | 60-second fuzz smoke bound. | Planning minimum; not exhaustive. | Longer campaign optional; smoke evidence is not whole-input proof. |
| TBP-vb-t6hx-R04 | PO-vb-t6hx-R04 | Storage-open effect is instrumented as ghost/spy. | Harness cannot open real storage for all generated parser inputs. | Spy must bind to actual parse/open boundary. |
| TBP-vb-t6hx-R05 | PO-vb-t6hx-R05 | Test fixture open spy. | Need evidence parse error occurs before storage open. | Spy must be public-boundary compatible, not private-field cheating. |
| TBP-vb-t6hx-R06 | PO-vb-t6hx-R06 | Fuzz harness simulates storage-open classification. | Real storage mutation forbidden in fuzz. | Classification function must be same parser used by CLI. |
| TBP-vb-t6hx-R07 | PO-vb-t6hx-R07 | Existing Kani harness may require extension. | Existing coverage is storage-level `kani_postcard_envelope_wire.rs`, not CLI-specific. | State 5 must extend/repair rather than claiming stale coverage. CRC/BLAKE3 treated as external equality oracles. |
| TBP-vb-t6hx-R08 | PO-vb-t6hx-R08 | Controlled corruptions around canonical records. | Need valid base records to mutate. | Test must cover truncation, payload too large, digest mismatch, postcard failure. |
| TBP-vb-t6hx-R09 | PO-vb-t6hx-R09 | Fuzz stage markers are instrumentation. | Need observe forbidden Postcard-before-validation. | Instrumentation must not alter production branch decisions. |
| TBP-vb-t6hx-R10 | PO-vb-t6hx-R10 | CLI decode mapper fuzz uses bounded in-memory fixture. | Real filesystem not suitable for fuzz. | Mapper must be same code used by CLI output path. |
| TBP-vb-t6hx-R11 | PO-vb-t6hx-R11 | Kani finite max value bytes (256) and max preview (64). | Exhaustive only within declared bounds. | Include boundary classes: 0, cap-1, cap, cap+1, max. Use kani::Arbitrary. |
| TBP-vb-t6hx-R12 | PO-vb-t6hx-R12 | Output assertions must avoid false positives. | Payload bytes may appear in metadata/hex. | Tests assert full raw payload absent and metadata present. |
| TBP-vb-t6hx-R13 | PO-vb-t6hx-R13 | 60-second fuzz smoke bound. | Not exhaustive for all byte strings. | Deep campaign optional; smoke must at least run targeted harness. |
| TBP-vb-t6hx-R14 | PO-vb-t6hx-R14 | Kani decode-attempt marker instrumentation. | Need observe absence of decode call. | Instrumentation must be proof-only or test-only. Harness uses kani::Arbitrary. |
| TBP-vb-t6hx-R15 | PO-vb-t6hx-R15 | Fixture can store malformed bytes. | Storage APIs may validate on write. | If write path forbids malformed values, use lower-level fixture only with documented boundary. |
| TBP-vb-t6hx-R16 | PO-vb-t6hx-R16 | Fuzz harness isolates projection formatter. | Fuzz should not mutate real journal. | Bind to production projection function. |
| TBP-vb-t6hx-R17 | PO-vb-t6hx-R17 | Kani command generator bounds keyspaces/commands. | Bounded model checking needs finite domain. | Bounds documented in harness and non-vacuity cover statements. Use kani::Arbitrary. |
| TBP-vb-t6hx-R18 | PO-vb-t6hx-R18 | Engine metadata changes may be outside user-key inventory. | Hazard analysis notes read-only engine metadata ambiguity. | If metadata changes are observed, document non-user-key boundary; no user record/key mutation waiver allowed. |

## Waiver Boundary

No behavior-affecting proof obligation is waived by this plan. `waiver-candidates.jsonl` contains only a non-behavior candidate (WC-vb-t6hx-001) for deep dependency/supply-chain attestation if no dependency manifests change for this bead. This carries over from the prior plan.

## Kani Non-Applicable Notes

- Seed 7 (doctor-boundary): Kani not_applicable. This seed concerns crate/module placement and dependency drift, not a bounded runtime transition. Source/dependency inspection is the correct evidence channel.
