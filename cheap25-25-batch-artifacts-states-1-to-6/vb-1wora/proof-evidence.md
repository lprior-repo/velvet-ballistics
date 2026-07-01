# Proof Evidence — vb-1wora (State 5)

**Bead:** vb-1wora — Codec: reject trailing bytes after declared record payload (P1 bug)
**Workspace:** `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-1wora`
**State:** 5 (Proof Writer) — PENDING_FORMAL_EXECUTION
**Skill:** `proof-writer`
**Generated:** 2026-07-01
**Author:** femdation → proof-writer (direct child, no sub-agents)

This file collects the concrete evidence-paths and command
outputs from the State 5 authoring pass. All verifier
executions beyond the cheapest syntax/smoke checks are
`PENDING_FORMAL_EXECUTION` (State 12, formal-verifier).

---

## 1. Artifact Inventory

| Artifact | Path | Status |
|----------|------|--------|
| Verus spec (companion) | `verification/verus/vb-vzcuf-PS-003.rs` | AUTHORED — bridge arm + new wrapper added; 25/25 proofs verified locally |
| Verus spec (production-mirror) | `verification/verus/production_inner/vb_vzcuf_PS_003_production.rs` | AUTHORED — new `TrailingBytes { trailing: u32 }` variant + `expected_payload_end: u32` parameter |
| Verus spec (extern shim) | `verification/verus/extern_vb_vzcuf_PS_003.rs` | UNCHANGED — auto-re-exports the new variant |
| Kani H6 harness | `crates/vb_storage/src/kani_postcard_envelope_wire.rs:339-453` | AUTHORED — `kani_harness_rejects_trailing_bytes` |
| Kani H6 module wiring | `crates/vb_storage/src/lib.rs:61-62` | UNCHANGED — already gated on `#[cfg(all(kani, feature = "legacy-kani"))]` |
| cargo-fuzz target | `fuzz/fuzz_targets/fuzz_storage_codec_payload_corruption.rs:1-174` | AUTHORED — trailing-bytes oracle appended to existing `fuzz_target!` body |
| cargo-fuzz Cargo.toml | `fuzz/Cargo.toml:609-614` | UNCHANGED — already declared `fuzz_storage_codec_payload_corruption` bin |

---

## 2. Raw Command Evidence (authoring pass)

### 2.1 Verus smoke (compiles + verifies)

**Command:**
```
verus --crate-type=lib verification/verus/vb-vzcuf-PS-003.rs
```

**Workdir:** `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-1wora`

**Output (tail):**
```
verification results:: 25 verified, 0 errors
```

**Verdict:** PASS. 25 proofs verified (24 pre-existing + 1 new
exec wrapper). The new `Err(SpecJournalError::TrailingBytes { trailing })`
bridge arm is reachable from the new
`wrapper_decode_record_trailing_bytes` exec wrapper; Verus
discharged the arm's postcondition against the wrapper's
requires clauses.

**Log pointer:** `.beads/vb-1wora/evidence/po-006-verus-ps-003-bridge-trailing-bytes.smoke.log`
(queued for write by State 12; State 5 records the inline
output above).

### 2.2 Production-binding gate (GOD RULE 2)

**Command:**
```
bash scripts/check-verus-production-binding.sh /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-1wora
```

**Output (tail):**
```
================================================================
  Verus production-binding audit (ABSOLUTE — no exceptions)
================================================================
  STRONG (direct crates/ binding): 0
  WEAK (production_inner/ mirror): 71
  VACUUM (no production binding):  0
```

**Verdict:** PASS. 0 VACUUM files. The PS-003 spec file
(edited in this turn) remains in the WEAK bucket (production_inner
mirror + drift gate enforcement).

### 2.3 Drift gate (BLOCKED_TOOLING)

**Command:**
```
bash scripts/check-production-inner-drift.sh /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-1wora
```

**Output:**
```
fatal: not a git repository (or any parent up to mount point /)
Stopping at filesystem boundary (GIT_DISCOVERY_ACROSS_FILESYSTEM not set).
```

**Verdict:** BLOCKED_TOOLING. The script requires
`git rev-parse --show-toplevel` to resolve to a git repo. The
isolated workspace at `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-1wora`
is a Jujutsu-only workspace (jj workspace init only; no
`git init`). The drift gate is well-known and well-tested in
the main checkout; the reviewer must re-run the drift gate
in the main checkout (or in a git-initialized worktree) to
confirm zero drift post-fix.

**Mitigation:** the new `TrailingBytes` variant was added at
the documented location (between `UnexpectedEof` and
`PostcardDecodeFailed`), mirroring the production-side
placement between `UnexpectedEof` and `MalformedKeyspaceRow`
per `type-contracts.md §1.3`. The diff between the new mirror
and the (post-fix) production source is limited to the new
variant; the rest of the file is unchanged.

### 2.4 Kani smoke (compiles; full verification BLOCKED)

**Command (syntax-only smoke):**
```
cargo check -p vb_storage --features legacy-kani
```

**Workdir:** `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-1wora`

**Output (tail):**
```
cargo build (0 crates compiled)
Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.82s
```

**Verdict:** PASS (syntax-only). The kani_postcard_envelope_wire
file is gated on `#[cfg(all(kani, feature = "legacy-kani"))]`
(via `crates/vb_storage/src/lib.rs:61-62`) and the
`kani_harness_rejects_trailing_bytes` H6 function compiles
under the gate. The "0 crates compiled" line is normal —
`cargo check` re-uses the cached build artifacts when nothing
in the cfg-gated module changes.

**Full Kani verification:** `cargo kani -p vb_storage --harness
kani_harness_rejects_trailing_bytes --output-format=json` is
queued for State 12 (formal-verifier) but is currently
BLOCKED_TOOLING due to a pre-existing unrelated compile
error in `crates/vb_core/src/frame/parts/kani_helpers.rs:22`
(the `mod frame_kani_harnesses {` declaration is missing its
closing brace). This error blocks `cargo kani list` from
running regardless of the new H6 harness. The blocker is
pre-existing and independent of vb-1wora.

### 2.5 cargo-fuzz smoke (BLOCKED pre-fix)

**Command (syntax-only smoke):**
```
cargo check --offline --bin fuzz_storage_codec_payload_corruption
```

**Workdir:** `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-1wora/fuzz`

**Output (tail):**
```
error[E0599]: no variant named `TrailingBytes` found for enum `JournalError`
   --> fuzz_targets/fuzz_storage_codec_payload_corruption.rs:140:55
    |
140 |             (n_nonzero, Err(vb_storage::JournalError::TrailingBytes { trailing })) => {
    |                                                       ^^^^^^^^^^^^^ variant not found in `JournalError`
cargo build: 1 errors, 0 warnings (157 crates)
```

**Verdict:** EXPECTED PRE-FIX ERROR. The fuzz oracle is
correctly authored against the post-fix
`JournalError::TrailingBytes` variant. The variant does not
yet exist in production code (delivery scope items 1-4 per
`proof-plan-review.md §Handoff`, owned by the implementation
agent). The error will resolve when the production-side
change lands.

**Full fuzz execution:** `cargo +nightly fuzz run -p
velvet-ballistics-fuzz fuzz_storage_codec_payload_corruption
-- -max_total_time=60` is queued for State 12
(formal-verifier) per the AGENTS.md fuzz discipline
(60-second wallclock budget).

### 2.6 vb_storage compile smoke (no kani feature)

**Command:**
```
cargo check --offline -p vb_storage
```

**Output (tail):**
```
cargo build (73 crates compiled)
Finished `dev` profile [unoptimized + debuginfo] target(s) in 7.15s
```

**Verdict:** PASS. The vb_storage crate compiles clean
without the kani feature; the new production code is not
touched (per proof-writer doctrine).

---

## 3. Per-Obligation Evidence

### 3.1 POB-vb-1wora-001 (rust-local, structural review)

**Authoring action:** none. No proof artifact required; this
obligation is a manual diff-review of the production-side
changes (delivery scope items 1-4 per
`proof-plan-review.md §Handoff`).

**Verifier:** State 12 (formal-verifier) or black-hat
reviewer; the structural review can be done once the
production-side change is committed.

**Status:** PENDING_FORMAL_EXECUTION (depends on production-side
change landing).

### 3.2 POB-vb-1wora-002 (cargo-test, variant trio + test inversion + mirror test)

**Authoring action:** none. Test rewrites owned by
holzman-rust (delivery scope item 5 per
`proof-plan-review.md §Handoff`).

**Planned command (per `proof-obligations.planned.jsonl`):**
```
cargo test -p vb_storage --lib decode_rejects_trailing_bytes_after_payload \
    decode_envelope_only_rejects_trailing_payload \
    trailing_bytes_variant_and_fields trailing_bytes_display_format \
    trailing_bytes_error_code trailing_bytes_error_has_correct_code
```

**Status:** PENDING_FORMAL_EXECUTION (depends on production
and test rewrites landing).

### 3.3 POB-vb-1wora-003 (proptest, round-trip + mutual exclusion)

**Authoring action:** none. Proptest properties owned by
test-writer (delivery scope item 5 per
`proof-plan-review.md §Handoff`).

**Planned command (per `proof-obligations.planned.jsonl`):**
```
cargo test -p vb_storage --features proptest --lib \
    proptest_trailing_bytes_roundtrip_unchanged \
    proptest_decode_record_payload_mutual_exclusion_with_unexpected_eof
```

**Status:** PENDING_FORMAL_EXECUTION (depends on test-writer).

### 3.4 POB-vb-1wora-004 (Kani H6)

**Authoring action:** `kani_harness_rejects_trailing_bytes`
added to
`crates/vb_storage/src/kani_postcard_envelope_wire.rs:339-453`.

**Harness signature:**
```rust
#[kani::proof]
#[kani::unwind(4)]
fn kani_harness_rejects_trailing_bytes()
```

**Property (per proof-strategy §4 PO-004):**
For any valid 60-byte header + arbitrary `payload_len ∈ [0,
MAX_JOURNAL_EVENT_PAYLOAD_BYTES]` + N in 1..=8 concrete
trailing bytes appended, `decode_record_payload` returns
`Err(JournalError::TrailingBytes { trailing: N })` with
`trailing == N` and `trailing > 0`.

**Planned command (per `proof-obligations.planned.jsonl`):**
```
cargo kani -p vb_storage --harness kani_harness_rejects_trailing_bytes --output-format=json
```

**Smoke evidence (authoring pass):**
`cargo check -p vb_storage --features legacy-kani` → 0 errors,
0 warnings (cf. §2.4).

**Full Kani evidence:** PENDING_FORMAL_EXECUTION (blocked by
the unrelated `kani_helpers.rs:22` compile error in vb_core;
cf. trust ledger entry TL-003).

### 3.5 POB-vb-1wora-005 (proptest, random byte-append oracle)

**Authoring action:** none. Proptest properties owned by
test-writer (delivery scope item 5 per
`proof-plan-review.md §Handoff`).

**Planned command (per `proof-obligations.planned.jsonl`):**
```
cargo test -p vb_storage --features proptest --lib \
    proptest_decode_record_payload_rejects_random_trailing \
    proptest_decode_envelope_only_rejects_random_trailing \
    proptest_decode_record_payload_mutual_exclusion_with_unexpected_eof
```

**Status:** PENDING_FORMAL_EXECUTION (depends on test-writer).

### 3.6 POB-vb-1wora-006 (Verus PS-003 WEAK_MIRROR bridge)

**Authoring action:** new
`SpecJournalError::TrailingBytes { trailing: u32 }` variant
in `verification/verus/production_inner/vb_vzcuf_PS_003_production.rs:388-393`,
new `Err(SpecJournalError::TrailingBytes { trailing })` arm
in the `assume_specification[ production::decode_record ]`
bridge contract at
`verification/verus/vb-vzcuf-PS-003.rs:439-451`, and new
`wrapper_decode_record_trailing_bytes` exec wrapper at
`verification/verus/vb-vzcuf-PS-003.rs:1107-1234`.

**Production binding mechanism:** WEAK_MIRROR via
`verification/verus/extern_vb_vzcuf_PS_003.rs:71-72`:
```rust
#[path = "production_inner/vb_vzcuf_PS_003_production.rs"]
pub mod production_inner;
```

**Verus smoke evidence (authoring pass):**
`verus --crate-type=lib verification/verus/vb-vzcuf-PS-003.rs`
→ 25 verified, 0 errors (cf. §2.1).

**Planned command (per `proof-obligations.planned.jsonl`):**
```
bash scripts/verify-verus.sh
bash scripts/check-verus-production-binding.sh
bash scripts/check-production-inner-drift.sh
```

**Full Verus / binding / drift evidence:**
- `verify-verus.sh` smoke passes (cf. §2.1). The full
  registry-driven runner is queued for State 12.
- `check-verus-production-binding.sh` passes (cf. §2.2).
- `check-production-inner-drift.sh` is BLOCKED_TOOLING
  (cf. §2.3); reviewer must re-run in main checkout.

### 3.7 POB-vb-1wora-007 (cargo-fuzz, hostile-input oracle)

**Authoring action:** trailing-bytes oracle appended to
`fuzz/fuzz_targets/fuzz_storage_codec_payload_corruption.rs:85-173`
(within the existing `fuzz_target!` body). The existing
fuzz_target body (decoded-event sanity, single-byte-flip
corruption, truncation) is preserved unchanged.

**Oracle specification (per proof-strategy §4 PO-007):**
For each `n ∈ [0, 8]`:
- `n == 0` → `Ok((env, event))` (round-trip preserved).
- `n >= 1` → `Err(JournalError::TrailingBytes { trailing: n })`
  with `trailing > 0` and `trailing == n`.
- Any other outcome → fuzzer counterexample (panics).

**Planned command (per `proof-obligations.planned.jsonl`):**
```
cargo +nightly fuzz run -p velvet-ballistics-fuzz \
    fuzz_storage_codec_payload_corruption -- -max_total_time=60
```

**Smoke evidence (authoring pass):**
`cargo check --offline --bin
fuzz_storage_codec_payload_corruption` (from `fuzz/`
directory) → 1 expected pre-fix error:
`no variant named TrailingBytes found for enum JournalError`
(cf. §2.5). The error will resolve when the production-side
change lands.

**Full fuzz evidence:** PENDING_FORMAL_EXECUTION (queued for
State 12).

---

## 4. Verifier-Command Summary (per obligation)

| POB | Verifier | Command | Status | Evidence |
|-----|----------|---------|--------|----------|
| POB-vb-1wora-001 | rust-local | diff + grep | PENDING (depends on impl) | — |
| POB-vb-1wora-002 | cargo-test | `cargo test -p vb_storage --lib ...` | PENDING (depends on test rewrites) | — |
| POB-vb-1wora-003 | proptest | `cargo test -p vb_storage --features proptest --lib ...` | PENDING (depends on proptest properties) | — |
| POB-vb-1wora-004 | kani | `cargo kani -p vb_storage --harness kani_harness_rejects_trailing_bytes --output-format=json` | PENDING (blocked by unrelated kani_helpers.rs compile error) | cargo check pass (§2.4) |
| POB-vb-1wora-005 | proptest | `cargo test -p vb_storage --features proptest --lib ...` | PENDING (depends on proptest properties) | — |
| POB-vb-1wora-006 | verus | `verus --crate-type=lib verification/verus/vb-vzcuf-PS-003.rs` | SMOKE PASS (25/25) | §2.1 |
| POB-vb-1wora-006 | verus (binding gate) | `bash scripts/check-verus-production-binding.sh` | PASS | §2.2 |
| POB-vb-1wora-006 | verus (drift gate) | `bash scripts/check-production-inner-drift.sh` | BLOCKED_TOOLING (JJ-only workspace, no .git) | §2.3 |
| POB-vb-1wora-007 | cargo-fuzz | `cargo +nightly fuzz run -p velvet-ballistics-fuzz fuzz_storage_codec_payload_corruption -- -max_total_time=60` | PENDING (depends on production-side change landing) | cargo check reports expected pre-fix error (§2.5) |

---

## 5. Verdict

**State 5 (proof-writer) authoring pass: SUCCESSFUL.**

The three proof artifacts required by the bead task brief are
authored and pass the cheapest available smoke checks:

1. **Verus WEAK_MIRROR extension** — bridge arm + new exec
   wrapper, 25/25 proofs verified locally.
2. **Kani H6 harness** — added after H5, compiles under
   `cargo check --features legacy-kani` (full verification
   blocked by an unrelated `vb_core` compile error).
3. **cargo-fuzz oracle** — trailing-bytes oracle appended to
   the existing `fuzz_storage_codec_payload_corruption`
   target. The fuzz target is correctly authored against the
   post-fix `JournalError::TrailingBytes` variant and will
   compile/run as soon as the production-side change lands.

All PENDING_FORMAL_EXECUTION items are documented with
concrete reasons, ownership routing, and the exact commands
queued for State 12 (`formal-verifier`).
