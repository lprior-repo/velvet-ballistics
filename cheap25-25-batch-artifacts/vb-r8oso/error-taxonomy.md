# Error Taxonomy — vb-r8oso

**bead_id:** vb-r8oso
**owner_stage:** rust-contract
**upstream_artifacts:** `domain-model.md`, `type-contracts.md`, `workflow-model.md`

This artifact fixes the existing and new error variants, their diagnostic
codes, their symbolic codes, the dispatch rules that decide which variant
fires at which call site, and the legal "collision surface" between
read-time and write-time sequence mismatches.

---

## 1. Existing Sequence-Related Variants (Unchanged, Documented for Context)

| Variant | Diagnostic | Symbolic | Call site | Source |
|---|---|---|---|---|
| `JournalError::SequenceGap { expected, actual }` | `0x4009` | `JOURNAL_SEQUENCE_GAP` | Read-time: `events_for_run` validates the contiguity of the durable log for a run. | `crates/vb_storage/src/error/mod.rs:48-52`, `codec/mod.rs:170-178`. |
| `JournalError::SequenceOverflow` | `0x400A` | `SEQUENCE_OVERFLOW` | `codec::next_seq` returns this when `checked_add` overflows on `EventSeq::MAX`. | `crates/vb_storage/src/codec/mod.rs:153-158`. |
| `JournalError::ReplayKeyMismatch { run, key_seq, payload_seq }` | `0x4040` | `REPLAY_KEY_PAYLOAD_MISMATCH` | Replay-time: Fjall key's seq disagrees with the decoded event's seq. | `crates/vb_storage/src/error/mod.rs:53-60`. |
| `JournalError::ReplayEnvelopeSequenceMismatch { run, envelope_seq, payload_seq }` | `0x4041` | `REPLAY_ENVELOPE_SEQUENCE_MISMATCH` | Replay-time: envelope's seq field disagrees with the decoded event's seq. | `crates/vb_storage/src/error/mod.rs:61-68`. |

All four continue to exist with their existing diagnostic codes. The
read/replay chain (`SequenceGap`, `ReplayKeyMismatch`,
`ReplayEnvelopeSequenceMismatch`) is fundamental and unchanged.

## 2. New Variant (Subject of This Bead)

### 2.1 `JournalError::SequenceMismatch { run, expected, actual }`

| Attribute | Value |
|---|---|
| Diagnostic code | `0x4042` |
| Symbolic code | `"JOURNAL_SEQUENCE_MISMATCH_AT_WRITE"` |
| `Display` format | `"journal append sequence mismatch for run {run:?}: expected {expected:?}, actual {actual:?}"` |
| Call site | Write-time, in any of the five append paths listed in §3 of `type-contracts.md`. |
| Companion in `diagnostic_code()` | `Self::SequenceMismatch { .. } => Self::SEQUENCE_MISMATCH_AT_WRITE_CODE` |
| Companion in `symbolic_code()` | `Self::SequenceMismatch { .. } => "JOURNAL_SEQUENCE_MISMATCH_AT_WRITE"` |
| Companion in `codes.rs` | `pub const SEQUENCE_MISMATCH_AT_WRITE_CODE: DiagnosticCode = DiagnosticCode::new(0x4042);` |

### 2.2 Field Semantics

```rust
JournalError::SequenceMismatch {
    run: RunId,           // event.run_id() of the rejected call
    expected: EventSeq,   // == next_sequence_at_write(run) at write time
    actual: EventSeq,     // == event.seq() of the rejected call
}
```

Constructor pre-condition (must be enforced before constructing the variant):

- `expected != actual` — invariant tested by `error_tests::sequence_mismatch_constructor_fields`.

### 2.3 Why a New Variant, Not Overloading `SequenceGap`

`SequenceGap` and `SequenceMismatch` look superficially identical
(`{expected, actual}`). They are deliberately distinct:

| Axis | `SequenceGap` | `SequenceMismatch` (NEW) |
|---|---|---|
| **Call site** | Read (replay / `events_for_run`). | Write (any append path). |
| **Diagnostic code** | `0x4009` | `0x4042` |
| **Why distinct?** | The on-disk tail has a hole that was either pre-existing or created in a previous run with a buggy build. | The current caller's `seq` value mismatches the expected next seq. |
| **Action** | Recovery: surface corruption, halt run or attempt repair. | Caller fix: the caller's seq source is buggy. |
| **Companion diagnostic** | `ReplayKeyMismatch`/`ReplayEnvelopeSequenceMismatch` for related decode-side mismatches. | None — the **only** write-time sequence variant. |

Both errors MUST coexist. Renaming or merging violates downstream code that
switches on the variant.

### 2.4 Why `0x4042` and Not Adjacent `0x4009`

`0x400x`/`0x401x`/`0x402x` blocks are exhausted for the storage-error
diagnostic codes. The `0x404x` block was reserved for replay-time
deviations from the keyspace or envelope (`0x4040`, `0x4041`). The next
free slot is `0x4042`. A new code keeps the existing
sequence/replay hierarchy distinct: replay/wire mismatches (`0x4040`/`0x4041`)
remain visibly different from write-time mismatches (`0x4042`).

## 3. Diagnostic Code Registry (Authoritative)

| Code | Variant | Direction |
|---|---|---|
| `0x4009` | `SequenceGap` | Read |
| `0x400A` | `SequenceOverflow` | Helper (succ overflow) |
| `0x4040` | `ReplayKeyMismatch` | Read |
| `0x4041` | `ReplayEnvelopeSequenceMismatch` | Read |
| **`0x4042`** | **`SequenceMismatch`** | **Write (NEW)** |

No other code collision is permitted.

## 4. Symbolic Code Registry

`SymbolicCode::from_static(s)` resolves a registered name. The
implementation guidance (matching the historic pattern in `codes.rs`
where strings like `"FJALL_ERROR"` and `"KEY_CAPACITY_EXCEEDED"` are
**not** registered and fall through to `INTERNAL_INVARIANT`) is:

- Register `"JOURNAL_SEQUENCE_MISMATCH_AT_WRITE"` in `CODE_REGISTRY` (preferred).
- If registration is not feasible for v1, the existing fallback to `SymbolicCode::INTERNAL_INVARIANT` is acceptable and matches the historic convention for `0x40xx` codes.

Either outcome is a passing surface; the variant's diagnostic-code arm
(`0x4042`) is mandatory regardless of registration status.

## 5. Error Wiring Checklist (Holzman-Rust Stage)

Files modified to add the variant (no surface beyond what the
deliberately-updated tests assert):

- `crates/vb_storage/src/error/mod.rs` — variant declaration.
- `crates/vb_storage/src/error/codes.rs` — constant + match arms in `diagnostic_code()` and `symbolic_code()`.
- `crates/vb_storage/src/tests.rs` — updated assertions at lines 1737 and 4612.
- `crates/vb_storage/src/error_tests.rs` — display + construction assertions.
- `crates/vb_storage/src/error_code_tests.rs` — diagnostic-code round-trip + symbolic-code resolution assertion.
- `crates/vb_storage/tests/proptest_journal_error_codes.rs` — exhaustiveness arm for the new variant.
- `fuzz/src/journal_target/errors.rs`, `fuzz/fuzz_targets/journal_decode.rs`, `fuzz/fuzz_targets/decode_record.rs`, `fuzz/tests/proptest_journal_error_exhaustiveness.rs` — fuzz-arm update (downstream `proof-writer`/test stages).
- Cross-crate exhaustiveness registrations in `crates/workspace_tests/tests/proptest_error_types_registration.rs` and `proptest_error_types_nonzero_codes.rs`.

## 6. Display and Debug Output

- `Display` format MUST mention `expected` and `actual` so operator logs are immediately useful.
- `Debug` is auto-derived and includes `run`, `expected`, `actual`.

## 7. Legal-Collision Notes

A caller may legitimately receive BOTH `SequenceGap` and `SequenceMismatch`
across the lifetime of a run if a buggy build pre-existed and then
upgrades; the new guard does not rewrite history. Operators must
diagnose upgrades with both variants in mind.

The variant `SequenceMismatch` MUST NOT be reachable from a *read* path.
If a reviewer observes `SequenceMismatch` emitted from `events_for_run`
or any replay, the contract is broken and the implementation is wrong.

The variant `SequenceGap` MUST NOT be reachable from a *write* path under
the fix. If a reviewer observes `SequenceGap` from `append_*`, the fix is
incomplete (the guard was inserted in a wrong location or with the wrong
sequencing).

## 8. Tests Covering the Taxonomy (Seeds)

- `error_tests::sequence_mismatch_display` — `Display` mentions `expected` and `actual`.
- `error_tests::sequence_mismatch_constructor_fields` — variant carries the three fields exactly.
- `error_code_tests::sequence_mismatch_at_write_code` — `.diagnostic_code() == 0x4042`.
- `error_code_tests::sequence_mismatch_at_write_symbolic_registered_or_fallback` — `.symbolic_code()` is `JOURNAL_SEQUENCE_MISMATCH_AT_WRITE` when registered, else `INTERNAL_INVARIANT`.
- `proptest_journal_error_codes::journal_error_codes_*` — exhaustiveness arm covers the new variant.
- Tests in `crates/vb_storage/src/tests.rs:1737` and `:4612` updated to assert the new variant (and to NOT assert `SequenceGap` on the append path).
