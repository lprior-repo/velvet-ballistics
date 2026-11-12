# Code Review — vb-ynp2k (independent review of vb-hrvix input-slot mirror repair)

- **Delta reviewed:** `8a7e2340f..27090007f600` (three local3090 checkpoints a40a299a2 / dc2e55d57 / 27090007f).
- **Reviewer role:** fresh independent DeepSeek, read-only on source; evidence/reviews written under `.beads/vb-ynp2k/` only.
- **Source files in delta:** `crates/vb_core/src/workflow/tests.rs`, `verification/verus/extern_try_from_parts.rs`, `verification/verus/try_from_parts.rs`, `verification/verus/vb_xi2f_compile_source.rs`, `verification/verus/production_inner/try_from_parts_production.rs`, `verification/verus/production_inner/vb_xi2f_compile_source_production.rs`. No production `crates/vb_core/src/workflow/mod.rs` or `crates/vb_compile/src/mod_compile_lowering/part_01.rs` changes in this delta.

## Verdict

**STATUS: APPROVED** — no blocker. Two LOW findings (F1, F2) and one OBSERVATION (F3). All must be dispositioned by the author before landing, but none blocks acceptance of the delta's behavior.

The delta is a faithful mirror/spec repair plus behavior tests. Production behavior was already correct; the bug was exclusively in the mirrors/spec (always-Ok `validate_input_slots` stub, never-updated `prev_slot`, digest conflation). The delta fixes all three, does not weaken any assertion, introduces no new `assume_specification`, and introduces no new vacuous `ensures true` lemma.

---

## Verified correct (independent re-run, immutable runner)

All commands run through `/tmp/vb-run-gate.py` with `CARGO_BUILD_JOBS=2`; raw logs at `.beads/vb-ynp2k/evidence/`.

| Check | Command | Result |
|---|---|---|
| Input-slot tests | `cargo test -p vb_core --lib input_slot -- --test-threads=1` | **7 passed, 0 failed** (6 new constructor + 1 pre-existing `engine::validate::…::rejects_do_with_out_of_bounds_input_slot`), 2176 filtered out, exit 0 |
| Main spec | `verus --crate-type=lib verification/verus/try_from_parts.rs` | **67 verified, 0 errors**, exit 0 |
| Compile-source spec | `verus --crate-type=lib verification/verus/vb_xi2f_compile_source.rs` | **15 verified, 0 errors**, exit 0 |
| Mirror drift gate | `bash scripts/check-production-inner-drift.sh` | PASS — 0 drift findings (64 mirrors, 78 extern files), exit 0 |
| Production-binding gate | `bash scripts/check-verus-production-binding.sh` | exit 0 — **STRONG 0, WEAK 72, VACUUM 0** |

### Faithfulness checks (line-by-line against production)

1. **Mirror `validate_input_slots`** (`production_inner/try_from_parts_production.rs:537-563`) is a faithful body of production `workflow/mod.rs:876-906`. Same precedence `bounds (>= slot_count) → duplicate (== prev) → order (< prev)`; same error-variant field construction; same `prev_slot = Some(current)` at loop tail. Only benign substitutions: `.as_ref()` → `.iter()`, and an explicit `let current: SlotIdx` type annotation. `SlotIdx::get()` added at the mirror (lines ~90-93) matches production `ids/parts/chunk_001_custom_types.rs` `get()` semantics (raw `u16`).
2. **`try_from_parts` order** — mirror `CompiledWorkflow::try_from_parts` (`try_from_parts_production.rs:445-447`) calls `validate_parts` → `validate_budget` → `validate_input_slots`, exactly matching production `workflow/mod.rs:38-40`. Input-slot validation is correctly **third**, after `validate_parts` + `validate_budget`.
3. **Extern projection `prev_slot` fix** (`extern_try_from_parts.rs:809-826`) — `let mut prev_slot: Option<u32> = None` + `prev_slot = Some(*current)` at loop tail. The duplicate/order arms now actually fire. Correct.
4. **Spec mirror** (`try_from_parts.rs:555-618`) — `try_from_parts_pure_spec` now routes to `input_slots_valid`, which is `check_input_slots(parts, 0, None)` (recursive, `decreases parts.input_slots.len() - index`, `prev: Option<int>` carried, `current as int` cast). Precedence `current >= slot_count → InputSlotOutOfBounds`, `prev == current → DuplicateInputSlot`, `current < prev → NonCanonicalInputSlotOrder` matches production exactly.
5. **Error classification** (`try_from_parts.rs:115-127`, `:405-422`) — the three new variants are added to `spec_workflow_error_is_bound_violation_spec` and `lemma_bound_violation_in_closed_set`. Not added to the structural set. Internally consistent; Verus discharges by closed enumeration.
6. **Digest separation** (`production_inner/vb_xi2f_compile_source_production.rs:720-727`, caller `vb_xi2f_compile_source.rs:285-294`) — mirror now takes `semantic_source_identity: WorkflowDigest` and `digest: WorkflowDigest` as **two distinct parameters**, mapping to `WorkflowParts.semantic_source_identity = Some(semantic_source_identity)` and `WorkflowParts.digest = digest`. This matches production `part_01.rs`: `semantic_source_identity = canonical_digest(source)` (line 24) vs `digest = blake3(encoded_parts)` (line 76). The conflation is gone.
7. **`lemma_well_formed_parts_accepted`** (`try_from_parts.rs:702-740`) adds `input_slots_valid(parts) == None::<SpecWorkflowError>` to `requires` and a matching `else if` branch. This is a **strengthening** of the precondition (narrower applicability), not a weakening: required because the spec mirror now rejects invalid input slots, so the forward direction must assume them valid. Correct.

---

## Findings

### F1 (LOW) — `validate_input_slots` faithful body is not drift-annotated; surrounding comments stale

- **Source:** `verification/verus/production_inner/try_from_parts_production.rs:519` (section header `// VERBATIM PRODUCTION: stub validation helpers`) and `:438-439` (`/// … The mirror provides stub bodies for these helpers (return Ok) …`).
- **Problem:** The delta replaced the `Ok(())` stub with a faithful body, but (a) did not add a `// Production crates/vb_core/src/workflow/mod.rs:876-906` per-section drift annotation for `validate_input_slots`, and (b) left two comments asserting the helpers are `Ok(())` stubs — now false. The drift gate therefore does **not** enforce faithfulness of this body (it checks only annotated sections via identifier diff, and the original always-Ok stub also passed because the gate is identifier-level, not body-level). The misleading comments invite a future agent to re-stub it.
- **Impact:** No current behavior defect — the body is faithful today. This is an enforcement/maintainability gap plus false documentation.
- **Author fix:** add the `// Production crates/vb_core/src/workflow/mod.rs:876-906` annotation immediately above `validate_input_slots`, and rewrite lines 519-521 and 437-439 to state that `validate_parts`/`validate_budget` are stubs while `validate_input_slots` is a verbatim body.

### F2 (LOW) — stale variant-count comment

- **Source:** `verification/verus/try_from_parts.rs:417` — `// exactly enumerates these 8 variants.` now enumerates **11**.
- **Author fix:** update the count (or drop the numeric claim).

### F3 (OBSERVATION) — widened numeric model (no action required)

- `SpecWorkflowParts.input_slots: Vec<u32>` and `slot_count: u32` widen production `u16` slot indices/count. Faithful for the bounded domain (indices 0..=65535; `slot_count` additionally capped ≤1024 by `max_slots`), so no information loss. Noted only so the model's widening is on record; no change required.

---

## Trust boundary (explicit)

This delta **does not** constitute production verification. It is a mirror/spec repair. The honest statement of proof scope:

1. **The 67 obligations in `try_from_parts.rs` and 15 in `vb_xi2f_compile_source.rs` are spec-level proofs**, not whole-production verification. They reason over `SpecWorkflowParts`/`SpecCompileInput` projections and spec predicates, and discharge through `#[verifier::external]` bodies that are **opaque to Verus** (`extern_try_from_parts.rs:113`, `:574-575`, `:688-689`; `vb_xi2f_compile_source.rs` projection).
2. **`assume_specification` bridges** (`try_from_parts.rs:188`, `:202`) attach spec contracts to those external bodies. These are **pre-existing**; the delta adds no new `assume_specification`.
3. **Binding is WEAK** (production_inner mirrors), confirmed by the gate: STRONG 0 / WEAK 72 / VACUUM 0. The mirrors are hand-maintained verbatim copies gated by an identifier-level drift heuristic, not by Verus.
4. **Vacuous `ensures true` lemmas** (`try_from_parts.rs:451`, `:477`, `:498`, `:634`, `:649`, `:687`) are **pre-existing debt**, unchanged by this delta; the delta adds none. `lemma_slot_out_of_bounds_rejected` (`:623-635`) is the canonical example — its `ensures true` is discharged by the projection `assume_specification`, not by a real proof.
5. **What this delta actually gains (real, and correctly scoped):** the spec mirror (`try_from_parts_pure_spec` + `check_input_slots`) now *models* the input-slot decision, and the external projection body now *implements* the same decision — the earlier divergence (projection returning `Ok` for invalid slots while the spec knew nothing of slots) is closed. The two layers agree, and that agreement is verified for the spec layer. The projection body's own correctness remains a **trusted base** (drift-gate heuristic only; see F1).

### Digest honesty

The caller supplies `WorkflowDigest([1u8; 32])` / `WorkflowDigest([2u8; 32])` (`vb_xi2f_compile_source.rs:285-286`). This is **honest**: the spec postcondition `spec_compiled_workflow_validated` (`:354-375`) checks only four invariants (nodes non-empty, entry in bounds, slot_count ≤ u16, symbols_count ≤ u32) and does **not** assert `digest == blake3(encoded_parts)` or `semantic_source_identity == canonical_digest(source)`. No cryptographic/input-generic digest property is claimed; the mirror comment (`production_inner/vb_xi2f_compile_source_production.rs:769-774`) states "No opaque crypto step is modeled; the caller supplies both." **No new math/assumption is smuggled in.**

---

## Resolution of prior review findings (vb-fg1pi, "56db")

| Prior finding | Status in this delta | Evidence |
|---|---|---|
| always-Ok `validate_input_slots` stub | **Fixed** — faithful body | `try_from_parts_production.rs:537-563` vs `workflow/mod.rs:876-906` |
| immutable `prev_slot` never updated (duplicate/order never fired) | **Fixed** — `let mut` + loop-tail update | `extern_try_from_parts.rs:813,826` |
| semantic/artifact digest conflation | **Fixed** — two distinct params | `vb_xi2f_compile_source_production.rs:722-727` |
| extern 0-verified vs actual main spec | **Fixed** — actual main `try_from_parts.rs` runs 67 verified | my re-run: 67 verified / 0 errors; author final evidence also 67 |
| missing third validation step / error variants | **Fixed** — `validate_input_slots` third in `try_from_parts` + 3 `SpecWorkflowError` variants | `try_from_parts_production.rs:445-447`; `extern_try_from_parts.rs:167-174` |

---

## Out of scope (tracked separately, not this delta)

Root previously identified a pre-existing bridge gap: `WorkflowDigest` external type missing in `idempotency_replay_tracker`. That is a separate bead and is **not** part of this delta's source-repair scope; nothing here touches it.
