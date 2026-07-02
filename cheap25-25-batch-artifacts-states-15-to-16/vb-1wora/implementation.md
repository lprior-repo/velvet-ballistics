# Implementation — vb-1wora

**Bead:** `vb-1wora` — Codec: reject trailing bytes after declared record payload (P1 bug)
**State:** 11 (p11-holzman-rust)
**Skill:** `holzman-rust`
**Workspace:** `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-1wora`
**JJ workspace:** `cheap25-vb-1wora`
**Parent commit:** `0ac987c0` (p5-proof-writer)

## Reference files read

- `/home/lewis/.opencode/skill/holzman-rust/SKILL.md`
- `/home/lewis/.agents/skills/holzman-rust/SKILL.md`
- `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-1wora/.beads/vb-1wora/contracts/contract.md` (State 3 rust-contract)

## Power-of-Ten rules affected

| Rule | Status |
|---|---|
| Rule 1: simple control flow | Satisfied — single `if` + `return` per check site, no recursion, no panic paths. |
| Rule 2: bounded control flow | Satisfied — no loops introduced; `bytes.len() > payload_end` is a single comparison. |
| Rule 3: no post-init allocation in critical paths | Satisfied — no allocations added; existing `bytes.get(..)` slice reused. |
| Rule 4: functions fit on one page | Satisfied — `decode_record_payload` < 25 logical lines (added 5 lines, total ~17 logical lines); `decode_envelope_only` similarly bounded. |
| Rule 5: assertion/invariant density | Strengthened — `bytes.len() > payload_end` is a typed-failure invariant (returns `JournalError::TrailingBytes { trailing }`). |
| Rule 6: smallest scope | Satisfied — borrows are narrowed (`bytes.len()`, `payload_end`); no new `mut` introduced. |
| Rule 7: checked returns/parameters | Satisfied — uses `checked_sub` for `trailing` arithmetic; never ignores `Result`. |
| Rule 8: limited macro power | Satisfied — no new macros; `thiserror` Display still drives format. |
| Rule 9: restricted pointer/indirect call use | Satisfied — no `unsafe`, no raw pointers, no trait objects added. |
| Rule 10: warnings/analysis mandatory | Satisfied — `cargo clippy -- -D warnings -D clippy::arithmetic_side_effects ...` passes on touched files. |

## Zero-panic rules affected

| Rule | Status |
|---|---|
| `zero_forbidden_constructs` | Satisfied — no `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `unreachable!` introduced in production. |
| `no_panic_paths` | Satisfied — `bytes.len() > payload_end` is a typed failure that returns `Err(TrailingBytes { .. })`. |
| `arithmetic_side_effects` (strict clippy) | Satisfied — used `bytes.len().checked_sub(payload_end).ok_or(...)` instead of bare subtraction. |

## Code changes

### 1. `crates/vb_storage/src/error/mod.rs`

Added the new variant immediately after `UnexpectedEof` (per the contract's "insertion after `UnexpectedEof`" guidance):

```diff
     #[error("unexpected end of record")]
     UnexpectedEof,
+    #[error("trailing bytes after declared payload: {trailing}")]
+    TrailingBytes { trailing: usize },
```

### 2. `crates/vb_storage/src/error/codes.rs`

Added `TRAILING_BYTES_CODE` constant, the `diagnostic_code()` arm, and the `symbolic_code()` arm:

```diff
     pub const REPLAY_ENVELOPE_SEQUENCE_MISMATCH_CODE: DiagnosticCode = DiagnosticCode::new(0x4041);
+    /// Diagnostic code for trailing bytes after declared record payload
+    /// (`JournalError::TrailingBytes`). Returned by `decode_record_payload`
+    /// when the input slice extends past `RECORD_HEADER_BYTES + payload_len`.
+    pub const TRAILING_BYTES_CODE: DiagnosticCode = DiagnosticCode::new(0x4042);
@@
             Self::UnexpectedEof => Self::UNEXPECTED_EOF_CODE,
+            Self::TrailingBytes { .. } => Self::TRAILING_BYTES_CODE,
@@
             Self::UnexpectedEof => "UNEXPECTED_EOF",
+            Self::TrailingBytes { .. } => "JOURNAL_TRAILING_BYTES",
```

### 3. `crates/vb_storage/src/codec/payload.rs` — cheap-before-expensive check

The check is inserted between `bytes.get(payload_start..payload_end)` and `verify_digest_match(payload, header.payload_digest)?;`. This pins INV-CODEC-TB-003.

```diff
     let payload = bytes
         .get(payload_start..payload_end)
         .ok_or(JournalError::UnexpectedEof)?;
+    // Cheap-before-expensive: reject trailing bytes before the BLAKE3
+    // digest. The encoder never produces a record that ends past
+    // `payload_end`, so any overshoot is corruption or truncation that
+    // must fail closed (INV-CODEC-TB-003).
+    if bytes.len() > payload_end {
+        let trailing = bytes
+            .len()
+            .checked_sub(payload_end)
+            .ok_or(JournalError::UnexpectedEof)?;
+        return Err(JournalError::TrailingBytes { trailing });
+    }
     verify_digest_match(payload, header.payload_digest)?;
```

### 4. `crates/vb_storage/src/codec/envelope.rs` — mirror check

The same check is inserted at the mirror site (INV-CODEC-TB-004):

```diff
     let raw_payload = bytes
         .get(payload_start..payload_end)
         .ok_or(JournalError::UnexpectedEof)?;
+    // Cheap-before-expensive: reject trailing bytes before the BLAKE3
+    // digest. Mirrors the canonical `decode_record_payload` check
+    // (INV-CODEC-TB-004). The encoder never produces a record that
+    // ends past `payload_end`, so any overshoot is corruption or
+    // truncation that must fail closed.
+    if bytes.len() > payload_end {
+        let trailing = bytes
+            .len()
+            .checked_sub(payload_end)
+            .ok_or(JournalError::UnexpectedEof)?;
+        return Err(JournalError::TrailingBytes { trailing });
+    }
+
     // Fail closed: envelope-only decode must still detect payload tampering.
     verify_digest_match(raw_payload, header.payload_digest)?;
```

### 5. `crates/vb_storage/src/codec/tests.rs` — test inversion

Renamed and inverted `decode_ignores_trailing_bytes_beyond_payload` to `decode_rejects_trailing_bytes_after_payload`. The 3-byte `0xFF 0xFE 0xFD` tail is preserved; the assertion now expects `Err(TrailingBytes { trailing: 3 })` (INV-CODEC-TB-001).

### 6. `crates/vb_storage/src/codec/envelope.rs` — mirror test

Added `decode_envelope_only_rejects_trailing_payload` as a sibling of `decode_envelope_only_rejects_truncated_payload`, asserting `Err(TrailingBytes { trailing: 4 })` on a 4-byte `0xAA 0xBB 0xCC 0xDD` tail (INV-CODEC-TB-004).

### 7. `crates/vb_storage/src/error_tests.rs` — variant trio

Added `trailing_bytes_variant_and_fields`, `trailing_bytes_display_format`, and `trailing_bytes_error_code` mirroring the `MissingRequiredProofFlag` pattern. Updated the audit header to move `TrailingBytes` into the "Tested variants" block.

### 8. `crates/vb_storage/src/error_code_tests.rs` — diagnostic-code registration test

Added `trailing_bytes_error_has_correct_code` after `payload_too_large_error_has_correct_code` (lines 144-151). It asserts `TrailingBytes { trailing: 3 }.diagnostic_code() == JournalError::TRAILING_BYTES_CODE` and `TRAILING_BYTES_CODE == DiagnosticCode::new(0x4042)`.

### 9. `crates/vb_storage/src/tests.rs` — exhaustive-match update

The compile-time exhaustive match in `journal_error_match_covers_all_variants` (line 7631) gained a `JournalError::TrailingBytes { .. } => "trailing_bytes"` arm to keep the test passing.

### 10. `crates/vb_storage/src/security_tests.rs` — BH-13 update

`zero_payload_len_with_bytes_fails_digest_check` was previously asserting `PayloadDigestMismatch`. With the new cheap-before-expensive ordering, it now asserts `TrailingBytes { .. }` (the test name and BH-13 invariant are preserved; the test doc-comment was updated to document the new ordering).

### 11. `crates/vb_storage/tests/proptest_vb_vzcuf_PS_003.rs` — proptest coverage

Added two new proptests:
- `ps003_trailing_bytes_are_rejected(run, trailing_len)` — INV-CODEC-TB-001. Appends 1..=8 trailing bytes and asserts `Err(TrailingBytes { trailing: N })` with `trailing > 0`.
- `ps003_exact_boundary_roundtrips(run)` — INV-CODEC-TB-002. Asserts a record that ends exactly at the declared payload boundary round-trips successfully (no trailing-bytes false positive).

## Exact commands run

| Command | Result |
|---|---|
| `cargo check -p vb_storage --all-features` | exit=0, "Finished `dev` profile" |
| `cargo check -p vb_storage --all-features --tests` | exit=0, "20 crates compiled" |
| `cargo test -p vb_storage --all-features --no-run` | exit=0 |
| `cargo test -p vb_storage --all-features` | **1678 passed (17 suites, 11.59s)** |
| `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_003` | **8 passed (1 suite, 1.62s)** — 2 new proptests added |
| `cargo test -p vb_storage --all-features 'trail'` (binary `vb_storage-0947b7d5f74e6fe3 trailing`) | **6 passed; 0 failed; 1529 filtered out** |
| `proptest_vb_vzcuf_PS_003-8ef2cda9823755f2` (full) | **8 passed; 0 failed** including new `ps003_trailing_bytes_are_rejected` and `ps003_exact_boundary_roundtrips` |
| `cargo clippy -p vb_storage --lib --bins --examples --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use -D clippy::await_holding_lock` | **No issues found** |
| `cargo fmt --check -p vb_storage` | exit=0 (no diff) |

## Performance-layer decision

No performance claim is made. The new check is `if bytes.len() > payload_end` (a single comparison) and is positioned to save the BLAKE3 hash computation when the input is oversize — strictly an improvement, not a regression. No benchmarks are run because this is a correctness fix, not a perf change. The "cheap-before-expensive" ordering is itself a perf argument recorded in the contract, not measured here.

## Second-ring evidence

None required. This is a pure correctness fix; no zero-cost-abstraction, vectorization, bounds-check-removal, public-API-compatibility, or release-provenance claims are made.

## Skipped gates and concrete reasons

- `cargo fmt --check` (workspace-wide) is **skipped**. The remaining 4 fmt violations are in `vb_core/src/lib.rs:26`, `vb_core/src/time.rs:71`, and `vb_runtime/src/frame_pool/tests.rs:114,139` — all **pre-existing in the parent commit** (`rsvywymk 1d6c017f`, AGENTS.md round10 forward-port). They are out of this bead's scope and classified as `BLOCK_GLOBAL` prerequisite repair, not new regressions introduced by vb-1wora. The touched files (`vb_storage/**` and the proptest) are fmt-clean.
- Production `assert!/unreachable!` scan was not run because the touched files do not contain any new `assert!` / `assert_eq!` / `unreachable!` macros. The strict source lint (`-D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro`) passes.
- `cargo geiger` / `cargo machete` / `cargo audit` / `cargo deny` / `cargo vet` / `cargo mutants` were not run; no new dependencies or unsafe code were introduced.

## Residual risks

- The `SymbolicCode::from_static("JOURNAL_TRAILING_BYTES")` arm is registered in `codes.rs::symbolic_code()` but **not yet** registered in `crates/vb_core/src/diagnostic.rs::CODE_REGISTRY`. Per the contract, this is "Recommended (not mandatory)" (section 4.2). The fallback to `SymbolicCode::INTERNAL_INVARIANT` is the existing convention for unregistered symbolic names; this is tracked in the contract's risk register as LOW severity.
- The pre-existing workspace-wide fmt failures in `vb_core`/`vb_runtime` are unrelated to this bead but block `cargo fmt --check` at the repo root. They should be repaired as a separate `BLOCK_GLOBAL` gate before final landing.
