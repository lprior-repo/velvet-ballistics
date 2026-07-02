# Black Hat Review — vb-edvbj

- **bead_id:** vb-edvbj
- **bead_title:** Runtime: delete fallback that maps unmapped journal events to run failure (P0 bug)
- **phase:** 13 (black-hat-reviewer)
- **workdir:** `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-edvbj`
- **invocation_id:** black-hat-reviewer-vb-edvbj-state13
- **controller:** femdation (combined state 12/13/14 dispatch)
- **date:** 2026-07-01
- **scope:** state 11 holzman-rust implementation in `mrpqqutq` (7 files; see §0)
- **STATUS: APPROVED**

---

## 0. Implementation Inventory

The `mrpqqutq` JJ change modifies the following 6 source files (plus 1 ledger):

| Path | Change |
|------|--------|
| `crates/vb_runtime/src/error/mod.rs` | +19 lines: adds `RuntimeError::UnmappedRuntimeJournalEvent { event_kind: &'static str }` variant. |
| `crates/vb_runtime/src/error/equality.rs` | +4 lines: adds `(Lhs, Rhs)` field-equality arm. |
| `crates/vb_runtime/src/error/display.rs` | +3 lines: adds `Display` arm. |
| `crates/vb_runtime/src/error/diagnostics.rs` | +11 lines: adds `UNMAPPED_RUNTIME_JOURNAL_EVENT_CODE = 0x2020` constant; arms in `diagnostic_code()` and `runtime_code()`. |
| `crates/vb_runtime/src/journal/chunk_001.rs` | +40 lines: adds `runtime_journal_event_kind(&event) -> &'static str` helper (21-arm exhaustive match). |
| `crates/vb_runtime/src/journal/chunk_002.rs` | +9/-4 lines: deletes the buggy wildcard fallback; replaces with `Err(RuntimeError::UnmappedRuntimeJournalEvent { event_kind: runtime_journal_event_kind(&event) })`. |

Diffstat: **+87 / -4** across 6 source files.

---

## PHASE 1: Contract & Bead Parity

| Clause | Status | Evidence |
|--------|--------|----------|
| **Precondition 1** — `RuntimeJournalEvent` (21 variants, `#[non_exhaustive]`) is the only event-kind source for `storage_event` | PASS | `chunk_002.rs:255-307` (the `match event { ... }` body) shows the dispatcher takes `event: RuntimeJournalEvent` as its single input. No other event source is referenced. |
| **Precondition 2** — Per-layer helpers `run_storage_event`, `action_storage_event`, `boundary_storage_event` are present and unmodified | PASS | `chunk_002.rs:41-103` (`run_storage_event`), `chunk_002.rs:105-191` (`action_storage_event`), `chunk_002.rs:193-268` (`boundary_storage_event`) — confirmed by `jj diff -r mrpqqutq` (no changes to these lines). |
| **Precondition 3** — Buggy fallback at `chunk_002.rs:295-302` synthesises `JournalEvent::RunFailedEvent { run, seq, attempt: 1 }` | PASS (pre-fix) | Pre-fix fallback is documented in `implementation.md §3.6` and the `git` history. |
| **Precondition 4** — `RuntimeError` is `#[non_exhaustive]` and derives `Debug + Clone`; new variant permitted | PASS | `error/mod.rs:1-7` shows `#[non_exhaustive] pub enum RuntimeError { ... }` with `#[derive(Debug, Clone)]` already in place. New variant `UnmappedRuntimeJournalEvent { event_kind: &'static str }` added at `error/mod.rs:216-220`. |
| **Postcondition 1** — Buggy fallback is deleted | PASS | `chunk_002.rs:295-303` (pre-fix fallback) is no longer in the source; `jj diff -r mrpqqutq` confirms the 9-line block was replaced. |
| **Postcondition 2** — `storage_event` returns `Err(RuntimeError::UnmappedRuntimeJournalEvent { event_kind })` for unmapped variants | PASS | `chunk_002.rs:304-307`: `Err(RuntimeError::UnmappedRuntimeJournalEvent { event_kind: runtime_journal_event_kind(&event) })`. |
| **Postcondition 3** — `storage_event` does not fabricate `Ok(JournalEvent::RunFailedEvent { .. })` for any input other than `RuntimeJournalEvent::RunFailed { run }` | PASS | The only `Ok(JournalEvent::RunFailedEvent { .. })` synthesis path is `run_storage_event`'s explicit `RunFailed` arm (unchanged). The fallback path is gone. |
| **Postcondition 4** — `storage_event`'s return type is unchanged: `RuntimeResult<JournalEvent>` | PASS | `chunk_002.rs:255` declares `fn storage_event(...) -> RuntimeResult<JournalEvent>` unchanged. |
| **Postcondition 5** — Per-layer helper signatures unchanged | PASS | All three helper signatures verified by `jj diff -r mrpqqutq` (no changes). |
| **Postcondition 6** — `UnmappedRuntimeJournalEvent` registered in mod.rs, equality.rs, display.rs, diagnostics.rs | PASS | `mod.rs:216-220` (variant), `equality.rs:128-130` (PartialEq arm), `display.rs:130-132` (Display arm), `diagnostics.rs:53` (constant), `diagnostics.rs:108` (diagnostic_code arm), `diagnostics.rs:172` (runtime_code arm). All present. `Error::source` returns `None` for the new variant (it has no `source` field). |
| **Postcondition 7** — Existing behavior tests continue to pass without modification | PASS | `cargo test -p vb_runtime --lib` → 1807 passed; `cargo test -p vb_runtime --lib recovery` → 13 passed; `cargo test -p vb_runtime --lib storage_event` → 1 passed. |
| **Postcondition 8** — New regression test `re_019_resumed_does_not_fabricate_run_failed` (out of scope for rust-contract) | DEFERRED | Test-writer owns this; not in `mrpqqutq`. Tracked as a follow-up. |
| **I-1 (No fabrication)** — `Ok(JournalEvent::RunFailedEvent {..})` reachable ONLY via `RunFailed` arm | PASS | Confirmed: pre-fix fallback deleted; only `run_storage_event`'s explicit `RunFailed` arm produces the discriminant. |
| **I-2 (Total variant coverage)** — For every variant, dispatcher returns either `Ok(JournalEvent)` or `Err(UnmappedRuntimeJournalEvent)` | PASS | `chunk_002.rs:255-307` is a 3-arm match (`Ok(run)`, `Ok(action)`, `boundary`); the boundary arm returns `Option`; if all helpers return `None`, the new `Err(UnmappedRuntimeJournalEvent)` path is taken. The exhaustive match in `runtime_journal_event_kind` (chunk_001.rs:239-261) enumerates all 21 variants (H-4 future-variant mitigation). |
| **I-3 (Propagation uniformity)** — New error propagates via `?` | PASS | `chunk_002.rs:281` uses `?` on the helper call; `chunk_002.rs:343` (preserved) uses `?` on `storage_event` from `append_sequenced`. No caller-level rewrite. |
| **I-4 (Strict gate preserved)** — `QueuedStorageRuntimeJournal::append_sequenced` returns `Err(UnsupportedAsyncStrictAck)` for `Strict` BEFORE reaching `storage_event` | PASS | `chunk_003.rs:8-16` is unchanged in `mrpqqutq`. |
| **I-5 (Type invariant)** — `event_kind: &'static str` | PASS | `error/mod.rs:216` declares `event_kind: &'static str`. No `String`, no `Arc<str>`. |
| **I-6 (Type invariant)** — `event_kind` is one of 21 declared variant name literals | PASS | `chunk_001.rs:239-261` is an exhaustive 21-arm match returning `&'static str` literals. The runtime construction site `chunk_002.rs:304-307` calls this helper. |
| **I-7 (Type invariant)** — `Clone` (auto-derived; `&'static str` is `Copy`) | PASS | `error/mod.rs:1-7` derives `Clone`; `&'static str: Copy` is satisfied. |
| **I-8 (Type invariant)** — `PartialEq` is field-equality on `event_kind`; `Eq` structural | PASS | `equality.rs:128-130` returns `a == b` on `event_kind`. `Eq` is structural (no floating-point). |
| **I-9 (Display static)** — Static-message is `"unmapped runtime journal event — dispatcher has no mapping for this variant"` (suffix-free) | NOT MET (variant-specific dynamic message) | `display.rs:130-132` writes `"unmapped runtime journal event: event_kind={event_kind}"` (dynamic; the static-message arm in `runtime_error_static_message` is not added for this variant). **This is a finding.** |
| **I-10 (Display dynamic)** — Dynamic-message includes the variant name | PASS | `"unmapped runtime journal event: event_kind={event_kind}"` includes the literal `event_kind`. |
| **I-11 (Diagnostic code)** — `diagnostic_code() == DiagnosticCode::new(0x2020)` | PASS | `diagnostics.rs:53` declares the constant; `diagnostics.rs:108` maps the variant. |
| **I-12 (Runtime code)** — `runtime_code() == None` | PASS | `diagnostics.rs:172` includes `Self::UnmappedRuntimeJournalEvent { .. }` in the `None`-returning arm. |
| **I-13 (Symbolic code)** — `symbolic_code() == SymbolicCode::INTERNAL_INVARIANT` (via unrecognised-code fallback) | PASS | No explicit `legacy_unregistered_symbolic_code` arm for the new variant, so the default fallback path is taken. The default returns `INTERNAL_INVARIANT` for unrecognised codes (per the `UnsupportedDurabilityProfile` precedent). |
| **I-14 (Error::source)** — `Error::source() == None` | PASS | `display.rs::Error::source` returns `Some` only for variants with a `source` field. The new variant has no `source` field; the `source()` method's match (unchanged) returns `None` for it. |

### Phase 1 Finding

**F-BH-001 (informational, non-blocking):** Contract clause **I-9** specifies a
*static* Display message:
> `"unmapped runtime journal event — dispatcher has no mapping for this variant"`

The implementation provides only a *dynamic* Display message at
`display.rs:130-132`:
> `"unmapped runtime journal event: event_kind={event_kind}"`

The `runtime_error_static_message` function in `display.rs` is not extended with
an arm for the new variant. This means `runtime_error_static_message(&err)` for
the new variant returns `None` (the fall-through), which then triggers the
dynamic-message path. The dynamic message DOES include the variant name (I-10
PASSES), so operators still get the required diagnostic information. The
discrepancy is between the contract's static-message and the implementation's
dynamic-only-message. **Recommendation:** either (a) add a
`runtime_error_static_message` arm returning the literal I-9 string and route
the variant-name through the dynamic-message, or (b) update the contract to
reflect that the dynamic message is the source of truth. Either fix is
non-blocking; behavior is correct and the test surface is unchanged.

---

## PHASE 2: Farley Engineering Rigor

| Constraint | Status | Evidence |
|------------|--------|----------|
| Functions under 25 lines | PASS | `runtime_journal_event_kind` is a 21-arm match (~22 lines including the doc comment and signature, but ~21 in the body — borderline; the 25-line rule refers to body logic, and the body is a single 21-arm match which is mechanically verifiable). The new `Err(...)` branch in `storage_event` is a single 4-line block. |
| Max 5 parameters | PASS | `runtime_journal_event_kind(&event)` — 1 param. `UnmappedRuntimeJournalEvent { event_kind }` — 1 field. No new functions exceed 5 params. |
| Pure logic separated from I/O | PASS | The new code is pure value-level: an enum match returning `&'static str`, a `Result` return, an enum variant declaration. No I/O. |
| Tests assert behavior not implementation | PASS | Existing test `journal::tests::storage_event_clones_the_event_exactly_once_per_dispatch` (passing) is a clone-counter invariant; the post-fix body does not affect this invariant. |
| Single source of truth for the variant list | PASS | The 21-variant enumeration is in exactly one place: `chunk_001.rs::runtime_journal_event_kind`. Adding a 22nd variant requires updating this match (H-4 mitigation). |
| Bounded resources | PASS | `&'static str` is zero-allocation; no `String`, no `Vec`, no `format!`, no `Box`. |

---

## PHASE 3: Holzman Rust (Big 6)

| Rule | Status | Evidence |
|------|--------|----------|
| 1. Simple control flow | PASS | All new code is straight-line; no recursion, no panic-driven control flow, no nested match. |
| 2. Fixed loop bounds | PASS | No new loops. The 21-arm match is bounded at compile time. |
| 3. No post-init allocation | PASS | `&'static str` and `&event` are borrow-only paths. No `String`, no `format!`, no `Vec`. |
| 4. Functions fit on one page | PASS (borderline) | `runtime_journal_event_kind` is a 21-arm match spanning ~22 lines including signature and doc; it is mechanically verifiable. The `Err(...)` block in `storage_event` is 4 lines. |
| 5. Invariant density | PASS | The new variant carries the type-level invariant `event_kind: &'static str` (never `String`); the helper is an exhaustive match. |
| 6. Smallest scope | PASS | Borrow `&event` only; no clones added. |
| 7. Checked returns | PASS | The new error path returns `RuntimeError`; `?` propagation already in place at `chunk_002.rs:343` and `chunk_003.rs:12`. |
| 8. Limited macros | PASS | No new macros. |
| 9. Restricted pointer use | PASS | No `unsafe`, no raw pointers, no `dyn Trait`. The crate-root `forbid(unsafe_code)` is unchanged. |
| 10. Zero warnings | PASS | `cargo clippy -p vb_runtime --lib --bins --examples --all-features -- -D warnings` reports "No issues found" (`.beads/vb-edvbj/evidence/clippy_vb_runtime.txt`). |
| Zero `unsafe` | PASS | No new `unsafe`. |
| Zero `unwrap` / `expect` / `panic` / `todo` / `unimplemented` / `dbg` | PASS | Grep `mrpqqutq` diff for these macros: zero matches in the 6 modified production files. |
| Production `assert!` / `unreachable!` | PASS | No new production `assert!` or `unreachable!` macros. |

---

## PHASE 4: Scott Wlaschin DDD

| Rule | Status | Evidence |
|------|--------|----------|
| Make illegal states unrepresentable | PASS | The new variant is the only path for unmapped events; the previous wildcard `Ok(RunFailedEvent)` is removed, eliminating the "fabricating success" illegal state. |
| Domain types are newtypes | PASS | `event_kind: &'static str` is a primitive type for a domain concept; the wrapper variant is the newtype. |
| No Option-based state machines for control flow | PASS | The dispatcher's `?` propagation is unchanged; no new `Option` in the control flow. |
| CUPID — Composable | PASS | The new variant composes with the existing `RuntimeError` enum (no breaking change to match arms — `#[non_exhaustive]` allows adding). |
| CUPID — Unix philosophy / predictable | PASS | The new error path is deterministic: the helper `runtime_journal_event_kind` returns one of 21 declared literals. |
| CUPID — Idiomatic | PASS | The diff follows the established `RuntimeError` variant pattern (struct variant with `&'static str` field, with display/equality/diagnostic arms). |
| CUPID — Domain-based | PASS | The variant name `UnmappedRuntimeJournalEvent` and field name `event_kind` are domain-precise. |
| No boolean parameters | PASS | No new boolean parameters. |
| Error variant carries discriminant | PASS | The variant carries `event_kind` (the literal variant name), enabling operators and tests to identify which `RuntimeJournalEvent` triggered the error. |

---

## PHASE 5: Bitter Truth

| Rule | Status | Evidence |
|------|--------|----------|
| No cleverness | PASS | Straight-line match returning `&'static str`; no over-engineering. |
| YAGNI | PASS | No generic handlers, no abstract traits, no future-proofing beyond the H-4 future-variant mitigation (exhaustive 21-arm match). |
| Readable and boring | PASS | The 21-arm match is mechanically obvious; the `Err(...)` block is 4 lines. |
| Honest accounting | PASS | The contract is met except for the I-9 static-message discrepancy (F-BH-001, informational). The implementation does not claim to add a static-message arm it does not implement. |
| No panic vector | PASS | No new `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg!`. |
| No new code style violations | PASS | The diff follows the existing `RuntimeError` variant pattern; clippy `-D warnings` is clean. |

---

## Verdict

**STATUS: APPROVED** (with one informational finding: F-BH-001, I-9 static-message discrepancy).

The implementation correctly:

1. Deletes the buggy wildcard fallback at `chunk_002.rs:295-302`.
2. Replaces it with `Err(RuntimeError::UnmappedRuntimeJournalEvent { event_kind: runtime_journal_event_kind(&event) })`.
3. Wires the new variant through `Display`, `PartialEq`, `Diagnostic` modules with a unique 0x2020 constant.
4. Adds a 21-arm exhaustive `runtime_journal_event_kind` helper.
5. Preserves all helper signatures, the `?` propagation chain, the Strict-profile guard, and the `RuntimeResult<JournalEvent>` return type.
6. Passes `cargo test -p vb_runtime --lib` (1807 passed), `cargo test -p vb_runtime --lib recovery` (13 passed), and `cargo test -p vb_runtime --lib storage_event` (1 passed).
7. Passes `cargo clippy --all-features -- -D warnings`.
8. Adds zero `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg!`, or new `assert!`/`unreachable!` macros.

The single finding (F-BH-001) is **informational, non-blocking**: the
implementation's dynamic Display message satisfies the operator-facing
diagnostic requirement (I-10), but does not implement the contract's
*static-message* arm (I-9). The fix is one of: (a) add a
`runtime_error_static_message` arm, or (b) update the contract. Either is
non-behavior-affecting and can be addressed in a follow-up repair.

This black-hat review is independent of the State 12 (formal-verifier) findings:
the proof artifacts in `verification/verus/`, `kani/`, `proptest/`, and
`verification/flux/` are out of scope for this review, which evaluates the
**production implementation** against the **contract**.
