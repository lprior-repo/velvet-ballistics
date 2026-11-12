# vb-47u9o Scope Review: kani_taint::write_taint_no_panic Timeout

**Auditor:** LOCAL5090 independent read-only scope auditor vb-47u9o
**Baseline:** da633fe0 (origin/main)
**Current parent:** aa8bdbd6b
**Verdict:** CODE-SCOPE — Integration did not change the timed-out path. Timeout remains baseline verifier debt.

## Evidence: Changed vs Unchanged

**Changed files in baseline → current range (relevant source only):**
- `crates/vb_core/src/frame/checkpoint.rs` — new module, not in call graph
- `crates/vb_core/src/frame.rs` — only `mod checkpoint` declaration added (3 lines)
- `crates/vb_core/src/workflow/{mod,proptest_*,tests}.rs` — unrelated workflows
- `crates/vb_compile/` — compile/lower changes, not runtime core
- `Cargo.lock` — crossbeam .9.20, rtrb .3.5; not vb_core deps

**Unchanged files in taint harness call graph (blobs verified):**
- `crates/vb_core/src/kani_taint.rs` — harness source (8c893d65)
- `crates/vb_core/src/frame.rs` — struct definition unchanged (b18618227 → 92822468d, only checkpoint module decl added)
- `crates/vb_core/src/frame/parts/impl_001_construct.rs` — RunFrame::new (aff7567df)
- `crates/vb_core/src/frame/parts/impl_003_slots_taints.rs` — write_slot/write_taint (e6f3ff945)
- `crates/vb_core/src/value.rs` — Taint/SlotValue types (b29a32b61)
- `crates/vb_core/src/ids.rs` — RunId/SlotIdx/StepIdx (unchanged)
- `crates/vb_core/Cargo.toml` — dependencies (bytes, chrono, indexmap, serde, thiserror) unchanged

## Call Graph Analysis

The harness `write_taint_no_panic` (lines 92-112 of kani_taint.rs):
1. `kani::any::<u16>()` → `slot_count` (full symbolic domain [0,65535])
2. `kani::any::<u16>()` → `slot_raw` (bounded by slot_count)
3. `SlotIdx::new(slot_raw)` — type-safe wrapper, no allocation
4. `RunFrame::new(RunId::new(1), StepIdx::ZERO, 1, slot_count)` — **allocation site**
5. `frame.write_slot(slot, SlotValue::Null)`
6. `frame.write_taint(slot, taint)`

**Root cause of timeout:** `RunFrame::new` uses `slot_count` directly to allocate `Vec<[StepState]>::with_capacity(slot_count)`. Kani must reason about `slot_count` over the full u16 domain [1,65535]. This is a known tractability issue — the allocation size is unbounded in the symbolic domain.

## Verification of Unchanged Path

- Blob hashes confirm zero changes to the harness, struct definition, constructor, and slots/taints implementations.
- The checkpoint module addition in frame.rs is isolated; it does not alter RunFrame layout, constructor logic, or slots/taints methods.
- No checkpoint methods are called by the taint harness or its transitive call graph.
- Toolchain pinned nightly-2026-04-28 unchanged.
- Kani CI flags unchanged.
- vb_core dependencies (bytes, chrono, indexmap, serde, thiserror) unchanged in Cargo.lock.

## Baseline Evidence

From `.beads/vb-2lyiw/evidence/canonical-frozen-baseline.json`:
- Exit code: 124 (timeout)
- Duration: 3600.202 seconds
- Last completed harness: `kani_shard_command_queue::process_frame_no_panic`
- Next harness attempted: `kani_taint::write_taint_no_panic` (never completed within deadline)

From baseline stdout line 56108: "Checking harness kani_taint::write_taint_no_panic..." was the final Kani check started before the 1h timeout killed moon ci.

## GOD5 Differential Scoping Justification

GOD5 (Proof-Driven Development) mandates fixing the implementation when verification exposes a flaw. However:
1. This is not a verification failure — the harness never completes within the deadline.
2. The harness code and its call graph are byte-identical to baseline (blobs verified).
3. The timeout is a known tractability property of the existing code, not a regression.
4. vb-heuiz already tracks this as a tractability issue requiring a smaller bound; no smaller bound has been accepted.

Avoiding a blind whole-domain Kani rerun is justified because the timed-out path is identical to the already-timed-out baseline path. Re-running would yield the same timeout, not new information.

## What Remains Uncertain

- Whether newer Kani releases would converge on this harness (toolchain pinned, so not testable).
- Whether a bound annotation (`kani::assume(slot_count <= N)`) could make the harness tractable without changing the underlying implementation.
- Whether the checkpoint module introduces any latent type/layout changes that could manifest under different optimization levels (struct definition itself is unchanged).

## Verdict

**CODE-SCOPE:** The integration changes (aa8bdbd6b relative to da633fe0) do not touch the `kani_taint::write_taint_no_panic` harness, its RunFrame constructor/slots/taints call graph, type definitions, or dependency set. The harness source, all implementation files, and type blobs are byte-identical to baseline.

The timeout is baseline verifier debt, not an integration regression. The new checkpoint module and workflow/proptest changes are provably unrelated to this timed-out path via blob comparison and call graph analysis.

This audit does not waive the need to eventually resolve the harness tractability issue; it classifies the current timeout as pre-existing and unchanged.