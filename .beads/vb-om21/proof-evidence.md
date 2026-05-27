# Proof Evidence — vb-om21 State 5 proof-writer-repair Attempt 8 (Kani Assertion Repair)

## Commands run

### Tool versions

`command -v cargo; cargo --version; rustc --version --verbose; rustup show active-toolchain; command -v verus; verus --version; command -v java; java --version; cargo kani --version; cargo flux --version; cargo +nightly miri --version; cargo +nightly fuzz --version`

- cargo 1.97.0-nightly; rustc 1.97.0-nightly; active override `nightly-2026-04-28-x86_64-unknown-linux-gnu`.
- Verus `0.2026.05.05.d03e906`.
- OpenJDK `26.0.1`.
- cargo-kani `0.67.0`; cargo-flux `4d329f2 (2026-05-23)`; miri `0.1.0`; cargo-fuzz `0.13.1`.
- `tools/tla2tools.jar` missing.

### Kani (Attempt 8 — kani::assert assertion repair)

All 7 Kani harnesses flagged as KANI_COVER_ONLY now use `kani::assert(...)` alongside `cover!` calls. Each assertion encodes the corresponding PO domain claim directly as a Kani-level proof obligation. All harnesses pass:

**PO-vb-om21-prefix-bound-kani** (`vb_om21_prefix_bound_harness`):
- `cargo kani -p vb_storage --harness vb_om21_prefix_bound_harness`
- Result: PASS, `VERIFICATION:- SUCCESSFUL`, `0 of 224 failed`, 2 covers satisfied.
- Assertions: correct run prefix matched; correct run parses encoded sequence; mismatched run prefix yields None.

**PO-vb-om21-big-endian-max-kani** (`vb_om21_big_endian_max_harness`):
- `cargo kani -p vb_storage --harness vb_om21_big_endian_max_harness`
- Result: PASS, `VERIFICATION:- SUCCESSFUL`, `0 of 251 failed`, 2 covers satisfied.
- Assertions: key-a roundtrips sequence a; key-b roundtrips sequence b; lexicographic order matches numeric EventSeq order.

**PO-vb-om21-tail-mismatch-kani** (`vb_om21_tail_mismatch_harness`):
- `cargo kani -p vb_storage --harness vb_om21_tail_mismatch_harness`
- Result: PASS, `VERIFICATION:- SUCCESSFUL`, `0 of 14 failed (1 unreachable)`, 1 cover satisfied.
- Assertion: metadata below reconstructed tail yields TailMismatch.

**PO-vb-om21-tail-overflow-kani** (`vb_om21_tail_overflow_harness`):
- `cargo kani -p vb_storage --harness vb_om21_tail_overflow_harness`
- Result: PASS, `VERIFICATION:- SUCCESSFUL`, `0 of 10 failed`, 2 covers satisfied.
- Assertions: u64::MAX yields TailOverflow (no wrap to zero); non-MAX yields Ok successor tail+1.

**PO-vb-om21-key-parse-kani** (`vb_om21_key_parse_harness`):
- `cargo kani -p vb_storage --harness vb_om21_key_parse_harness`
- Result: PASS, `VERIFICATION:- SUCCESSFUL`, `0 of 163 failed`, 1 cover satisfied.
- Assertion: Some decodes only from prefix-matching keys (malformed bytes rejected without panic).

**PO-vb-om21-replay-parity-kani** (`vb_om21_replay_parity_harness`):
- `cargo kani -p vb_storage --harness vb_om21_replay_parity_harness`
- Result: PASS, `VERIFICATION:- SUCCESSFUL`, `0 of 2 failed`, 2 covers satisfied.
- Assertions: accepted events match run/sequence; rejected events have mismatch.

**PO-vb-om21-typed-errors-kani** (`vb_om21_typed_errors_harness`):
- `cargo kani -p vb_storage --harness vb_om21_typed_errors_harness`
- Result: PASS, `VERIFICATION:- SUCCESSFUL`, `0 of 18 failed`, 3 covers satisfied.
- Assertions: no keys seen in recovery yields MissingJournal; below metadata yields TailMismatch; MAX sequence yields TailOverflow.

All 7 harnesses use `kani::assert(condition, description)` (Kani 0.67.0 function syntax) wrapping the behavioral domain claims at the harness level, plus `kani::cover!` for non-vacuity evidence. All 7 previously E_KANI_COVER_ONLY violations are resolved.

### Kani (Attempt 7 — original pass evidence)

### Verus

`verus --crate-type=lib verification/verus/vb_om21_tail_fallback_prefix_bound.rs && ... && verus --crate-type=lib verification/verus/vb_om21_tail_fallback_typed_errors.rs`

Result: PASS for all 11 repaired Verus files; outputs included `verification results:: ... verified, 0 errors`.

### proptest

Exact planned proptest commands were run through nextest filters for all 11 `vb_om21_*_proptest` names.

Result: PASS; each nextest run reported `1 test run: 1 passed`.

### Miri

Exact planned command:

`cargo +nightly miri test -p vb_storage vb_om21_key_parse_miri`

Result: COMMAND_NOTE on host alias (rust-src path mismatch): `fatal error: given Rust source directory /home/lewis/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library does not exist`.

Pinned repo command:

`cargo +nightly-2026-04-28 miri test -p vb_storage vb_om21_key_parse_miri`

Result: PASS; `test vb_om21_key_parse_miri::vb_om21_key_parse_miri ... ok`; `1 passed`.

### Flux

Exact planned command:

`cargo flux -p vb_storage --lib --features flux-proofs -- --check vb_om21_prefix_bound`

Result: COMMAND_PLAN_NOTE for installed CLI (--lib not accepted): `error: unexpected argument '--lib' found`.

Supported installed command:

`cargo flux -p vb_storage -F flux-proofs`

Result: PASS; `Finished flux profile`.

### cargo-fuzz

Exact planned command:

`cargo +nightly fuzz run vb_om21_key_parse_key_parser -- -runs=100000`

Result: COMMAND_PLAN_NOTE/tool default (musl target mismatch): cargo-fuzz 0.13 defaults to `x86_64-unknown-linux-musl`; ASan build fails with `sanitizer is incompatible with statically linked libc`.

GNU-target command:

`cargo +nightly fuzz run vb_om21_key_parse_key_parser --target x86_64-unknown-linux-gnu -- -runs=100000`

Result: PASS; libFuzzer reported `#100000 DONE` and `Done 100000 runs`.

### TLA+

Exact planned command:

`java -jar tools/tla2tools.jar verification/tla/vb_om21_tail_fallback_prefix_bound.tla -config verification/tla/vb_om21_tail_fallback_prefix_bound.cfg`

Result: RECORDED_TOOLING_GAP: `Error: Unable to access jarfile tools/tla2tools.jar` (TLA+ jar not present in this checkout; TLC evidence deferred).

## Trust / bounds recorded

- Kani model boundary: `crates/vb_storage/src/kani_vb_om21_model.rs` mirrors `[0x11][run_id_u64_be][seq_u64_be]` with fixed arrays and scalar conversions because the production ArrayVec encoder caused Kani `UNDETERMINED` memory checks before obligation assertions.
- Kani bounds: scalar `u64` symbolic inputs, `[u8; 17]` for key parser, 4-event bounded scan, `#[kani::unwind(18)]` or `#[kani::unwind(5)]` where loops exist.
- Fuzz budget: 100000 libFuzzer runs on GNU target.
- TLA+ recorded as pending tooling availability; no TLC pass claimed.
