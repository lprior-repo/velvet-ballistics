# Hazard Analysis — vb-vzo9b

> **Scope.** Hazards introduced by — or surfaced by — the post-fix
> `fuzz_recovery_decode` body. The fuzz body is a test, not a production
> surface; hazards are categorised by what the post-fix change can break,
> weaken, or leave unaddressed.

## H-1 — Disjunction hides single-run divergence *(severity: high, status: fixed by this bead)*

| Aspect | Detail |
|---|---|
| Where | `fuzz/src/journal_target/readback.rs:196` (pre-fix). |
| Hazard | `assert!(summary.run == run || summary.run == RunId::new(0))`. |
| Triggering input | Fuzz payload where `data[0] == 0x00`. Pre-fix: trivially passes. |
| Detection | Production-side divergence in any of the 10 non-`run` fields, or in `run` itself when `data[0]` happens to coincide with the production sentinel `RunId::new(0)` by accident (e.g. after future refactor renames). |
| Severity | High — masks real bugs in `summarize_recovery_events` fuzz coverage. |
| Fix | Replace with `assert_eq!(summary, expected)` over all 11 fields. |
| Residual | None. Post-fix body has no disjunctive shortcut. |

## H-2 — Sentinel collision: `RunId::new(0)` *(severity: informational, status: documented)*

| Aspect | Detail |
|---|---|
| Where | Production code at `apply.rs:90` (`RecoveryError::NoRecoveryData { run: RunId::new(0) }`). |
| Hazard | `RunId::new(0)` is overloaded — it is both a sentinel ("no events" → `Err`) and a legitimate `u64` payload. |
| Triggering input | A real-world fuzz corpus that produces a `data` slice whose first byte is `0x00`. Pre-fix: disjunction masks it. Post-fix: `expected.run = RunId::new(0)` (the locally constructed run), so the assertion still passes when production returns the same value, and **fails** if production ever returns a different `run`. |
| Severity | Informational — the post-fix `expected` literal pins `run` to the fuzz-derived value, which is correct. |
| Fix | None required — the post-fix body handles this correctly. |
| Residual | A future refactor that decouples production sentinel from `RunId::new(0)` could surface as a diff in fuzz output; this is the desired behavior. |

## H-3 — Single-`RunAccepted` event under-tests `last_seq > first_seq` *(severity: medium, status: documented, not in-scope)*

| Aspect | Detail |
|---|---|
| Where | `fuzz/src/journal_target/readback.rs:187-191`. |
| Hazard | Fuzz body always emits exactly one event. Therefore `first_seq == last_seq == EventSeq::new(1)` in `expected`. Any divergence bug in the `last_seq` update path (`apply.rs:124`) is **not** caught because no second event is ever sent. |
| Severity | Medium — this is a fuzz-coverage gap. |
| Fix | Out of scope for this bead. A follow-on (e.g. `vb-82snf` epic) should extend the fuzz driver with a multi-event payload. |
| Residual | Acknowledged in `STATE.md`, `codebase-map.md §8`, and `delivery-scope.jsonl`. The contract documents that `expected.last_seq == seq` is the only stable invariant for the current payload shape. |

## H-4 — Coverage-only vs behavior-checking fuzz target *(severity: informational, status: closed)*

| Aspect | Detail |
|---|---|
| Where | `fuzz/src/journal_target/readback.rs:183-204`. |
| Hazard | Pre-fix body is **coverage-only**: it does not assert any non-`run` field of the summary. |
| Triggering input | Any non-empty-events payload. |
| Severity | Informational — but the audit that produced vb-vzo9b explicitly classified it as a P1 bug because of the disjunction (H-1). |
| Fix | Post-fix body asserts all 11 fields, becoming behavior-checking. |
| Residual | None for this bead. Downstream test-planner may add a deterministic `#[test]` wrapper to lock the assertions against future regressions. |

## H-5 — `assert_typed_recovery_error` catch-all silently absorbs new variants *(severity: low, status: documented)*

| Aspect | Detail |
|---|---|
| Where | `fuzz/src/journal_target/errors.rs:70` (`_ => {}`). |
| Hazard | A future variant added to `RecoveryError` would route through the catch-all without panic. |
| Triggering input | A future `RecoveryError::NewVariant { .. }`. |
| Severity | Low — the catch-all is intentional project convention. |
| Fix | None. Documented for future maintainers. |
| Residual | If a new variant is intended to be tested, the enumerator MUST be updated; otherwise fuzz coverage of the new path is silent. |

## H-6 — WASM/Nightly-feature regression *(severity: low, status: N/A)*

| Aspect | Detail |
|---|---|
| Where | `fuzz/src/journal_target/readback.rs` (file itself, not specific lines). |
| Hazard | Pre-fix body uses `data.len().is_multiple_of(2)` (stable since Rust 1.84) and `data.first().copied().unwrap_or(0)` (stable since Rust 1.0). No `unsafe`, no nightly features. |
| Severity | Low — purely stable. |
| Fix | None. |
| Residual | None. |

## H-7 — Concurrent / cancellation / scheduling hazards *(severity: N/A, status: out-of-scope)*

| Aspect | Detail |
|---|---|
| Where | The fuzz body, the production decoders, and the `RecoveryRuntimeSummary` type. |
| Hazard | None — no `async`, no `Send`/`Sync`, no atomics, no channels, no timers, no locks. |
| Severity | N/A. |
| Fix | N/A. |
| Residual | Loom/proptest-concurrency profile is intentionally absent from `proof-seeds.jsonl`. |

## H-8 — Production invariant drift *(severity: low, status: documented)*

| Aspect | Detail |
|---|---|
| Where | `crates/vb_storage/src/recovery/replay/summary/apply.rs:88-129`. |
| Hazard | If the production decoder changes its field-derivation rules, the post-fix `expected` literal will diverge. |
| Severity | Low — desirable. |
| Fix | None — divergence is the *purpose* of the assertion. |
| Residual | Future maintainers updating production must update `expected` in lockstep. This is documented in `type-contracts.md TC-2` and `domain-model.md` and enforced by `assert_eq!` itself. |

## H-9 — Spec-vs-code drift in `RecoveryRuntimeSummary` *(severity: medium, status: documented)*

| Aspect | Detail |
|---|---|
| Where | `crates/vb_storage/src/recovery/types.rs:547-570`. |
| Hazard | If a 12th field is added to the struct, the post-fix `expected` literal (which constructs a struct literal with 11 named fields) will fail to compile. |
| Severity | Medium — compile error is loud. |
| Fix | On the next struct addition, update `expected` and `TC-2` simultaneously. |
| Residual | Compile-time enforcement is desirable here. |

## Hazard Roll-Up

| ID | Severity | Status | In-Scope? |
|---|---|---|---|
| H-1 | high | fixed by this bead | ✅ |
| H-2 | informational | documented | ✅ |
| H-3 | medium | documented (out-of-scope) | ❌ (vb-82snf follow-on) |
| H-4 | informational | closed by this bead | ✅ |
| H-5 | low | documented | ✅ |
| H-6 | low | N/A | ✅ |
| H-7 | N/A | out-of-scope | ❌ |
| H-8 | low | documented | ✅ |
| H-9 | medium | documented | ✅ |

## Forbidden-State Coverage

Every forbidden state in `domain-model.md §Forbidden` is the contrapositive of
one of the post-fix `expected` field pins. Therefore the post-fix body covers
all listed forbidden states via the single `assert_eq!` line, and the
hazard analysis above quantifies each forbidden-state residual.
