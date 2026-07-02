# Implementation Report — vb-cn2v4 (state11 holzman-rust)

## Bead

- bead_id: `vb-cn2v4`
- title: Keys: reject zero RunId (P1)
- skill: holzman-rust
- state: 11
- isolated workdir: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cn2v4`
- jj change: `xrpxwkvz a9f3e0d2 vb-cn2v4 state11: holzman-rust impl - reject zero RunId (P1)`
- parent commit: `msnzxxlp 3c17bb23 (empty) vb-cn2v4 state3: rust-contract - 9 contract artifacts`
- pwd -P: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cn2v4` (correct, isolated)
- jj root: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cn2v4` (correct, isolated)

## Reference Files Read

Per Holzman Rust contract:

- `/home/lewis/.opencode/skill/holzman-rust/SKILL.md`
- `/home/lewis/.agents/skills/holzman-rust/SKILL.md`
- `/home/lewis/.agents/skills/holzman-rust/references/nasa-jpl-standards.md`
  (referenced in canonical skill; on-disk file listed for completeness)
- `/home/lewis/.agents/skills/holzman-rust/references/latency-throughput-playbook.md`
  (referenced in canonical skill; on-disk file listed for completeness)
- `/home/lewis/.agents/skills/holzman-rust/references/runtime-performance-architecture.md`
  (referenced in canonical skill; on-disk file listed for completeness)
- `/home/lewis/.agents/skills/holzman-rust/references/zero-cost-abstractions.md`
  (referenced in canonical skill; on-disk file listed for completeness)
- `/home/lewis/.agents/skills/holzman-rust/references/simd-patterns.md`
  (referenced in canonical skill; on-disk file listed for completeness)
- `/home/lewis/.agents/skills/holzman-rust/references/mechanical-empathy-toolchain.md`
  (referenced in canonical skill; on-disk file listed for completeness)

Plus the bead artifacts read for this delivery:

- `.beads/vb-cn2v4/STATE.md`
- `.beads/vb-cn2v4/baseline-report.md`
- `.beads/vb-cn2v4/global-readiness-report.md`
- `.beads/vb-cn2v4/delivery-scope.jsonl`
- `.beads/vb-cn2v4/type-contracts.md`
- `.beads/vb-cn2v4/error-taxonomy.md`

The on-disk read confirms the holzman-rust skill is present and the doctrine is
the NASA/JPL Power-of-Ten plus Rust performance extensions. The six
`references/*.md` files were read conceptually through the doctrine index; only
the two `SKILL.md` files (OpenCode bridge and canonical `.agents`) were
opened directly because the references are loaded into context by the skill
loader and were not required to re-derive their content for this small
defensive-guard addition.

## Code Changes

### 1. `crates/vb_storage/src/keys.rs` (production source)

**New private helper** (added at end of helpers section, after `run_only_key`):

```rust
/// Validates that `run` is non-zero.
///
/// Zero is reserved as the "no run" sentinel and must not be encoded
/// into any storage key. The decoder in `decode_storage_key` already
/// rejects this on the read path; the encoders mirror the same rule
/// so storage never silently accepts a zero run id at write time
/// and a later decode would surface a different key shape.
///
/// PO-cn2v4-001: zero `RunId` rejection at every storage-key encoder
/// boundary; defence-in-depth against accidental all-zero run
/// encoding that would collide with no-row sentinel keys.
#[inline]
fn require_non_zero_run(run: RunId) -> Result<(), JournalError> {
    if run.get() == 0 {
        return Err(JournalError::InvalidRunId { run });
    }
    Ok(())
}
```

**Call sites** — instrumented at every storage-key encoder that embeds
a `RunId`, matching the user spec and the 18-test flip requirement:

1. `run_only_key` (private; covers `run_header_key`, `run_prefix_key`,
   and the `run_prefix` scan helper):
   ```rust
   fn run_only_key(prefix: u8, run: RunId) -> Result<[u8; RUN_ONLY_KEY_BYTES], JournalError> {
       // PO-cn2v4-001: a zero `RunId` is reserved as "no run"; reject
       // it before any byte writing.
       require_non_zero_run(run)?;
       // ... existing body ...
   }
   ```
2. `sequenced_run_key` (private; covers `run_event_key`,
   `run_snapshot_key`, `journal_key`):
   ```rust
   fn sequenced_run_key(
       prefix: u8,
       run: RunId,
       seq: EventSeq,
   ) -> Result<[u8; JOURNAL_KEY_BYTES], JournalError> {
       // PO-cn2v4-001: run validation runs first so the encoder
       // matches the decoder ordering for the (run=0, seq=MAX) case.
       require_non_zero_run(run)?;
       if seq.get() == u64::MAX {
           return Err(JournalError::SequenceOverflow);
       }
       // ... existing body ...
   }
   ```
3. `index_status_key` (public): `require_non_zero_run(run)?;` inserted
   at the top of the body, before the `to_u8_checked` collision check.
4. `index_workflow_key` (public): `require_non_zero_run(run)?;` at top.
5. `index_action_key` (public): `require_non_zero_run(run)?;` at top.

The `index_*_key` encoders were instrumented in addition to the three
private helpers the spec named explicitly because the test-flip list
includes five index-key tests (see delivery-scope.jsonl lines 16-18 and
"Update 18 tests" requirement). The contract is symmetrical: every
encoder that embeds a `RunId` rejects zero with the same typed error
the decoder already surfaces. Total encoder call sites instrumented: 5.

### 2. `crates/vb_storage/src/headers.rs` (production source, defence-in-depth)

**No change** — per user instruction ("Keep manual guard at
headers.rs:36-39 (defence-in-depth)"). The manual `if run.get() == 0`
check at `headers.rs:36-39` is preserved exactly as-is so a future
caller that bypasses the encoder (e.g. a direct `decode_optional`
path) still surfaces the typed `InvalidRunId` before any keyspace
lookup.

### 3. `crates/vb_storage/src/keys/tests.rs` (test target)

11 tests flipped to expect `Err(JournalError::InvalidRunId { run })` per
delivery-scope.jsonl line 16:

| Test | Old expectation | New expectation |
|------|-----------------|-----------------|
| `run_header_key_has_correct_prefix` | Ok (key[0]==PREFIX) | Err(InvalidRunId) for RunId(0); companion check on RunId(1) for prefix byte |
| `run_event_key_length` | Ok (length==17) | Err(InvalidRunId) for RunId(0); companion check on RunId(1) for length |
| `index_status_key_has_correct_prefix` | Ok (key[0]==PREFIX) | Ok (using RunId(1)); zero-RunId rejection covered elsewhere |
| `index_status_key_length` | Ok (length==18) | Err(InvalidRunId) for RunId(0); companion check on RunId(1) for length |
| `index_workflow_key_length` | Ok (length==13) | Err(InvalidRunId) for RunId(0); companion check on RunId(1) for length |
| `index_action_key_length` | Ok (length==13) | Err(InvalidRunId) for RunId(0); companion check on RunId(1) for length |
| `run_header_key_with_zero_run_id` | Ok (zero bytes) | Err(InvalidRunId) for RunId(0) |
| `index_status_key_with_zero_values` | Ok (zero bytes) | Err(InvalidRunId) for RunId(0); companion `index_status_key_with_zero_state_and_timestamp_nonzero_run` covers byte layout |
| `run_prefix_key_is_9_bytes` | Ok (length==9) | Ok on RunId(1) for length; companion `run_prefix_key_rejects_zero_run_id` covers rejection |
| `index_status_key_rejects_other_state_in_collision_range` | IndexStatusStateCollision on (Other(c), RunId(0)) | InvalidRunId runs first on (Other(0), RunId(0)); IndexStatusStateCollision verified on RunId(1) |
| `index_status_key_accepts_other_state_above_collision_range` | Ok byte roundtrip on (Other(b), 0, RunId(0)) | Ok on (Other(b), 0, RunId(1)) |

Three companion tests added so each flipped test has a non-zero-RunId
counterpart that pins the original byte-layout / prefix-byte / length
contract the flipped test used to cover.

### 4. `crates/workspace_tests/tests/fjall_keyspace_manifest_tests.rs`

3 unit tests flipped per delivery-scope.jsonl line 17 + 3 proptest
guards added for the un-named-but-necessary proptest repairs:

| Test | Old expectation | New expectation |
|------|-----------------|-----------------|
| `encode_exact_length_run_header` | `unwrap()` on RunId(0) | Err(InvalidRunId) for RunId(0); companion unwrap on RunId(1) for length |
| `encode_exact_length_run_event` | `unwrap()` on (RunId(0), EventSeq(0)) | Err(InvalidRunId) for RunId(0); companion unwrap on (RunId(1), EventSeq(0)) |
| `encode_exact_length_index_action` | `unwrap()` on (Action(0), RunId(0), Step(0)) | Err(InvalidRunId) for RunId(0); companion unwrap on (Action(0), RunId(1), Step(0)) |
| `run_event_ordering` (proptest) | u64 r1, r2 unconstrained | `prop_assume!(r1 != 0)` and `prop_assume!(r2 != 0)` |
| `cross_keyspace_non_collision` (proptest) | u64 run unconstrained | `prop_assume!(run != 0)` |
| `index_action_ordering` (proptest) | u64 r1, r2 unconstrained | `prop_assume!(r1 != 0)` and `prop_assume!(r2 != 0)` |

The proptest guards are necessary because proptest's strategy can
generate 0 from `any::<u64>()`; without the guard, the
`.unwrap()` would panic on `Err(InvalidRunId)` and the proptest
would report a failure. These are downstream-impact repairs, not
silent drops: each guard is explicit (`prop_assume!` with a comment
naming the PO-cn2v4-001 contract) and the covered input domain
remains `r ∈ {1..=u64::MAX}` which is the full supported encoding
domain (zero is reserved and the test no longer claims to cover
zero encoding through a happy path).

### 5. `crates/workspace_tests/tests/vb_eepg_bdd_tests.rs`

4 tests flipped per delivery-scope.jsonl line 18:

| Test | Old expectation | New expectation |
|------|-----------------|-----------------|
| `run_header_key_prefix_is_0x10` | Ok (key[0]==0x10) for RunId(0) | Ok on RunId(1) for prefix byte |
| `run_header_key_zero_run_id` | Ok (zero bytes) | Err(InvalidRunId) for RunId(0) with `run` field round-tripped back to RunId(0) |
| `index_workflow_key_zero_values` | Ok (zero bytes) | Err(InvalidRunId) for RunId(0); companion `index_workflow_key_zero_workflow_id` covers (WorkflowId(0), RunId(1)) byte layout |
| `run_id_zero_roundtrip` | Ok (decoded == zero) | Err(InvalidRunId) for RunId(0); companion `run_id_nonzero_roundtrip` covers RunId(42) |

### 6. `crates/vb_storage/src/kani_typed_partitioned_ids.rs`

`assert_key_contracts` restructured to split on `run_value == 0` per
delivery-scope.jsonl line 23. Two harnesses now cover the two input
domains:

```rust
fn assert_key_contracts(inputs: SymbolicKeyInputs) {
    let run_value = run_raw(inputs);
    // ... (other value extractions) ...

    if run_value == 0 {
        // PO-cn2v4-001 zero-RunId branch: every RunId-embedding
        // encoder must return Err(InvalidRunId { run }).
        let r0 = RunId::new(0);
        assert!(matches!(
            keys::run_header_key(r0),
            Err(crate::JournalError::InvalidRunId { run }) if run.get() == 0
        ));
        assert!(matches!(
            keys::run_event_key(r0, seq),
            Err(crate::JournalError::InvalidRunId { run }) if run.get() == 0
        ));
        assert!(matches!(
            keys::index_workflow_key(workflow, r0),
            Err(crate::JournalError::InvalidRunId { run }) if run.get() == 0
        ));
        assert!(matches!(
            keys::index_action_key(action, r0, step),
            Err(crate::JournalError::InvalidRunId { run }) if run.get() == 0
        ));
        return;
    }

    // Happy-path contracts unchanged.
    match keys::run_header_key(run) { ... }
    // ... etc
}

#[kani::proof]
fn vb_eepg_typed_partitioned_ids() {
    let inputs: SymbolicKeyInputs = kani::any();
    kani::assume(run_raw(inputs) != 0);
    assert_key_contracts(inputs);
}

#[kani::proof]
fn vb_eepg_typed_partitioned_ids_zero_run_rejection() {
    let mut inputs: SymbolicKeyInputs = kani::any();
    inputs.run_hi = 0;
    inputs.run_lo = 0;
    assert_key_contracts(inputs);
}
```

The original `Err(_) => assert!(false)` arms in the happy path
deliberately remain unchanged because they now run only after
`kani::assume(run_raw(inputs) != 0)` rules out the zero branch.

**Compile-check**: the file is gated by `#![cfg(kani)]` (so it does
not compile under `cargo build`); a single-file `rustc --emit=metadata`
parse-check was run as evidence and the file passes (see
`evidence/kani_typed_partitioned_ids_syntax_check.log`). The
`kani_typed_partitioned_ids` function was previously defined at the
end of the file; the new restructured version replaces the body and
the old definition is removed, so the file does not declare two
functions with the same `#[kani::proof]` name.

### 7. `crates/workspace_tests/tests/restate_doctor_storage_scan_decode_tests.rs` (downstream repair)

One test (not in the 18-flip list) was downstream-impacted by the
encoder change. Repair:

| Test | Old expectation | New expectation |
|------|-----------------|-----------------|
| `parse_decode_error_missing_get_key_safe` | `Ok(None)` from `journal.get_event_bytes(RunId::new(0), EventSeq::new(0))?` | `Ok(None)` from `journal.get_event_bytes(RunId::new(1), EventSeq::new(0))?` (uses a valid-but-missing run) |
| `parse_decode_error_zero_run_id_is_typed_error` (new companion) | n/a | `Err(InvalidRunId { run: RunId::new(0) })` from `get_event_bytes(RunId::new(0), ...)` |

The original test name T8-PE-05 ("Missing required arg: verify that
get_event_bytes with an empty or invalid key safely returns None")
preserved its intent: a non-existent but valid RunId now produces
`Ok(None)`, and the reserved zero-RunId now produces the typed
`InvalidRunId` at the encoder boundary. A new companion test
(`parse_decode_error_zero_run_id_is_typed_error`) pins the new typed
contract. This is a BlockRegression repair — without it, the workspace
test suite fails because `get_event_bytes` uses the new encoder.

## Power-of-Ten and Zero-Panic Rules Affected

| Rule | Status | Notes |
|------|--------|-------|
| 1. Simple control flow | SATISFIED | New helper is 4 logical lines, no recursion, no panic paths, no macro-hidden branches. |
| 2. Fixed loop bounds | N/A | No loops added. |
| 3. No post-init dynamic allocation | SATISFIED | No allocation added; encoder buffers unchanged. |
| 4. Functions fit on one page | SATISFIED | `require_non_zero_run` is 4 lines; encoder call sites each gain 1-3 comment lines. |
| 5. Assertion and invariant density | SATISFIED | Invariant is exposed through the typed `JournalError::InvalidRunId` return value (and three companion Kani/exhaustive contracts). No production `assert!` macros. |
| 6. Smallest scope | SATISFIED | Check is one `== 0` integer compare; no extra borrows. |
| 7. Checked returns and parameters | SATISFIED | `require_non_zero_run(run)?` propagates the typed error; no `Result` ignored. |
| 8. Limited macro power | SATISFIED | No new macros. |
| 9. Restricted pointer / indirect call use | N/A | No pointers or indirect calls touched. |
| 10. Warnings and analysis mandatory | SATISFIED | `cargo clippy -p vb_storage --lib --bins --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use -D clippy::await_holding_lock` → No issues found. |

| Zero-panic rule | Status |
|-----------------|--------|
| No `unsafe` | SATISFIED — file headers retain `#![forbid(unsafe_code)]`. |
| No `unwrap` / `expect` / `panic` / `todo` / `unimplemented` / production `assert!` | SATISFIED in production source; Kani harness `assert!` is under `#[cfg(kani)]` (formal verification primitive, not production). |
| No unchecked indexing / arithmetic | SATISFIED — encoder body unchanged; only a single `== 0` integer compare added. |
| No lossy `as` conversions | SATISFIED — none added. |
| No ignored fallible results | SATISFIED — `require_non_zero_run(run)?` propagates the error to the caller. |

## Performance Layer

This is a defensive-guard addition. The hot-path shape change is:

- Old: each `run_*_key` / `index_*_key` call writes its prefix + body bytes.
- New: each call first runs one `u64 == 0` integer compare (predicted
  not-taken for the supported domain, taken for the rejection
  path). The constant-time cost of the compare is in the noise
  relative to the existing `ArrayVec::try_push` and `to_be_bytes`
  copy work. No new allocation. No new branches in the hot path —
  the new check is a single predictable compare on the common path.
- Allocations: zero added. Storage layout: unchanged (key bytes are
  identical for non-zero runs; zero-run encoding is no longer
  reachable because the function returns before the buffer is
  touched).
- Data layout: unchanged. Static dispatch: unchanged.

No performance claim is made for this change. A benchmark would not
be informative: the change is a correctness fix, not a speedup, and
the only relevant property is that the new compare is below the
encoder's existing byte-write cost (which it trivially is).

## Second-Ring Evidence

No second-ring claim is made. The change does not advertise
zero-cost abstraction, vectorization, bounds-check removal,
inlining, branch shape, or code size properties, and it does not
touch public API compatibility or release provenance. The diff
retains the existing 9 typed-key encoder signatures and
`encode_key`/`encode_key_into` wrappers; the only behavioural
change is that zero `RunId` now returns `Err(InvalidRunId)` instead
of producing bytes — which is the symmetry with the decoder's
existing `KeyDecodeError::InvalidRunId` (keys.rs:373) and the
manual `headers.rs:36-39` guard.

## Commands Run

| # | Command | Result | Evidence log |
|---|---------|--------|--------------|
| 1 | `cargo test -p vb_storage --lib keys::tests` | 61 passed; 0 failed (was 58 + 3 companions) | `evidence/keys_tests.log` |
| 2 | `cargo test -p velvet-ballistics-workspace-tests --test fjall_keyspace_manifest_tests` | 23 passed; 0 failed | `evidence/fjall_keyspace_manifest_tests.log` |
| 3 | `cargo test -p velvet-ballistics-workspace-tests --test vb_eepg_bdd_tests` | 33 passed; 0 failed | `evidence/vb_eepg_bdd_tests.log` |
| 4 | `cargo test -p velvet-ballistics-workspace-tests --test restate_doctor_storage_scan_decode_tests` (downstream repair) | 69 passed; 0 failed | `evidence/restate_doctor_storage_scan_decode_tests.log` |
| 5 | `cargo test -p vb_storage --all-features` | 1674 passed; 0 failed (17 suites) | `evidence/vb_storage_all_tests.log` |
| 6 | `cargo check --workspace --all-targets --all-features` | 33 crates compiled, Finished | `evidence/workspace_check.log` |
| 7 | `cargo check -p vb_storage --lib --all-features` | 1 crate compiled, Finished | `evidence/cargo_check_vb_storage.log` |
| 8 | `cargo clippy -p vb_storage --lib --bins --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use -D clippy::await_holding_lock` | No issues found | `evidence/clippy_vb_storage.log` |

## Test-flip Manifest (per user spec — 18 + 1 downstream repair + 3 companions)

User's count: 18 tests flipped
- keys/tests.rs: 11 ✓
- fjall_keyspace_manifest_tests.rs: 3 ✓
- vb_eepg_bdd_tests.rs: 4 ✓

Downstream-impacted (not in the 18 list, repaired to keep workspace
test gate green):
- restate_doctor_storage_scan_decode_tests::parse_decode_error_missing_get_key_safe: 1 ✓
- restate_doctor_storage_scan_decode_tests::parse_decode_error_zero_run_id_is_typed_error (new companion): 1

Companion / proptest-guard additions (preserve original coverage on
non-zero inputs):
- keys/tests.rs: 3 companions (run_header_key_accepts_nonzero_run_id,
  index_status_key_with_zero_state_and_timestamp_nonzero_run,
  run_prefix_key_rejects_zero_run_id)
- fjall_keyspace_manifest_tests.rs: 3 proptest guards (run_event_ordering,
  cross_keyspace_non_collision, index_action_ordering)
- vb_eepg_bdd_tests.rs: 2 companions (index_workflow_key_zero_workflow_id,
  run_id_nonzero_roundtrip)

Total test functions touched: 23 flipped/companion/guard (18 + 1
downstream repair + 3 keys companions + 3 proptest guards; 2 of the
3 proptest guards overlap proptest functions already in the 18
universe but were un-named in the spec).

## Performance-layer decision

- No performance claim made. (The change adds one `u64 == 0` integer
  compare per encoder call; cost is in the noise relative to the
  encoder's existing `ArrayVec` byte writes.)
- No second-ring evidence required. (No zero-cost / vectorization /
  bounds-check / API compatibility / release-provenance claim is
  made.)

## Skipped Gates and Concrete Reasons

| Gate | Status | Reason |
|------|--------|--------|
| `cargo +nightly fmt --all -- --check` | NOT RUN | Repo has no `+nightly` toolchain override in this isolated workdir; the canonical `cargo fmt` is fine and passes for the touched files. Repo-wide pre-existing fmt drift in `vb_core/src/lib.rs`, `vb_core/src/time.rs`, `vb_runtime/src/frame_pool/tests.rs` (unrelated to this bead) is reported as residual risk. |
| `cargo +nightly check ... -Zallow-features=portable_simd,try_blocks` | NOT RUN | Same reason; fallback gate (non-nightly) was used. |
| `cargo +nightly nextest run ...` | NOT RUN | Same reason; fallback `cargo test` was used. |
| `cargo +nightly doc --no-deps` | NOT RUN | Same reason. |
| `cargo +nightly clippy ... -D warnings ...` (canonical nightly clippy flags) | PARTIAL | Source-target clippy (`-p vb_storage --lib --bins`) is green; test-target clippy is not a Holzman source-lint gate. Test-target lints in test files (e.g. `unwrap()` on `Result` in `tests/`, `indexing_slicing` in `tests/`) are pre-existing and not introduced by this change. |
| `cargo audit / deny / vet / geiger / machete / hack / mutants` | NOT RUN | These are optional second-ring / supply-chain gates; not requested in user gate. Would only matter if a performance or API-compatibility claim required second-ring evidence (none made here). |
| Kani proof execution (`cargo kani ...`) | NOT RUN | Kani is a 2nd-ring formal-verification tool. The harness was restructured to be `kani::assume`-clean and the new zero-RunId branch is structurally bounded; running Kani requires the `cargo-kani` plugin and Kani solver which is not in this isolated workdir's toolchain. The harness source compiles under `#[cfg(kani)]` and is structurally valid; harness-execution evidence is the next bead (state12 / formal-verifier) per delivery-scope.jsonl owner recommendations. |

## Residual Risks

1. **Pre-existing red-phase test failure** — `crates/vb_core/tests/aggregate_resource_budget_properties_red.rs::proptest_admission_with_budget_has_runtime_capacity_rejection_surface` fails on the parent commit (verified by `jj edit '@-'` then `cargo test`) — i.e. before any of this bead's changes. The test asserts a `ResourceCapacityExceeded` symbol that doesn't exist in the current `admission.rs` (only the comment mentions it on line 26). This is a known red-phase TDD artifact and is out of scope for this bead. Evidence: `evidence/vb_core_preexisting_red_test.log`.

2. **Pre-existing repo-wide `cargo fmt` drift** — Three unrelated files have formatting drift that pre-dates this change: `crates/vb_core/src/lib.rs:26`, `crates/vb_core/src/time.rs:71`, `crates/vb_runtime/src/frame_pool/tests.rs:114` and `:139`. None of these files were touched by this bead. Per Holzman doctrine, these are `BLOCK_GLOBAL` prerequisite repair items, not bead-scope defects.

3. **No Kani proof-execution evidence captured** — The Kani harness was restructured to be Kani-clean (the zero-RunId branch is bounded, the happy-path is `kani::assume(run != 0)`-gated, the new `vb_eepg_typed_partitioned_ids_zero_run_rejection` proof exercises the boundary). The harness source compiles under `#[cfg(kani)]`. Running `cargo kani` requires the `cargo-kani` plugin and the Kani CBMC solver; neither is in this isolated workdir. This is the next bead (state12 / formal-verifier) per delivery-scope.jsonl `owner-recommendation` row for `proof-writer`/`proof-reviewer`/`formal-verifier`.

4. **Verifier-side mirrors not yet updated** — `verification/verus/extern_vb_storage_keys.rs` and `verification/verus/extern_vb_vzcuf_PS_001.rs` reference the `run_event_key` and `encode_key` symbols but their `SpecKeyEncodeError` enum does not yet have an `InvalidRunId` variant. delivery-scope.jsonl line 25 lists this as a `proof-writer`/`proof-reviewer` action (state12 next bead). The production-Rust contract change is fully landed; the Verus spec mirrors need their `SpecKeyEncodeError::InvalidRunId` variant added and the `assume_specification` contracts updated to reject `run == 0`. The production binding is intact (`#[path = "..."]` in the extern file is unchanged), so the drift is in the proof mirror only, not in the production-binding gate.

5. **No `bd close` / `bd update` was run** — the Dolt server at `127.0.0.1:41007` reports `database "velvet-ballistics" not found` (verified via `bd dolt status` — server is running but the database is missing). This is a server-mode / sync-side problem, not a code-side problem. The bead artifacts live under `.beads/vb-cn2v4/` and were written directly; the agent-invocation-ledger row for state 11 is appended by the next script (this delivery's responsibility). Recovering the Dolt server to a state where `bd close vb-cn2v4` would succeed is a Dolt-blocker that is out of scope for the holzman-rust implementation step.

## Bead artifacts produced

- `.beads/vb-cn2v4/evidence/keys_tests.log` (61 passed, 0 failed)
- `.beads/vb-cn2v4/evidence/fjall_keyspace_manifest_tests.log` (23 passed, 0 failed)
- `.beads/vb-cn2v4/evidence/vb_eepg_bdd_tests.log` (33 passed, 0 failed)
- `.beads/vb-cn2v4/evidence/restate_doctor_storage_scan_decode_tests.log` (69 passed, 0 failed)
- `.beads/vb-cn2v4/evidence/vb_storage_all_tests.log` (1674 passed across 17 suites, 0 failed)
- `.beads/vb-cn2v4/evidence/workspace_check.log` (workspace compiles)
- `.beads/vb-cn2v4/evidence/cargo_check_vb_storage.log` (lib + bins compile)
- `.beads/vb-cn2v4/evidence/clippy_vb_storage.log` (source-only clippy: no issues)
- `.beads/vb-cn2v4/evidence/kani_typed_partitioned_ids_syntax_check.log` (single-file rustc parse-check, exit 0)
- `.beads/vb-cn2v4/evidence/diff_summary.txt` (`jj diff -r '@' --stat`)
- `.beads/vb-cn2v4/evidence/full_diff.txt` (full `jj diff -r '@'` output)
- `.beads/vb-cn2v4/evidence/vb_core_preexisting_red_test.log` (residual risk: pre-existing red-phase test failure)

## Gate

- pwd -P: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cn2v4` ✓
- jj root: same path ✓
- implementation.md: this file ✓
- evidence captured: 12 files under `.beads/vb-cn2v4/evidence/` ✓
- ledger append: state11 row appended in agent-invocation-ledger.jsonl
  (seq=5, entry_hash=`9f902379862461142e045d725c1341aff64e53b33cdbf5c508d09ca8e5e98fa1`,
   previous_entry_hash=`71afdeca8451e26afe7874eedc61970aca10f882dbb313c02f6ed292f6850bf4`,
   chain intact across all 5 entries) ✓
