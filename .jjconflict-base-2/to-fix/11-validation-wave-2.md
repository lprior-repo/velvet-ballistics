# Wave 2 — Runtime / Action / Durability / Shard Bug Validation

**Generated:** 2026-06-24
**Scope:** Last-week bug beads (created 2026-06-17 → 2026-06-24) touching runtime/action/durability/shard/lifecycle/journal domain. Total: **269 bugs**.
**Method:** Read-only validation, no source mods, no beads. 15 parallel local subagents (12 core + 3 ad-hoc deep-dive).
**Pass criteria:** Source fix present + targeted cargo test passes + no Holzman regression.

## Verdict Roll-up

| Verdict | Count | % |
|---------|------:|--:|
| PATCHED | 158 | 58.7% |
| PARTIAL | 17 | 6.3% |
| NOT-PATCHED | 65 | 24.2% |
| UNKNOWN | 14 | 5.2% |
| BLOCKED (parent in_progress) | 3 | 1.1% |
| NOT-A-BUG (premise false) | 2 | 0.7% |
| Out-of-scope (chunk filter mismatch) | 10 | 3.7% |
| **Total** | **269** | **100%** |

## Agent-by-Agent Tally

| Agent | Role | PATCHED | PARTIAL | NOT-PATCHED | UNKNOWN | Other | Notes |
|-------|------|--------:|--------:|------------:|--------:|------:|-------|
| 00 | holzman-rust A | 9 | 0 | 5 | 0 | 2 dup | vb-1rqz7.1/.2/.11 NOT-PATCHED |
| 01 | holzman-rust B | 6 | 1 | 11 | 0 | 0 | 3 duplicates; DEEP-DIVE: vb-1rqz7.6, vb-1xa5j, vb-36fly |
| 02 | explore | 14 | 0 | 4 | 0 | 0 | vb-574zr, vb-42nj8, vb-4bq3r NOT-PATCHED |
| 03 | black-hat | 15 | 1 | 1 | 0 | 1 | Pass rate 83%; vb-7gm7c RE-OPEN |
| 04 | truth-serum | 7 | 1 | 8 | 1 | 1 | 4 hallucinations: vb-9ccdx, vb-b0pfx, vb-a5vsl, vb-aexu6 |
| 05 | flux-rs | 12 | 2 | 4 | 0 | 0 | All bugs flux-surface=NO; pre-existing trust debt |
| 06 | arch-drift | 11 | 3 | 5 | 0 | 0 | 3 drift-introduced cases |
| 07 | test-reviewer | 8 | 4 | 6 | 0 | 0 | vb-hv2xc, vb-hjz7r, vb-i6n4o NOT-PATCHED |
| 08 | miri | 8 | 2 | 8 | 0 | 0 | 0 unsafe-touch cases (all `#![forbid(unsafe_code)]`) |
| 09 | verus | 9 | 1 | 7 | 1 | 0 | 3 vacuum-proofs; vb-maupz UNKNOWN (phantom) |
| 10 | hands-on-qa | 14 | 1 | 3 | 0 | 0 | No regressions; 4 `#[allow(dead_code)]` still in cli_envelope.rs |
| 11 | rust-contract | 15 | 1 | 2 | 0 | 0 | 0 typestate broken; 0 error-taxonomy mismatch |
| 12 | ad-hoc: action-ticket | 8 | 2 | 7 | 1 | 0 | 1 hardcoded `attempt:1`; RunKilled→Unknown |
| 13 | ad-hoc: journal-replay | 12 | 0 | 5 | 1 | 0 | 0 mutate-before-append; 1 wildcard `apply_frame_event` |
| 14 | ad-hoc: shard-arena | 13 (eff.) | 2 | 2 | 13 (out-of-scope) | 0 | TimerWheel BTreeMap/HashMap present (master Phase 55 approves) |
| **Totals** | | **161** | **21** | **77** | **17** | **5** | |

(Note: counts include some duplication across agents; reconciled totals above)

## Major Phantom Closures (bead claims files/functions that don't exist)

| Bead | Cited symbol | Reality |
|------|--------------|---------|
| vb-9ccdx (RS-207) | "coalesce" fix | Entire fix is fictional for current source |
| vb-b0pfx (RS-011) | `discard_buffered_events_for_run` | Helper does not exist |
| vb-a5vsl (vb_cli) | `system_status_payload` `db` param, `from_live_journal`, `SystemConnectionState`, `output.rs` | All fictional |
| vb-aexu6 (RS-217) | `ShardConfig::validate()`, `RuntimeError::ConfigInvalid` | Both fictional |
| vb-1rqz7.1 (SJ-002) | `RecordKind::SequenceGap=60`, `MAGIC_JOURNAL_SEQUENCE_GAP` | Don't exist |
| vb-1rqz7.2 (SJ-003) | `inject_raw_event`, `inject_seq_gap` dedup | Lacking `write_lock`/`contains_key` |
| vb-32pmb (carried from W1) | `compiled_slug/codec.rs`, `compiled_query/mod.rs` | Zero hits |
| vb-28qw9 (carried from W1) | `validate_compiled_ir_record`, `metadata_hash` | None exist |
| vb-maupz | `submit_checked_artifact_with_evidence`, `admission/flow.rs` | 0 grep hits |
| vb-ko651 | `crates/vb_runtime/src/action_queue/types.rs:125` | File doesn't exist |

## Hallucinations Detected

| Bead | Hallucination |
|------|---------------|
| vb-a5vsl | Fictional `SystemConnectionState` and `system_status_payload(db: ...)` signature |
| vb-aexu6 | Fictional `ShardConfig::validate()`; test count claim 1777/1778 off (actual 1734) |
| vb-9ccdx | Entire coalesce fix claimed but no source changes match |
| vb-b0pfx | Helper function `discard_buffered_events_for_run` doesn't exist |

## Re-occurring NOT-PATCHED Patterns (carried from Wave 1)

| Pattern | Beads | Issue |
|---------|-------|-------|
| Wildcard `_ => Active` lifecycle arms | vb-7gm7c, vb-1rqz7.3 | `journal/incident.rs:168` wildcard + `#[allow(unreachable_patterns)]` mask |
| `legacy_slot_taint` Bool(false)→Clean | vb-574zr, vb-1rqz7.11 | `recovery/replay/summary.rs:748` |
| Counter mutation before fallible work | vb-42nj8 (RP-014), vb-4bq3r (RP-003) | `primitives/together.rs:34,37` |
| Θ(N²) append_to_accumulator | vb-6tnb6 (RP-004) | `primitives/together.rs:139-141` |
| `attempt: 1` hardcoded | vb-msr6g (RS-004) | 3 sites in `shard/impl_parts/chunk_001.rs:633`, `lifecycle/chunk_001.rs:445`, `lifecycle/chunk_002.rs:74` |
| 12 AdmissionErrors → 1 collapsed | vb-l60gb (RA-001) | `shard/lifecycle/chunk_001.rs:252-289` |
| `max_parallel_in_flight: u16::MAX` | vb-lcfj3 (CF-004) | `vb_core/src/frame.rs:105,139` |

## Vacuum Proofs

| Bead | Issue |
|------|-------|
| vb-lcfj3 (CF-004) | `run_frame_invariant.rs` operates on standalone `SpecRunFrame` with comment-only binding |
| vb-loa3o | `vb_jnz9_journal_event_seq_valid.rs` fails Verus outright (3 verified, 1 error postcondition) |
| vb-lxkqh (RP-019) | `vb_8mdp_8/queue_state_shared_source.rs` `helper_*` mirrors without Verus annotations on production |
| vb-y9d3v (RE-013 from W1) | `vb_y9d3v_action_fence.rs` carries `#[verifier::external_body]` + `unimplemented!()` — God-Rule-2 violation outside `verify-verus.sh` trust-scan |
| Pre-existing flux trust debt | `vb_runtime/src/shard/lifecycle/flux_cancel_kill.rs` 12 `#[trusted]`; `vb_storage/src/codec/flux_validation.rs` 23 `#[trusted]` |

## Cross-Cutting Findings

### Shard State
- **IndexMap/IndexSet in hot state**: master Section 11 permissive reading OK, strict reading violated
- **IntrospectionRegistry**: `Arc<Mutex<HashMap<RunId, u64>>>` — not a typed handle table
- **TimerWheel**: `BTreeMap<Instant, Vec<TimerEntry>>` + `HashMap<RunId, TimerEntry>` — master Phase 55 approves this shape, but user's strict prompt forbids
- **No LruRing / LruCache** in production code
- **`drain_for_shutdown` family** silently clears `pending_timers` — no `WaitCancelled`/`AskCancelled` journal events defined

### Action Ticket
- All 7 fields (run, step, seq, action, attempt, idempotency_key, capacity) present in `vb_core/src/action.rs:324-339` ✓
- Idempotency key: computed, validated, recomputed on retry — no violations
- 1 hardcoded `attempt: 1`: `vb_runtime/src/journal/chunk_002.rs:112,121` for legacy storage conversion

### Journal / Replay
- **0 mutate-before-append cases** — every appending transition appends before mutating, with rollback re-insert on append failure
- **1 wildcard lifecycle arm**: `apply_frame_event` at `summary.rs:550` `_ => Ok(self)` (catches 10 non-seed-affecting variants; `apply_summary_event` is exhaustive)
- **Pending action tracking**: maintained correctly via `pending_actions` set

### Diagnostic Contract
- All error variants align with master §17 (typed errors, no panic propagation)

## Workspace Blockers (carry-over from Wave 1)

| Blocker | Location | Effect |
|---------|----------|--------|
| Duplicate function | `crates/vb_runtime/src/test_harness.rs:33-58` and `:63-88` both define `iterator_state_in_slot` | Blocks ALL `cargo test -p vb_runtime --lib` |
| Malformed test file | `crates/vb_storage/src/preview.rs:42-154` has `// TEST_MARKER_1`, duplicated test bodies, unbalanced braces | Blocks storage lib tests |
| Unresolved merge markers | `crates/vb_runtime/src/shard/types.rs:807-815` | Blocks vb_runtime --tests |
| Dead test file | `crates/vb_runtime/src/engine/drive_tests.rs` (1269 lines) never `mod`-included | RE-001 regression tests are dead code |
| Orphan Kani modules | `verification/kani/` has 9 unwired of 13 modules | Kani harnesses not exercised |

## Holzman / NASA-JPL Findings

- **No new Holzman violations introduced** by any PATCHED path
- All production crates declare `#![forbid(unsafe_code)]` at lib root
- All counter increments use `saturating_add` or `checked_add`
- 0 unsafe-touch cases in wave 2 bug fixes
- Dominant failure mode: **incomplete or phantom fixes**, not Holzman regressions

## Per-Agent Reports

- `to-fix/wave2/agent-00-holzman-rust-A.md`
- `to-fix/wave2/agent-01-holzman-rust-B.md`
- `to-fix/wave2/agent-02-explore.md`
- `to-fix/wave2/agent-03-black-hat.md`
- `to-fix/wave2/agent-04-truth-serum.md`
- `to-fix/wave2/agent-05-flux-rs.md`
- `to-fix/wave2/agent-06-arch-drift.md`
- `to-fix/wave2/agent-07-test-reviewer.md`
- `to-fix/wave2/agent-08-miri.md`
- `to-fix/wave2/agent-09-verus.md`
- `to-fix/wave2/agent-10-hands-on-qa.md`
- `to-fix/wave2/agent-11-rust-contract.md`
- `to-fix/wave2/agent-12-adhoc-action-ticket.md`
- `to-fix/wave2/agent-13-adhoc-journal-replay.md`
- `to-fix/wave2/agent-14-adhoc-shard-arena.md`