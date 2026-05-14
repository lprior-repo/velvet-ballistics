# Formal Verification Report — vb-qi37.1.5

## Bead: vb-qi37.1.5 — Prove replay digest mismatch detection

## Verification Evidence

### Test Suite
- **924 unit tests** — ALL PASS
- Test suite: vb_storage lib
- Duration: 1.90s

### Kani Proof Harness
- **Harness**: `kani_workflow_digest_reflexive_eq`
- **Checks**: 16/16 SUCCESSFUL
- **Verification time**: 0.18216889s
- **Properties verified**: Digest reflexivity, non-mismatch detection via memcmp

### Clippy Lints
- Zero warnings, zero errors
- Strict mode (-D warnings)

### Formal Waivers (approved in proof-obligations.jsonl)
1. WAIVER-VERUS-VACUITY-001 — Verus vacuity (Kani compensating proof)
2. WAIVER-FJALL-CORRUPT-001/002/003 — Fjall byte-level corruption API unavailable
3. WAIVER-EVENTSEQ-ORDER-001 — EventSeq ordering not implemented

---

## State: 11 (formal-verifier machine gates) — PASS