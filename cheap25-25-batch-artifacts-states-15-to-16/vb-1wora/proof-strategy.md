# Proof Strategy — vb-1wora: Codec rejects trailing bytes after declared record payload

**Bead:** `vb-1wora` — Codec: reject trailing bytes after declared record payload (P1 bug)
**Workspace:** `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-1wora`
**State:** 4 (proof-planner)
**Skill:** `proof-planner`
**Captured:** 2026-07-01 (fwd-port from `cheap25-vb-1wora` workspace)
**Sources of truth:** `STATE.md`, `codebase-map.md`, `delivery-scope.jsonl`, `contracts/contract.md`, `contracts/proof-seeds.jsonl`

This bead closes a P1 silent-acceptance bug in the v1 storage record codec. The fix is local — one new `JournalError::TrailingBytes { trailing: usize }` variant, one new diagnostic constant `TRAILING_BYTES_CODE = 0x4042`, and one cheap shape check inserted into two decoders (`decode_record_payload` and its mirror `decode_envelope_only`) — but it must be locked by every applicable verifier lane because the surface is fail-closed hardening of a public, hot-path decoder reached by `has_terminal_event` at `crates/vb_storage/src/trimming/logic.rs:251`. The cheapest fix is to insert the check; the strongest proof is to lock the new shape invariant across all four observation regimes (structural, unit, bounded-model, property).

---

## 1. Scope Summary

| Item | Value |
|---|---|
| Production functions modified | `decode_record_payload` (`crates/vb_storage/src/codec/payload.rs:56-82`), `decode_envelope_only` (`crates/vb_storage/src/codec/envelope.rs:48-83`) |
| Production enum extended | `JournalError::TrailingBytes { trailing: usize }` (`crates/vb_storage/src/error/mod.rs:~97`, between `UnexpectedEof` and `MalformedKeyspaceRow`) |
| Diagnostic constant added | `JournalError::TRAILING_BYTES_CODE = DiagnosticCode::new(0x4042)` (`crates/vb_storage/src/error/codes.rs:~50`) |
| Wiring | `diagnostic_code()` arm + `symbolic_code()` arm in `crates/vb_storage/src/error/codes.rs:99-176` / `180-268`; symbolic string `"JOURNAL_TRAILING_BYTES"` (registration in `CODE_REGISTRY` is recommended-only) |
| Test rewrites | `decode_ignores_trailing_bytes_beyond_payload` → `decode_rejects_trailing_bytes_after_payload` (`crates/vb_storage/src/codec/tests.rs:1498-1524`); add `decode_envelope_only_rejects_trailing_payload` (`crates/vb_storage/src/codec/envelope.rs:153-170`); add error-variant trio + diagnostic-code test |
| Verus mirror extended | `SpecJournalError::TrailingBytes { trailing: u32 }` (`verification/verus/production_inner/vb_vzcuf_PS_003_production.rs:335-413`); bridge `ensures` arm added (`verification/verus/vb-vzcuf-PS-003.rs:387-451`) |
| Forbidden behavior | `decode_record_payload` must NOT silently accept `bytes.len() > payload_end` (the pre-fix P1 bug); the encoder is NOT touched |

This is a **decode-pipeline hardening**: no unsafe, no concurrency, no state machine, no heap predicate. The proof budget is therefore concentrated in four cheap-verifier lanes (`rust-local` structural review, `cargo test` unit, `kani` bounded-model, `proptest` property) plus one production-binding `verus` bridge and one additive `cargo-fuzz` hostile-input oracle. Lane choice is dictated by the cheapest sufficient verifier for each contract clause (see §2).

## 2. Verifier Selection (Lane Decisions)

| Lane | Required? | Obligations | Justification |
|---|---|---|---|
| **rust-local** (structural review) | **required** | PO-001 | `INV-CODEC-TB-003` (decoder ordering: trailing check must precede `verify_digest_match`) and `INV-CODEC-TB-006` (diagnostic-code wiring parity) are mechanical review checks that no verifier can express. Diff-vs-pre-fix + call-site inspection is the strongest evidence. |
| **cargo test** (`cargo test -p vb_storage`) | **required** | PO-002, PO-003 | `INV-CODEC-TB-001` and `INV-CODEC-TB-002` need a direct test inversion (the pre-fix test `decode_ignores_trailing_bytes_beyond_payload` documented the bug; the new test `decode_rejects_trailing_bytes_after_payload` locks the regression). The error-variant trio (`variant_and_fields`, `display_format`, `error_code`) locks `INV-CODEC-TB-005` and the `0x4042` numeric code. |
| **proptest** (property test, roundtrip) | **required** | PO-003, PO-005 | `INV-CODEC-TB-002` and `INV-CODEC-TB-010` (round-trip preservation) are universal claims: any fresh encoded record round-trips with `Ok`, never `TrailingBytes`. Property-based random byte appends give the strongest cheap coverage for the "no false-positive trailing" side of the invariant, complementing the directed unit tests. |
| **kani** (postcard envelope wire) | **required** | PO-004 | `INV-CODEC-TB-001` + `INV-CODEC-TB-003` + `INV-CODEC-TB-005` need bounded model checking over the trailing-bytes path. The existing `crates/vb_storage/src/kani_postcard_envelope_wire.rs` H5 harness already exhaustively covers digest-before-postcard; H6 (this bead) extends it to "trailing-before-digest" by stubbing `verify_digest_match` as a counted call and asserting the count is zero for any input with `bytes.len() > payload_end`. Kani's symbolic execution is the only verifier that can express "step ordering" mechanically rather than as code review. |
| **verus** (PS-003 bridge) | **required** | PO-006 | `INV-CODEC-TB-007` (bridge enumerates new `Err(SpecJournalError::TrailingBytes {..})` arm) is mandatory because adding a new production variant without the matching bridge arm breaks the production-binding gate (`scripts/check-verus-production-binding.sh`) and the drift gate (`scripts/check-production-inner-drift.sh`). This is a **WEAK_MIRROR** binding via the existing `production_inner/vb_vzcuf_PS_003_production.rs` mirror + drift gate; it is the only way to satisfy GOD RULE 2 for this variant given that `decode_record_payload` reaches `postcard` / `blake3` / `crc32c` (non-modelable in single-file Verus). |
| **cargo-fuzz** (hostile-input oracle) | **required** | PO-007 | `HOSTILE-INPUT-001` and `INV-CODEC-TB-001` from a fuzzer's perspective. The existing `fuzz/fuzz_targets/fuzz_storage_codec_payload_corruption.rs` already feeds random bytes; the additive oracle is "build a valid record + append `N` (0..=8) random trailing bytes → assert `Err(TrailingBytes { trailing: N })`". This is the cheapest way to prove the fail-closed semantics on attacker-shaped input. |

### 2.1 Verifiers Explicitly Not Used

| Verifier | Reason (concrete evidence required) |
|---|---|
| **Loom** | The codec is single-threaded and synchronous; no concurrent memory ordering, no lock-free structures, no channel interleavings. `crates/vb_storage/src/codec/` modules are pure parsers over `&[u8]`. No loom model would expose new bugs beyond what Kani already proves about byte-level inputs. **Non-applicable evidence**: `#![forbid(unsafe_code)]` in `vb_storage` crate header; no `Send`/`Sync` shared-state across the decode boundary; `decode_record_payload` has no `Arc`/`Mutex`/atomic in its body or its callees (`verify_digest_match`, `decode_record_header`). |
| **Miri** | `vb_storage` is `#![forbid(unsafe_code)]`. The new check is a pure `usize` compare + subtraction with no raw pointers, no `MaybeUninit`, no aliasing. Miri would find nothing. **Non-applicable evidence**: workspace `#![forbid(unsafe_code)]` invariant; the new code is `if bytes.len() > payload_end { return Err(JournalError::TrailingBytes { trailing: bytes.len() - payload_end }); }`. |
| **Flux RS** | The fix introduces no new refinement type, no indexed type, no constraint refinement. The invariant `trailing > 0` is enforced by the producer site's `if bytes.len() > payload_end` (which mathematically implies `bytes.len() - payload_end > 0`), not by a refinement annotation. The error variant is a plain enum with a `usize` field; Flux would not add coverage beyond what Kani and proptest already provide. **Non-applicable evidence**: no refinement types are introduced; `trailing: usize` is not an indexed type — its `> 0` invariant is structural (not type-level). |

## 3. Risk Classification (Pre-Lane-Selection)

Risks are classified first; lanes are chosen to address each risk with the cheapest sufficient verifier.

| Risk tag | Risk description | Lane response |
|---|---|---|
| **p1-bug / fail-closed / shape-defect / decode-order** | Decoder must fail closed when `bytes.len() > payload_end`. The new check must run before the BLAKE3 digest op (cheap-before-expensive). | Kani (H6 in `kani_postcard_envelope_wire.rs`) proves the step ordering; cargo test (PO-002) proves the variant is produced; proptest (PO-005) proves the round-trip side; verus (PO-006) binds the bridge; rust-local (PO-001) reviews the position. |
| **verus-spec-drift / god-rule-2-vacuum / production-binding** | Adding the new variant without updating the Verus mirror and bridge breaks `scripts/check-verus-production-binding.sh`. The drift gate `scripts/check-production-inner-drift.sh` would also fail. | Verus bridge obligation PO-006 declares WEAK_MIRROR binding to the existing `production_inner/vb_vzcuf_PS_003_production.rs` mirror; drift gate + production-binding gate are the trusted-base enforcement. |
| **diagnostic-code-coverage / symbolic-code-registration** | `0x4042` must be unique in the `0x40xx` journal range; symbolic name `JOURNAL_TRAILING_BYTES` should resolve to `TRAILING_BYTES_CODE` (or fall back to `INTERNAL_INVARIANT` if not registered). | Cargo test `trailing_bytes_error_has_correct_code` (PO-002) locks `0x4042`. CODE_REGISTRY registration is **recommended but optional**; the contract treats it as non-blocking. |
| **hostile-input / fuzz-coverage** | An attacker who can append bytes to a stored record should not be able to confuse the decoder. | Cargo-fuzz PO-007. |
| **round-trip / regression-prevention** | Encoder unchanged; round-trip must continue to work. | Proptest PO-005 + existing round-trip suite. |
| **mirror-consistency / dead-code-but-surfaced** | `decode_envelope_only` is `pub(crate)` and `#[allow(dead_code, …)]`. Mirror check keeps the two decoders semantically aligned. | Cargo test `decode_envelope_only_rejects_trailing_payload` (PO-002) + proptest (PO-005) mirror coverage. |
| **variant-exclusivity / railway-correctness** | `TrailingBytes` must be mutually exclusive with `UnexpectedEof` (one fires on `bytes.len() < payload_end`, the other on `bytes.len() > payload_end`). | Proptest PO-005 + cargo test PO-002 + Kani PO-004. |

## 4. Proof Strategy Detail (per obligation)

### PO-001 — rust-local / structural review (decoder ordering + diagnostic wiring)

- **Closes invariants:** `INV-CODEC-TB-003` (trailing check precedes `verify_digest_match`); `INV-CODEC-TB-006` (diagnostic code wiring parity).
- **Artifact target:** `crates/vb_storage/src/codec/payload.rs:56-82` (post-fix diff), `crates/vb_storage/src/codec/envelope.rs:48-83` (post-fix diff), `crates/vb_storage/src/error/codes.rs:99-176` and `180-268` (post-fix diff).
- **Mode:** `structural-review` — diff inspection, no verifier invocation.
- **Expected evidence:** pre-fix diff at line 60 of `payload.rs` reads `verify_digest_match(payload, header.payload_digest)?;` immediately after `bytes.get(...).ok_or(UnexpectedEof)?`; post-fix diff inserts an `if bytes.len() > payload_end { return Err(TrailingBytes { trailing: bytes.len() - payload_end }); }` between those two lines. Diagnostic-code match in `codes.rs:99-176` includes `Self::TrailingBytes { .. } => Self::TRAILING_BYTES_CODE,`. Symbolic-code match in `codes.rs:180-268` includes `Self::TrailingBytes { .. } => "JOURNAL_TRAILING_BYTES",`.

### PO-002 — cargo test (variant + display + code trio; test inversion; mirror test)

- **Closes invariants:** `INV-CODEC-TB-001`, `INV-CODEC-TB-005` (direct test of the `0xFF 0xFE 0xFD` 3-byte fixture in `codec/tests.rs:1498-1524`); mirror consistency via `decode_envelope_only_rejects_trailing_payload`.
- **Artifact target:** `crates/vb_storage/src/codec/tests.rs:1498-1524` (renamed `decode_rejects_trailing_bytes_after_payload`); `crates/vb_storage/src/codec/envelope.rs:153-170` (new `decode_envelope_only_rejects_trailing_payload`); `crates/vb_storage/src/error_tests.rs:454-557` (variant trio); `crates/vb_storage/src/error_code_tests.rs:144-160` (diagnostic-code test).
- **Mode:** `cargo test -p vb_storage --lib trailing_bytes_decode_rejects_trailing_bytes_after_payload trailing_bytes_decode_envelope_only_rejects_trailing_payload trailing_bytes_variant_and_fields trailing_bytes_display_format trailing_bytes_error_code trailing_bytes_error_has_correct_code`.
- **Expected evidence:** all six test functions exit with status 0. The pre-fix test name `decode_ignores_trailing_bytes_beyond_payload` no longer exists.

### PO-003 — cargo test + proptest (round-trip preservation; exact-fit Ok; mutual exclusion)

- **Closes invariants:** `INV-CODEC-TB-002` (Ok only if `bytes.len() == payload_end`); `INV-CODEC-TB-009` (TrailingBytes / UnexpectedEof mutual exclusion); `INV-CODEC-TB-010` (round-trip unchanged).
- **Artifact target:** `crates/vb_storage/src/codec/tests.rs` (existing `roundtrip_*` test set, post-fix should all pass); new proptest function `proptest_trailing_bytes_roundtrip_unchanged` lives in `crates/vb_storage/src/codec/tests.rs` under a `#[cfg(test)] mod proptests` block.
- **Mode:** `cargo test -p vb_storage --lib roundtrip` (existing tests) and a new proptest property.
- **Expected evidence:** existing round-trip tests pass; the new proptest generates 1024 random `JournalEvent` values, encodes them, decodes them, and asserts `Ok((env, payload))` with `payload.len() == header.payload_len` — never `TrailingBytes`. The proptest also feeds 1024 random `bytes.len() < payload_end` fixtures and asserts `Err(UnexpectedEof)` — never `Err(TrailingBytes)`. Together this locks the mutual exclusion.

### PO-004 — Kani (H6 extends `kani_postcard_envelope_wire.rs`)

- **Closes invariants:** `INV-CODEC-TB-001` (TrailingBytes iff `bytes.len() > payload_end`); `INV-CODEC-TB-003` (trailing check precedes digest); `INV-CODEC-TB-005` (TrailingBytes only when `trailing > 0`).
- **Artifact target:** `crates/vb_storage/src/kani_postcard_envelope_wire.rs` — add `kani_harness_rejects_trailing_bytes` after H5 (line 337). Stubs `verify_digest_match` as a counted `static mut DIGEST_CALL_COUNT: u32 = 0;` and asserts the count is 0 after `decode_record_payload` returns on any input where `bytes.len() > payload_end`.
- **Mode:** `cargo kani -p vb_storage --harness kani_harness_rejects_trailing_bytes --output-format=json` (mirrors the H5 invocation profile per `crates/vb_storage/src/kani_postcard_envelope_wire.rs:1-11` doc).
- **Expected evidence:** Kani reports all assertions provable for any valid header + arbitrary `payload_len ∈ [0, MAX_JOURNAL_EVENT_PAYLOAD_BYTES]` + arbitrary `trailing ∈ [1, 8]` bytes appended; the result is `Err(JournalError::TrailingBytes { trailing: N })` with `DIGEST_CALL_COUNT == 0`.

### PO-005 — proptest (random byte-append oracle)

- **Closes invariants:** `INV-CODEC-TB-002`, `INV-CODEC-TB-004` (decode_envelope_only mirror consistency), `INV-CODEC-TB-009` (mutual exclusion).
- **Artifact target:** `crates/vb_storage/src/codec/tests.rs` — new proptest functions `proptest_decode_record_payload_rejects_random_trailing`, `proptest_decode_envelope_only_rejects_random_trailing`, `proptest_decode_record_payload_mutual_exclusion_with_unexpected_eof`.
- **Mode:** `cargo test -p vb_storage --features proptest --lib proptest_trailing_bytes`.
- **Expected evidence:** proptest generates 1024 inputs where a valid header is followed by `bytes.len() - payload_end ∈ [1, 32]` random trailing bytes; for both `decode_record_payload` and `decode_envelope_only`, the result is `Err(JournalError::TrailingBytes { trailing: N })` with `N == bytes.len() - payload_end`. Mutual-exclusion proptest feeds 1024 inputs with `bytes.len() < payload_end` and asserts `Err(UnexpectedEof)` — never `Err(TrailingBytes)`.

### PO-006 — Verus PS-003 bridge (WEAK_MIRROR production-binding)

- **Closes invariants:** `INV-CODEC-TB-007` (bridge enumerates `Err(SpecJournalError::TrailingBytes { trailing: u32 })`); partial coverage of `INV-CODEC-TB-001` (via the `bytes.len() as u32 > expected_payload_end && trailing == (bytes.len() as u32) - expected_payload_end && trailing > 0` precondition).
- **Artifact target:** `verification/verus/vb-vzcuf-PS-003.rs:387-451` (add bridge `ensures` arm); `verification/verus/production_inner/vb_vzcuf_PS_003_production.rs:335-413` (add `SpecJournalError::TrailingBytes { trailing: u32 }` variant + update enumeration comment at lines 280-327); `verification/verus/extern_vb_vzcuf_PS_003.rs` (no change; re-exports pick up the new variant automatically).
- **Mode:** `bash scripts/verify-verus.sh` (registry-driven obligations per AGENTS.md doctrine). The WEAK_MIRROR binding relies on `scripts/check-production-inner-drift.sh` (mirror vs production parity) and `scripts/check-verus-production-binding.sh` (bridge arm enumeration parity).
- **Production binding:** `WEAK_MIRROR` to `verification/verus/production_inner/vb_vzcuf_PS_003_production.rs` with drift gate `scripts/check-production-inner-drift.sh` (zero-drift tolerance). The exec wrapper at `vb-vzcuf-PS-003.rs:480-…` (the spec exec wrapper, present in current file) binds the production `decode_record` to the bridge via `assume_specification[ production::decode_record ]( ... )`.
- **Expected evidence:** `bash scripts/verify-verus.sh` exits 0 with `SpecJournalError::TrailingBytes { trailing: u32 }` enumerated in the mirror and the new `Err(SpecJournalError::TrailingBytes { trailing }) => { ... }` arm present in the `decode_record` bridge `ensures`. The drift gate `bash scripts/check-production-inner-drift.sh` exits 0; the production-binding gate `bash scripts/check-verus-production-binding.sh` exits 0.

### PO-007 — cargo-fuzz (hostile-input oracle)

- **Closes invariants:** `INV-CODEC-TB-001` from a hostile-input perspective; partial coverage of `INV-CODEC-TB-005` (trailing > 0).
- **Artifact target:** `fuzz/fuzz_targets/fuzz_storage_codec_payload_corruption.rs` — extend with a new `fuzz_target_trailing_bytes` function that builds a valid record, appends `N ∈ [0, 8]` random bytes, and asserts `Err(TrailingBytes { trailing: N })` (when `N > 0`) or `Ok` (when `N == 0`).
- **Mode:** `cargo +nightly fuzz run -p vb_storage_fuzz fuzz_target_trailing_bytes -- -max_total_time=60` (60-second wallclock budget per AGENTS.md fuzz discipline).
- **Expected evidence:** fuzzer runs 60 seconds without finding a counterexample (no `Ok` when `N > 0`, no `Err(TrailingBytes { trailing: N })` when `N == 0`, no panic, no `unwrap`/`expect` failure). Crash directory `.fuzz/artifacts/fuzz_target_trailing_bytes/` empty.

## 5. Obligations Coverage Summary

| Lane | # Obligations | Coverage |
|---|---|---|
| rust-local (structural) | 1 (PO-001) | INV-CODEC-TB-003, INV-CODEC-TB-006 |
| cargo test | 2 (PO-002, PO-003) | INV-CODEC-TB-001, INV-CODEC-TB-002, INV-CODEC-TB-005, INV-CODEC-TB-009, INV-CODEC-TB-010, mirror INV-CODEC-TB-004 |
| proptest | 2 (PO-003, PO-005) | INV-CODEC-TB-002, INV-CODEC-TB-004, INV-CODEC-TB-009, INV-CODEC-TB-010 |
| kani (postcard envelope wire) | 1 (PO-004) | INV-CODEC-TB-001, INV-CODEC-TB-003, INV-CODEC-TB-005 |
| verus (PS-003 bridge, WEAK_MIRROR) | 1 (PO-006) | INV-CODEC-TB-007 (+ partial INV-CODEC-TB-001) |
| cargo-fuzz (hostile input) | 1 (PO-007) | INV-CODEC-TB-001 (hostile perspective), INV-CODEC-TB-005 (partial) |
| **Total** | **7** | **all 7 INV-CODEC-TB-* invariants covered** |

This is 7 obligations, within the 5–7 budget specified by the bead.

## 6. Assumptions

| # | Assumption | Impact if false |
|---|---|---|
| A-001 | `verify_digest_match` is the only expensive call reachable from `decode_record_payload` after the slice step. | If another expensive op runs between slice and digest, PO-004's "DIGEST_CALL_COUNT == 0" assertion would falsely pass. Mitigation: Kani harness also asserts `result.is_err()` and `matches!(result, Err(JournalError::TrailingBytes { .. }))`, not just the count. |
| A-002 | The `production_inner/vb_vzcuf_PS_003_production.rs` mirror is regenerated from `crates/vb_storage/src/codec/payload.rs:56-82` and `crates/vb_storage/src/error/mod.rs:~97` within this bead. | Drift gate would fail at landing; reviewer must re-run `scripts/check-production-inner-drift.sh`. |
| A-003 | `kani::any()` can construct a valid 60-byte header with arbitrary `payload_len` and arbitrary 0..=MAX_JOURNAL_EVENT_PAYLOAD_BYTES payload without exhausting Kani's unwinding budget. | If Kani's `#[kani::unwind(N)]` must increase beyond 4, the harness becomes slow. Mitigation: the existing H5 harness already uses `#[kani::unwind(4)]` for the same shape; H6 inherits that. |
| A-004 | `0x4042` is unused by every other branch / WIP bead in the workspace. | Diagnostic-code collision. Mitigation: `codebase-map.md` verified free (highest used is `0x4041`); reviewer should confirm no concurrent bead is using `0x4042`. |
| A-005 | `cargo +nightly fuzz` toolchain is installed. | PO-007 falls back to "blocked_tooling" status; cargo test still proves the same invariant on directed fixtures. |
| A-006 | `decode_record_payload` and `decode_envelope_only` are pure functions over `&[u8]` with no observable side effects beyond the `Result` return. | If either function has side effects (logging, metrics), the proptest and Kani invariants still hold but the production behavior may diverge from the spec. Mitigation: existing tests confirm purity; no logging/metrics in the post-fix diff. |
| A-007 | `cargo test -p vb_storage --features proptest` enables the proptest macros. | If the feature flag is misconfigured, PO-003/PO-005 fail to compile. Mitigation: existing proptest patterns in `vb_storage` already use this feature flag convention. |

## 7. Forbidden Patterns (no obligation may express these)

| Pattern | Why forbidden | Enforced by |
|---|---|---|
| `unwrap()`, `expect()`, `panic!()`, `todo!()`, `unimplemented!()`, `dbg!()` in the post-fix decode path. | Existing AGENTS.md doctrine. | Source lint (`cargo clippy -- -D warnings`) |
| Two `JournalError` variants both reachable on `bytes.len() > payload_end`. | Violates mutual-exclusion invariant `INV-CODEC-TB-009`. | PO-002, PO-003, PO-005 |
| `TrailingBytes { trailing: 0 }`. | Violates `INV-CODEC-TB-005`. | PO-004 (Kani), PO-005 (proptest) |
| Hand-written shadow types without `#[path = "..."]` binding in the Verus mirror. | GOD RULE 2 (vacuum-proof prohibition). | `scripts/check-verus-production-binding.sh` (PO-006) |
| Modifying the encoder to "balance" the new check. | The encoder is correct; modifying it risks round-trip breakage. | PO-003 + PO-005 + code review (PO-001) |
| Numeric codes outside the `0x40xx` journal range for storage-layer errors. | Existing convention. | PO-002 (`trailing_bytes_error_has_correct_code`) |
| Re-using `0x4042` or `"JOURNAL_TRAILING_BYTES"` for a different variant. | Diagnostic-code / symbolic-code uniqueness invariant. | PO-002 + code review |

## 8. Risk Assessment

| Risk | Severity | Mitigation |
|---|---|---|
| Verus mirror drift breaks production-binding gate | HIGH | Mirror + bridge updates are in PO-006; drift gate is the trusted-base enforcement. |
| Kani H6 takes too long to verify | MED | Reuse H5's `#[kani::unwind(4)]` and `cargo kani -p vb_storage --harness ...` invocation pattern; if budget exceeded, fall back to `cargo kani -p vb_storage` (smoke check) plus PO-002 directed tests. |
| Test `decode_ignores_trailing_bytes_beyond_payload` forgotten to rename | MED | PO-002 specifies the rename explicitly; black-hat review will catch a leftover. |
| Cargo-fuzz not installed in CI | LOW | A-005 documents the fallback: PO-007 marked `blocked_tooling`, and PO-002's directed tests still cover the invariant. |
| Round-trip regression from accidental encoder change | MED | PO-003 + PO-005 explicitly assert round-trip preservation; encoder is out of scope per `delivery-scope.jsonl:13` and `contracts/contract.md §4.3`. |
| `INV-CODEC-TB-003` violated (trailing check after digest) | MED | PO-004 (Kani H6) counts `verify_digest_match` calls and asserts zero; PO-001 (structural) reviews the diff. |
| Diagnostic-code collision (`0x4042` already taken by another branch) | LOW | `codebase-map.md` confirmed free; reviewer cross-checks. |

## 9. Handoff

- **State 4b** (`proof-plan-reviewer`): reviewer dispositions each `verifier-lane-decision/v1` row; non-applicable lanes (`loom`, `miri`, `flux`) must be backed by concrete evidence (`applicability: not_applicable` + `non_applicability_evidence_refs`).
- **State 5** (`proof-writer`): authors the Kani H6 harness, the proptest properties, the cargo-fuzz oracle, and the Verus bridge arm. Does NOT touch production Rust.
- **State 7** (`proof-to-implementation`): produces the bridge map from each claim to its production site and test fixture.
- **State 12** (`formal-verifier`): executes each obligation's `command` and closes the ledger.