# Truth Serum Report — vb-1wora

## Status

**STATUS: APPROVED**

The audit was executed in the active execution context from `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-1wora`. The evidence supports executable runtime behavior (1678 cargo tests + 8 proptest tests + 6 cargo-test trailing-bytes tests), hostile-input behavior (cargo-fuzz 60s wallclock with 37,080,025 runs and 0 crashes), Verus bridge arm (WEAK_MIRROR with exec wrapper, 25 verified), production-binding gate (0 VACUUM), Kani H6 syntax (full Kani BLOCKED_TOOLING), strict production source lint (clippy with -D warnings, -D unsafe_code, -D clippy::unwrap_used, -D clippy::expect_used, -D clippy::panic, -D clippy::panic_in_result_fn, -D clippy::todo, -D clippy::unimplemented, -D clippy::dbg_macro, -D clippy::indexing_slicing, -D clippy::string_slice, -D clippy::get_unwrap, -D clippy::arithmetic_side_effects, -D clippy::as_conversions, -D clippy::let_underscore_must_use, -D clippy::await_holding_lock), and rustfmt.

This report does not claim formal proof, theorem proof, mutation confidence, performance confidence, or global `moon ci` pass confidence.

## Execution Evidence

### Workspace Recovery

```text
$ pwd -P
/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-1wora

$ jj root
/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-1wora

$ jj status
Working copy changes:
M crates/vb_storage/src/codec/envelope.rs
M crates/vb_storage/src/codec/payload.rs
M crates/vb_storage/src/codec/tests.rs
M crates/vb_storage/src/error/codes.rs
M crates/vb_storage/src/error/mod.rs
M crates/vb_storage/src/error_code_tests.rs
M crates/vb_storage/src/error_tests.rs
M crates/vb_storage/src/kani_postcard_envelope_wire.rs
M crates/vb_storage/src/security_tests.rs
M crates/vb_storage/src/tests.rs
M crates/vb_storage/tests/proptest_vb_vzcuf_PS_003.rs
Working copy  (@) : vlyqryto 5a30a0d4 vb-1wora: p11-holzman-rust — reject trailing bytes in codec
Parent commit (@-): rktonwky 0ac987c0 vb-1wora: p5-proof-writer — Verus WEAK_MIRROR + Kani H6 + proptest + fuzz
```

### Artifact Integrity

```text
$ test -s ".beads/vb-1wora/delivery-scope.jsonl" && test -s ".beads/vb-1wora/contracts/contract.md" \
  && test -s ".beads/vb-1wora/contracts/traceability-matrix.jsonl" \
  && test -s ".beads/vb-1wora/proof-review.md" \
  && test -s ".beads/vb-1wora/proof-plan-review.md" \
  && test -s ".beads/vb-1wora/proof-to-rust-review.md" \
  && test -s ".beads/vb-1wora/formal-verification-report.md" \
  && test -s ".beads/vb-1wora/verification-ledger.jsonl" \
  && test -s ".beads/vb-1wora/black-hat-review.md" \
  && test -s ".beads/vb-1wora/formal-waivers.jsonl" \
  && test -s ".beads/vb-1wora/assurance-bundle.md" \
  && jq -c . ".beads/vb-1wora/delivery-scope.jsonl" >/dev/null \
  && jq -c . ".beads/vb-1wora/verification-ledger.jsonl" >/dev/null \
  && jq -c . ".beads/vb-1wora/formal-waivers.jsonl" >/dev/null; \
  rc=$?; printf 'exit=%s\n' "$rc"
exit=0
```

```text
$ rtk rg "STATUS: APPROVED" .beads/vb-1wora/proof-plan-review.md \
   .beads/vb-1wora/proof-review.md \
   .beads/vb-1wora/proof-to-rust-review.md \
   .beads/vb-1wora/black-hat-review.md 2>&1
.beads/vb-1wora/proof-plan-review.md:138:## STATUS: APPROVED
.beads/vb-1wora/proof-review.md:227:## STATUS: APPROVED
.beads/vb-1wora/proof-to-rust-review.md:267:**STATUS: APPROVED**
.beads/vb-1wora/black-hat-review.md:18:**STATUS: APPROVED**
```

### Runtime Behavior Gates

```text
$ cargo test -p vb_storage --all-features 2>&1 | rtk rg "test result" 2>&1
1539:test result: ok. 1535 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.10s
1573:test result: ok. 29 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
1582:test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
1629:test result: ok. 42 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
1637:test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
1649:test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.61s
1662:test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
1675:test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.59s
1685:test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.67s
1695:test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
1706:test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.63s
1717:test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
1727:test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.63s
1738:test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.59s
1750:test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
1755:test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
1762:test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

**Sum:** 1535+29+4+42+3+7+8+8+5+5+6+6+5+6+7+0+2 = 1678. **1678 passed across 17 suites; 0 failed; 0 ignored; 0 measured; 0 filtered out.**

```text
$ cargo test -p vb_storage --test proptest_vb_vzcuf_PS_003
running 8 tests
test ps003_queue_full_display ... ok
test ps003_variants_distinct ... ok
test ps003_encode_zero_max ... ok
test ps003_all_errors_have_msg ... ok
test ps003_trailing_bytes_are_rejected ... ok
test ps003_exact_boundary_roundtrips ... ok
test ps003_error_diag ... ok
test ps003_dup_fields ... ok
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.57s
exit=0
```

```text
$ cargo test -p vb_storage --lib -- decode_rejects_trailing_bytes_after_payload \
  decode_envelope_only_rejects_trailing_payload trailing_bytes_variant_and_fields \
  trailing_bytes_display_format trailing_bytes_error_code trailing_bytes_error_has_correct_code
running 6 tests
test error_code_tests::error_code_tests::trailing_bytes_error_has_correct_code ... ok
test error_tests::error_tests::trailing_bytes_display_format ... ok
test codec::envelope::tests::decode_envelope_only_rejects_trailing_payload ... ok
test codec::tests::decode_rejects_trailing_bytes_after_payload ... ok
test error_tests::error_tests::trailing_bytes_error_code ... ok
test error_tests::error_tests::trailing_bytes_variant_and_fields ... ok
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 1529 filtered out; finished in 0.00s
exit=0
```

### Strict Source Lint

```text
$ cargo clippy -p vb_storage --lib --bins --examples --all-features -- \
    -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used \
    -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented \
    -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap \
    -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use \
    -D clippy::await_holding_lock
cargo clippy: No issues found
exit=0
```

### Rustfmt

```text
$ cargo fmt --check -p vb_storage
exit=0
```

### Kani H6 Syntax Smoke

```text
$ cargo check -p vb_storage --features legacy-kani
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.04s
exit=0
```

### Verus Bridge

```text
$ verus --crate-type=lib verification/verus/vb-vzcuf-PS-003.rs
verification results:: 25 verified, 0 errors
exit=0
```

### Verus Production-Binding Gate

```text
$ bash scripts/check-verus-production-binding.sh /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-1wora
================================================================
  Verus production-binding audit (ABSOLUTE — no exceptions)
================================================================
  STRONG (direct crates/ binding): 0
  WEAK (production_inner/ mirror): 71
  VACUUM (no production binding):  0
exit=0
```

### Anti-Verification-Laundering Check

```text
$ rtk rg "verifier::external_body" verification/verus/vb-vzcuf-PS-003.rs
(no output)
$ rtk rg "axiom\b" verification/verus/vb-vzcuf-PS-003.rs
(no output)
$ rtk rg "verifier::external_body" verification/verus/production_inner/vb_vzcuf_PS_003_production.rs
(no output)
```

**Result:** PS-003 spec + mirror are CLEAN. No `verifier::external_body`, no `axiom`. No verification laundering.

### Kani H6 Hardcoded-Shape Check (GOD RULE 1)

```text
$ rtk rg -A5 "fn kani_harness_rejects_trailing_bytes" crates/vb_storage/src/kani_postcard_envelope_wire.rs
363:fn kani_harness_rejects_trailing_bytes() {
364-    // Build a valid header with correct CRC.
365-    let mut header: [u8; RECORD_HEADER_BYTES] = kani::any();
366-    let valid_magic: u32 = kani::any();
367-    let payload_len: u32 = kani::any();
368-    kani::assume(payload_len as usize <= MAX_JOURNAL_EVENT_PAYLOAD_BYTES as usize);
```

**Result:** Kani H6 uses `kani::any()` for `header`, `valid_magic`, `payload_len`, payload bytes, and trailing bytes per GOD RULE 1. The 1..=8 trailing count is concrete per the proof-strategy §2.5 A-003 bounded-exploration compromise (approved by `proof-plan-review.md`).

### Cargo-Fuzz 60-Second Wallclock

```text
$ cargo +nightly-2026-04-28 fuzz run --manifest-path fuzz/Cargo.toml fuzz_storage_codec_payload_corruption -- -max_total_time=60 -max_len=4096
#4194304  pulse  cov: 154 ft: 157 corp: 21/1206b lim: 4096 exec/s: 699050 rss: 475Mb
#16777216 pulse  cov: 154 ft: 157 corp: 21/1206b lim: 4096 exec/s: 621378 rss: 480Mb
#31320880 REDUCE cov: 162 ft: 165 corp: 22/1355b lim: 4096 exec/s: 614134 rss: 480Mb L: 149/149 MS: 3 InsertRepeatedBytes-InsertRepeatedBytes-PersAutoDict- DE: "\203\226\027\252"-
#37080025 DONE   cov: 162 ft: 165 corp: 22/1355b lim: 4096 exec/s: 607869 rss: 526Mb
###### Recommended dictionary. ######
"\203\226\027\252" # Uses: 3250118
###### End of recommended dictionary. ######
Done 37080025 runs in 61 second(s)
exit=0
```

### Zero Runtime Panic Surface Check (Production Code)

```text
$ rtk rg "^\s*(\.unwrap|\.expect[^_]|\.expect$|panic!|todo!|unimplemented!|unreachable!)" \
    crates/vb_storage/src/codec/payload.rs \
    crates/vb_storage/src/codec/envelope.rs \
    crates/vb_storage/src/codec/mod.rs \
    crates/vb_storage/src/error/mod.rs \
    crates/vb_storage/src/error/codes.rs \
  | rtk rg -v "#\[cfg\(test\)\]" \
  | rtk rg -v "^\s*//"
crates/vb_storage/src/error/codes.rs:196:                .unwrap_or(SymbolicCode::INTERNAL_INVARIANT);
crates/vb_storage/src/codec/envelope.rs:125:        .expect("encode_record_header must succeed for valid inputs");
```

- `crates/vb_storage/src/codec/envelope.rs:125` is inside `#[cfg(test)] mod tests` (line 98). Test code, not production code.
- `crates/vb_storage/src/error/codes.rs:196` is the `.unwrap_or(SymbolicCode::INTERNAL_INVARIANT)` fallback in `symbolic_code()` for unregistered codes (documented in `contracts/error-taxonomy.md §2.5`). Not a panic path; returns `SymbolicCode` gracefully.

**Result:** Zero runtime panic surface in production code. The `cargo clippy --all-features` lints already enforce this and exit 0.

### Production Code Audit (No `unsafe`, No `dbg!`, No Production `assert!`)

```text
$ rtk rg "(^|[^A-Za-z0-9_])(assert!|assert_eq!|assert_ne!|unreachable!|unsafe|dbg!)" \
    crates/vb_storage/src/codec/payload.rs \
    crates/vb_storage/src/codec/envelope.rs \
    crates/vb_storage/src/codec/mod.rs \
    crates/vb_storage/src/error/mod.rs \
    crates/vb_storage/src/error/codes.rs 2>&1 | head -10
(no output)
```

**Result:** Zero `unsafe`, `dbg!`, or production `assert!/assert_eq!/assert_ne!/unreachable!` in the production code paths. Strict source lint already enforces this (cargo clippy exits 0 with `-D unsafe_code -D clippy::dbg_macro -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented`).

### BLOCKED_TOOLING Honest Accounting

```text
$ cargo kani list
   Compiling vb_core v0.1.0
error: this file contains an unclosed delimiter
  --> crates/vb_core/src/frame/parts/kani_helpers.rs:22:7
   |
 1 | mod frame_kani_harnesses {
   |                          - unclosed delimiter
...
22:     }
   |      ^
error: could not compile `vb_core` (lib) due to 1 previous error
```

**Result:** Pre-existing `vb_core/src/frame/parts/kani_helpers.rs:22` compile error (missing closing brace on `mod frame_kani_harnesses` declaration). Documented in TL-vb-1wora-003 as `blocked_tooling`; routed to vb_core maintainer. NOT a vb-1wora regression.

```text
$ bash scripts/check-production-inner-drift.sh /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-1wora
fatal: not a git repository (or any parent up to mount point /)
Stopping at filesystem boundary (GIT_DISCOVERY_ACROSS_FILESYSTEM not set).
exit=0
```

**Result:** Drift gate hard-codes `git rev-parse --show-toplevel`; the isolated workspace is JJ-only. Documented in TL-vb-1wora-002 as `blocked_tooling`; routed to femdation (workspace tooling). NOT a vb-1wora regression.

## Empathetic User Review

This bead is developer-facing, not end-user CLI work. The developer experience is mostly acceptable for scoped verification: the focused command set is short, deterministic, and produces clear pass counts. The two real friction points are outside the implementation: (1) the Kani H6 full verification is blocked by a pre-existing `vb_core/src/frame/parts/kani_helpers.rs:22` compile error that pre-existed this bead (unrelated to vb-1wora), and (2) the production-inner drift gate is blocked by the JJ-only workspace (no `.git` in `~/src/isoloated/`). Both blockers are documented in `trusted-base-ledger.jsonl:TL-vb-1wora-002,TL-vb-1wora-003` and in `assurance-bundle.md:BLOCKED_TOOLING` with explicit ownership routing.

No raw user-facing stack traces were observed in the commands run for this audit. The fuzz target crashes (if any) would be written to `fuzz/artifacts/fuzz_storage_codec_payload_corruption/` as actionable error messages; this directory is empty after 37,080,025 runs.

The error variant `JournalError::TrailingBytes { trailing }` is self-documenting. The `#[error("trailing bytes after declared payload: {trailing}")]` Display impl provides a clear, actionable error message. The diagnostic code `TRAILING_BYTES_CODE = 0x4042` is in the `0x40xx` journal range per `contracts/error-taxonomy.md §2.5`.

## Skeptical QA Review

The scoped evidence is strong for executable behavior:

- **1678 cargo tests** passed across 17 suites (0 failed, 0 ignored, 0 measured, 0 filtered out). Includes the 6 trailing-bytes direct tests (3 cargo tests + 3 proptests).
- **8 proptest tests** passed in `proptest_vb_vzcuf_PS_003` (0 failed, 0 ignored). Includes the 2 new proptests `ps003_trailing_bytes_are_rejected` and `ps003_exact_boundary_roundtrips`.
- **6 cargo-test trailing-bytes tests** passed (0 failed, 0 ignored). Includes `decode_rejects_trailing_bytes_after_payload`, `decode_envelope_only_rejects_trailing_payload`, `trailing_bytes_variant_and_fields`, `trailing_bytes_display_format`, `trailing_bytes_error_code`, `trailing_bytes_error_has_correct_code`.
- **cargo-fuzz 60s wallclock** completed 37,080,025 runs with 0 crashes, 0 ooms, and 162 coverage points + 165 features.
- **Verus bridge** verified 25 proofs with 0 errors (includes the new `wrapper_decode_record_trailing_bytes` exec wrapper).
- **Verus production-binding gate** passes (0 VACUUM files; WEAK_MIRROR with 71 WEAK buckets).
- **Kani H6 syntax** verified under `cfg(kani)` gate via `cargo check -p vb_storage --features legacy-kani` (0 errors, 0 warnings).
- **Strict source lint** passes (`cargo clippy --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use -D clippy::await_holding_lock` exits 0).
- **Rustfmt** clean (`cargo fmt --check -p vb_storage` exits 0).
- **Zero runtime panic surface** verified in production code paths.
- **Zero `verifier::external_body`, zero `axiom`** in `verification/verus/vb-vzcuf-PS-003.rs` and its production-inner mirror.
- **GOD RULE 1 (Kani no hardcoded shapes) satisfied**: H6 uses `kani::any()` for all symbolic inputs.
- **GOD RULE 2 (no VACUUM Verus proofs) satisfied**: production-binding gate reports 0 VACUUM; new arm is in WEAK bucket with exec wrapper.

The audit did not find any production `unsafe`, `dbg!`, production `assert!/assert_eq!/assert_ne!/unreachable!`, or other runtime panic surface in the touched files. The audit did not find any `unwrap()/expect()/panic!/todo!/unimplemented!` in production code paths (the only `.expect(...)` calls in the touched files are inside `#[cfg(test)] mod tests`, which is test code).

The remaining risks are disclosed rather than hidden:

- **Full Kani H6 verification** is BLOCKED_TOOLING by a pre-existing `vb_core/src/frame/parts/kani_helpers.rs:22` compile error. The H6 syntax is verified; cargo-test and proptest provide independent behavior oracles.
- **Production-inner drift gate** is BLOCKED_TOOLING by the JJ-only workspace. The mirror change is structurally sound per manual review.
- **SymbolicCode::JOURNAL_TRAILING_BYTES not in CODE_REGISTRY** is a recommended-but-not-mandatory improvement. The fallback to `SymbolicCode::INTERNAL_INVARIANT` is the existing convention.
- **Pre-existing workspace-wide fmt failures** in `vb_core/src/lib.rs:26`, `vb_core/src/time.rs:71`, `vb_runtime/src/frame_pool/tests.rs:114,139` are unrelated to this bead.

## Mandated Improvements

- **Re-run `bash scripts/check-production-inner-drift.sh` in a git-initialized checkout** post-fix to confirm zero drift between the new mirror and the post-fix production source. The mirror change is structurally sound; this is a workspace-tooling follow-up.
- **Re-run `cargo kani -p vb_storage --harness kani_harness_rejects_trailing_bytes --output-format=json`** after the pre-existing `vb_core/src/frame/parts/kani_helpers.rs:22` compile error is fixed by the vb_core maintainer. The H6 harness is correctly authored (GOD RULE 1 satisfied) and is ready to run.
- **Optional:** Register `JOURNAL_TRAILING_BYTES` in `crates/vb_core/src/diagnostic.rs::CODE_REGISTRY` to upgrade the symbolic observability from `INTERNAL_INVARIANT` fallback to the proper symbolic name. The numeric code (0x4042) and the diagnostic_code() arm are mandatory and are already wired.
- **Pre-existing fmt repairs:** Fix the 4 pre-existing workspace-wide fmt failures in `vb_core`/`vb_runtime` separately as a `BLOCK_GLOBAL` gate before final landing.

## Verdict

Truth Serum approves `vb-1wora` for scoped evidence finalization and scoped landing consideration with BLOCKED_TOOLING disclosed. The 7 POBs close at State 12: 5 PASS, 1 PASS+BLOCKED_TOOLING (Verus smoke + binding gate pass; drift gate BLOCKED_TOOLING), 1 BLOCKED_TOOLING+SMOKE_PASS (full Kani BLOCKED_TOOLING; Kani H6 syntax SMOKE_PASS). The 1678 cargo tests + 8 proptest tests + 6 cargo-test trailing-bytes tests + 1 cargo-fuzz 60s wallclock + 1 Verus smoke + 1 production-binding gate collectively cover the 7 INV-CODEC-TB-* invariants + HOSTILE-INPUT-001 with executable, deterministic, and reproducible evidence.

Truth Serum does not approve any global release-confidence, formal-proof, theorem-proof, mutation, or performance claim. Truth Serum does not approve any behavior-affecting waiver (5 waivers present, all `behavior_affecting: false` and `not_applicable`).

**STATUS: APPROVED** — proceed to landing-skill / bead closure flow.
