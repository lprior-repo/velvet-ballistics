STATUS: PASS

# State 8 Gate 08 Miri Repair

## Root Cause

Gate 08 was correctly treating `PathSegment::Field(SymbolId::new(n))` as valid only when `n < symbols_count`. Two passing-intended fixtures accidentally constructed invalid accessor paths:

- `gate_08_accessor::tests::proptest_gate_08_reports_first_invalid_accessor_with_root_precedence` used `symbols_count = 1` with `SymbolId::new(1)`, so valid roots reached path validation and returned `AccessorPathInvalid { accessor_index: 0, segment_index: 0 }`.
- `gates::tests::gate_08_accepts_valid_accessor` used `make_parts`, whose default `symbols_count` is `0`, with `SymbolId::new(1)`, so the accessor fixture was invalid.

## Files Changed

- `crates/vb_validate/src/gate_08_accessor.rs`
- `crates/vb_validate/src/gates.rs`

## Repair

- Changed the root-precedence proptest fixture path to `SymbolId::new(0)` so valid roots exercise the expected `Ok(())` path while invalid roots still prove root error precedence.
- Set `parts.symbols_count = 2` in the aggregate Gate 08 valid-accessor fixture so `SymbolId::new(1)` is inside the declared symbol table range.

## Diagnostics Preserved

- Field segment validation remains exclusive: `symbol.get() < symbols_count`.
- Invalid field diagnostics still return `AccessorPathInvalid { accessor_index, segment_index }` for the first invalid path segment.
- Root validation still runs before segment validation, preserving `AccessorSlotOutOfRange` precedence when the accessor root is invalid.
- Sentinel index validation remains unchanged.

## Commands Run

- PASS: `cargo +nightly-2026-04-28 miri test -p vb_validate --lib 'gate_08_accessor::tests::proptest_gate_08_reports_first_invalid_accessor_with_root_precedence'`
- PASS: `cargo +nightly-2026-04-28 miri test -p vb_validate --lib 'gates::tests::gate_08_accepts_valid_accessor'`
- PASS: `moon run 'velvet-ballastics:miri'` (`908 passed; 0 failed`; task completed)

## Full CI Status

- Not run: `moon ci --base HEAD --head HEAD` was optional observation only. No State 8 full-green claim is made.

## Residual Risk

- Miri emitted existing warnings for unused imports and unexpected `cfg(kani)`; these were not the targeted failure and were not repaired here.
- Full CI may still contain later non-Miri failures; not classified in this targeted repair.
