# Implementation — vb-svvr7

- bead_id: vb-svvr7
- state: 5 implementation
- date: 2026-07-01
- jj change_id: ca97a6023b45
- skill: p11-holzman-rust
- workspace: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-svvr7

## Summary

Tightened the strict-length frame check in `vb_cli::cli_postcard::decode_postcard` so
that a valid frame plus trailing bytes returns a new typed error
`PostcardError::TrailingBytes`. Truncated input still returns
`PostcardError::DecodeFailed` (preserved per CC-TB-2 and required by
existing tests `test_decode_data_too_short` and
`decode_rejects_truncated_header`). The new variant propagates through
`decode_postcard_json` via `?` without any code change in `codec.rs`,
matching the parity reference at `crates/vb_ipc/src/frame.rs:35-51`.

The user-supplied one-line summary "`if data.len() != payload_end { return
Err(PostcardError::TrailingBytes) }`" would have collapsed the two
distinct failure modes and broken the existing
`DecodeFailed` assertions on truncated input. The contract
(CC-TB-2 + CC-TB-3) and existing tests in `tests.rs:79-83, 171-177`
require preserving both branches. Implementation reflects the contract.

## Files Touched

- `crates/vb_cli/src/cli_postcard/validation.rs` (3 lines added at lines 90-92)
- `crates/vb_cli/src/cli_postcard/error.rs` (1 variant + 1 Display arm added)
- `crates/vb_cli/src/cli_postcard/tests.rs` (4 new unit tests appended after line 177)

## Diffs

### crates/vb_cli/src/cli_postcard/validation.rs

```diff
     if data.len() < payload_end {
         return Err(PostcardError::DecodeFailed);
     }
+    if data.len() > payload_end {
+        return Err(PostcardError::TrailingBytes);
+    }

     let header_bytes = data
         .get(0..HEADER_SIZE)
```

### crates/vb_cli/src/cli_postcard/error.rs

```diff
     /// Data too short to contain valid header.
     DecodeFailed,
+    /// Input buffer contains a valid frame followed by one or more trailing bytes.
+    TrailingBytes,
 }
 ...
             Self::DecodeFailed => write!(f, "postcard decode failed: data too short"),
+            Self::TrailingBytes => {
+                write!(
+                    f,
+                    "postcard decode failed: trailing bytes after valid frame"
+                )
+            }
         }
```

### crates/vb_cli/src/cli_postcard/tests.rs (4 new tests)

```rust
#[test]
fn decode_rejects_trailing_bytes_after_valid_frame() {
    let mut encoded = encode_test_postcard(CLI_SCHEMA_VERSION, CLI_POSTCARD_KIND, b"payload");
    encoded.push(0xAA);
    assert_eq!(decode_postcard(&encoded), Err(PostcardError::TrailingBytes));
}

#[test]
fn decode_accepts_exact_length_frame() {
    let encoded = encode_test_postcard(CLI_SCHEMA_VERSION, CLI_POSTCARD_KIND, b"payload");
    let (header, payload) = decode_postcard(&encoded).expect("exact-length frame decodes");
    assert_eq!(header.len(), HEADER_SIZE);
    assert_eq!(payload, b"payload");
}

#[test]
fn decode_postcard_json_propagates_trailing_bytes() {
    let mut encoded = encode_test_postcard(CLI_SCHEMA_VERSION, CLI_POSTCARD_KIND, b"payload");
    encoded.extend_from_slice(&[0u8; 8]);
    assert_eq!(
        decode_postcard_json(&encoded),
        Err(PostcardError::TrailingBytes)
    );
}

#[test]
fn postcard_error_trailing_bytes_is_unit_variant_and_distinct() {
    let trailing = PostcardError::TrailingBytes;
    let failed = PostcardError::DecodeFailed;
    assert_ne!(trailing, failed);
    assert_eq!(trailing, PostcardError::TrailingBytes);
    let display = format!("{trailing}");
    assert!(!display.is_empty());
    assert!(display.contains("trailing"));
    assert_ne!(display, format!("{failed}"));
}
```

## Power of Ten / Holzman Rules Affected

| Rule | Status |
| --- | --- |
| Rule 1 (simple control flow) | satisfied — pure length-compare, no hidden branches |
| Rule 2 (fixed loop bounds) | N/A — no loops added |
| Rule 3 (no post-init alloc) | satisfied — no allocation in the new branch |
| Rule 4 (functions ≤ ~25 logical lines) | satisfied — `decode_postcard` body grew by 3 lines |
| Rule 5 (invariant density) | satisfied — typed error instead of silent accept |
| Rule 6 (smallest scope) | satisfied — three lines added; no new locals |
| Rule 7 (checked returns) | satisfied — `?` propagation preserved in `codec.rs:27` |
| Rule 8 (limited macros/pointers) | N/A — no macros/pointers |
| Rule 9 (restricted pointer/indirect calls) | N/A |
| Rule 10 (warnings and analysis) | satisfied — zero warnings, lint clean |
| Zero forbidden constructs | satisfied — no `unsafe`/`unwrap`/`expect`/`panic`/`todo`/`unreachable`/`assert!` macros in production |
| Bounded control flow | N/A — no loops |
| Static dispatch | satisfied — `Result<&[u8], PostcardError>` |

## Performance Layer

- No claim made. Single integer compare (`>`) added at lines 90-92 of
  `validation.rs`; negligible latency impact, no allocation change.

## Commands Run And Pass/Fail

| Command | Status | Evidence |
| --- | --- | --- |
| `cargo test -p velvet-ballistics --lib cli_postcard` | PASS — 21/21 | `evidence/cargo-test-vb_cli-cli_postcard.txt` |
| `cargo test -p velvet-ballistics --lib` | PASS — 218/218 | `evidence/cargo-test-vb_cli-full.txt` |
| `cargo test -p vb_ipc --lib` | PASS — 540/540 (parity preserved) | `evidence/cargo-test-vb_ipc-full.txt` |
| `cargo clippy -p velvet-ballistics -p vb_ipc --lib --bins --examples --all-features --` strict Holzman lints | PASS | `evidence/cargo-clippy-vb_cli-vb_ipc.txt` |
| `cargo fmt --check -p velvet-ballistics` | PASS | `evidence/cargo-fmt-vb_cli.txt` |
| `bash scripts/check-panic-surface.sh` | PASS — NoViolationFound | `evidence/check-panic-surface.txt` |
| `cargo check -p velvet-ballistics --lib` (preflight) | PASS | (build warm) |

## Test Additions (4 new tests, all passing)

1. `decode_rejects_trailing_bytes_after_valid_frame` — encoded frame + 1 trailing byte
   yields `Err(PostcardError::TrailingBytes)` (CC-TB-3, CC-TB-5).
2. `decode_accepts_exact_length_frame` — no trailing bytes yields `Ok` with header and
   payload intact (regression guard for CC-TB-1, CC-TB-7).
3. `decode_postcard_json_propagates_trailing_bytes` — JSON entry point propagates
   `TrailingBytes` via `?` without remapping (CC-TB-6, CC-TB-9).
4. `postcard_error_trailing_bytes_is_unit_variant_and_distinct` — `TrailingBytes`
   equals itself, differs from `DecodeFailed`, has non-empty Display containing the
   substring `"trailing"`, and Display differs from `DecodeFailed` (CC-TB-4, CC-TB-5,
   INV-TB-3).

## Test Outputs (excerpt)

```
running 21 tests
test cli_postcard::tests::decode_postcard_json_propagates_trailing_bytes ... ok
test cli_postcard::tests::decode_accepts_exact_length_frame ... ok
test cli_postcard::tests::decode_rejects_trailing_bytes_after_valid_frame ... ok
test cli_postcard::tests::postcard_error_trailing_bytes_is_unit_variant_and_distinct ... ok
... (17 existing tests pass unchanged)
test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 197 filtered out
```

## Cross-Crate Parity

- `vb_ipc::frame::decode_frame_payload` enforces `payload.len() != expected_len` and
  returns `IpcError::PayloadLengthMismatch` (frame.rs:44-49). The CLI envelope now
  matches that pattern with `PostcardError::TrailingBytes`.
- `vb_ipc` library tests (540) and `vb_cli` library tests (218) all pass.
- Cross-crate strict Holzman clippy set passes for both crates.

## Out of Scope (per contract)

- No `Frame<'a>` newtype introduced.
- No `TrailingBytes { count: usize }` payload (kept unit for parity).
- No `CLI_SCHEMA_VERSION` bump (bug fix).
- No dependency changes.
- Miri/Verus/Flux/Kani/Loom/fuzz harnesses — not warranted for this single-int-compare fix.

## Residual Risk

- None observed in touched code. The change is a three-line guard in production,
  plus four tests; clippy, fmt, panic-surface, and full vb_cli + vb_ipc test suites
  all pass.