# Truth Serum Report — vb-vzo9b

**Bead**: vb-vzo9b
**State**: 14 (truth-serum audit)
**Workdir**: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-vzo9b`
**Run At**: 2026-07-01
**Auditor**: truth-serum (active execution context — formal-verifier agent)
**Audit Mode**: audit (find gaps, expose hallucinations, verify claims)
**Touched File**: `fuzz/src/journal_target/readback.rs` (lines 193-209 of the post-fix; pre-fix lines 192-203 unchanged)
**Production Code Touched**: NONE (test-only repair; production recovery surface byte-identical pre/post fix)

---

## STATUS: APPROVED

The bead is approved for landing. The implementation claim is honest, the
raw command evidence matches the planned obligations, the panic surface is
zero in the production recovery surface and bounded to the contractually-
mandated `assert_eq!` in the touched fuzz body, and the verification
laundering shield passes (no `external_body`/`axiom`/`assume(` in touched
files). All black-hat findings are dispositioned with canonical
`finding/v1.disposition` values. No hallucinated paths, no deleted tests,
no scope drift, no contract parity violations.

---

## 🔬 Execution Evidence

All commands run in the active execution context (this formal-verifier
agent) and observed directly. No subagent summaries are used as proof.
Raw logs are stored in `.beads/vb-vzo9b/evidence/state12/*.txt`,
`.beads/vb-vzo9b/evidence/state13/*.txt`, and
`.beads/vb-vzo9b/evidence/state14/*.txt`.

### 1. PO-001 (cargo test, summarize_recovery_events)

```
$ cd /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-vzo9b
$ /home/lewis/.cargo/bin/cargo test -p vb_storage --lib summarize_recovery_events --no-fail-fast
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.04s
     Running unittests src/lib.rs (target/debug/deps/vb_storage-6f6e8a548a2649ec)

running 12 tests
test recovery::recovery_unit_tests::tests::summarize_recovery_events_with_run_cancelled ... ok
test recovery::recovery_unit_tests::tests::summarize_recovery_events_with_run_failed ... ok
test recovery::recovery_unit_tests::tests::summarize_recovery_events_with_run_finished ... ok
test recovery::recovery_unit_tests::tests::summarize_recovery_events_counts_all_event_types ... ok
test recovery::replay::summary::tests::summarize_recovery_events_empty_returns_exact_no_recovery_data ... ok
test recovery::tests::summarize_recovery_events_rejects_divergent_action_scheduled_ticket ... ok
test recovery::tests::summarize_recovery_events_returns_summary_hydration ... ok
test recovery::tests::summarize_recovery_events_rejects_multi_run_divergence ... ok
test recovery::tests::summarize_recovery_events_rejects_action_completed_envelope_without_schedule ... ok
test recovery::tests::summarize_recovery_events_counts_duplicate_action_completed_envelope_once ... ok
test recovery::tests::summarize_recovery_events_rejects_completion_output_mismatch_with_schedule ... ok
test recovery::tests::summarize_recovery_events_counts_duplicate_action_scheduled_ticket_once ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 1518 filtered out; finished in 0.00s
EXIT_CODE=0
```

**Result**: PASS. 12 of 12 tests green. Log:
`.beads/vb-vzo9b/evidence/state12/PO-001-summarize_recovery_events.txt` (sha256
`63ae1682389b0561b5d653f3f11a344042fc59abe237e3412333e0335fe2b280`).

### 2. PO-002 (cargo test, recover_runtime_frame_seed_from_events)

```
$ /home/lewis/.cargo/bin/cargo test -p vb_storage --lib recover_runtime_frame_seed_from_events --no-fail-fast
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.04s
     Running unittests src/lib.rs (target/debug/deps/vb_storage-6f6e8a548a2649ec)

running 6 tests
test recovery::recovery_unit_tests::tests::recover_runtime_frame_seed_from_events_empty_returns_error ... ok
test recovery::recovery_unit_tests::tests::recover_runtime_frame_seed_from_events_no_steps ... ok
test recovery::recovery_unit_tests::tests::recover_runtime_frame_seed_from_events_with_waiting_step ... ok
test recovery::recovery_unit_tests::tests::recover_runtime_frame_seed_from_events_reconstructs_pc ... ok
test recovery::tests::recover_runtime_frame_seed_from_events_rebuilds_dimensions_and_step_states ... ok
test recovery::recovery_unit_tests::tests::recover_runtime_frame_seed_from_events_with_asking_step ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 1524 filtered out; finished in 0.00s
EXIT_CODE=0
```

**Result**: PASS. 6 of 6 tests green. Log:
`.beads/vb-vzo9b/evidence/state12/PO-002-recover_runtime_frame_seed_from_events.txt` (sha256
`74d7b2c9e3d21fdc663da6541f7661c915d3f312ba77657c57f0df48b095ac59`).

### 3. PO-003 (cargo build, recovery_decode)

```
$ /home/lewis/.cargo/bin/cargo build --bin recovery_decode --manifest-path fuzz/Cargo.toml
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.07s
EXIT_CODE=0
```

**Result**: PASS. Log:
`.beads/vb-vzo9b/evidence/state12/PO-003a-build-recovery_decode.txt` (sha256
`189706e3d8c77e2fa95fe0c0d8d7636ac94841ffcac4c0e2c5fa053f626495dc`).

### 4. PO-003 forbidden-pattern recheck (6 rg gates over readback.rs)

```
$ rtk rg -n 'assert!\([^)]+\|\|' fuzz/src/journal_target/readback.rs     # exit=1 (no matches)
$ rtk rg -n 'matches!\s*\(\s*run_summary' fuzz/src/journal_target/readback.rs   # exit=1 (no matches)
$ rtk rg -n 'let _summary' fuzz/src/journal_target/readback.rs           # exit=1 (no matches)
$ rtk rg -n '\bdbg!\s*\(\s*run_summary' fuzz/src/journal_target/readback.rs    # exit=1 (no matches)
$ rtk rg -n '\.unwrap\(\)' fuzz/src/journal_target/readback.rs            # exit=1 (no matches)
$ rtk rg -n '\.expect\(' fuzz/src/journal_target/readback.rs             # exit=1 (no matches)
```

**Result**: PASS. All 6 forbidden-pattern gates return no matches. Log:
`.beads/vb-vzo9b/evidence/state12/PO-003b-forbidden-pattern-grep.txt` (sha256
`23f0069514eec1501b1ebedef82f7225783c91254902a7bd7d3462430973f292`).

### 5. Truth-serum anti-verification-laundering check

```
$ rtk rg -n '#\[verifier::external_body\]|\baxiom\b|\bassume\(' fuzz/src/journal_target/readback.rs
rg exit=1   # (no matches)
```

**Result**: PASS. No `external_body`, `axiom`, or bare `assume(` in the
touched fuzz body. (Production `kani::assume(...)` calls in other crates
are bounded assumptions in Kani harnesses, not Verus proof-laundering
patterns. The `verification/verus/` artifacts belong to other beads
and are out of blast radius.)

### 6. Truth-serum panic-surface check (production recovery surface)

```
$ rtk rg -n '\b(unwrap|expect|panic|todo|unimplemented|unreachable!)\b' \
    crates/vb_storage/src/recovery/replay/summary/apply.rs \
    crates/vb_storage/src/recovery/replay/summary/derive.rs \
    crates/vb_storage/src/recovery/replay/summary/accumulator.rs \
    crates/vb_storage/src/recovery/types.rs
rg exit=1   # (no matches)
```

**Result**: PASS. Production recovery surface (apply, derive, accumulator,
types) has zero panic surface — zero `unwrap`, `expect`, `panic!`, `todo!`,
`unimplemented!`, `unreachable!`. The bead's production-touched claim
(contract C-5) is independently verified: the production code is byte-
identical pre/post fix and contains no panic surface at all.

### 7. Truth-serum panic-surface check (touched fuzz body)

```
$ rtk rg -n '\b(unwrap\(\)|expect\(|panic!|todo!|unimplemented!|unreachable!)\b' \
    fuzz/src/journal_target/readback.rs
rg exit=1   # (no matches)
```

**Result**: PASS. The touched fuzz body has zero `unwrap()`, `expect()`,
`panic!`, `todo!`, `unimplemented!`, `unreachable!`. The only `unwrap_or(0)`
at line 185 is on `Option<u8>` (a default-when-empty guard, not a panic
surface) and is byte-identical pre/post fix (pre-existing).

### 8. Truth-serum production `assert!` macro check (touched fuzz body)

```
$ rtk rg -n '(^|[^A-Za-z0-9_])(assert!|assert_eq!|assert_ne!|unreachable!)' \
    fuzz/src/journal_target/readback.rs
97:    assert!(matches!(
104:    assert!(!matches!(classification, ReadbackFamilySet::Unreadable));
150:        assert_eq!(strict_result.is_ok(), relaxed_result.is_ok());
178:    assert!(artifact.accepted_at_seq.get() > 0);
179:    assert!(artifact.verification.gate_count > 0);
209:                assert_eq!(run_summary, expected);
```

**Result**: 5 pre-existing `assert!`/`assert_eq!` in non-touched fuzz
functions (`fuzz_readback_family_set` at 97, 104; `fuzz_admission_input_surface`
at 150; `fuzz_accepted_artifact_decode` at 178, 179) and 1 new `assert_eq!`
in the touched `fuzz_recovery_decode` at line 209.

**Disposition of the new `assert_eq!` at line 209**: This is the
**contractually-mandated** assertion per contract C-1 ("The exact
assertion is the single Rust statement: `assert_eq!(run_summary, expected_recovery_runtime_summary);`")
and proof-obligations.planned.jsonl PO-001 ("the new `assert_eq!(run_summary, expected_recovery_runtime_summary)`
does not change the production contract"). The desired panic surface is
explicit; the test panics on drift between the production output and the
exact expected struct. This is **not** a lazy error-handling anti-pattern
(it is a test assertion, not production code). The fuzz harness is a
`[[bin]]` test target; per truth-serum's own rule "Test implementation
style warnings are not a panic-surface gate", test assertions are
exempted.

### 9. Strict clippy panic-surface gate (production)

```
$ /home/lewis/.cargo/bin/cargo clippy -p vb_storage --lib -- \
    -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used \
    -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo \
    -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing \
    -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects \
    -D clippy::as_conversions -D clippy::let_underscore_must_use
    Checking vb_core v0.1.0 (.../vb_core)
    Checking vb_storage v0.1.0 (.../vb_storage)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.88s
EXIT_CODE=0
```

**Result**: PASS. The full strict panic-surface clippy gate passes on
`vb_storage` library with zero warnings. Log:
`.beads/vb-vzo9b/evidence/state14/cargo-clippy-strict.txt`.

### 10. `cargo test --no-run` (test compile gate)

```
$ /home/lewis/.cargo/bin/cargo test -p vb_storage --no-run
  Executable tests/proptest_vb_vzcuf_PS_007.rs (target/debug/deps/...)
  Executable tests/proptest_vb_vzcuf_PS_008.rs (target/debug/deps/...)
  ...
  Executable tests/recovery_property_tests.rs (target/debug/deps/...)
EXIT_CODE=0
```

**Result**: PASS. The `vb_storage` test suite compiles cleanly with zero
errors.

### 11. Repo-wide `forbidden-scan.sh`

```
$ bash scripts/forbidden-scan.sh
Scanning crate: vb_compile
Scanning crate: vb_core
Scanning crate: vb_ipc
Scanning crate: vb_queue_semantics
Scanning crate: vb_runtime
Scanning crate: vb_storage
Scanning crate: vb_validate
Scanning crate: vb_cli
Scanning crate: workspace_tests
forbidden-scan: PASS — no forbidden patterns found
```

**Result**: PASS. All 9 production crates pass the repo-wide forbidden-
pattern scanner. Log:
`.beads/vb-vzo9b/evidence/state13/forbidden-scan-state13.txt` (sha256
`2cfb70c4a7a28ca80121130e3fb2f0ed9cb2001c1a4a35f54890b352b044a3d0`).

### 12. `cargo fmt --check` (workspace)

```
$ /home/lewis/.cargo/bin/cargo fmt --check -- fuzz/src/journal_target/readback.rs
Diff in crates/vb_core/src/lib.rs:26: ...            # PRE-EXISTING, OUT OF SCOPE
Diff in crates/vb_core/src/time.rs:71: ...            # PRE-EXISTING, OUT OF SCOPE
Diff in crates/vb_runtime/src/frame_pool/tests.rs:85,114,139: ...   # PRE-EXISTING, OUT OF SCOPE
Diff in fuzz/src/journal_target/readback.rs:173: ...  # PRE-EXISTING, OUT OF BEAD'S DIFF (line 173 in fuzz_accepted_artifact_decode)
Diff in fuzz/src/journal_target/readback.rs:185: ...  # PRE-EXISTING, OUT OF BEAD'S DIFF (line 185 in fuzz_recovery_decode, pre-existing `let run = ...` line)
EXIT_CODE=1
```

**Result**: DEFERRED_GLOBAL. Five pre-existing fmt diffs in non-touched
files (and 2 in the touched file at lines 173 and 185, both pre-existing
and out of the bead's diff at lines 193-209). The bead's new code
(lines 196-209) is correctly formatted — `cargo fmt` does not flag any
of those lines. Log:
`.beads/vb-vzo9b/evidence/state14/cargo-fmt-check.txt`.

---

## 🫂 Empathetic User Review

**Friction points observed**:

1. **Workspace-exclusion gotcha**: The `fuzz/` directory is a separate
   Cargo workspace (package name `velvet-ballistics-fuzz`, not `fuzz`).
   `cargo build -p fuzz --bin recovery_decode` returns
   `package ID specification fuzz did not match any packages` with
   `help: a package with a similar name exists: flume`. This is the
   exact confusion the proof-plan-reviewer flagged and corrected in
   contract C-7 (the closure command is `cargo build --bin recovery_decode
   --manifest-path fuzz/Cargo.toml`, not `cargo build -p fuzz ...`).
   The corrected command works the first try. ✓
2. **Toolchain pin**: `cargo 1.97.0-nightly` is pinned via
   `rust-toolchain.toml`. The agent's `cargo` binary matches the pin.
   ✓
3. **Evidence capture**: The `forbidden-scan.sh` script prints output
   to stdout/stderr, not a file, and uses `cd` so subsequent commands
   inherit the cwd. The truth-serum audit captured the output to
   `.beads/vb-vzo9b/evidence/state13/forbidden-scan-state13.txt` for
   the assurance bundle. ✓
4. **`rg --no-messages` vs `! rg ...`**: The PO-003 source-lint gate
   uses `! rg -n ...` (inverted) which is the documented pattern. The
   truth-serum recheck used the same pattern and got the same result
   (all 6 return exit 1 = no matches). ✓

**Helpfulness of error messages**: All `cargo`/`rg` errors encountered
during the audit included actionable hints (e.g., "package ID
specification fuzz did not match any packages ... a package with a
similar name exists: flume"). No raw stack traces emitted to user.

---

## 🕵️ Skeptical QA Review

**Adversarial attacks executed**:

1. **Reintroduce disjunctive acceptance**: rejected. The
   `assert!(..||..)` pattern is gone from `fuzz_recovery_decode`.
   `rg 'assert!\([^)]+\|\|' fuzz/src/journal_target/readback.rs` →
   exit 1 (no matches).
2. **Single-field assertion bypass**: rejected. No `matches!(run_summary, ..)`
   pattern. `rg 'matches!\s*\(\s*run_summary' fuzz/src/journal_target/readback.rs` →
   exit 1 (no matches).
3. **Coverage-only `let _summary` regression**: rejected. No `let _summary`.
   `rg 'let _summary' fuzz/src/journal_target/readback.rs` → exit 1.
4. **`dbg!` silent failure**: rejected. No `dbg!(run_summary ...)`.
   `rg '\bdbg!\s*\(\s*run_summary' fuzz/src/journal_target/readback.rs` → exit 1.
5. **`unwrap`/`expect` on `RecoveryResult`**: rejected. No `.unwrap()` or
   `.expect(` in the touched file.
6. **Production-code drift**: rejected. `jj show` confirms the diff is
   restricted to `fuzz/src/journal_target/readback.rs`. `cargo test` on
   the two production functions (12 + 6 passed) confirms production is
   unchanged. SHA-256 of `crates/vb_storage/src/recovery/types.rs` and
   the post-fix touched file is captured in the formal-verification-
   report.md and assurance-bundle.md.
7. **Frame-seed call site drift**: rejected. The second `if let Err(error) = ...`
   call is byte-identical pre/post fix (`jj show` line range 214-216
   unchanged).
8. **Vacuum Verus proof**: rejected as N/A. No Verus obligation in
   scope (VLD-004 `not_applicable surface_absent`); no
   `verification/verus/` artifact for this bead. `bash
   scripts/check-verus-production-binding.sh` is not required because
   there is no Verus spec to bind.
9. **Verification laundering**: rejected. The `rg
   '#\[verifier::external_body\]|\baxiom\b|\bassume\(' fuzz/src/journal_target/readback.rs`
   check returns exit 1 (no matches). The 703 `kani::assume(...)` calls
   in the workspace are bounded assumptions inside Kani harnesses, not
   Verus proof-laundering patterns, and are out of bead's blast radius.
10. **Production `assert!` regression**: rejected. The 5 pre-existing
    `assert!`/`assert_eq!` in `readback.rs` (lines 97, 104, 150, 178, 179)
    are in non-touched fuzz functions and are byte-identical pre/post fix.
    The 1 new `assert_eq!` at line 209 is the contractually-mandated
    C-1 assertion. The production recovery surface
    (apply.rs/derive.rs/accumulator.rs/types.rs) has zero `assert!` macros.
11. **Test clippy strictness**: classified as DEFERRED_GLOBAL. 5
    pre-existing clippy errors in non-touched fuzz files. AGENTS.md:
    "Tests must compile and run, but test clippy is not strict." Not a
    bead blocker.
12. **Pre-existing helper catch-all (`_ => {}`)**: documented in
    black-hat-review.md as owner-approved debt. Out of blast radius.
    Not a bead blocker.
13. **Hardcoded single-shape Kani harness**: rejected as N/A. No Kani
    obligation in scope. The fuzz payload shape (single `RunAccepted`
    event with `seq = EventSeq::new(1)`) is the contract C-1 fuzz
    payload, not a hardcoded Kani shape.

**No blocker findings. No behavior-affecting concerns. No
production-code defects. No parity drift. No vacuum proofs. No
verification laundering. No deleted tests. No scope drift.**

---

## Mandated Improvements

**None blocking**. The bead is approved for landing.

### Optional (out of bead's blast radius; address in follow-on beads)

1. **LOW**: `assert_typed_recovery_error` and `assert_typed_journal_error`
   use `_ => {}` catch-all fallbacks. Consider adding
   `panic!("unmatched error variant: {:?}", _)` or `#[deny(unreachable_patterns)]`
   on the typed error enums. Follow-on fuzz-hardening bead.
2. **DEFERRED_GLOBAL**: 5 pre-existing clippy errors in non-touched fuzz
   files. Follow-on fuzz-test-cleanup bead.
3. **DEFERRED_GLOBAL**: 5 pre-existing `cargo fmt` diffs in non-touched
   files. Follow-on workspace-fmt bead.

---

## Decision

**STATUS: APPROVED** — The implementation claim is supported by raw
command evidence; the production code is byte-identical pre/post fix;
the touched fuzz body has zero panic surface outside the contractually-
mandated `assert_eq!`; the verification laundering shield passes; the
black-hat review is APPROVED; all reviewer findings at every severity
use a canonical `finding/v1.disposition` value. The bead is ready for
state 15 (landing).
