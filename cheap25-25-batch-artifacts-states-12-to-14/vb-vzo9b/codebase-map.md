# Bead vb-vzo9b — Codebase Map

- bead_id: vb-vzo9b
- title: Tests: replace multi-run recovery disjunction with exact slots (P1 bug)
- finding_origin: 2026-06-30 20-agent audit (related epic vb-82snf "Fuzz Test: recovery corruption assertions and mutation strength")
- isolated_workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-vzo9b
- jj_workspace: cheap25-vb-vzo9b
- jj_parent_commit: rsvywymk 1d6c017f (AGENTS.md round10 forward-port)
- captured_at: 2026-07-01T15:21:37Z
- controller: femdation
- author: explore (go-skill state 2)

## Section 1: Target Disjunction (Primary Defect Site)

### 1.1 Exact source line of the OR-disjunction

- File: `fuzz/src/journal_target/readback.rs`
- Function: `fuzz_recovery_decode` (lines 183-204)
- Defect line: **line 196**

```rust
pub fn fuzz_recovery_decode(data: &[u8]) {
    let digest = vb_core::WorkflowDigest::from_bytes(blake3::hash(data).into());
    let run = vb_core::RunId::new(u64::from(data.first().copied().unwrap_or(0)));
    let seq = vb_storage::EventSeq::new(1);
    let events = if data.len().is_multiple_of(2) {
        vec![vb_storage::JournalEvent::RunAccepted { run, seq, workflow: digest }]
    } else {
        Vec::new()
    };
    match vb_storage::recovery::summarize_recovery_events(&events) {
        Ok(hydration) => {
            if !events.is_empty() {
                let run_summary = hydration.summary();
                // >>> DEFECT: weak disjunctive assertion (P1 bug) <<<
                assert!(run_summary.run == run || run_summary.run == vb_core::RunId::new(0));
            }
        }
        Err(error) => assert_typed_recovery_error(error),
    }
    if let Err(error) = vb_storage::recovery::recover_runtime_frame_seed_from_events(&events) {
        assert_typed_recovery_error(error);
    }
}
```

The OR-disjunction `run == actual_run || run == RunId(0)` accepts two distinct values where only one is correct for the non-empty `events` branch (which is the only branch executed inside `if !events.is_empty()`). `RunId::new(0)` is the sentinel used by the production empty-events path in `RecoveryError::NoRecoveryData { run: RunId::new(0) }` (see `crates/vb_storage/src/recovery/replay/summary/apply.rs:90` and `derive.rs:148`); it is **not** a valid `run` field for a non-empty `RecoveryRuntimeSummary`.

### 1.2 Why this is a multi-run recovery disjunction

The fuzz driver constructs `run` from the first byte of the fuzz payload (`RunId::new(u64::from(data.first().copied().unwrap_or(0)))`). When the payload starts with `0x00` the test trivially passes for any returned `run` field, hiding real divergence bugs in `summarize_recovery_events`. The disjunction defeats the per-slot/per-run slot-state assertion that the audit requires.

## Section 2: Production Surface Read by the Target Test

### 2.1 `summarize_recovery_events` (called by fuzz_recovery_decode)

- File: `crates/vb_storage/src/recovery/replay/summary/apply.rs`
- Lines: 88-129
- Signature: `pub fn summarize_recovery_events(events: &[JournalEvent]) -> RecoveryResult<RecoveryHydration>`
- Behavior: Builds a `RecoveryRuntimeSummary` from the leading event's `run` and `seq`, asserts all events share the same `run_id`, applies each event via `apply_summary_event_checked`.
- Multi-run guard: returns `Err(RecoveryError::ReplayDivergence { step: StepIdx::ZERO, detail: "recovery summary received events for multiple runs".to_owned() })` if events disagree on `run_id`.

### 2.2 `recover_runtime_frame_seed_from_events` (called by fuzz_recovery_decode)

- File: `crates/vb_storage/src/recovery/replay/summary/derive.rs`
- Lines: 69 (signature); see `replay/summary/accumulator.rs:86` for the multi-run guard detail string `"frame seed recovery received events for multiple runs"`.
- Signature: `pub fn recover_runtime_frame_seed_from_events(events: &[JournalEvent]) -> RecoveryResult<RecoveryFrameSeed>`
- Behavior: Same first-event run derivation as `summarize_recovery_events`; emits `RecoveryFrameSeed { summary, steps, slots, unsupported, ... }`.

### 2.3 `RecoveryRuntimeSummary` (return type field)

- File: `crates/vb_storage/src/recovery/types.rs`
- Lines: 547-570
- Field list (all pin-able):
  - `pub run: RunId` — only field currently asserted by fuzz_recovery_decode
  - `pub first_seq: EventSeq`
  - `pub last_seq: EventSeq`
  - `pub workflow: Option<WorkflowDigest>`
  - `pub steps_started: u64`
  - `pub steps_succeeded: u64`
  - `pub actions_scheduled: u64`
  - `pub actions_resolved: u64`
  - `pub suspensions: u64`
  - `pub slots_written: u64`
  - `pub terminal: Option<RecoveryTerminalState>`

For the non-empty branch of `fuzz_recovery_decode` (single `RunAccepted` event with known `digest`, `run`, and `seq = EventSeq::new(1)`), every field above has a determinate exact value:

| Field | Exact expected value |
|---|---|
| `run` | `run` (the locally constructed one) |
| `first_seq` | `EventSeq::new(1)` |
| `last_seq` | `EventSeq::new(1)` |
| `workflow` | `Some(digest)` |
| `steps_started` | `0` |
| `steps_succeeded` | `0` |
| `actions_scheduled` | `0` |
| `actions_resolved` | `0` |
| `suspensions` | `0` |
| `slots_written` | `0` |
| `terminal` | `None` |

### 2.4 `RecoveryHydration::summary`

- File: `crates/vb_storage/src/recovery/types.rs`
- Lines: 596-605
- `RecoveryHydration::Summary(s)` returns `s`; `RecoveryHydration::FrameSeed(seed)` returns `seed.summary`.

## Section 3: Fuzz Target Wiring

- `fuzz/src/bin/recovery_decode.rs:1-31` — binary wrapper reading stdin and dispatching to `fuzz_lib::fuzz_recovery_decode`.
- `fuzz/src/journal_target.rs:30-33` — re-export of `fuzz_recovery_decode`.
- `fuzz/src/journal_target/readback.rs:183-204` — function body (defect site).
- `fuzz/src/lib.rs:46` — top-level re-export (`pub use journal_target::fuzz_recovery_decode;`).
- `fuzz/Cargo.toml:241-246` — `[[bin]]` registration as `recovery_decode` (`path = "src/bin/recovery_decode.rs"`).

## Section 4: Adjacent / Cross-Cutting Files

### 4.1 Files cited by the bead as "must_read_first"

- `crates/workspace_tests/` — cross-crate integration tests; no recovery-disjunction finding confirmed in this directory (see Section 4.3 for the only other `assert!(* || *)` recovery-adjacent pattern, which is a permissive boundary check, not a multi-run disjunction).
- `crates/vb_storage/src/tests.rs` (275.8 KB, 7700 lines) — large storage test surface; no multi-run recovery OR-disjunction found (the only `||` in an `assert!` macro in this file or its sub-modules is `tests.rs:186` `entry.slot == slot && entry.value == SlotValue::I64(42) && entry.taint == Taint::Clean`, an AND-chain, not a disjunction).
- `fuzz/` — confirmed defect site in `fuzz/src/journal_target/readback.rs`.
- `velvet-ballistics-MASTER.md` — referenced for governance, no per-bead defect registry entry found for vb-vzo9b.

### 4.2 Files read for context (production surface, not for editing)

- `crates/vb_storage/src/recovery/types.rs:540-605` — `RecoveryRuntimeSummary`, `RecoveryHydration`, `RecoveryHydration::summary`.
- `crates/vb_storage/src/recovery/replay/summary/apply.rs:88-129` — `summarize_recovery_events` body.
- `crates/vb_storage/src/recovery/replay/summary/derive.rs:60-77, 140-200` — `recover_runtime_frame_seed_from_events` signature; derives `first_seq/last_seq` from the event stream.
- `crates/vb_storage/src/recovery/replay/summary/tests.rs:285-302` — confirms `RunId::new(0)` is the sentinel used by the empty-events path; explains why accepting `RunId(0)` as a non-empty summary run is wrong.

### 4.3 Other OR-patterns in recovery test surface (verified non-target)

These were inspected and confirmed to NOT be the targeted multi-run disjunction:

- `crates/vb_storage/src/recovery/vb_h6ix_tests.rs:858` — `assert!(terminal.is_none() || { match terminal { Some(RunFinished|RunFailedEvent|RunCancelled) => true, _ => false } }, ...)` — single-run attempt-vs-terminal disjunction, not multi-run; out of scope.
- `crates/workspace_tests/tests/vb_test_cli_storage_io_behavior.rs:191` — `record.digest == requested || artifact.source_digest == requested` — workflow resolution lookup, not recovery; out of scope.
- `crates/workspace_tests/tests/integration_compile_error_message_quality.rs:238,263,286,344` — `assert!(result.is_ok() || result.is_err())` — tautological permissive checks; out of scope.
- `crates/vb_runtime/src/runtime_tests.rs:1289` — `assert!(result.is_ok() || matches!(result, Err(RuntimeError::QueueFull)))` — runtime queue disjunction; out of scope.
- `crates/workspace_tests/tests/integration_runtime_storage_fault_tolerance.rs:79` — `assert!(result.is_ok() || result.is_err())` — permissive boundary; out of scope.

## Section 5: Test Invocation / Closure Surface

| Surface | Path | Notes |
|---|---|---|
| Fuzz harness | `fuzz/src/bin/recovery_decode.rs` | Reads stdin, calls `fuzz_recovery_decode`. |
| Fuzz body | `fuzz/src/journal_target/readback.rs:183-204` | Defect site. |
| Module re-export | `fuzz/src/journal_target.rs:30-33` | `pub use ... fuzz_recovery_decode`. |
| Top-level re-export | `fuzz/src/lib.rs:46` | `pub use journal_target::fuzz_recovery_decode`. |
| Cargo bin entry | `fuzz/Cargo.toml:241-246` | `recovery_decode`. |
| Existing fuzz unit tests | `fuzz/tests/` (only proptest files, no `#[test]` for fuzz targets) | No regression harness dedicated to `fuzz_recovery_decode` exists; a new unit test wrapping a deterministic payload is recommended downstream. |
| Closure commands (evidence-gathering) | `cargo test -p vb_storage --lib summarize_recovery_events`, `cargo test -p vb_storage --lib recover_runtime_frame_seed_from_events`, `cargo build -p fuzz --bin recovery_decode` | Targeted gates available without invoking the heavy `moon ci`. |

## Section 6: Risk Tags

- risk: behavior-test (the assertion IS the test; replacing it is a behavior change)
- risk: fuzz-coverage (replacing `let _summary`/weak-disjunction with exact field assertions converts a coverage-only target into a behavior-checking target)
- risk: parser-or-decoder (the fuzz payload still flows through `summarize_recovery_events` which is the recovery decoder; multi-run guards and overflow sentinels remain in production)
- risk: evidence-packaging (raw `cargo test` logs will need to be cited in `.beads/vb-vzo9b/evidence-bundle.md` downstream)
- NO production-code change is required — the defect is solely in test code (`fuzz/src/journal_target/readback.rs`); the production behavior is correct.

## Section 7: Out-of-Scope / Excluded

- `crates/vb_storage/src/recovery/replay/summary/apply.rs`, `derive.rs`, `accumulator.rs` — production code that ALREADY enforces multi-run rejection via `RecoveryError::ReplayDivergence { detail: "recovery summary received events for multiple runs" }` and `"frame seed recovery received events for multiple runs"`. Production is correct.
- `crates/vb_storage/src/recovery/replay/summary/tests.rs` — already uses exact `matches!` checks; not the target.
- `crates/vb_storage/src/recovery/vb_h6ix_tests.rs` — attempt-vs-terminal disjunction is single-run, not multi-run; out of scope.
- Verifier proof artifacts (`verification/verus/**`, `verification/flux/**`, `kani/**`) — not required; defect is in test code.
- `Cargo.toml` dependency changes — none required.

## Section 8: Open Questions for Downstream Owners

1. Should the fuzz driver be extended to enumerate two-run scenarios (run_a + run_b in the same `events` slice) and assert `RecoveryError::ReplayDivergence { detail: "recovery summary received events for multiple runs" }` is returned exactly? The audit's "exact per-run slot vectors" wording implies this, but the current fuzz payload shape is single-RunAccepted. Downstream contract/proof/test-planner may decide whether to add this as a follow-on bead or as a sibling test.
2. The fuzz harness currently uses `EventSeq::new(1)` for the constructed event; downstream may want to vary seq to exercise `first_seq != last_seq`. Not strictly required for closing vb-vzo9b, but recommended as a strengthening pass.
3. No dedicated `#[test]` for `fuzz_recovery_decode` exists. Downstream test-planner may add a deterministic `#[test]` (e.g., in `fuzz/src/journal_target/readback.rs` under `#[cfg(test)]`) with a known payload to lock the exact field assertions against future regressions.

## Section 9: Recommended Downstream Owners

- rust-contract: convert the `RecoveryRuntimeSummary` field contract into a stricter exact-equality rule for fuzz harnesses (already `PartialEq + Eq + Debug + Copy` per `types.rs:546`).
- proof-planner: not required (no production change, no proof obligation triggered).
- test-planner: design deterministic `#[test]` wrapper for `fuzz_recovery_decode` covering at minimum: empty payload, single-RunAccepted payload, multi-run divergence payload, overflow seq payload.
- holzman-rust: implement the fix in `fuzz/src/journal_target/readback.rs:196` by removing the OR and asserting each `RecoveryRuntimeSummary` field individually.
- black-hat-reviewer: confirm the fix does not weaken any existing recovery assertion in `vb_h6ix_tests.rs`, `recovery_unit_tests.rs`, or `replay/summary/tests.rs`.

## Section 10: DISCOVERY_BLOCKED / MISSING

- NONE. All required artifacts (`STATE.md`, `baseline-report.md`, target source line, production signatures) are locatable and read.
