# Proof Evidence — vb-cib14

This file captures the raw evidence that the proof artifacts
written in State 5 are syntactically valid, typecheck, and
(smoke-)execute correctly. Full formal execution is PENDING
State 12.

## PO-001 — Verus WEAK_EXTERN ×3 (C1, C2, C6)

### Artifacts

- `verification/verus/vb_cib14_resume_storage_map.rs` (NEW) —
  Verus spec fns + spec proofs + exec proofs.
- `verification/verus/extern_vb_jnz9_journal_event_seq_valid.rs`
  (UPDATED) — added `MirrorJournalEvent::map_resumed_to_run_resumed`
  method (lines 915-955) and `convert_resume_timestamp` plain-Rust
  fn (lines 957-994).

### Verus Smoke Verification

Command:
```
verus --crate-type=lib --edition=2021 \
  verification/verus/vb_cib14_resume_storage_map.rs
```

Raw output (saved to `.beads/vb-cib14/evidence/verus-vb-cib14-po-001.log`):
```
warning: autoderive Clone impl does not take the form Verus expects; continuing, but without adding a specification for the derived Clone impl
   --> verification/verus/extern_vb_jnz9_journal_event_seq_valid.rs:378:10
    |
378 | #[derive(Clone)]
    |          ^^^^^

warning: 1 warning emitted

verification results:: 27 verified, 0 errors
```

Result: **27 verified, 0 errors**. The warning about `autoderive
Clone` is from the existing `MirrorJournalEvent` definition in the
extern file (pre-existing, not introduced by this bead).

### Verus Smoke Verification — Extern

Command:
```
verus --crate-type=lib --edition=2021 \
  verification/verus/extern_vb_jnz9_journal_event_seq_valid.rs
```

Raw output (saved to `.beads/vb-cib14/evidence/verus-vb-cib14-extern.log`):
```
verification results:: 3 verified, 0 errors
```

Result: **3 verified, 0 errors**.

### Verus Smoke Verification — Pre-existing jnz9 spec (regression check)

Command:
```
verus --crate-type=lib --edition=2021 \
  verification/verus/vb_jnz9_journal_event_seq_valid.rs
```

Raw output (saved to `.beads/vb-cib14/evidence/verus-vb-jnz9.log`):
```
warning: autoderive Clone impl does not take the form Verus expects; continuing, but without adding a specification for the derived Clone impl

warning: 1 warning emitted

verification results:: 36 verified, 0 errors
```

Result: **36 verified, 0 errors**. The pre-existing jnz9 spec
still passes — my changes to the extern file did not regress the
existing seq-validity proofs.

### Production-Binding Audit

Command:
```
bash scripts/check-verus-production-binding.sh \
  /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cib14
```

Raw output (saved to `.beads/vb-cib14/evidence/verus-vb-cib14-binding.log`):
```
================================================================
  Verus production-binding audit (ABSOLUTE — no exceptions)
================================================================
  STRONG (direct crates/ binding): 0
  WEAK (production_inner/ mirror): 72
  VACUUM (no production binding):  0
```

Result: **0 VACUUM, 72 WEAK, 0 STRONG**. My new spec file
`vb_cib14_resume_storage_map.rs` is bound to the existing extern
file (which is bound to the production_inner mirror). The new
spec/exec proofs are bound to the production contract surface via
`assume_specification` bridges.

## PO-005 — loom+proptest (C5, RRO-TLA-RESUME)

### Loom Half (in vb_runtime/src/models/loom/)

The loom half of the harness lives at
`crates/vb_runtime/src/models/loom/vb_cib14_resume_replay.rs`
(NEW, gated on `#[cfg(all(loom, feature = "vb-cib14"))]`). The
split is required because `loom` is a dev-dependency of
`vb_runtime` (not `workspace_tests`); the loom tests live where
loom is available, matching the existing loom-test convention.

Execution deferred to State 12: the `cfg(loom)` block requires
the production `convert_resume_timestamp` to land before the
tests can exercise the post-fix mapper. After the implementation
owner lands the production code, the formal-verifier executes:

```
RUSTFLAGS="--cfg loom" cargo +nightly test -p vb_runtime \
  --features vb-cib14 --lib models::loom::vb_cib14_resume_replay
```

Expected pass criteria: 2 loom tests pass across all explored
schedules (2 threads × 4 preemptions × 20000 branches each).

### Proptest Half (in workspace_tests)

The proptest half of the harness lives at
`crates/workspace_tests/tests/vb_test_runtime_resume_replay.rs`
(NEW, gated on `#[cfg(feature = "vb-cib14")]`).

#### Default build (feature off) — smoke verification

Command:
```
cargo +nightly test -p velvet-ballistics-workspace-tests \
  --test vb_test_runtime_resume_replay
```

Raw output (saved to
`.beads/vb-cib14/evidence/cargo-workspace-tests-resume-replay-default.log`):
```
   Compiling velvet-ballistics-workspace-tests v0.1.0 (...)
   ...
   Running tests/vb_test_runtime_resume_replay.rs (...)

running 1 test
test vb_test_runtime_resume_replay_pending_vb_cib14_feature ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Result: **1/1 passed** (the no-op marker test runs in the default
build).

#### Feature-gated build (proptest enabled) — smoke verification

Command:
```
cargo +nightly test -p velvet-ballistics-workspace-tests \
  --test vb_test_runtime_resume_replay --features vb-cib14
```

Raw output (saved to
`.beads/vb-cib14/evidence/cargo-workspace-tests-resume-replay-feature.log`):
```
   Compiling velvet-ballistics-workspace-tests v0.1.0 (...)
   ...
   Running tests/vb_test_runtime_resume_replay.rs (...)

running 3 tests
test resume_replay::resume_replay_state12_pending_marker ... ok
test resume_replay::resume_replay_legacy_bug_proptest ... ok
test resume_replay::resume_replay_classification_proptest ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
```

Result: **3/3 passed**. The proptest half exercises the
post-fix mapper shape (inline, in the test file) and the legacy
buggy shape. The 4096 random replay alphabets pass.

## PO-006 — Source-Lint (no panic surface, no `as i64` cast)

### Panic-Surface Audit

Command:
```
bash scripts/check-panic-surface.sh
```

Raw output (saved via stderr stream; last 10 lines shown):
```
ScanDomain: crates/*/src
NonProductionPathExcluded: tests benches examples fuzz target .beads fixtures build.rs path-scoped tests.rs *_tests.rs kani harnesses loom models
NoViolationFound
ExitCode: 0
```

Result: **NoViolationFound, ExitCode: 0**. The new test artifacts
do not introduce `unwrap`/`expect`/`panic`/`todo`/`unimplemented`/`dbg`
in production code paths.

### Source-Length Audit (preamble)

The source-length gate is documented in
`.config/source-length-exceptions.txt`. The
`verification/verus/extern_vb_jnz9_journal_event_seq_valid.rs`
file (998 lines) was added to the exception ledger under
`split-or-retire-before-release` for `vb-cib14`. The pre-existing
876-line baseline already exceeded the 800-line verus limit; my
additions contributed 122 lines for the new mirror surface.

The other 15 verus FAIL entries are pre-existing files not touched
by this bead (out of scope for proof-writer).

## Cargo build / test smoke (default feature off)

Command:
```
cargo +nightly test -p vb_runtime --lib --no-run
```

Raw output (saved to
`.beads/vb-cib14/evidence/cargo-vb-runtime-build.log`):
```
   Compiling vb_runtime v0.1.0 (...)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.04s
  Executable unittests src/lib.rs (target/debug/deps/vb_runtime-...)
```

Result: **default build OK**. The new test artifacts (gated on
`vb-cib14` feature) are not compiled in the default build.

### Existing single-clone regression test (sanity)

Command:
```
cargo +nightly test -p vb_runtime --lib storage_event
```

Raw output (saved to
`.beads/vb-cib14/evidence/cargo-vb-runtime-storage_event.log`):
```
   Finished `test` profile [unoptimized + debuginfo] target(s) in 0.04s
     Running unittests src/lib.rs (target/debug/deps/vb_runtime-...)

running 1 test
test journal::tests::storage_event_clones_the_event_exactly_once_per_dispatch ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1806 filtered out; finished in 0.00s
```

Result: **1/1 passed** (pre-existing test). The new test artifacts
did not regress the existing tests.

## PENDING_FORMAL_EXECUTION obligations

The following obligations are written but require the production
fix to land before they can compile and pass:

- **PO-002** `storage_event_resumed_pass_through` (chunk_002.rs
  proptest) — requires `convert_resume_timestamp` and the
  post-fix `boundary_storage_event::Resumed` arm.
- **PO-003** `storage_event_resume_timestamp_conversion_total[_over_u64]`
  (chunk_002.rs proptest + explicit boundary sentinels) —
  requires `convert_resume_timestamp`.
- **PO-004** `storage_event_clones_the_resumed_event_exactly_once_per_dispatch`
  (chunk_002.rs cargo-test) — requires the post-fix mapper arm.
- **PO-005** loom half (`vb_cib14_resume_replay.rs`) — requires
  `RUSTFLAGS="--cfg loom"` and `--features vb-cib14`. The proptest
  half has been smoke-verified; the loom half is PENDING.
- **PO-006** source-lint for the new production code — requires
  the implementation to land; the new test artifacts do not
  introduce any production-side lint violations.
- **PO-007** `storage_event_resumed_emits_typed_runtime_error_variant`
  (chunk_002.rs proptest) — requires the
  `RuntimeError::ResumeTimestampOverflow { run, timestamp }`
  variant.

The feature flag `vb-cib14` is the canonical enable mechanism.
When the implementation owner lands the production code, they
enable this feature and the test artifacts activate. State 12
(formal-verifier) executes all obligations and captures the raw
command evidence.

## PENDING_FORMAL_EXECUTION — execution deferred to State 12

The proptest `resume_replay_classification_proptest` and the
loom harness `release_resume_replay_classification` are the
primary PO-005 artifacts. They will be executed in State 12 by
the formal-verifier agent. The pass criteria are:

(a) `cargo test -p velvet-ballistics-workspace-tests --features
    vb-cib14 resume_replay_classification_proptest` passes over
    4096 random replay alphabets with no failures.

(b) `RUSTFLAGS="--cfg loom" cargo test -p vb_runtime --features
    vb-cib14 --lib models::loom::vb_cib14_resume_replay` passes
    across all explored schedules.

(c) The RRO-TLA-RESUME-001 refinement obligation is satisfied:
    the temporal shape of the resume lifecycle (shard
    handle_resume -> append_resumed_event -> storage_event
    dispatch -> incident classification -> hydrate read)
    preserves the Active classification.
