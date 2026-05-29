# Proof-to-Rust Map — vb-t6hx State 7

bridger_skill: proof-to-implementation
bridger_invocation_id: proof-to-implementation-vb-t6hx-state7-001
bridge_state: 7
bead: vb-t6hx
workspace: /home/lewis/isolated/femdation-velvet-ballistics/vb-t6hx
reviewer_input: proof-reviewer-vb-t6hx-state6-001 (APPROVED)
parent_invocation: femdation-controller-vb-t6hx-state7

## Source Review

| Source artifact | SHA-256 |
|---|---|
| `proof-review.md` | (approved, reviewed 2026-05-27T17:00:00Z) |
| `proof-obligations.planned.jsonl` | `12bb9ad62bd6444727c82a2b160a0c3eeb657162173a2401e21352f1a51833ea` |
| `proof-evidence.md` | `853f7e60159370a66c340376ed7ac96bbd829b0a4a778214f095342f831faa3f` |
| `proof-findings.jsonl` | 5 findings (PF-vb-t6hx-R001 through R005) |
| `proof-to-implementation-input.md` | State 4 bridger handoff |

## Production Source Targets

### codec spine

| Symbol | Path | Lines |
|---|---|---|
| `decode_record_header` | `crates/vb_storage/src/codec/header.rs` | 26-58 |
| `decode_journal_event` | `crates/vb_storage/src/codec/mod.rs` | 54-64 |
| `decode_record` | `crates/vb_storage/src/codec/mod.rs` | 35-44 |
| `encode_record_header` | `crates/vb_storage/src/codec/header.rs` | 14-24 |
| `verify_digest_match` | `crates/vb_storage/src/codec/payload.rs` | (pub use in mod.rs:18) |
| `payload_len_u32` | `crates/vb_storage/src/codec/payload.rs` | (internal) |
| `validate_kind_family` | `crates/vb_storage/src/codec/validation.rs` | (internal) |

### storage constants

| Symbol | Path | Lines |
|---|---|---|
| `MAGIC_JOURNAL_EVENT` | `crates/vb_storage/src/constants.rs` | 52 |
| `MAX_JOURNAL_EVENT_PAYLOAD_BYTES` | `crates/vb_storage/src/constants.rs` | 78 |
| `RECORD_HEADER_BYTES` | `crates/vb_storage/src/constants.rs` | 74 |
| `RECORD_HEADER_LEN` | `crates/vb_storage/src/constants.rs` | 46 |
| `DIGEST_BYTES` | `crates/vb_storage/src/constants.rs` | 72 |
| `CURRENT_SCHEMA_VERSION` | `crates/vb_storage/src/constants.rs` | 48 |

### error taxonomy

| Symbol | Path | Lines |
|---|---|---|
| `JournalError` | `crates/vb_storage/src/error/mod.rs` | 20 |
| `JournalError::UnexpectedEof` | `crates/vb_storage/src/error/mod.rs` | 123-125 |
| `JournalError::PayloadTooLarge` | `crates/vb_storage/src/error/mod.rs` | 109-116 |
| `JournalError::PostcardDecodeFailed` | `crates/vb_storage/src/error/mod.rs` | 126-128 |
| `JournalError::HeaderChecksumMismatch` | `crates/vb_storage/src/error/mod.rs` | 118-119 |
| `JournalError::BadMagic` | `crates/vb_storage/src/error/mod.rs` | 70-74 |
| `JournalError::InvalidEvent` | `crates/vb_storage/src/error/mod.rs` | 129-131 |

### CLI dispatch (doctor command)

| Symbol | Path | Lines |
|---|---|---|
| `cmd_doctor` | `crates/vb_cli/src/app_impl.rs` | active dispatch (mutating, NOT reused for read-only) |
| `parse_doctor` | `crates/vb_cli/src/args.rs` | doctor arg parsing |
| `Command` / `ActionRegistryMode` | `crates/vb_cli/src/args.rs` | CLI command enum |

## Obligation-to-Rust Bridge

| Proof ID | Claim | Behavior Affecting | Rust Source Refs | Behavior Test Refs | Refinement Harness Refs | Verifier | Evidence Command | Rerun From |

### Proptest (6 of 6: PASS — materialized)

All 6 proptest properties live in `crates/workspace_tests/tests/restate_doctor_storage_scan_decode_tests.rs` and call production `vb_storage` codec functions directly.

| Obligation ID | Behavior test | Production binding | Status |
|---|---|---|---|
| PO-vb-t6hx-R02 | `proptest_doctor_scan_rows_never_exceed_limit` (line 36) | `decode_record_header` | PASS |
| PO-vb-t6hx-R05 | `proptest_invalid_hex_rejected_before_storage_open` (line 66) | `decode_record_header` | PASS |
| PO-vb-t6hx-R08 | `proptest_envelope_decode_errors_before_postcard` (line 105) | `decode_journal_event` | PASS |
| PO-vb-t6hx-R12 | `proptest_large_value_preview_truncated_with_hint` (line 145) | `decode_record_header` | PASS |
| PO-vb-t6hx-R15 | `proptest_projection_scan_skips_malformed_decode` (line 181) | `decode_record_header` + `decode_journal_event` | PASS |
| PO-vb-t6hx-R18 | `proptest_doctor_storage_readonly_inventory_unchanged` (line 229) | `decode_journal_event` | PASS |

Evidence command: `cargo test -p velvet-ballistics-workspace-tests --test restate_doctor_storage_scan_decode_tests -- --nocapture`
Result: 6 passed, EXIT: 0, 0.02s.

**Non-vacuity**: Every test calls at least one production function. No property is a self-proving tautology. Each has concrete assertions: output-count bounds, error-variant matching, payload bound enforcement, header-vs-full-decode discrimination, and determinism.

### Fuzz (6 of 6: PASS — materialized)

All 6 fuzz targets live in `fuzz/fuzz_targets/vb_t6hx_*.rs` and call production `vb_storage` codec functions via `libfuzzer_sys::fuzz_target!`.

| Obligation ID | Target | Production binding | Status | Iterations |
|---|---|---|---|---|
| PO-vb-t6hx-R03 | `vb_t6hx_doctor_scan_args` | `decode_record_header` | PASS | ~10.3M |
| PO-vb-t6hx-R06 | `vb_t6hx_doctor_get_args` | `decode_record_header` + `decode_journal_event` | PASS | ~7.8M |
| PO-vb-t6hx-R09 | `vb_t6hx_envelope_decode` | `decode_journal_event` | PASS | ~8.8M |
| PO-vb-t6hx-R10 | `vb_t6hx_doctor_decode_cli` | `decode_journal_event` | PASS | ~8.4M |
| PO-vb-t6hx-R13 | `vb_t6hx_bounded_preview` | `decode_record_header` | PASS | ~7.7M |
| PO-vb-t6hx-R16 | `vb_t6hx_projection_skip_decode` | `decode_record_header` + `decode_journal_event` | PASS | ~7.3M |

Evidence command (per target):
```
cargo +nightly fuzz run --sanitizer none --target x86_64-unknown-linux-gnu "$t" -- -max_total_time=3
```
Result: ~50M total iterations, 0 crashes, EXIT: 0 for all.

**Limitation**: FUZZ_SANITIZER_BLOCKER — evidence uses GNU/no-sanitizer mode. Planned musl+ASAN lane blocked by static libc incompatibility.

### Kani (1 COMPILE_PASS + 5 BLOCKED — planned/trust-boundary)

| Obligation ID | File | Status |
|---|---|---|
| PO-vb-t6hx-R07 | `crates/vb_storage/src/kani_postcard_envelope_wire.rs` | COMPILE_PASS, VERIFY_BLOCKED (crc32c InlineAsm) |
| PO-vb-t6hx-R01 | `crates/vb_cli/src/kani_vb_t6hx_scan_limit.rs` | BLOCKED (module tree, cfg(kani) errors, pure API absent) |
| PO-vb-t6hx-R04 | `crates/vb_cli/src/kani_vb_t6hx_hex_key.rs` | BLOCKED |
| PO-vb-t6hx-R11 | `crates/vb_cli/src/kani_vb_t6hx_bounded_preview.rs` | BLOCKED |
| PO-vb-t6hx-R14 | `crates/vb_cli/src/kani_vb_t6hx_skip_decode.rs` | BLOCKED |
| PO-vb-t6hx-R17 | `crates/vb_cli/src/kani_vb_t6hx_readonly_doctor.rs` | BLOCKED |

**R07 details**: Harness strengthened with `kani::cover!()` macros and property assertions. Uses `kani::any()` generators (GOD RULES compliant). `cargo kani --only-codegen -p vb_storage` → EXIT: 0 (all 30 harnesses compile). Verification blocked by `TerminatorKind::InlineAsm` in crc32c.

**R01/R04/R11/R14/R17 blockers**: Three independent reasons:
1. `CLI_KANI_MODULE_BLOCKER` — harnesses not declared in `vb_cli/src/lib.rs` module tree; vb_runtime has 49+ cfg(kani) type errors
2. `KANI_INLINE_ASM_BLOCKER` — crc32c blocks all harnesses
3. `CLI_NO_PURE_API` — scan/envelope/hex/preview/skip/read-only behaviors are in FjallJournal I/O, TTY format, IPC orchestration; no extractable pure function exists

## Trust Boundary Mapping

All blocker codes from proof-review.md map to trusted-base entries:

| Blocker | Affected obligations | Rust impact |
|---|---|---|
| KANI_INLINE_ASM_BLOCKER | R01,R04,R07,R11,R14,R17 | Any codec path (including `decode_record_header`, `decode_journal_event`) traverses crc32c |
| CLI_KANI_MODULE_BLOCKER | R01,R04,R11,R14,R17 | `vb_cli` module tree + vb_runtime cfg(kani) type errors |
| CLI_NO_PURE_API | R01,R04,R11,R14,R17 | FjallJournal (I/O), TTY, IPC layers — no pure Rust function extractable |
| FUZZ_SANITIZER_BLOCKER | R03,R06,R09,R10,R13,R16 | musl+ASAN blocked; GNU/no-sanitizer evidence provided |

## Unresolved Bridge Gaps

1. **R01,R04,R11,R14,R17 (5 CLI Kani harnesses)**: These obligations model CLI behavior (scan limit, hex validation, bounded preview, skip-decode orchestration, read-only mode). The mathematical models in `crates/vb_cli/src/kani_vb_t6hx_*.rs` cannot currently be compiled or verified. The implementation-bound bridge for these is `mapping_status: planned` — they require:
   - Module tree declaration in `vb_cli/src/lib.rs`
   - Resolution of vb_runtime cfg(kani) type errors
   - Kani InlineAsm support or crc32c stub
   - Extractable pure functions for the modeled properties
   - These are implementation-side tasks tracked for State 12 closure.

2. **R07 (vb_storage Kani)**: Harness is strengthened and compiles. Only crc32c InlineAsm blocks verification. Bridge is `mapping_status: planned` — closure requires Kani upgrade supporting InlineAsm.

3. **R03,R06,R09,R10,R13,R16 (6 fuzz targets)**: Fuzz evidence uses GNU/no-sanitizer mode. Full musl+ASAN lane is a future optimization. Bridge accepts bounded-confidence evidence; full sanitizer campaign tracked as future improvement.

## Behavior Test Independence

All behavior test obligations (R02, R05, R08, R12, R15, R18) execute independently of verifier harnesses. No verifier harness is reused as a behavior test. The proptest properties in `restate_doctor_storage_scan_decode_tests.rs` are the primary evidence channel.

## Refinement Harness Status

| Obligation group | Refinement harness | Status |
|---|---|---|
| Proptest (R02,R05,R08,R12,R15,R18) | `restate_doctor_storage_scan_decode_tests.rs` | materialized (PASS) |
| Fuzz (R03,R06,R09,R10,R13,R16) | `fuzz/fuzz_targets/vb_t6hx_*.rs` | materialized (PASS, no-sanitizer) |
| Kani R07 | `kani_postcard_envelope_wire.rs` | planned (COMPILE_PASS, verify blocked) |
| Kani R01,R04,R11,R14,R17 | `crates/vb_cli/src/kani_vb_t6hx_*.rs` | planned (not yet compilable) |

## State 12 Closure Obligations

These must be materialized or verified before State 12:

1. Kani R07: produce Kani VERIFY:PASS evidence (requires Kani InlineAsm support or crc32c stub)
2. Kani R01,R04,R11,R14,R17: either achieve Kani PASS or obtain explicit reviewer-accepted waiver for each
3. Fuzz: optional musl+ASAN lane (tracked as improvement, not blocking)
4. Trust boundaries: `KANI_INLINE_ASM_BLOCKER`, `CLI_KANI_MODULE_BLOCKER`, `CLI_NO_PURE_API` must be dispositioned in trusted-base-ledger.jsonl

## Handoff

This bridge maps 18 approved proof obligations to production Rust source, behavior tests, refinement harnesses, and evidence commands. Six (6) Kani obligations remain at `ACCETED_TRUST_BOUNDARY` with documented blockers and explicit closure paths. Twelve (12) obligations (proptest + fuzz) have materialized PASS evidence with production bindings.

Reviewer handoff inputs:
- This file: `.beads/vb-t6hx/proof-to-rust-map.md`
- Machine-readable bridge: `.beads/vb-t6hx/rust-refinement-obligations.jsonl`
- Ledger append: `proof-to-implementation-vb-t6hx-state7-001` in `verification-ledger.jsonl`
