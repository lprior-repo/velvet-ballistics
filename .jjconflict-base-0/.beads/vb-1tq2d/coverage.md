# vb-1tq2d coverage map (deepseek-review independent audit)

Bead: **vb-1tq2d** — status **IN_PROGRESS** — bead notes **OPEN**.
Final candidate: `78352490a370`. Working-tree source == final-candidate source (HEAD
`2b06966e0` is evidence-only; `01dd92b6e` is empty).

## Verdicts

| Scope | Verdict |
|---|---|
| Fixture-migration code review (`a8452f9e` vs parents) | **REJECTED (bounded)** — `symbolic_ticket()` missing `attempt` |
| Source preservation audit (13 SHAs) | **PASS** — all ancestors, meaningful counterparts present, no drop |
| Branch disposition verification (43+1+80) | **PASS** — complete, consistent, no unaccounted source |
| `assert_ne!` bounded fix (`5766dff9`/`f4a40546`) | **CONFIRMED bounded** — `.ok()` error-discard NOT approved |
| Kani gate defects | **present in HEAD** — author-owned, 5090-assigned |

## Source preservation audit — 13 saved SHAs

All 13 SHAs are `git merge-base --is-ancestor` of `78352490a370` (confirmed). Each has
a meaningful current counterpart verified by file existence + key-symbol grep:

| source | sha | current counterpart |
|---|---|---|
| reviewed-base | e36e4c385b6b | workspace_tests `vb_c1s0_orchestration_runtime_tests.rs`, `vb_test_runtime_lifecycle_state_behavior.rs` |
| artifact-fixtures | 729778e06599 | `vb_runtime/tests/support/verified_artifact.rs`; `runtime_version_barrier_tests.rs` (→workspace_tests); `vb_wave16_mqa.rs` |
| admission-serializer | 6d6ce25c0836 | `vb_storage/src/events/wire_serialize_run.rs` (`serialize_admission*`) |
| semantic-source-schema3 | 376979e05670 | `vb_storage/src/admission/verification.rs`; `WorkflowParts.semantic_source_identity` |
| IPC-helpers | 521559b3b58c | `vb_ipc/src/server/{response_io,connection_io,helpers}.rs` (`WriteResult`) |
| shutdown | 6a5a4d4e9861 | `vb_runtime/src/runtime/shutdown.rs` (`ShutdownPhase/Report/Failure`, `quiesce`, `drain_journal`); `DuplicateEvent{run,seq}` assertion in shutdown_tests.rs |
| yaml-event-grammar | c849f7bdc173 | `vb_yaml/src/yaml_ast/parse_trigger.rs` (`manual/webhook/ipc/http`) |
| vb-mdi3w retained-input-recovery | 45693ea51b53 | `vb_runtime/src/recovery/input_binding.rs`; `vb_core/src/action/registry.rs` |
| vb-clp3q artifact-fixtures-wip | 6fc6e0ed1a96 | workspace_tests `vb_ko29_5_public_idempotency.rs`, `vb_lp2v_proof_admission_bdd.rs` |
| vb-r5c1c checkpoint-wip | de13504f655d | `vb_runtime/src/run_checkpoint.rs` (`RunCheckpointV1`, `recover_retry`); `vb_core/src/frame/checkpoint.rs` |
| vb-qzxto ask-ticket-wip | afb002df4493 | `vb_runtime/src/shard/ask_authority.rs` (`lookup_pending_ask_ticket`) |
| vb-7r7jv executor-wip | da2da5e2eb72 | `vb_storage/src/executor/*`; `vb_core/src/action_pack.rs` |
| vb-idxsn source-length-wip | eba9e5fe8ae2 | `vb_runtime/src/error/equality/{admission,core,unit}.rs`; `flux_sequence/{,tests}.rs`; `vb_expr` expr_* modules |

Two "missing" file pairs in a naive scan are **explained, not drops**:
- `vb_expr/src/expression.rs` + `expression_bytecode.rs` → renamed/split to `expr_*`
  modules (canonical naming migration).
- `vb_runtime/tests/{runtime_version_barrier_tests.rs, artifact_evidence_support/runtime_cases.rs}`
  → moved to `crates/workspace_tests/tests/` (canonical workspace_tests crate).

**No meaningful source dropped.**

## Branch disposition verification

`.beads/vb-1nx31/branch-disposition.json` (44 entries) matches `.md` (44 refs):
integrated 1 · redundant 2 · superseded 7 · blocked 2 · review 28 · contract 3 · base 1.
Plus 80 ancestor refs. The only genuinely-missing source (`bb1245638843` direct-API)
is recorded **integrated**; the 2 blocked refs are canonical-digest (owned by
`c789b1b3` elsewhere). The 13 saved SHAs are all in the 80-ancestor set. **No
unaccounted meaningful source.**

## Concrete CI counterexample — `assert_ne!` bounded fix

- `action_ticket_from_different_run_is_not_equal` builds `ticket_a`(run=1) and
  `ticket_b`(run=2) but, in `78352490`/`c816f6b3`/`6e3c5375`/HEAD, **has no
  `assert_ne!(ticket_a, ticket_b)`** — a vacuous test.
- `f4a405469` (vb-k5936) restores the assertion; merge `5766dff91` carries it.
- `ActionTicket` derives `PartialEq/Eq/Hash/Debug`; tickets differ on `run` ⇒ the
  assertion is meaningful. **CONFIRMED correct and bounded.**
- `5766dff91` conflict resolution **also** turned two
  `let x = runtime.tick_all();` into `runtime.tick_all().ok()` in
  `durable_retry_run_attempt_authority.rs` (lines 264, 314) — discards the `Result`
  error. **REJECTED, not approved.** Correct resolution = keep `assert_ne!`, do NOT
  adopt the `.ok()` error-discard.

## Kani gate defects (unchanged, author-owned)

- `crates/vb_core/src/kani_idempotency_gates.rs:169` `symbolic_ticket()` missing
  `attempt` (E0063) — introduced by `a8452f9e`.
- `crates/vb_core/src/replay/kani_harnesses.rs:22` `make_minimal_plan` missing
  `input_slots` (E0063) — pre-existing (`31e5182d`), GOD-RULE-1 shape.
- Evidence: `.beads/vb-1tq2d/evidence/kani-ticket-compile.json` (exit 1).

## Bead notes: OPEN

Blockers: `symbolic_ticket()` `attempt` repair (one line) + `replay` `input_slots`
repair — root assigned 5090. `assert_ne!` restore + rejection of `.ok()` discard
carried as review finding. No production/VCS mutations made by this reviewer.
