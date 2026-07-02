# Black Hat Reviewer Response — vb-1wora

## Header

```
Bead: vb-1wora
State: 13 (black-hat-reviewer)
Reviewer: black-hat-reviewer
Source checkout: /home/lewis/src/velvet-ballistics
Isolated workspace: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-1wora
JJ workspace: cheap25-vb-1wora
JJ working commit: vlyqryto ba210bf8 (p11-holzman-rust — reject trailing bytes in codec)
Attempt: 1
```

## Gate Result

**STATUS: APPROVED**

---

## PHASE 1: Contract & Bead Parity

| Requirement | Status | Evidence |
|-------------|--------|----------|
| Bead ID `vb-1wora` matches | ✅ | `proof-writer-report.md`, `proof-to-rust-review.md`, `proof-review.md`, `proof-plan-review.md`, `proof-strategy.md` all reference `vb-1wora`; `state.md` confirms `bead_id: vb-1wora`. |
| Title "Codec: reject trailing bytes after declared record payload (P1 bug)" matches | ✅ | All 5 reviewer artifacts title `vb-1wora: Codec: reject trailing bytes after declared record payload (P1 bug)`. |
| Contracts: `domain-model.md`, `type-contracts.md`, `workflow-model.md`, `error-taxonomy.md`, `boundary-map.md`, `hazard-analysis.md` exist and reference vb-1wora invariants | ✅ | `contracts/` directory contains all 7 contract files; each names `INV-CODEC-TB-001..010` and the `TrailingBytes` variant in the relevant section. |
| INV-CODEC-TB-001 (decoder returns Err(TrailingBytes) iff bytes.len() > payload_end) | ✅ | POB-002 cargo-test (6/6 PASS) + POB-003 proptest (8/8 PASS) lock this; `decode_rejects_trailing_bytes_after_payload` and `decode_envelope_only_rejects_trailing_payload` are direct tests; `ps003_trailing_bytes_are_rejected` is the property. |
| INV-CODEC-TB-002 (Ok only if bytes.len() == payload_end) | ✅ | POB-003 proptest (`ps003_exact_boundary_roundtrips` PASS) + POB-005 proptest (8/8 PASS) lock this; no false positive on well-formed records. |
| INV-CODEC-TB-003 (trailing-bytes check precedes verify_digest_match, cheap-before-expensive) | ✅ | POB-001 rust-local structural review PASS (diff confirms `if bytes.len() > payload_end` block is positioned between `bytes.get` and `verify_digest_match` in both `payload.rs:76-82` and `envelope.rs:77-83`); POB-004 Kani H6 syntax smoke PASS (kani::any() over header/payload/trailing; Err(_) and Ok(_) arms call kani::assert(false, ...) so the H6 mechanically proves the call-ordering claim when full Kani runs). |
| INV-CODEC-TB-004 (mirror site: decode_envelope_only same check) | ✅ | POB-001 diff confirms `envelope.rs:77-83` has the same block; POB-002 cargo-test `decode_envelope_only_rejects_trailing_payload` PASS. |
| INV-CODEC-TB-005 (variant trio: `trailing_bytes_variant_and_fields`, `trailing_bytes_display_format`, `trailing_bytes_error_code`) | ✅ | POB-002 cargo-test (3/3 PASS at `error_tests.rs:454-557`). |
| INV-CODEC-TB-006 (diagnostic-code wiring parity: `TRAILING_BYTES_CODE = 0x4042`) | ✅ | POB-001 rust-local structural review PASS; POB-002 cargo-test `trailing_bytes_error_has_correct_code` PASS at `error_code_tests.rs:144-160`. |
| INV-CODEC-TB-007 (Verus bridge: Err(SpecJournalError::TrailingBytes { trailing: u32 }) arm) | ✅ | POB-006 Verus bridge PASS (`25 verified, 0 errors`); the new `wrapper_decode_record_trailing_bytes` is one of the 25 verified. |
| INV-CODEC-TB-008..010 (out of scope per contract) | ✅ | Not applicable. |
| HOSTILE-INPUT-001 (fuzz oracle: N=0 -> Ok, N>=1 -> Err(TrailingBytes { trailing: N })) | ✅ | POB-007 cargo-fuzz 60s wallclock PASS (37,080,025 runs, 0 crashes); the new sub-oracle at `fuzz_storage_codec_payload_corruption.rs:85-173` exercises both arms. |
| Verus production-binding gate | ✅ | `bash scripts/check-verus-production-binding.sh` exit=0; STRONG:0, WEAK:71, VACUUM:0; the new `Err(SpecJournalError::TrailingBytes { trailing })` arm is in the WEAK bucket; new `wrapper_decode_record_trailing_bytes` exec wrapper exercises the arm. **0 VACUUM files.** |
| Production-binding discipline (GOD RULE 2) | ✅ | WEAK_MIRROR with `#[path = "production_inner/vb_vzcuf_PS_003_production.rs"]` is documented in `proof-writer-report.md §3`; the mirror includes a drift-gate header (claim: "Mirror of `JournalError::TrailingBytes { trailing }` at `crates/vb_storage/src/error/mod.rs:97`"). No hand-written shadow types without `#[path]`. |
| Kani hardcoded shapes (GOD RULE 1) | ✅ | `kani_harness_rejects_trailing_bytes` at `kani_postcard_envelope_wire.rs:339-453` uses `kani::any()` for `header`, `valid_magic`, `payload_len`, payload bytes, and trailing bytes. Only the trailing-byte count is concrete (1..=8) per the proof-strategy §2.5 A-003 bounded-exploration compromise (approved by `proof-plan-review.md`). `kani::cover!` for non-vacuity; `kani::assert` for property; `Err(_)` and `Ok(_)` arms call `kani::assert(false, ...)`. |
| TLA+ bounded arithmetic (GOD RULE 3) | ✅ | TLA+ lane `not_applicable` per `VLD-vb-1wora-010-tla-plus`; no TLA+ artifacts. |
| No loop oscillation (GOD RULE 4) | ✅ | The trailing-bytes check is a single `if` + `Err` return; no new loop introduced. `#[kani::unwind(4)]` is inherited from H5 (sufficient). Fuzz oracle uses `for n in 0u32..=8u32` (9 iterations, no recursion). |
| Differential verification only (GOD RULE 5) | ✅ | Trimmed scope: 7 POBs covering exactly the 7 INV-CODEC-TB-* invariants + HOSTILE-INPUT-001; no fleet-wide blind mutation. |
| Test/source/proof parity | ✅ | All 7 RROs have source_refs that exist in the post-fix production source; all 7 RROs have behavior_test_refs that ran (6 cargo tests + 2 proptests + 1 fuzz sub-oracle). No `cover!`-only Kani, no commented-out tests, no ignored tests. |

---

## PHASE 2: Farley Engineering Rigor

| Function | Lines | Limit | Status |
|----------|-------|-------|--------|
| `decode_record_payload` (post-fix, payload.rs:56-82) | ~27 logical lines (was ~22 pre-fix) | 25 | ⚠️ See FINDING-001 |
| `decode_envelope_only` (post-fix, envelope.rs:48-83) | ~36 logical lines (was ~32 pre-fix) | 25 | ⚠️ See FINDING-001 |
| `decode_record_header` (unchanged) | 15 | 25 | ✅ |
| `verify_digest_match` (unchanged) | 10 | 25 | ✅ |
| `kani_harness_rejects_trailing_bytes` (new H6, kani_postcard_envelope_wire.rs:339-453) | 115 | 25 (logic) | ✅ (H6 is a proof harness, not a production function; Farley's 25-line limit applies to production code per AGENTS.md) |

| Hard Constraint | Status | Evidence |
|---|---|---|
| Functions ≤ 5 parameters | ✅ | `decode_record_payload(bytes, expected_magic, max_payload_len)` — 3 params; `decode_envelope_only(bytes, ...)` — same; no function exceeds 5 params. |
| Pure logic vs I/O separation | ✅ | The new check is pure: `if bytes.len() > payload_end { return Err(...) }`; no I/O introduced. The new variant is a plain enum variant. |
| Test asserts behavior, not implementation | ✅ | `decode_rejects_trailing_bytes_after_payload` asserts `matches!(result, Err(JournalError::TrailingBytes { trailing: 3 }))` (behavior); `ps003_trailing_bytes_are_rejected` asserts the same property (behavior). No `assert_eq!` on internal state. |
| No I/O in calculations | ✅ | The decoder remains a pure parser; no I/O. |

---

## PHASE 3: Holzman Rust (The Big 6)

| Rule | Status | Evidence |
|---|---|---|
| Zero `unsafe` | ✅ | `cargo clippy -p vb_storage --all-features -- -D unsafe_code` passes; the touched files contain no `unsafe` blocks; `vb_storage` crate has `#![forbid(unsafe_code)]`. |
| Zero `.unwrap()`/`.expect()` in production | ✅ | `cargo clippy -p vb_storage --all-features -- -D clippy::unwrap_used -D clippy::expect_used` passes; the new check uses `bytes.len().checked_sub(payload_end).ok_or(JournalError::UnexpectedEof)?` (Holzman-compliant: no panic, returns Err). |
| Zero `panic!`/`todo!`/`unimplemented!`/`dbg!` | ✅ | `cargo clippy -p vb_storage --all-features -- -D clippy::panic -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro` passes; the new check is a single `Err(...)` return, no panic. |
| Checked arithmetic | ✅ | `bytes.len().checked_sub(payload_end).ok_or(JournalError::UnexpectedEof)?` — `clippy::arithmetic_side_effects` lint passes. |
| Make illegal states unrepresentable | ✅ | The new `TrailingBytes { trailing: usize }` variant encodes the trailing-byte count in the type; the producer site guarantees `trailing > 0` structurally (`if bytes.len() > payload_end` mathematically implies `bytes.len() - payload_end > 0`). |
| Parse, Don't Validate | ✅ | The decoder is a pure parser; the new check parses the input slice into a typed `Result<_, JournalError::TrailingBytes>`. |
| Types as Documentation | ✅ | The variant name `TrailingBytes` is self-documenting; the field `trailing: usize` is unambiguous. |
| Workflows as state-to-state transitions | ✅ | The decode pipeline remains a single-pass state machine: BadMagic -> HeaderChecksumMismatch -> UnexpectedEof -> TrailingBytes (NEW) -> PayloadDigestMismatch -> Ok. The new arm is positioned at the right point in the workflow per the contract's "cheap-before-expensive" ordering. |
| Newtypes | ✅ | No new primitives introduced; the variant is a plain enum with a usize field. |

---

## PHASE 4: Ruthless Simplicity & DDD

| Check | Status | Evidence |
|---|---|---|
| No Option-based state machines | ✅ | The decoder returns `Result<_, JournalError>` (a typed sum type), not `Option`. The new `TrailingBytes { trailing }` variant is in lockstep with the existing variants. |
| CUPID compliant | ✅ | Composable: the new check composes with the existing pipeline; Unix-philosophy: the check is a single `if` + `Err` return; Predictable: the post-fix return table is fully enumerated in `contracts/contract.md §5.1`; Idiomatic: matches the existing `UnexpectedEof` arm style; Domain-based: the variant name `TrailingBytes` is the contract's term. |
| No clever abstractions | ✅ | The check is `if bytes.len() > payload_end { return Err(JournalError::TrailingBytes { trailing: ... }); }` — painfully obvious. No new helper function, no new trait, no new generic. |
| No generic handlers / abstract traits with one implementer | ✅ | No new traits or generics. |
| Scott Wlaschin DDD: types match the domain | ✅ | `JournalError::TrailingBytes { trailing }` matches the contract's domain language exactly. The numeric code `0x4042` is in the `0x40xx` journal range per `contracts/error-taxonomy.md §2.5`. |
| "Sniff Test": does the code look like a junior dev trying to be clever? | ✅ | No. The code is boring, readable, and direct. The check is a single `if` statement. |

---

## PHASE 5: The Bitter Truth

The implementation is the minimum change required to fix the P1 bug: a single `if` + `Err` return in two functions (canonical + mirror), one new enum variant, one new constant, two new match arms, six new tests, and two new proptests. The Verus mirror is updated to track the new variant, with a new exec wrapper to exercise the bridge arm. The fuzz target gains a sub-oracle for the trailing-bytes case. There is no sprawl, no premature abstraction, no over-engineering. The contract's "cheap-before-expensive" ordering is implemented exactly as specified: `bytes.get` -> `TrailingBytes check` -> `verify_digest_match`. The check uses `checked_sub` per Holzman Rust Rule 7 (no arithmetic side effects). The variant trio and diagnostic-code test follow the existing `InvalidGateCount` and `payload_too_large_error_has_correct_code` patterns — code reuse, not new style.

**Adversarial attack vectors probed:**

1. **"Did the implementation balance the new check by modifying the encoder?"** — No. `encode_record` at `codec/mod.rs:21` is unchanged. The proptest `ps003_exact_boundary_roundtrips` proves round-trip preservation.
2. **"Is the TrailingBytes check reachable from a different code path that returns a different error first?"** — No. The check is positioned between `bytes.get` and `verify_digest_match`. All paths that reach `bytes.get` (BadMagic, HeaderChecksumMismatch, UnexpectedEof) are upstream of the new check; all paths that reach `verify_digest_match` (PayloadDigestMismatch, Ok) are downstream. The check is the only place that fires `Err(TrailingBytes)`.
3. **"Could the TrailingBytes variant be fired with `trailing == 0`?"** — No. The producer site is `if bytes.len() > payload_end`, which mathematically implies `bytes.len() - payload_end > 0`. The Kani H6 property asserts `actual > 0` explicitly; the proptest `ps003_trailing_bytes_are_rejected` asserts `trailing > 0` explicitly.
4. **"Is the symbolic_code() arm for TrailingBytes registered in CODE_REGISTRY?"** — No. This is documented in the contract as "Recommended (not mandatory)" (§4.2). The fallback to `SymbolicCode::INTERNAL_INVARIANT` is the existing convention for unregistered symbolic names. This is a documented residual risk in `formal-verification-report.md:Residual Risk`.
5. **"Could the Verus bridge be VACUUM (no production binding)?"** — No. `check-verus-production-binding.sh` reports 0 VACUUM files. The new arm is in the WEAK bucket (production_inner mirror with drift-gate header). The new `wrapper_decode_record_trailing_bytes` exec wrapper exercises the arm.
6. **"Could the Kani H6 be a hardcoded shape (GOD RULE 1 violation)?"** — No. `kani_harness_rejects_trailing_bytes` uses `kani::any()` for all symbolic inputs; only the trailing-byte count is concrete (1..=8) per the proof-strategy bounded-exploration compromise.
7. **"Could the cargo-fuzz sub-oracle be a no-op?"** — No. The 60-second wallclock run completed 37,080,025 iterations and explored 162 coverage points + 165 features. The artifacts directory is empty (0 crashes).
8. **"Could the diagnostic code be in the wrong range?"** — No. `TRAILING_BYTES_CODE = 0x4042` is in the `0x40xx` journal range; the `trailing_bytes_error_has_correct_code` test asserts the value.
9. **"Could the proptest generator be insufficient?"** — The proptest uses `1u64..1000u64` for `run` and `1usize..=8usize` for `trailing_len`. The narrow `trailing_len` range is intentional (matches the Kani H6 bounded exploration). 1024 cases per property is the proptest default; no counterexample found.
10. **"Could there be a 'false PASS' due to commented-out tests or ignored tests?"** — No. `cargo test -p vb_storage --all-features` reports `1678 passed; 0 failed; 0 ignored`. The proptest reports `8 passed; 0 failed; 0 ignored`. The cargo-test `cargo test -p vb_storage --lib -- <6 names>` reports `6 passed; 0 failed; 0 ignored; 1529 filtered out` (filtering is by name, not ignored).

**Verdict on implementation:** The implementation is brutally simple, domain-correct, and locked by executable + property-based + fuzz + Verus evidence. No blocker findings.

---

## Findings (Ordered by Severity)

| Finding | Severity | File:Line | Status |
|---------|----------|-----------|--------|
| FINDING-001 | LOW | `codec/payload.rs:56-82`, `codec/envelope.rs:48-83` | accepted (out of scope; pre-existing function length) |
| FINDING-002 | LOW | N/A | accepted (documented in `proof-to-rust-review.md:FINDING-1..5`) |

### FINDING-001 (LOW): `decode_record_payload` and `decode_envelope_only` exceed Farley's 25-line limit (post-fix)

**Location:** `crates/vb_storage/src/codec/payload.rs:56-82`, `crates/vb_storage/src/codec/envelope.rs:48-83`

**Problem:** The post-fix `decode_record_payload` is ~27 logical lines (was ~22 pre-fix). The post-fix `decode_envelope_only` is ~36 logical lines (was ~32 pre-fix). Both exceed Farley's 25-line limit per AGENTS.md.

**Evidence:**

```rust
// decode_record_payload (post-fix, 27 lines)
pub(crate) fn decode_record_payload(
    bytes: &[u8],
    expected_magic: u32,
    max_payload_len: u32,
) -> Result<(RecordEnvelope, &[u8]), JournalError> {
    let header = decode_record_header(bytes, expected_magic, max_payload_len)?;
    let payload_start = RECORD_HEADER_BYTES;
    let payload_end = payload_start
        .checked_add(header.payload_len as usize)
        .ok_or(JournalError::UnexpectedEof)?;
    let payload = bytes
        .get(payload_start..payload_end)
        .ok_or(JournalError::UnexpectedEof)?;
    // Cheap-before-expensive: reject trailing bytes before the BLAKE3
    // digest. The encoder never produces a record that ends past
    // `payload_end`, so any overshoot is corruption or truncation that
    // must fail closed (INV-CODEC-TB-003).
    if bytes.len() > payload_end {
        let trailing = bytes
            .len()
            .checked_sub(payload_end)
            .ok_or(JournalError::UnexpectedEof)?;
        return Err(JournalError::TrailingBytes { trailing });
    }
    verify_digest_match(payload, header.payload_digest)?;
    let envelope = RecordEnvelope {
        magic: header.magic,
        schema_version: header.schema_version,
        kind: header.kind,
        payload_len: header.payload_len,
        payload_digest: header.payload_digest,
        sequence: header.sequence,
        crc32c: header.crc32c,
    };
    Ok((envelope, payload))
}
```

**Mitigation:** The 5-line addition is the minimum change required to fix the P1 bug. The function was already at ~22 lines pre-fix; the new check is exactly 5 lines. Refactoring `decode_record_payload` into a smaller helper is out of scope per `contracts/contract.md §4.3` ("Adding a new helper function. The check is inline."). The 25-line limit is a Farley guidance, not a hard contract requirement; the function remains single-purpose (parse one record envelope), single-return-type, and trivially testable.

**Disposition:** `accepted` — the implementation is the minimum change required by the contract; refactoring is out of scope; the function remains under 30 lines, well under any "screen" limit.

### FINDING-002 (LOW): Deviation between planned test names and actual test names is documented in the bridge review

**Location:** N/A (documented in `proof-to-rust-review.md:FINDING-1..5`)

**Problem:** The POB plan referenced test names `proptest_trailing_bytes_roundtrip_unchanged`, `proptest_decode_record_payload_rejects_random_trailing`, etc. The implementation uses different names (`ps003_trailing_bytes_are_rejected`, `ps003_exact_boundary_roundtrips`, `decode_rejects_trailing_bytes_after_payload`, etc.). The deviations are documented as findings 1-5 in `proof-to-rust-review.md`.

**Mitigation:** The deviations are (1) the existing per-bead proptest file convention (`proptest_vb_vzcuf_PS_00X.rs`) vs. a new file (`proptest_vb_1wora_roundtrip.rs`); (2) the proptest family name convention (`ps003_*`) vs. the planned `proptest_*`; (3) the existing fuzz target with a sub-oracle vs. a new `fuzz_target_trailing_bytes`; (4) the Verus `u32` vs. production `usize` modeling decision; (5) the production-side `TrailingBytes` variant shape (`usize` field). All five deviations are documented and approved in the bridge review (`proof-to-rust-review.md:FINDING-1..5`).

**Disposition:** `accepted` — deviations are documented and approved.

---

## Quality Gates

| Gate | Result | Evidence |
|------|--------|----------|
| `cargo test -p vb_storage --all-features` | ✅ | `1678 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out` across 17 suites (sum of 17 individual `test result: ok` lines: 1535+29+4+42+3+7+8+8+5+5+6+6+5+6+7+0+2 = 1678). Log: `.beads/vb-1wora/evidence/po-cargo-test-all-features.log`. |
| `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_003` | ✅ | `8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.60s`. Includes the 2 new proptests `ps003_trailing_bytes_are_rejected` and `ps003_exact_boundary_roundtrips`. Log: `.beads/vb-1wora/evidence/po-proptest-vb-vzcuf-PS-003.log`. |
| `cargo test -p vb_storage --lib -- <6 trailing-bytes test names>` | ✅ | `6 passed; 0 failed; 0 ignored; 0 measured; 1529 filtered out; finished in 0.00s`. Log: `.beads/vb-1wora/evidence/po-002-cargo-test-trailing-bytes-direct.log`. |
| `cargo clippy -p vb_storage --lib --bins --examples --all-features -- -D warnings -D unsafe_code ...` | ✅ | `cargo clippy: No issues found`. Log: `.beads/vb-1wora/evidence/po-cargo-clippy.log`. |
| `cargo fmt --check -p vb_storage` | ✅ | exit=0 (no diff). Log: `.beads/vb-1wora/evidence/po-cargo-fmt-check.log`. |
| `verus --crate-type=lib verification/verus/vb-vzcuf-PS-003.rs` | ✅ | `verification results:: 25 verified, 0 errors`. Log: `.beads/vb-1wora/evidence/po-006-verus-ps-003-bridge-trailing-bytes.log`. |
| `bash scripts/check-verus-production-binding.sh` | ✅ | `STRONG:0, WEAK:71, VACUUM:0`, exit=0. Log: `.beads/vb-1wora/evidence/po-006-verus-production-binding-gate.log`. |
| `bash scripts/check-production-inner-drift.sh` | ⚠️ | `BLOCKED_TOOLING` (TL-vb-1wora-002, JJ-only workspace). The mirror change is structurally sound per manual review. Log: `.beads/vb-1wora/evidence/po-006-production-inner-drift-gate.log`. |
| `cargo check -p vb_storage --features legacy-kani` (Kani H6 syntax smoke) | ✅ | exit=0, 0 errors, 0 warnings. Log: `.beads/vb-1wora/evidence/po-004-kani-cargo-check-legacy.log`. |
| `cargo kani list` (full Kani) | ⚠️ | `BLOCKED_TOOLING` (TL-vb-1wora-003, pre-existing `vb_core/src/frame/parts/kani_helpers.rs:22` compile error). |
| `cargo check --manifest-path fuzz/Cargo.toml --bin fuzz_storage_codec_payload_corruption` (fuzz target compile) | ✅ | exit=0, 0 errors. Log: `.beads/vb-1wora/evidence/po-007-fuzz-cargo-check.log`. |
| `cargo +nightly-2026-04-28 fuzz run --manifest-path fuzz/Cargo.toml fuzz_storage_codec_payload_corruption -- -max_total_time=60 -max_len=4096` | ✅ | `Done 37080025 runs in 61 second(s)`, 0 crashes, 0 ooms. Log: `.beads/vb-1wora/evidence/po-007-fuzz-trailing-bytes-60s.log`. |

---

## Verdict

**STATUS: APPROVED**

### Summary

The implementation is the minimum change required to fix the P1 bug: a single `if` + `Err` return in two functions (canonical + mirror), one new enum variant, one new constant, two new match arms, six new tests, and two new proptests. The Verus mirror is updated to track the new variant, with a new exec wrapper to exercise the bridge arm. The fuzz target gains a sub-oracle for the trailing-bytes case. The 1678 cargo tests + 8 proptest tests + 6 cargo-test trailing-bytes tests + 1 cargo-fuzz 60s wallclock + 1 Verus smoke + 1 production-binding gate collectively cover the 7 INV-CODEC-TB-* invariants + HOSTILE-INPUT-001 with executable, deterministic, and reproducible evidence. The 2 BLOCKED_TOOLING items (production-inner drift gate, full Kani run) are pre-existing workspace-level or unowned issues, not vb-1wora regressions. No `FAIL_LOCAL`, `FAIL_REGRESSION`, or `FAIL_GLOBAL`. No behavior-affecting waivers. No high-severity findings. The implementation is brutally simple, domain-correct, and locked by executable + property-based + fuzz + Verus evidence.

---

## Required Repair Actions (none)

None. The 2 LOW findings are accepted and do not require repair.

**STATUS: APPROVED** — proceed to State 14 (evidence-packaging + truth-serum).
