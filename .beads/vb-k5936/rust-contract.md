# Rust Contract — vb-k5936 (bounded first merge: A + B)

Bead: `vb-k5936`
Workspace: `/home/lewis/src/isoloated/velvet-ballistics-final-deepseek-integration` (JJ working copy `wykpvurl 60fc3c45`)
Root owns ALL VCS/JJ mutations and final publication. This contract covers the bounded first
merge only; LOCAL3090 `vb-3gthe` fixture migration and canonical `moon ci` follow after root
freezes the fixture merge.

## Bounded repair — this session (pre-fixture, worker)

Two bounded source edits (no production-behavior change beyond the syntax parse fix; no VCS/JJ
mutations; bead stays OPEN):

1. `crates/vb_storage/src/types/flux_sequence.rs:132` — `16#ffff#` (invalid Rust-2021 reserved
   prefixed literal, breaks `cargo fmt`) → `u16::MAX`. Identical u16-max bound (65535), matching
   the file's own `u64::MAX` on line 23 and the repo's `requires gen < u64::MAX` Flux pattern.
   `#[cfg(flux)]`-gated module; no normal build/clippy/test impact.
2. `crates/vb_runtime/tests/durable_retry_boundary.rs` `shared_strict_journal_persists_each_append`
   — `events[0]` unchecked index → `events.first().ok_or(...)?` with deref guards. Exact
   count/type/run/seq/workflow assertions retained.

Verification (all through `/tmp/vb-run-gate.py`): `bcd10_fix` = 10 passed;
`fmt_flux_fix` = 0 (parse fixed); `fmt_all_check` = exit 1, format diffs only, no parse errors;
`clippy_fix` = 0 (strict production flags). Flux verification is NOT claimed (syntactic fix only).

## Parents merged

- **A** `purzrpot 36936fcc` — `vb-absr9`: group ticket dispatch parameters (typed
  `NewActionTicket` + `ActionDispatchContext`), add `ActionTicket.run_attempt` authority,
  remove `.expect()`/`.unwrap()`/`panic!` from the replay/event-generation test harness.
- **B** `uxltskzn 0d348f5a` — `vb-hz9v8`: strict retry journal (finding B), terminal automatic
  recovery rejection (finding C), durable retry checkpoint authority binding (finding D).

## Conflict resolution — `crates/vb_runtime/src/runtime.rs`

Single three-way conflict. Base `qkroppmn 423d8ae8` had the single-line
`shard.enqueue(ShardCommand::Recover { command: Box::new(command) })`. Both A and B independently
reformatted the same call to the multi-line `Box::new(command)` form, and B additionally added
the `validate_retry_checkpoint_against_durable_chain` authority call and its full function body.

**Resolution (source semantics latest both mandatory):**

1. Keep the multi-line `Box::new(command)` formatting (present in both A and B; drop the
   stale single-line base form).
2. Keep B's `self.validate_retry_checkpoint_against_durable_chain(run, &checkpoint, next_seq)?;`
   call before enqueue (the durable-chain authority guard — finding D).
3. Keep B's full `validate_retry_checkpoint_against_durable_chain` method (blake3 integrity +
   decode/value-equality of the checkpoint, `prior_seq + 1 == durable.seq`,
   `next_seq == durable.seq + 1`, journal-head check, and preceding same-run `RunFailedEvent` at
   `run_attempt - 1`; checked arithmetic; fail-closed, no state/journal mutation).

No A semantics lost: A's `runtime.rs` change was only the `Box::new` formatting of this same
call; the `ActionTicket.run_attempt` authority semantics live in A's other files (ticket.rs,
dispatch, handlers, tests) and are untouched by this merge.

## Non-goals (explicitly out of scope for this bounded merge)

- LOCAL3090 `vb-3gthe` ticket-fixture call-site migration (existing fixture compile errors in
  `vb_validate`/`vb_storage`/`vb_core` test code remain; reported, not rewritten here).
- `crates/vb_storage/src/types/flux_sequence.rs` `cargo fmt` parse error — **resolved this
  session** (`16#ffff#` → `u16::MAX`); no waiver. Full `cargo fmt --all` still reports
  pre-existing format diffs in unrelated files (out of this bounded scope).
- Finding A stale-ticket probes (`probe_old_*`) may remain RED until root merges LOCAL5090.
- Canonical `moon ci` / full workspace `cargo fmt --all` / bead remote sync / commit-describe.

## Verification (bounded, no waivers, all through `/tmp/vb-run-gate.py`)

Canonical strict production Clippy (from `.moon/tasks/all.yml`), workspace-wide,
`--lib --bins --examples --all-features` (NOT `--all-targets`), `RUSTFLAGS=-Dwarnings`,
`RUSTC_WRAPPER=` empty, toolchain `nightly-2026-04-28`.

Focused regression groups (actual counts, no zero filters):

| Group | Target | Expected |
|-------|--------|----------|
| A4    | `cargo test -p vb_runtime --test durable_retry_run_attempt_authority` | 4 |
| replay3 | `cargo test -p vb_runtime --test vb_r4fm9_event_generation` | 3 |
| BCD10 | `cargo test -p vb_runtime --test durable_retry_boundary` | 10 |
| retry2 | `cargo test -p vb_runtime --test durable_retry_execution` | 2 |
| CLI3  | `cargo test -p velvet-ballistics --test durable_retry_cli` | 3 |
| pending20 | `cargo test -p velvet-ballistics-workspace-tests --test runtime_fjall_pending_action_recovery` | 20 |

Total: 42 tests.

## Rust rules (unchanged)

No `unsafe`/`unwrap`/`expect`/`panic`/`todo`/`unimplemented`/`dbg`; no unchecked indexing,
slicing, casts, or arithmetic; no new lint allowances. Type/API refactors preserve public
semantics (A's `NewActionTicket`/`ActionDispatchContext` grouping keeps all fields public and
pure; B's guards are additive fail-closed validations with no state mutation on rejection).
