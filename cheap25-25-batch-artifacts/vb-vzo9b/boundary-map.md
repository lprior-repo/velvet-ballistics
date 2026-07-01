# Boundary Map — vb-vzo9b

> **Scope.** Boundaries crossed by the post-fix `fuzz_recovery_decode` body.
> The fuzz body sits at a single boundary (hostile-input fuzz) and calls two
> production decoders (`summarize_recovery_events` and
> `recover_runtime_frame_seed_from_events`) that share internal boundaries.
> The fuzz body is the imperative shell; `summarize_recovery_events` is the
> pure-core decoder. No new boundaries are introduced.

## Boundary 1 — Fuzz Payload → Fuzz Body (hostile-input)

| Aspect | Value |
|---|---|
| Boundary kind | Hostile-input fuzz |
| Input | `data: &[u8]` (arbitrary bytes) |
| Parser/decoder | None at this boundary; the fuzz body directly consumes `data[0]` as `run` and `data.len() % 2` as a branch selector. |
| Validation | `data.first().copied().unwrap_or(0)` for `run`; otherwise unchecked. |
| Trust | **Untrusted.** The fuzz body is the producer; the fuzz harness / OSS-Fuzz is the threat model. |
| Defects this boundary can introduce | `RunId::new(0)` collision with the empty-events sentinel; `data.len() == 0` short-circuit; oversized payloads. The current fuzz body treats 0-length payloads as `S-Odd`. |

## Boundary 2 — Fuzz Body → `summarize_recovery_events` (pure-core decoder)

| Aspect | Value |
|---|---|
| Boundary kind | Production decoder |
| Input | `events: &[JournalEvent]` (vec of 0 or 1 element) |
| Output | `RecoveryResult<RecoveryHydration>` |
| Module | `crates/vb_storage/src/recovery/replay/summary/apply.rs:88-129` |
| Trust | Trusted (production code, unchanged). |
| Failure modes | `Err(NoRecoveryData { run: RunId::new(0) })` for empty; `Err(ReplayDivergence { .. })` for multi-run/seq-overflow; `Err(WorkflowSourceDigestMismatch { .. })` etc. for incompatibilities. |
| Pre-fix contract | Weak: `assert!(run_summary.run == run || run_summary.run == RunId::new(0))` — accepts two distinct values. |
| **Post-fix contract** | **Strong: `assert_eq!(run_summary, expected)` over all 11 fields.** |

## Boundary 3 — Fuzz Body → `recover_runtime_frame_seed_from_events` (pure-core decoder)

| Aspect | Value |
|---|---|
| Boundary kind | Production decoder |
| Input | `events: &[JournalEvent]` |
| Output | `RecoveryResult<RecoveryFrameSeed>` |
| Module | `crates/vb_storage/src/recovery/replay/summary/derive.rs:60-77` |
| Trust | Trusted. |
| Failure modes | Same shape as Boundary 2 — `NoRecoveryData { run: RunId::new(0) }` and the two `ReplayDivergence` sub-cases (multi-run and seq-overflow). |
| Pre-fix contract | None observable from the fuzz body (only the error path is inspected via `assert_typed_recovery_error`). |
| Post-fix contract | **Unchanged**: error path remains `assert_typed_recovery_error`; success path was never asserted and remains so. |

## Boundary 4 — Fuzz Body → `assert_typed_recovery_error` (typed-error sink)

| Aspect | Value |
|---|---|
| Boundary kind | Test-only typed-error exhaustiveness sink. |
| Input | `RecoveryError` |
| Output | `()` |
| Module | `fuzz/src/journal_target/errors.rs:57-72` |
| Trust | Trusted. The `_ => {}` catch-all makes it `()`-returning, so the fuzz body cannot panic through this sink. |
| Post-fix obligation | None — already correct. |

## Functional core / imperative shell split

| Layer | Belongs to vb-vzo9b? | Notes |
|---|---|---|
| Functional core | **No** — production `summarize_recovery_events` and `recover_runtime_frame_seed_from_events` are unchanged. | Both are `&[JournalEvent] → RecoveryResult<...>` — pure with respect to time, storage, RNG. |
| Imperative shell | **Yes** — the fuzz body. | The fuzz body is the simplest possible imperative shell: derive inputs, call pure core, assert. |
| Async shell | **No.** | No `async`/`await` in the production decoders or the fuzz body. |
| Storage | **No.** | The fuzz driver does not touch storage; it works on in-memory `Vec<JournalEvent>`. |
| Network | **No.** | |
| Time | **No.** | |
| FFI | **No.** | `blake3::hash` is pure and side-effect-free (no `unsafe`). |
| Unsafe | **No.** | Pre-fix and post-fix bodies contain no `unsafe` blocks. |
| Parser | **No — first-party parsers.** | The boundary between fuzz byte slice and `JournalEvent` is the fuzz body itself; it does not parse an external format. |

## Boundary-crossing invariants

| ID | Invariant |
|---|---|
| BCI-1 | `summarize_recovery_events(&[])` returns `Err(NoRecoveryData { run: RunId::new(0) })` — invariant produced by `apply.rs:89-91`. |
| BCI-2 | `summarize_recovery_events(&[RunAccepted { run, seq, workflow: digest }])` returns `Ok(Summary(RecoveryRuntimeSummary { run, first_seq: seq, last_seq: seq, workflow: Some(workflow), steps_*: 0, slots_written: 0, suspensions: 0, terminal: None }))` — invariant produced by `apply.rs:93-105` + `apply_summary_event_checked`. |
| BCI-3 | `assert_typed_recovery_error` returns `()` for every `RecoveryError` variant (including the `_ => {}` catch-all). |
| BCI-4 | The fuzz body is the only site permitted to call `assert_typed_recovery_error`; production code never calls it. |

## Boundaries NOT crossed (defense-in-depth notes)

- **No I/O.** No journal write, no disk, no socket — safe to run in CI under
  `cargo test`.
- **No RNG.** `blake3::hash(data)` is deterministic; fuzz corpora are stable
  across runs.
- **No async runtime.** No `tokio`, no `async`, no `await`.
- **No thread spawn.** Concurrency lane is **explicitly N/A** for this bead;
  see `proof-seeds.jsonl` and `hazard-analysis.md`.

## Closure commands (downstream evidence; not part of this contract)

| Command | Purpose |
|---|---|
| `cargo test -p vb_storage --lib summarize_recovery_events` | Confirms `apply.rs` invariants used by the post-fix body. |
| `cargo test -p vb_storage --lib recover_runtime_frame_seed_from_events` | Confirms `derive.rs` invariants. |
| `cargo build -p fuzz --bin recovery_decode` | Confirms the post-fix `readback.rs` body compiles. |
| `cargo test -p vb_storage --lib` (transitive) | Confirms the strong-pattern tests at `replay/summary/tests.rs:285-302` still pass. |

These gates are documented for `proof-to-implementation` and `formal-verifier`;
they are not enforced by this contract.
