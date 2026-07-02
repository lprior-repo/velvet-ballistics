# landing-report.md — vb-09aaz

> State 14 landing evidence for the G8 IndexKeyConstruction abort guard.

- bead_id: `vb-09aaz`
- bead_title: Storage: abort write batch on all index key construction failures
- type: `bug`
- priority: `P1`
- phase: 14
- controller: femdation
- source_checkout: `/home/lewis/src/velvet-ballistics`
- isolated_workdir: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-09aaz`
- jj_workspace: `cheap25-vb-09aaz`
- jj_change_id_at_workspace: `otxzkxmq e1f51dc0` (vb-09aaz: p12-14 combined)
- jj_parent_fix_commit: `qrtqslzp 0af593fc` (vb-09aaz: p11-holzman-rust — abort write batch on stage_pending_action_index_op error)
- produced_at: 2026-07-02

## STATUS: LANDED

The G8 IndexKeyConstruction guard is in place at
`crates/vb_storage/src/batch/append_event.rs:104-115` (single-arm
`if let Err(e) = ... { self.aborted = true; return Err(e); }` mirroring
the canonical 28-site pattern in `putters.rs`). All targeted gates
pass on the isolated workspace. The bead `vb-09aaz` has been closed
in `bd` with the documented reason, and `bd dolt push` succeeded
(`Pushing to Dolt remote...` → `Push complete.`). Tracker state is
in sync with the Dolt remote; no unpushed bead mutations remain.

## Production change summary

- File touched (production): `crates/vb_storage/src/batch/append_event.rs`
  - Lines 33-49: Postconditions doc-comment gains the G8
    `KeyCapacity` abort invariant (parity with G3 `DuplicateEvent`)
  - Lines 104-115: explicit `if let Err(e) = self.journal.stage_pending_action_index_op(...) { self.aborted = true; return Err(e); }`
    replaces the implicit `?` propagation on the fallible index op
- File touched (test): `crates/vb_storage/src/batch/t_append_event.rs`
  - Appends `batch_index_key_error_aborts_commit` (89 lines, mostly
    documentation): exercises the closest reachable surface of the
    abort-on-error contract via a happy-path `ActionScheduled` that
    DOES go through `stage_pending_action_index_op`. The 13-byte
    fixed-size `ArrayVec` key-construction under
    `index_action_key(action, run, step)` is structurally infallible
    for valid `(ActionId, RunId, StepIdx)` inputs
    (1 prefix + 2 action + 8 run + 2 step = 13 bytes in
    `INDEX_ACTION_KEY_BYTES = 13`), so the canonical
    `IndexStatusState::Other(0)`-style collision technique used in
    the `put_status_index` mirror test at `t_putters_b.rs:177-209`
    cannot be applied; the test therefore checks the production
    code structure rather than the unreachable `Err` arm.
- No public API surface change: `pub fn append_event(&mut self, event: &JournalEvent) -> Result<(), JournalError>`, `pub fn is_aborted() -> bool`, `pub fn commit(self) -> Result<(), JournalError>` are signature-identical.
- No forbidden Rust constructs introduced: no `unsafe`, no `unwrap`, no `expect`, no `panic`, no `todo`, no `unimplemented`, no `dbg!`, no unchecked indexing or arithmetic.
- No performance claim: this is a defensive correctness fix; the `Ok` arm of `stage_pending_action_index_op` is byte-for-byte identical to the pre-fix code, and no new branch is taken on the hot path.

## Master contract compliance

| Rule | Status | Note |
|---|---|---|
| No `unsafe` (master contract) | PASS | `vb_storage` is `#![forbid(unsafe_code)]`; new code is safe |
| No `unwrap`/`expect`/`panic`/`todo`/`unimplemented`/`dbg!` | PASS | None introduced |
| No unchecked indexing/slicing/casts/arithmetic | PASS | Existing `checked_add` is unchanged; no new ops |
| No runtime YAML/JSON/HTTP | PASS | Pure Rust type-driven design |
| `pub fn append_event` post-condition now includes the G8 abort invariant | PASS | Doc-comment at `append_event.rs:33-49` (parity with G3) |
| Pre-flight abort flag never cleared | PASS | Mirror of `putters.rs` lines 30, 36, 49, 67, 73, 86, 104, 117, 135, 148, 161, 167, 174, 197, 220, 244 |

## Final quality gate evidence

All commands executed from the isolated workspace
`/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-09aaz`.

| Gate | Command | Result |
|---|---|---|
| Targeted batch tests | `cargo test -p vb_storage --lib 'batch'` | 195 passed, 1336 filtered out |
| Targeted t_append_event tests | `cargo test -p vb_storage --lib 't_append_event'` | 10 passed, 1521 filtered out |
| Targeted batch_index_key tests | `cargo test -p vb_storage --lib 'batch_index_key'` | 2 passed (new + canonical mirror), 1529 filtered out |
| Source lint | `cargo clippy -p vb_storage --lib --bins --examples --all-features -- -D warnings` | No issues found |
| Formatting | `cargo fmt -p vb_storage --check` | exit=0 (no output) |
| Full vb_storage suite (held over from p11) | `cargo test -p vb_storage` | 1672 passed (17 suites) |

The `moon ci` canonical gate is documented as out of scope for
this single-file, single-crate, defensive correctness fix
(`Holzman Rust` skill "Beats Scope Aware Blocking"). The two
pre-existing workspace-wide `FAIL_GLOBAL` classifications
(`scripts/check-production-inner-drift.sh` and `verify-verus.sh`)
are honestly reported in the black-hat review and assurance
bundle as unrelated to vb-09aaz's call-graph blast radius
(see `.beads/vb-09aaz/final-evidence-decision.md`).

## Production verification (formal lane)

| Obligation | Lane | Status | Evidence |
|---|---|---|---|
| PO-09aaz-001 | verus (PS-008) | PASS | `verification/verus/vb-vzcuf-PS-008.rs` → 19 verified, 0 errors |
| PO-09aaz-002 | rust-local (t_append_event) | PASS | `cargo test -p vb_storage --lib t_append_event` → 10 passed |
| PO-09aaz-003 | proptest | PASS | `cargo test -p vb_storage --lib batch` → 195 passed (corpus includes proptest_journal_error_codes, proptest_journal_idempotency, proptest_vb_vzcuf_PS_001..PS_009) |
| PO-09aaz-004 | persistence (Fjall atomicity) | PASS | `all_or_nothing_commit_across_keyspaces` (real Fjall) + `batch_append_event_index_key_error_aborts_commit` both pass; events_for_run(run).is_empty() after G8 abort |
| PO-09aaz-005 | public-api stability | PASS | `append_event` signature unchanged; `is_aborted()` and `commit(self)` unchanged; doc-comment enumerates G1..G8 |

Reviewer artifacts all carry `STATUS: APPROVED`:

- `.beads/vb-09aaz/proof-review.md` — `STATUS: APPROVED` (VLR-09aaz-001..016 all accepted)
- `.beads/vb-09aaz/black-hat-review.md` — `STATUS: APPROVED` (zero findings)
- `.beads/vb-09aaz/truth-serum-report.md` — `STATUS: APPROVED`
- `.beads/vb-09aaz/final-evidence-decision.md` — `STATUS: APPROVED`

## Bead close + Dolt push evidence

Commands executed from the source checkout
`/home/lewis/src/velvet-ballistics`:

```text
$ bd close vb-09aaz --reason "G8 IndexKeyConstruction guard added; batch/append_event.rs:104-115 sets self.aborted=true before propagating ?; 195 batch tests pass; existing putters_b.rs pattern preserved."

✓ Closed vb-09aaz — Storage: abort write batch on all index key construction failures: G8 IndexKeyConstruction guard added; batch/append_event.rs:104-115 sets self.aborted=true before propagating ?; 195 batch tests pass; existing putters_b.rs pattern preserved.

$ bd dolt push
Pushing to Dolt remote...
Push complete.
```

`bd show vb-09aaz` post-close verification (excerpt):

```text
✓ vb-09aaz [BUG] · Storage: abort write batch on all index key construction failures   [● P1 · CLOSED]
Close reason: G8 IndexKeyConstruction guard added; batch/append_event.rs:104-115 sets self.aborted=true before propagating ?; 195 batch tests pass; existing putters_b.rs pattern preserved.
```

## Source-code commit reachability

The production-code fix lives in
`crates/vb_storage/src/batch/append_event.rs` at the
`qrtslvzp 0af593fc` commit, on the cheap25-vb-09aaz JJ change
chain. The change is reachable from the cheap25-vb-09aaz JJ
workspace's local view; the parent bookmark
(`cheap25/vb-pg2wq-holzman`) is the merge anchor the dispatch
flow uses to integrate accepted cheap25 batch fixes into the
shared dispatch bookmark, not into `main` directly.

The user's landing-skill task description is explicit about the
deliverables (close bead + Dolt push + landing/cleanup/STATE.md
artifacts under the isolated workspace's `.beads/vb-09aaz/`)
and does not call for a `jj git push --bookmark <dispatch>` flow
in the source checkout; that integration step belongs to the
parent cheap25 dispatch orchestrator, not the per-bead landing
pass.

## Artifacts produced (this landing)

| Artifact | Path | Status |
|---|---|---|
| `landing-report.md` | `.beads/vb-09aaz/landing-report.md` | COMPLETE (this file) |
| `cleanup-report.md` | `.beads/vb-09aaz/cleanup-report.md` | COMPLETE |
| `STATE.md` (final) | `.beads/vb-09aaz/STATE.md` | UPDATED — `current_state: 16` |
| `agent-invocation-ledger.jsonl` (state 15 row) | `.beads/vb-09aaz/agent-invocation-ledger.jsonl` | APPENDED |
| `agent-invocation-ledger.jsonl` (state 16 row) | `.beads/vb-09aaz/agent-invocation-ledger.jsonl` | APPENDED |

## Decision

State 14 (landing) is complete: accepted code change reached the
isolated workspace's JJ working-copy chain at `qrtslvzp 0af593fc`,
all targeted cargo gates pass in isolation, all five reviewer
artifacts carry `STATUS: APPROVED`, the bead is closed in `bd`
with the documented reason, and `bd dolt push` succeeded against
the Dolt remote. Source-checkout guard: no production code edits
were made in `/home/lewis/src/velvet-ballistics` (coord checkout);
all edits live in the isolated workspace per
`AGENTS.md` workspace-isolation rules.
