# Proof Plan Review Input: vb-qi37.13.3

## Bead
- **ID:** vb-qi37.13.3
- **Title:** cli: Implement text yaml and postcard emitters
- **Source checkout:** /home/lewis/src/Velvet-ballistics
- **Isolated workspace:** /home/lewis/src/vb-qi37-13-3
- **Contract verified:** APPROVED (contract-verification-review.md)

## Contract Clauses Requiring Proof

| Clause | Description | Proof obligation(s) |
|---|---|---|
| PRE-EMIT-001 | Output data serializable to requested format | PROP-EMIT-001 |
| PRE-EMIT-002 | Payload size <= MAX_CLI_PAYLOAD_BYTES | KAN-EMIT-005 |
| PRE-EMIT-003 | RunId non-zero for YAML emission | KAN-EMIT-007 |
| PRE-EMIT-004 | No ANSI escape sequences | KAN-EMIT-008, PROP-EMIT-003 |
| POST-EMIT-001 | `--emit yaml` produces valid YAML with required fields | SNAP-YAML-001 |
| POST-EMIT-002 | `--emit postcard` produces 52-byte header + payload | SNAP-POSTCARD-001, PROP-EMIT-002 |
| POST-EMIT-003 | Magic field = 0x56424C49 | KAN-EMIT-001 |
| POST-EMIT-004 | header_len field = 52 | KAN-EMIT-002 |
| POST-EMIT-005 | CRC32C over bytes 0..47 | KAN-EMIT-003 |
| POST-EMIT-006 | BLAKE3 over payload bytes only | KAN-EMIT-004 |
| INV-EMIT-004 | payload_len validated before allocation | KAN-EMIT-005 |
| INV-EMIT-005 | PayloadTooLarge returned before buffer alloc | KAN-EMIT-006 |
| INV-EMIT-006 | BLAKE3 infallible | WAIVER-EMIT-002 |
| INV-EMIT-007 | CRC32C infallible | WAIVER-EMIT-003 |
| ERR-YamlEncodeFailed | YAML serialization infallible for OutputEnvelope | WAIVER-EMIT-004 |
| POST-EMIT-008 | `--emit text` human-readable | SNAP-TEXT-001 |

## Verification Layer Summary

| Layer | Count | Obligations |
|---|---|---|
| kani | 8 | KAN-EMIT-001–008 |
| proptest | 3 | PROP-EMIT-001–003 |
| snapshot | 3 | SNAP-YAML-001, SNAP-POSTCARD-001, SNAP-TEXT-001 |
| cargo-fuzz | 1 | FUZZ-EMIT-001 |
| static-scan | 1 | STATIC-EMIT-001 |
| cargo-llvm-cov | 1 | COV-EMIT-001 |
| cargo-mutants | 1 | MUT-EMIT-001 |
| waiver | 3 | WAIVER-EMIT-002, WAIVER-EMIT-003, WAIVER-EMIT-004 |
| **Total** | **21** | |

## CRITICAL: Kani Harness Integration (READ BEFORE APPROVAL)

**History:** Prior proof-review (State 6) was REJECTED because proof-writer falsely claimed Kani harnesses were integrated into emitter.rs. Verification proved:
- `emitter.rs` has exactly 770 lines ending in `}`
- `rg "cfg\(kani\)" crates/vb_ui_model/src/` returns ZERO matches
- `rg "emitter_proofs" crates/vb_ui_model/src/` returns ZERO matches
- Kani harnesses exist at `kani/vb-qi37.13.3/emitter_proofs.rs` (303 lines) but are UNREACHABLE

**Required Action:** Before any Kani verification can execute, proof-writer MUST add to `crates/vb_ui_model/src/emitter.rs` (before final `}`):
```rust
#[cfg(kani)]
mod emitter_proofs {
    include!("../../kani/vb-qi37.13.3/emitter_proofs.rs");
}
```

**Proof-reviewer MUST verify:**
1. `#[cfg(kani)] mod emitter_proofs` exists in emitter.rs
2. `cargo kani --list` shows harnesses before accepting Kani results
3. Do NOT accept "harnesses found: 0" as proof of completion

## Waiver Justification Summary

### WAIVER-EMIT-002 (BLAKE3 infallible)
- `blake3::Hasher::finalize` has no error return path
- Compensating: STATIC-EMIT-001 (no unsafe) + COV-EMIT-001 (>90% coverage)
- Owner: proof-reviewer

### WAIVER-EMIT-003 (CRC32C infallible)
- `crc32c::crc32c` takes `&[u8]`, returns `u32` directly
- Compensating: STATIC-EMIT-001 (no unsafe) + COV-EMIT-001 (>90% coverage)
- Owner: proof-reviewer

### WAIVER-EMIT-004 (YAML serialization infallible)
- `serde_yaml::ser::to_string` can only fail for unsupported types/self-reference
- OutputEnvelope uses serde derive with primitive fields only
- Compensating: PROP-EMIT-001 (roundtrip exercise) + COV-EMIT-001 (>90% coverage)
- Owner: proof-reviewer

## Discovery Evidence (Mandatory Gate)

```
Discovery command: rg -n "unsafe|unwrap\(|expect\(|panic!|todo!|unimplemented!|dbg!" crates/vb_ui_model/src/emitter.rs
Result: emitter.rs:1 #![forbid(unsafe_code)] — no unsafe in production code
Result: expect() calls only in #[cfg(test)] blocks (lines 526, 538, 541, 563, 573, 590, etc.)
All in test code — acceptable.

Discovery command: rg -n "requires|ensures|proof fn|kani::|proptest!|fuzz_target" crates/vb_ui_model/src/emitter.rs
Result: No verification annotations found (only doc comments containing "requires migration")
No Kani harnesses, no Verus specs, no Flux refinements in scope.
```

## Proof Completeness Check
- All 15 contract clauses have at least one proof obligation ✓
- All 3 waivers have compensating evidence (STATIC + COV) ✓
- No unmapped proof obligations ✓
- All skipped lanes have explicit rationale (TLA+, Verus, Loom, Miri, Flux) ✓
- Artifact paths are concrete and scoped to bead-local or touched-crate ✓

## Fuzz Target Path Correction (v2)

- **Obligation:** FUZZ-EMIT-001
- **Old (wrong) artifact path:** `fuzz/fuzz_targets/fuzz_emitter.rs`
- **Correct artifact path:** `fuzz/fuzz_targets.rs::fuzz_emitter` (function at line 99)
- **Verified:** `fuzz/fuzz_targets.rs` contains `pub fn fuzz_emitter(data: &[u8])` at line 99
- This correction was applied to proof-obligations.planned.jsonl

## Open Questions for Reviewer
1. KAN-EMIT-007 (RunId non-zero validation): The emitter.rs does not currently contain RunId validation. Does this require a pre-condition check in the CLI layer or in the envelope construction layer?
2. SNAP-TEXT-001: "human-readable consistency" is subjective. What exact fields should the text snapshot validate?
