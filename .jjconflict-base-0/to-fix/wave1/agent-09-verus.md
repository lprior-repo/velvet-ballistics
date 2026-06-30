# Wave 1 — Agent 09 Verus Review (compiler/YAML/IR validation cluster)

Reviewer scope: every bug in chunk 09 checked for a Verus proof artifact that
binds (via `requires`/`ensures`/`extern_spec`) to the production exec fn
touched by the fix. Adversarial posture: GOD RULE 2 forbids "vacuum proofs"
(spec + proof fn side-by-side with production, no exec binding).

`bash scripts/verify-verus.sh` registry baseline: 5 targets, all PASS,
trust scan OK. Output captured in `.evidence/verus/summary.txt`.

All production paths under `crates/vb_runtime/`, `crates/vb_core/src/frame.rs`,
`crates/vb_storage/src/trimming/logic.rs`, and `crates/vb_ipc/` were inspected
for Verus annotations (`verus!`, `extern_spec`, `#[verifier::external_body]`).
Result: zero production exec fns carry Verus contracts. Every Verus proof in
`verification/verus/` is therefore a model-side mirror, not a binding.

Workspace caveat: `crates/vb_runtime/src/shard/types.rs:807-815` contains an
unresolved merge conflict (`<<<<<<< HEAD` … `>>>>>>> bead/vb-zpaad`) that
blocks `cargo test --tests` for vb_runtime. `cargo test --lib` is unaffected
and was used for the regression runs below.

## Bug table

| bug-id | pri | verus-artifact | vacuum-proof | source-fix | test | verus-cmd | verus-result | cargo-result | verdict | evidence |
|---|---|---|---|---|---|---|---|---|---|---|
| vb-lcfj3 | P2 | verification/verus/run_frame_invariant.rs | YES — `proof fn`+`spec fn`, no `requires`/`ensures` on `RunFrame::new`; "binding" is comments only | NOT-PATCHED — `crates/vb_core/src/frame.rs:105` and `:139` still set `max_parallel_in_flight: u16::MAX` | `cargo test -p vb_core --lib frame` → 43 passed, 0 failed | `verus --crate-type=lib verification/verus/run_frame_invariant.rs` | `verification results:: 20 verified, 0 errors` | 43 passed, 0 failed | NOT-PATCHED | run_frame_invariant.rs:172 (comment-only binding); frame.rs:105,139 (bug literal present) |
| vb-loa3o | P2 | verification/verus/vb_jnz9_journal_event_seq_valid.rs (FAILS to verify); verification/verus/cancel_kill_lattice.rs (passes) | YES — both files are pure spec/proof fn, no exec binding; journal_event_seq_valid also has a failing postcondition | PATCHED — `crates/vb_runtime/src/journal/chunk_002.rs:196-202` now maps `WaitResolved → JournalEvent::WaitResolvedEvent` | `cargo test -p vb_runtime --lib re_009` → `re_009_wait_resolved_maps_to_dedicated_journal_event` PASS | `verus --crate-type=lib verification/verus/vb_jnz9_journal_event_seq_valid.rs`; `verus --crate-type=lib verification/verus/cancel_kill_lattice.rs` | seq_valid: `verification results:: 3 verified, 1 errors` (FAIL); cancel_kill_lattice: `verification results:: 18 verified, 0 errors` | 1 passed (re_009) | PATCHED | chunk_002.rs:196-202 |
| vb-lpuw3 | P1 | NONE for `crates/vb_runtime/src/shard/completion_watermark.rs` | N/A | NOT-PATCHED — `complete()` at `crates/vb_runtime/src/shard/completion_watermark.rs:132` still calls `self.push_pending(seq)?;` BEFORE `let drained = self.drain_prefix();`. When `pending.len() == max_pending` and incoming seq is `boundary+1`, push rejects and the draining completion is lost. | No `CompletionWatermark`/`watermark` test names found; `cargo test -p vb_runtime --lib completion` → 59 passed, 0 failed (none touch the gap-closing drain case) | n/a | n/a | 59 passed (regression baseline; bug not exercised by name) | NOT-PATCHED | completion_watermark.rs:128-138 |
| vb-lrxy9 | P0 | N/A (miri test file) | N/A | PATCHED (false premise) — `crates/vb_storage/src/codec_miri_tests.rs` already exists (12.7K, 432+ lines); the gap was the path string, not the file. Bead correctly closed. | `cargo build -p vb_storage --tests` succeeds; codec test target `cargo test -p vb_storage --lib -- codec` → 158 passed, 0 failed | n/a | n/a | 158 passed | PATCHED | `ls crates/vb_storage/src/codec_miri_tests.rs` |
| vb-lxkqh | P3 | verification/verus/vb_8mdp_8/queue_state_shared_source.rs | YES — `pub fn helper_*` exec fns are stand-ins; the production `backpressure_threshold(capacity: ActionQueueCapacity) -> usize` in `crates/vb_runtime/src/action_queue.rs:226-234` is not annotated and not bound | PATCHED — `backpressure_threshold` uses `(capacity*8)/10` via `checked_mul(8).and_then(checked_div(10))` with `max(1)` floor; current `crates/vb_runtime/src/action_queue.rs:226-234` | `cargo test -p vb_runtime --lib backpressure` → 6 passed, 0 failed (`bounded_action_queue_with_backpressure_*`, `action_queue_emits_backpressure_warning_at_80_percent_capacity_*`) | `verus --crate-type=lib verification/verus/vb_8mdp_8/queue_state_shared_source.rs` | `verification results:: 19 verified, 0 errors` | 6 passed | PATCHED | action_queue.rs:226-234; queue_state_shared_source.rs:65-72 (model mirrors 80% rule) |
| vb-mw0v9 | P2 | NONE for the silent-fall-through sites | N/A | PARTIAL — `rg "let _ =" crates/vb_runtime/src/shard/lifecycle_tests/` returns 0 hits and `lifecycle/` returns 0 hits; only 1 survivor remains at `crates/vb_runtime/src/shard/tests/chunk_025.rs:142` (`let _ = shard.tick();`). Reported 85 sites, currently 1. | `cargo test -p vb_runtime --lib shard` → 615 passed, 0 failed | n/a | n/a | 615 passed | PARTIAL | chunk_025.rs:142 |
| vb-mx7qt | P2 | NONE — workspace_tests is harness wiring, not Verus-target | N/A | PARTIAL — `edge_submit_after_shutdown_enqueues_but_does_not_process` PASS, `valid_workspace_passes_sharpened_assertions` PASS; but `vb_a0t1_source_length_gate_tests` target does not exist (`cargo test --test vb_a0t1_source_length_gate_tests` → `no test target named …`) so the third failing test is unbuilt, not fixed | `cargo test -p velvet-ballistics-workspace-tests --tests` → all PASS (zero FAIL count across all suites) | n/a | n/a | all workspace_tests PASS; vb_a0t1 target missing | PARTIAL | workspace_tests target list excludes `vb_a0t1_source_length_gate_tests` |
| vb-mxsxm | P2 | n/a — closed as duplicate of vb-yasoz | N/A | PATCHED via vb-yasoz (RP-016 tail-copy); not verified in this slice because owner = vb-yasoz | covered by vb-yasoz regression; no separate verus artifact exists for this id | n/a | n/a | not re-run (duplicate) | PATCHED | bead note "Duplicate of vb-yasoz" |
| vb-n5ctl | P3 | NONE for `crates/vb_storage/src/trimming/logic.rs` | N/A | PATCHED — `crates/vb_storage/src/trimming/logic.rs:95-101` short-circuits with `TrimStatus::NoOp` when `deleted_count == 0`, skipping `batch.commit()`. Test `trim_zero_deletes_returns_noop_when_skip_noop_disabled` covers it. | `cargo test -p vb_storage --lib -- trim` → 38 passed, 0 failed | n/a | n/a | 38 passed | PATCHED | trimming/logic.rs:95-101; trimming/tests.rs covers NoOp path |
| vb-n8ylu | P1 | NONE for `handle_cancel_run` in `crates/vb_ipc/src/server/handlers.rs` | N/A | PATCHED — handlers.rs:117 routes to `runtime.cancel_run_with_reason` and shard appends `RuntimeJournalEvent::RunCancelled { reason }` via `append_journal_event`. Bead closed as not-reproducible; current source is correct. | `cargo test -p vb_ipc --tests cancel` → 6 passed, 0 failed (`dispatch_command_with_resolver_cancel_run`, `cancel_run_command`, `RunCancelled` trace roundtrip, etc.) | n/a | n/a | 6 passed | PATCHED | ipc handlers.rs:117; chunk_002.rs:159 |
| vb-nr45m | P3 | NONE — phantom regression target only | N/A | PATCHED — `SlotSet`/`ensure_insert_slot` not present in `crates/vb_runtime/src/`; phantom regression `rs_026_phantom` asserts the symbol absence (`rg "SlotSet" --include="*.rs" crates/vb_runtime/src/` → empty). | `cargo test -p vb_runtime --lib rs_026` returns 0 because phantom target is in `crates/vb_runtime/tests/rs_026_phantom.rs` (requires `--tests`, currently blocked by the types.rs merge conflict) | n/a | n/a | blocked by merge conflict in shard/types.rs; phantom file content confirms intent | PATCHED | tests/rs_026_phantom.rs:31-65 |

## Aggregate counts

- bugs checked: **11**
- PATCHED: **6** (vb-loa3o, vb-lrxy9, vb-lxkqh, vb-mxsxm, vb-n5ctl, vb-n8ylu)
- NOT-PATCHED: **2** (vb-lcfj3, vb-lpuw3)
- PARTIAL: **3** (vb-mw0v9, vb-mx7qt, plus vb-nr45m gated by the merge conflict)
- UNKNOWN: 0
- vacuum-proof cases (artifact exists but no `requires`/`ensures` binding to exec fn): **3** (vb-lcfj3, vb-loa3o, vb-lxkqh) — plus vb-jnz9 which fails to verify outright and is therefore worse than vacuum
- verus registry targets touched by this slice: 1 (`verification/verus/vb_jpq724_events_for_run_production.rs`, transitively related to vb-loa3o's journal event model — passes 5/5 verified)
- `verus --crate-type=lib` runs outside the registry: `cancel_kill_lattice.rs` 18/0, `run_frame_invariant.rs` 20/0, `vb_8mdp_8/queue_state_shared_source.rs` 19/0, `budget_bounded.rs` 15/0, `vb-fzgdn/PS-001..PS-010` mostly FAIL to compile (5/10 have hard parse/type errors), `vb_jnz9_journal_event_seq_valid.rs` 3 verified 1 error (failing postcondition), `vb_compile/encoding_injectivity.rs` fails to compile (`&str` lifetime), `crates/vb_runtime/src/verification/verus/vb_y9d3v_action_fence.rs` fails to compile (`Result<T,E>` inference)

## Top-3 NOT-PATCHED with reason

1. **vb-lcfj3 (CF-004)** — `crates/vb_core/src/frame.rs:105` (`RunFrame::new`) and `:139` (`reinitialize`) still initialise `max_parallel_in_flight: u16::MAX`. The Verus proof `verification/verus/run_frame_invariant.rs` proves dimensions/preconditions but is a vacuum proof: every `proof fn` operates on a standalone `SpecRunFrame` and `SpecTaint`, with "Binding to production code" only documented in comments. No `extern_spec` or `#[verifier::external_body]` decorates the actual `RunFrame::new` exec fn, so the proof never constrains the `u16::MAX` literal that is the bug.

2. **vb-lpuw3 (RS-209)** — `crates/vb_runtime/src/shard/completion_watermark.rs:128-138` still calls `self.push_pending(seq)?;` BEFORE `let drained = self.drain_prefix();`. When the pending queue is full and the incoming `seq == boundary+1` (gap-closing case), `push_pending` returns `QueueFull` and the draining completion is dropped. No Verus artifact exists for this module, and no `CompletionWatermark`-named regression test exercises the full-queue + drain case (filtered set is empty under `-- completion`). Bead is marked CLOSED but the source still reproduces the bug.

3. **vb-lxkqh (RP-019)** — the source fix in `crates/vb_runtime/src/action_queue.rs:226-234` is correct (`(capacity*8)/10` with `max(1)` floor; `cargo test backpressure` shows 6/6 green), but the only Verus artifact, `verification/verus/vb_8mdp_8/queue_state_shared_source.rs`, does not bind to the production exec fn. It defines `pub fn helper_*` mirrors and `spec fn warning_threshold`/`spec fn warning_payload` proofs over `Seq<int>` queues, while `backpressure_threshold(capacity: ActionQueueCapacity) -> usize` in the production crate has no Verus annotation. The proof passes (19 verified) but never constrains the real function.

## Per-bug adversarial verdict on binding

- **vb-lcfj3** — VACUUM. Spec/proof fn only; binding is comment text. The proof proves `step_count == 0 ⇒ !spec_run_frame_new_valid` and similar dimension bounds, but it does NOT prove anything about `max_parallel_in_flight`.
- **vb-loa3o** — VACUUM + BROKEN. `vb_jnz9_journal_event_seq_valid.rs` fails verification (postcondition `seq < u64::MAX` does not hold), and `cancel_kill_lattice.rs` is a model-only proof about RunLifecycle/Command, not a spec on `JournalEvent::WaitResolvedEvent`.
- **vb-lpuw3** — N/A. No Verus artifact.
- **vb-lrxy9** — N/A (miri test gap, not a Verus obligation).
- **vb-lxkqh** — VACUUM (mirror model). Production `backpressure_threshold` lacks Verus annotations.
- **vb-mw0v9** — N/A (test-quality bug, no Verus obligation).
- **vb-mx7qt** — N/A (workspace_tests wiring, no Verus obligation).
- **vb-mxsxm** — N/A (duplicate of vb-yasoz).
- **vb-n5ctl** — N/A. No Verus artifact for trimming.
- **vb-n8ylu** — N/A. No Verus artifact for `handle_cancel_run`.
- **vb-nr45m** — N/A. Phantom-symbol regression only.

## Verus registry + global scan summary

```
$ bash scripts/verify-verus.sh 2>&1 | tail -3
VERUS_TRUST_SCAN_OK
VERUS_REGISTRY_OK evidence=.evidence/verus
```

Registry passes (5/5 targets). `verification/verus/` is clean of `assume(` / `#[verifier::external_body]` / `#[verifier::external]` / `\baxiom\b`. However, `crates/vb_runtime/src/verification/verus/vb_sxkz6_shard_for_run.rs` and `vb_y9d3v_action_fence.rs` are NOT scanned by `verify-verus.sh` (different directory); `vb_y9d3v_action_fence.rs` carries 3 `#[verifier::external_body]` declarations and several `unimplemented!()` bodies — a God-Rule-2 violation in the broader sense even though the trust-scan gate does not see them.

## Environment caveats that affect verdicts

- `crates/vb_runtime/src/shard/types.rs:807-815` has unresolved merge markers (`<<<<<<< HEAD` … `>>>>>>> bead/vb-zpaad`). This blocks `cargo test -p vb_runtime --tests` and gates vb-nr45m's regression confirmation.
- `crates/vb_runtime/src/test_harness.rs` defines `iterator_state_in_slot` twice (lines 33 and 63). Same gate.
- Multiple `verification/verus/vb-fzgdn/PS-00[1,4,7,8,9,10]-proof.rs` files fail to compile under Verus (parse errors, type mismatches). They are not in the registry but are referenced in proof obligation metadata; they should not be relied on as evidence.
- `verification/verus/vb_compile/encoding_injectivity.rs` and `secret_results_injectivity.rs` fail to compile (`&str` lifetime, missing modules) — relevant if any Wave 1 bug is later claimed to have compiler-layer Verus coverage.

## Output paths

- File written: `/home/lewis/src/velvet-ballistics/to-fix/wave1/agent-09-verus.md`
- Verus evidence: `.evidence/verus/summary.txt`, `.evidence/verus/trust-scan.txt`