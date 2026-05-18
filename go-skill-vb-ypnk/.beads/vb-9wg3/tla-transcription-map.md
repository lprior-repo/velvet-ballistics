# vb-9wg3 TLA Transcription Map

## Source Identity

- Source: `specs/tla/BudgetArithmetic.tla`
- SHA-256: `e9e65b0e2875a79f3cda0e5503d976cd7db99472bf5e420858249d20683bae21`
- Hash evidence: `.beads/vb-9wg3/evidence/budget-arithmetic-tla.sha256`

## Operator Mapping

| TLA source | TLA operator or constant | Kani projection | Notes |
| --- | --- | --- | --- |
| lines 16-17 | `MAX_U16 == 65535`, `BASE == 65536` | `MAX_U16_LIMB`, `BASE` | Exact numeric transcription. |
| lines 55-64 | `ZeroWord`, `OneWord`, `MaxU16Word`, `MaxU32Word`, max/near-max words | `word_from_u64`, `TLA_MAX_U16_WORD`, `TLA_MAX_U32_WORD` | Kani proves Rust `u16::MAX` and `u32::MAX` encode to the transcribed TLA max words. |
| lines 66-71 | `WordTypeOK` | `word_type_ok` | Kani asserts successful add/sub outputs preserve this predicate. |
| lines 73-85 | `WordLT`, `WordLE` | `word_lt`, `word_le` | Kani proves `word_le(word_from_u64(a), word_from_u64(b)) == (a <= b)` for all symbolic `u64` pairs. |
| lines 95-97 | `Carry`, `Limb` | `carry`, `limb` | Same threshold and base-subtraction behavior over decoded 16-bit limbs. |
| lines 99-115 | `AddWord` | `add_word` | Same four-limb carry chain; Kani proves equivalence to Rust `u64::checked_add`. |
| lines 116-134 | `Borrow`, `SubLimb`, `SubWord` | `sub_limb_with_borrow`, `sub_word` | Same borrow behavior; Kani proves equivalence to Rust `u64::checked_sub`. |
| lines 187-195 | `AddResult`, `SubResult` error tags | Kani `WordError::{Overflow, Underflow}` assertions | Kani proves error classification matches Rust checked arithmetic failure. |

## Review Boundary

This map is a manual transcription review aid, not a parser-backed TLA AST proof. The Kani refinement claim is therefore: the reviewed Rust projection of the named TLA limb operators matches Rust full-width integer semantics. TLC separately checks the `.tla` source under its configured finite model.
