# Machine Gate Report — vb-cib14 (State 14)

Generated: 2026-07-02T03:05:38Z
Workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cib14

## Verdict: ALL VB-CIB14-RELEVANT GATES PASS

Pre-existing global failures (in other beads' code) are recorded honestly; none
are introduced by vb-cib14 and none are in the bead's blast radius.

## Per-Gate Results

| Gate | Result | Evidence |
|---|---|---|
| `scripts/check-panic-surface.sh` | PASS — NoViolationFound, ExitCode 0 | `.beads/vb-cib14/evidence/state12-lint-po-006-panic.log` |
| `scripts/check-hot-cold-forbidden-apis.sh` | PASS — violations=0, justified=0 | `.beads/vb-cib14/evidence/state12-lint-po-006-hot-cold.log` |
| `scripts/check-verus-production-binding.sh` | PASS — 0 VACUUM, 72 WEAK, 0 STRONG | `.beads/vb-cib14/evidence/check-verus-production-binding-state12.log` |
| `scripts/check-source-length.sh` (chunk_002.rs + extern file) | PASS — both ledgered under `split-or-retire-before-release` for vb-cib14 | `.beads/vb-cib14/evidence/state12-lint-po-006-length.log` |
| `scripts/check-source-length.sh` (overall) | FAIL — pre-existing 17+ entries in other beads' files | (out of scope for vb-cib14) |
| `scripts/check-error-exhaustiveness.sh` | FAIL — pre-existing failures in `JournalError` / `IpcError` / `ValidationError` fuzz harnesses | (out of scope for vb-cib14) |
| `scripts/check-test-integrity.sh` | PASS — base=@- | live |
| `scripts/forbidden-scan.sh` | PASS — no forbidden patterns found | live |
| `scripts/check-nightly-features.sh` | PASS — exit 0 | live |
| `scripts/check-workspace-assertions.sh` | PASS — exit 0 | live |
| `cargo build -p vb_runtime --all-targets --all-features` | PASS — warning-free | live |
| `cargo test -p vb_runtime --lib` (default features) | PASS — 1807 passed / 0 failed | live |
| `cargo test -p vb_runtime --lib --features vb-cib14` | PASS — 1812 passed / 0 failed | live |

## Pre-Existing Global Failures (out of scope)

### Source-Length FAIL entries (17 files)

```
FAIL crates/vb_compile/src/expr_eval/tests/integration.rs [category=test_in_src] has 1674 physical lines (hard limit 1500)
FAIL crates/vb_compile/src/expr_eval_tests.rs [category=test_in_src] has 2740 physical lines (hard limit 1500)
FAIL crates/vb_runtime/src/shard/lifecycle/chunk_003.rs [category=production] has 361 physical lines (hard limit 300)
FAIL crates/vb_runtime/src/shard/snapshot.rs [category=production] has 350 physical lines (hard limit 300)
FAIL verification/verus/budget_bounded.rs [category=verus] has 956 physical lines (hard limit 800)
FAIL verification/verus/collect_ir_structure.rs [category=verus] has 869 physical lines (hard limit 800)
FAIL verification/verus/error_parity.rs [category=verus] has 931 physical lines (hard limit 800)
FAIL verification/verus/extern_recovery_verification.rs [category=verus] has 1159 physical lines (hard limit 800)
FAIL verification/verus/idempotency_replay_tracker.rs [category=verus] has 816 physical lines (hard limit 800)
FAIL verification/verus/ipc_capacity_bounds.rs [category=verus] has 813 physical lines (hard limit 800)
FAIL verification/verus/ipc_runtime_transitions.rs [category=verus] has 903 physical lines (hard limit 800)
FAIL verification/verus/production_inner/cli_commands_journal_trace_production.rs [category=verus] has 1103 physical lines (hard limit 800)
FAIL verification/verus/run_loop_termination.rs [category=verus] has 882 physical lines (hard limit 800)
FAIL verification/verus/storage_kind_family.rs [category=verus] has 922 physical lines (hard limit 800)
FAIL verification/verus/vb-vzcuf-PS-003.rs [category=verus] has 1045 physical lines (hard limit 800)
FAIL verification/verus/vb_ahfl_graph_events_production.rs [category=verus] has 870 physical lines (hard limit 800)
FAIL verification/verus/vb_ahfl_redaction_production.rs [category=verus] has 956 physical lines (hard limit 800)
FAIL verification/verus/vb_ahfl_ui_artifact_contract.rs [category=verus] has 1226 physical lines (hard limit 800)
FAIL verification/verus/vb_runtime_execute_do_spec.rs [category=verus] has 1222 physical lines (hard limit 800)
```

None of these are in vb-cib14's blast radius. vb-cib14 introduces / modifies:
- `crates/vb_runtime/src/journal/chunk_002.rs` (447 lines, ledgered at `.config/source-length-exceptions.txt:111`)
- `verification/verus/extern_vb_jnz9_journal_event_seq_valid.rs` (998 lines, ledgered at `.config/source-length-exceptions.txt:374`)

### check-error-exhaustiveness FAIL entries

```
JournalError oracle parse failed in fuzz/src/lib.rs::assert_typed_journal_error
JournalError missing in fuzz/fuzz_targets/decode_record.rs::assert_typed_journal_error
JournalError missing in fuzz/fuzz_targets/journal_decode.rs::assert_typed_journal_error
JournalError missing in fuzz/tests/proptest_journal_error_exhaustiveness.rs::assert_known_journal_error
IpcError oracle parse failed in fuzz/src/lib.rs::assert_typed_ipc_error
ValidationError enum parse failed in crates/vb_validate/src/lib.rs
```

All pre-existing. The `JournalError` / `IpcError` / `ValidationError` enums are
not part of vb-cib14's surface. vb-cib14's error surface is
`RuntimeError::ResumeTimestampOverflow` which is correctly handled at
`crates/vb_runtime/src/error/{mod,display,diagnostics,equality}.rs`.

### Pre-existing test fragility

`vb_qi37_4_2_strict_runtime_admission::given_existence_only_artifact_check_when_strict_admission_then_bypass_is_denied`
failure in `velvet-ballistics-workspace-tests`. Verified to pre-date this bead
at the parent commit `b2a2ee46` (per `implementation.md:254-259`). Not introduced
by vb-cib14.

## vb-cib14-Blast-Radius Gates

All gates in the vb-cib14 blast radius PASS:

- **Production panic surface**: clean (chunk_002.rs, error/mod.rs, error/display.rs, error/diagnostics.rs, error/equality.rs).
- **Hot/cold forbidden APIs**: clean (chunk_002.rs Resumed arm, convert_resume_timestamp).
- **Verus production-binding**: 0 VACUUM (the new spec file vb_cib14_resume_storage_map.rs is correctly classified as WEAK_EXTERN).
- **Source-length**: chunk_002.rs (447 lines) + extern_vb_jnz9_journal_event_seq_valid.rs (998 lines) ledgered.
- **Cargo build + test (default + vb-cib14 feature)**: clean.

## STATUS: APPROVED — for vb-cib14 blast radius

The pre-existing global failures are NOT introduced by vb-cib14 and do NOT
block this bead's landing. They are tracked separately by their respective
beads.