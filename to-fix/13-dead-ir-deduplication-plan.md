# Dead IR-Type Deduplication — Implementation Plan

Repo: `/home/lewis/src/velvet-ballistics`
Audit source: Round 4 dead-IR-type confirmation (8 dead artifacts, 2,097 LOC)
Total work envelope: **4.5–7.5h** across 6 atomic beads
Definition of done: see end of document.

---

## Hazard Summary

Eight dead artifacts in `crates/vb_core/src/` exist as parallel IR-type
universes that are NOT wired into `lib.rs` and are NOT referenced by any
production code. A future agent editing the dead files per the master
contract (which mis-cites `CompiledNode` as 4-field and `ResourceContract`
as 16-field) would produce zero effect on production with no compiler
signal. The `kani_resource_contract_validation_18_fields.rs` harness
imports a non-existent `vb_core::validation::resource` path and is
therefore also dead. `.beads/vb-o5zb.5/closure-reconciliation-packet.md:69`
already acknowledges `compiled_workflow.rs:130-185` as residual dead
code under master rule line 494 ("code wins; doc drift is non-behavior
hygiene"). This plan executes that hygiene.

The canonical home is `crates/vb_core/src/workflow/` (re-exported through
`lib.rs:124-127` as `vb_core::CompiledNode`, `vb_core::CompiledNodeKind`,
`vb_core::CompiledWorkflow`, `vb_core::ExprOp`, `vb_core::ExprProgram`,
`vb_core::AccessorProgram`, `vb_core::PathSegment`, `vb_core::WorkflowParts`,
`vb_core::ResourceContract`, `vb_core::WorkflowError`, `vb_core::ExprBranch`,
`vb_core::SlotBranch`, `vb_core::check_expr_stack_bound`).

---

## Index of Dead Artifacts

| # | File:line | LOC | Why dead | Action | New Bead |
|---|-----------|-----|----------|--------|----------|
| D-1 | `crates/vb_core/src/nodes.rs:1-190` | 190 | 4-field `CompiledNode` (no `on_error`, `error_slot`); not declared in `lib.rs`; only ref'd by dead `validation.rs:22` and dead `compiled_workflow.rs:15,76,213` | DELETE | `vb-dedup.1` |
| D-2 | `crates/vb_core/src/expressions.rs:1-179` | 179 | Byte-identical duplicate of canonical `workflow/types.rs:413-518`; not declared in `lib.rs`; only ref'd by dead `validation/resource.rs:15` and dead `compiled_workflow.rs:16,82,215` | DELETE | `vb-dedup.2` |
| D-3 | `crates/vb_core/src/accessors.rs:1-24` | 24 | Duplicate of canonical `workflow/types.rs:277-294`; not declared in `lib.rs`; only ref'd by dead `validation/resource.rs:13` and dead `compiled_workflow.rs:17,88,217` | DELETE | `vb-dedup.3` |
| D-4 | `crates/vb_core/src/validation.rs:1-184` | 184 | Dead parent module of dead `validation/` directory; not declared in `lib.rs`; imports dead `crate::nodes::CompiledNode` (line 22) | DELETE | `vb-dedup.4a` |
| D-5 | `crates/vb_core/src/validation/{graph,nodes,resource,targets}.rs` | 897 | Parallel universe that imports dead `crate::nodes`/`crate::accessors`/`crate::expressions`; not declared in `lib.rs` | DELETE (entire directory) | `vb-dedup.4b` |
| D-6 | `crates/vb_core/src/compiled_workflow.rs:1-228` | 228 | 16-field `ResourceContract` duplicate (canonical is 18-field at `workflow/types.rs:167-206`); not declared in `lib.rs`; same-purpose code exists at `workflow/types.rs:13-275` | DELETE | `vb-dedup.5` |
| D-7 | `crates/vb_core/src/compiled_workflow.rs.removed:1-228` | 228 | Tombstone with `max_steps: 10_000` and `max_constants: u16::MAX`; pre-canonical DEFAULT; not declared in `lib.rs`; the `.removed` suffix is documentary only | DELETE | `vb-dedup.6` |
| D-8 | `crates/vb_core/src/kani_resource_contract_validation_18_fields.rs:1-167` | 167 | Stale Kani harness; NOT wired in `lib.rs`; imports `vb_core::validation::resource::validate_resource_contract` (path does not exist because `validation` is not declared); closed-bead evidence for `vb-xi2f.35` | DELETE | `vb-dedup.7` |
| **Total** | | **2,097** | | | |

**Verification of dead-ness** (re-confirmed 2026-06-07):

- `rg "use crate::nodes\b|use crate::expressions\b|use crate::accessors\b|use crate::compiled_workflow\b|use crate::validation\b" crates/` returns only the dead files themselves (`validation.rs`, `validation/{graph,nodes,resource,targets}.rs`, `compiled_workflow.rs`, `kani_resource_contract_validation_18_fields.rs`).
- `rg "super::nodes\b|super::expressions\b|super::accessors\b|super::compiled_workflow\b" crates/` returns only the dead `compiled_workflow.rs` and `compiled_workflow.rs.removed` (plus an unrelated `engine/expr_eval/accessors.rs`).
- `rg "vb_core::nodes\b|vb_core::expressions\b|vb_core::accessors\b|vb_core::compiled_workflow\b|vb_core::validation\b" --exclude-dir=.beads --exclude-dir=.git --exclude-dir=target` returns only `kani_resource_contract_validation_18_fields.rs`, `proof-to-rust-map.md`, `rust-refinement-obligations.jsonl` (two of these are documentation that will be updated by this plan; the third is the dead harness itself).
- `cargo check -p vb_core` exits 0 in 1.05s with all 8 dead artifacts present (they are not in the build graph because `lib.rs` does not declare them).

---

## Per-File Action Plan

### D-1: `crates/vb_core/src/nodes.rs` — DELETE

**Defect.** File contains a 4-field `CompiledNode { id, output, next, kind }`
that is missing the `on_error: Option<StepIdx>` and `error_slot: Option<SlotIdx>`
fields added in the canonical `workflow/types.rs:520-538`. The
`CompiledNodeKind` enum (lines 24-189) is a near-duplicate of canonical
lines 544-710 (one structural difference: dead version uses
`super::workflow::ExprBranch` and `super::workflow::SlotBranch`, canonical
uses local `ExprBranch`/`SlotBranch` after `use` import — same types,
just different path syntax). The file is not declared in `lib.rs` and
is referenced ONLY by the dead `validation.rs:22` and the dead
`compiled_workflow.rs:15,76,213`.

**Master contract mis-citation:**
- Line 572: `CompiledNode | Single IR node: id: StepIdx, output: Option<SlotIdx>, next: Option<StepIdx>, kind: CompiledNodeKind.` — describes 4 fields, but canonical is 6 fields (adds `on_error`, `error_slot`).

**Fix.**
1. `git rm crates/vb_core/src/nodes.rs`
2. Update master contract line 572 to read 6 fields: `id: StepIdx`, `output: Option<SlotIdx>`, `next: Option<StepIdx>`, `on_error: Option<StepIdx>`, `error_slot: Option<SlotIdx>`, `kind: CompiledNodeKind`.
3. Update master contract line 3443: replace `crates/vb_core/src/nodes.rs` with `crates/vb_core/src/workflow/types.rs:520-538` (canonical `CompiledNode` definition) and `crates/vb_core/src/engine/node_helpers.rs` (the `Finish` node handler).
4. Add a one-line commit-trailer citation in the master section heading for `workflow.rs` pointing to `crates/vb_core/src/workflow/types.rs:520-710` as the authoritative source.

**Acceptance criteria.**
- File does not exist in the working tree (`test ! -e crates/vb_core/src/nodes.rs`).
- `cargo check -p vb_core` still exits 0.
- Master contract lines 572 and 3443 reference 6 fields and the canonical path.
- `rg "crates/vb_core/src/nodes\.rs" velvet-ballistics-MASTER.md` returns 0 matches.

**Risk.** Low. No production code path uses the file (verified by ripgrep). The only risk is a stale doc/script somewhere; a final `rg` over the whole repo excluding `.beads/` and `.git/` and `target/` must return 0 hits before closing the bead.

**Hours.** 0.5h (delete, master update, verification).

**Bead.** `vb-dedup.1` (new, P1).

---

### D-2: `crates/vb_core/src/expressions.rs` — DELETE

**Defect.** File is byte-identical to canonical `workflow/types.rs:413-518`
for the shared types (`ExprProgram`, `ExprOp`, `check_expr_stack_bound`,
private stack-effect helpers). Not declared in `lib.rs`; only ref'd by
the dead `validation/resource.rs:15` and the dead `compiled_workflow.rs:16,82,215`.

**Master contract mis-citation.** None directly. Master line 574-575
describes `ExprProgram` and `ExprOp` correctly against the canonical
location, but a future agent searching for `expressions.rs` could be
misled into editing the dead file.

**Fix.**
1. `git rm crates/vb_core/src/expressions.rs`
2. Add a comment to `workflow/types.rs:413-518` header noting this is the canonical home and that `expressions.rs` (formerly a duplicate) was deleted (bead `vb-dedup.2`).
3. Pre-commit / CI check: forbid re-introduction of `crates/vb_core/src/expressions.rs` at the same path (see "Definition of done" below).

**Acceptance criteria.**
- File does not exist.
- `cargo check -p vb_core` still exits 0.
- `rg "expr_stack_effect|check_expr_stack_bound" crates/vb_core/src/workflow/types.rs` shows the canonical helpers are still present.
- CI check `scripts/check-no-dead-ir-duplicates.sh` exits 1 if any of the 7 deleted files re-appear.

**Risk.** Low. Byte-identical duplicate of canonical; deletion is lossless.

**Hours.** 0.25h (delete, header note, CI check).

**Bead.** `vb-dedup.2` (new, P1).

---

### D-3: `crates/vb_core/src/accessors.rs` — DELETE

**Defect.** File is a duplicate of canonical `workflow/types.rs:277-294`
(`AccessorProgram` struct and `PathSegment` enum). Not declared in
`lib.rs`; only ref'd by the dead `validation/resource.rs:13` and the
dead `compiled_workflow.rs:17,88,217`.

**Master contract mis-citation.** None directly. Master line 576
describes `AccessorProgram` correctly. The hazard is that a search for
`accessors.rs` would surface the dead file before the canonical
`workflow::AccessorProgram` definition.

**Fix.**
1. `git rm crates/vb_core/src/accessors.rs`
2. Note in master contract line 576 that the canonical location is `crates/vb_core/src/workflow/types.rs:277-294`, re-exported as `vb_core::AccessorProgram` from `lib.rs:125`.

**Acceptance criteria.**
- File does not exist.
- `cargo check -p vb_core` still exits 0.
- Master line 576 explicitly names the canonical path.
- CI check exits 1 on re-introduction.

**Risk.** Low. Trivial duplicate; deletion is lossless.

**Hours.** 0.15h (delete, master annotation, CI check update).

**Bead.** `vb-dedup.3` (new, P1).

---

### D-4: `crates/vb_core/src/validation.rs` + `validation/` directory — DELETE entire tree

**Defect.** 1,081 lines (184 + 897) of dead code forming a parallel
validation universe. The parent `validation.rs` declares
`pub mod graph; pub mod nodes; pub mod resource; pub(crate) mod targets;`
and imports `crate::nodes::CompiledNode` (line 22) and
`crate::workflow::WorkflowParts` (line 23). The submodule files at
`validation/{graph,nodes,resource,targets}.rs` import the dead
`crate::nodes::CompiledNodeKind`, `crate::accessors::AccessorProgram`,
`crate::expressions::ExprProgram`, and `crate::validation::WorkflowError`.
None of these types resolve to production code because `nodes`,
`accessors`, `expressions`, and `validation` are NOT declared in `lib.rs`.

**Master contract mis-citation.** None directly. The hazard is
structural: a future agent that finds `validation.rs` and sees a
`pub mod graph;` might believe it is the canonical validation module.
The actual canonical validation lives at `workflow/validation.rs:1-1046`
(re-exported as `vb_core::workflow::validation`).

**Fix.**
1. `git rm -r crates/vb_core/src/validation.rs crates/vb_core/src/validation/`
2. Add a doc comment to `workflow/validation.rs:1` header pointing readers away from the deleted `validation.rs` (bead `vb-dedup.4`).
3. Confirm `kani_resource_contract_validation_18_fields.rs` is also deleted (D-8) since it imports `vb_core::validation::resource::validate_resource_contract` which depended on this dead tree.

**Acceptance criteria.**
- File `validation.rs` and directory `validation/` do not exist.
- `cargo check -p vb_core` still exits 0.
- `rg "use crate::validation\b|vb_core::validation\b" crates/ --exclude-dir=.beads` returns 0 matches (proof the dead tree is fully excised from code).
- Canonical `workflow/validation.rs:1-1046` is unchanged and remains the only validation source of truth.
- CI check exits 1 on re-introduction of `crates/vb_core/src/validation.rs` or `crates/vb_core/src/validation/`.

**Risk.** Medium. Larger deletion than D-1..D-3, but the parallel universe is structurally isolated (no production ref) and the canonical `workflow/validation.rs` is fully self-contained. Mitigation: before deletion, `cargo check -p vb_core` baseline; after deletion, `cargo check -p vb_core` must still exit 0; `cargo test -p vb_core -- resource` must still pass 64 tests (this is the canonical validation surface).

**Hours.** 1h (delete, master note, cargo test rerun, CI check).

**Bead.** `vb-dedup.4` (new, P1) covering both `vb-dedup.4a` (parent file) and `vb-dedup.4b` (directory).

---

### D-5: `crates/vb_core/src/compiled_workflow.rs` — DELETE

**Defect.** 228-line duplicate of canonical `workflow/types.rs:13-275`.
The dead `ResourceContract` struct (lines 130-163) has **16 fields** —
matching the master contract's stale 16-field description at line 578,
but missing the canonical 18 fields (`max_transitions_per_tick` and
`allows_secret_results` added in `workflow/types.rs:185,205`). The dead
`DEFAULT` (line 167-184) uses `max_steps: 1_000` (matches canonical
line 211), but other defaults diverge from canonical
`workflow/types.rs:210-230`. The `try_from_parts` method (line 27) and
all the `CompiledWorkflow` accessors are duplicates of canonical
implementations in `workflow/types.rs:29-165`. The file imports
`super::nodes::CompiledNode`, `super::expressions::ExprProgram`,
`super::accessors::AccessorProgram`, and `super::validation::WorkflowError`
— none of which resolve because none of those modules are declared in
`lib.rs`. Therefore this file is already broken: it would not compile
if `lib.rs` were ever updated to declare `pub mod compiled_workflow;`.

**Master contract mis-citation:**
- Line 578: `ResourceContract | 16 fields controlling hard limits (Section 13).` — canonical is 18 fields (adds `max_transitions_per_tick: u64` and `allows_secret_results: bool`).
- The file is also cited indirectly in `proof-to-rust-map.md:50` and `rust-refinement-obligations.jsonl:56-57` as `vb_core::compiled_workflow::try_from_parts`; the canonical E2E path is `vb_core::workflow::CompiledWorkflow::try_from_parts` (or the re-exported `vb_core::CompiledWorkflow::try_from_parts` from `lib.rs:125`).

**Fix.**
1. `git rm crates/vb_core/src/compiled_workflow.rs`
2. Update master contract line 578: change "16 fields" to "18 fields" and name the canonical location `crates/vb_core/src/workflow/types.rs:167-206`.
3. Update `proof-to-rust-map.md:50` to reference the canonical path:
   - From: `vb_core::compiled_workflow::try_from_parts`
   - To: `vb_core::workflow::CompiledWorkflow::try_from_parts` (or `vb_core::CompiledWorkflow::try_from_parts`)
4. Update `rust-refinement-obligations.jsonl:56-57` to reference the canonical path in `rust_target` and `source_refs` fields for `RRO-vb-xi2f24-004` and `RRO-vb-xi2f24-005`.
5. Update `.beads/vb-o5zb.5/closure-reconciliation-packet.md:69` to mark the dead-file cleanup as completed (the packet already acknowledges the file as dead-code hygiene).

**Acceptance criteria.**
- File does not exist.
- `cargo check -p vb_core` still exits 0.
- `cargo test --workspace` still passes (or, at minimum, `cargo test -p vb_core`).
- Master line 578 names 18 fields and the canonical path.
- `proof-to-rust-map.md:50` and `rust-refinement-obligations.jsonl:56-57` reference the canonical path.
- `rg "vb_core::compiled_workflow\b" --exclude-dir=.beads --exclude-dir=.git` returns 0 matches.
- `.beads/vb-o5zb.5/closure-reconciliation-packet.md:69` annotated with "DEDUP-CLOSED bead vb-dedup.5".

**Risk.** Medium. The file is the most heavily-cited dead artifact (master contract, proof-to-rust map, refinement obligations, audit packet). Mitigation: update all four citation sites in the same commit; do not delete before citations are updated; verify with a final `rg` over the whole repo.

**Hours.** 1h (delete, four citation updates, validation rerun).

**Bead.** `vb-dedup.5` (new, P1).

---

### D-6: `crates/vb_core/src/compiled_workflow.rs.removed` — DELETE

**Defect.** 228-line tombstone. Diff vs. live `compiled_workflow.rs:1-228`
shows two `ResourceContract::DEFAULT` field divergences:
- `max_steps: 10_000` (live) vs. live canonical `1_000` (the `.removed` is closer to the live, not to canonical)
- `max_constants: u16::MAX` (live) vs. live canonical `8_192`

Wait — re-read: the user's report says "compiled_workflow.rs.removed (NOT byte-identical, has different ResourceContract::DEFAULT)". I verified the diff above. The `.removed` file has different `DEFAULT` field values than the live (non-removed) `compiled_workflow.rs`. Both differ from the canonical `workflow/types.rs:210-230`. The `.removed` suffix is documentary only — git would normally show a deleted file as a deletion, not a `.removed` rename. The presence of this file suggests it was a manual "save before delete" operation, not git-tracked. It is not declared in `lib.rs` and is therefore dead.

**Master contract mis-citation.** None. The `.removed` suffix is an internal artifact, not cited in master.

**Fix.**
1. `git rm crates/vb_core/src/compiled_workflow.rs.removed`
2. Add a comment to `workflow/types.rs:208` (`ResourceContract::DEFAULT`) noting the canonical DEFAULT and that any `.removed`/`.bak` tombstone files were cleaned up (bead `vb-dedup.6`).

**Acceptance criteria.**
- File does not exist.
- `find crates/vb_core/src -name "*.removed" -o -name "*.bak" -o -name "*.orig"` returns 0 matches.
- `cargo check -p vb_core` still exits 0.

**Risk.** Low. The `.removed` file is not in the build graph; it has the same 16-field `ResourceContract` shape as the live dead `compiled_workflow.rs`. There is no semantic loss; the canonical `workflow/types.rs` carries the full truth.

**Hours.** 0.15h (delete, find check, comment).

**Bead.** `vb-dedup.6` (new, P1).

---

### D-7 (re-numbered D-8 in index): `crates/vb_core/src/kani_resource_contract_validation_18_fields.rs` — DELETE

**Defect.** 167-line Kani harness. The file is NOT declared in
`lib.rs` (I confirmed by grep — no `pub mod kani_resource_contract_validation_18_fields;`
anywhere). The file imports `vb_core::validation::resource::validate_resource_contract`
(line 23), but `vb_core::validation` does not exist as a public module
(it is not in `lib.rs`'s `pub mod` list). Therefore this file is
already broken: it would not compile if it were ever wired up, because
the `validation` module tree was deleted at some point without
removing this harness.

The harness was created as evidence for closed bead `vb-xi2f.35`
(P1: digest covers resource contract semantics, closed 2026-06-03).
The bead closure cited this file in `.beads/vb-o5zb.5/black-hat-review.md:44,181`
and `.beads/vb-o5zb.5/closure-reconciliation-packet.md:73,82,130`
as proof that the Kani harness correctly asserts `Err` for
`allows_secret_results: true`. Since the bead is already closed and
the harness cannot compile, the safe action is to delete and amend
the audit citations to point to the canonical
`workflow/types.rs:167-206` (which is the actual source of truth for
the 18-field count).

**Master contract mis-citation.** None directly.

**Fix.**
1. `git rm crates/vb_core/src/kani_resource_contract_validation_18_fields.rs`
2. Update `.beads/vb-o5zb.5/black-hat-review.md:44,181` to note that the harness was deleted as dead code (bead `vb-dedup.7`) and that the canonical evidence is `workflow/types.rs:167-206` (18 fields, `allows_secret_results: bool` field exists).
3. Update `.beads/vb-o5zb.5/closure-reconciliation-packet.md:73,82,130` similarly.
4. Add a one-line check: if a future bead needs Kani verification of `ResourceContract::DEFAULT.allow_secret_results` rejection, the harness must be rewritten against the canonical `vb_core::workflow::validation::validate_resource_contract` path (not the dead `vb_core::validation::resource` path).

**Acceptance criteria.**
- File does not exist.
- `cargo check -p vb_core` exits 0 (this was true before too, since the harness was not in the build graph).
- `cargo check -p vb_core --features kani-diagnostic-codes` exits 0 (the `vb_core::kani` module's `mod.rs` is feature-gated; verify deletion of the undeclared file does not break the feature-gated build).
- The 3 audit citations are updated to point to canonical `workflow/types.rs:167-206`.
- `rg "kani_resource_contract_validation_18_fields" --exclude-dir=.beads --exclude-dir=.git` returns 0 matches.

**Risk.** Medium. The harness is cited in audit evidence for a closed P0 bead (`vb-o5zb.3` ResourceContract reconciliation, closed under `vb-o5zb.5`). Deleting the harness without updating the audit citations would leave dangling evidence. Mitigation: amend audit citations in the same commit, and have the audit reviewer (`vb-o5zb.5/black-hat-review.md`) record the deletion as a follow-up finding to be accepted at next parent reconciliation.

**Hours.** 0.5h (delete, three citation updates, feature-gated cargo check).

**Bead.** `vb-dedup.7` (new, P1).

---

## Master Contract Update Plan

Three lines in `velvet-ballistics-MASTER.md` require updates to remove
misleading citations and add canonical path anchors.

### Line 572 — `CompiledNode` field layout (4 fields → 6 fields)

**Before:**
```
| `CompiledNode` | Single IR node: `id: StepIdx`, `output: Option<SlotIdx>`, `next: Option<StepIdx>`, `kind: CompiledNodeKind`. |
```

**After:**
```
| `CompiledNode` | Single IR node: `id: StepIdx`, `output: Option<SlotIdx>`, `next: Option<StepIdx>`, `on_error: Option<StepIdx>`, `error_slot: Option<SlotIdx>`, `kind: CompiledNodeKind`. Canonical: `crates/vb_core/src/workflow/types.rs:520-538` (re-exported as `vb_core::CompiledNode` from `lib.rs:125`). |
```

### Line 578 — `ResourceContract` field count (16 fields → 18 fields)

**Before:**
```
| `ResourceContract` | 16 fields controlling hard limits (Section 13). |
```

**After:**
```
| `ResourceContract` | 18 fields controlling hard limits (Section 13): `max_steps`, `max_slots`, `max_constants`, `max_accessors`, `max_expressions`, `max_expr_stack`, `max_step_budget_per_tick`, `max_transitions_per_tick`, `max_input_bytes`, `max_output_bytes`, `max_blob_bytes`, `max_ipc_payload_bytes`, `max_retry_attempts`, `max_fanout`, `max_collect_items`, `max_queue_depth`, `max_journal_batch_bytes`, `allows_secret_results`. Canonical: `crates/vb_core/src/workflow/types.rs:167-206` (re-exported as `vb_core::ResourceContract` from `lib.rs:126`). |
```

### Line 3443 — DRIFT-1 evidence citation (dead path → canonical path)

**Before:**
```
- `Finish` node reads slot taint and emits `EngineSignal::Finished(SlotValue, Taint)` (`crates/vb_core/src/nodes.rs`, `node_helpers.rs`).
```

**After:**
```
- `Finish` node reads slot taint and emits `EngineSignal::Finished(SlotValue, Taint)` (`crates/vb_core/src/workflow/types.rs:706-709` for the IR variant, `crates/vb_core/src/engine/node_helpers.rs` for the engine handler).
```

### Section heading at line 565 — add canonical path

**Before:**
```
### `workflow.rs` — Compiled IR Types
```

**After:**
```
### `workflow.rs` — Compiled IR Types

Canonical Rust location: `crates/vb_core/src/workflow/` (re-exported through `lib.rs:124-127`). The `types.rs` submodule is the single source of truth for `CompiledWorkflow`, `CompiledNode`, `CompiledNodeKind`, `ExprProgram`, `ExprOp`, `AccessorProgram`, `PathSegment`, `WorkflowParts`, `ResourceContract`, `WorkflowError`, `ExprBranch`, `SlotBranch`, and `check_expr_stack_bound`. The `validation.rs` submodule is the single source of truth for `validate_parts`, `validate_budget`, and all `validate_*` helpers. Do not introduce parallel type definitions at `crates/vb_core/src/{nodes,expressions,accessors,validation,compiled_workflow}.rs` — those paths were eliminated as dead duplicates (bead series `vb-dedup.*`).
```

---

## Cross-Document Citation Updates

Beyond the master contract, three other documents reference the dead paths and must be updated in the same landing:

| File:line | Current citation | Replacement |
|-----------|------------------|-------------|
| `proof-to-rust-map.md:50` | `vb_core::compiled_workflow::try_from_parts` | `vb_core::workflow::CompiledWorkflow::try_from_parts` (or `vb_core::CompiledWorkflow::try_from_parts` from `lib.rs:125`) |
| `rust-refinement-obligations.jsonl:56` (RRO-vb-xi2f24-004 `rust_target`) | `vb_core::compiled_workflow::try_from_parts` | `vb_core::workflow::CompiledWorkflow::try_from_parts` |
| `rust-refinement-obligations.jsonl:57` (RRO-vb-xi2f24-005 `rust_target`) | `vb_core::compiled_workflow::try_from_parts` | `vb_core::workflow::CompiledWorkflow::try_from_parts` |
| `rust-refinement-obligations.jsonl:56` (`source_refs[1]`) | `crates/vb_core/src/compiled_workflow.rs::try_from_parts` | `crates/vb_core/src/workflow/types.rs::try_from_parts` (canonical impl at lines 32-48) |
| `.beads/vb-o5zb.5/closure-reconciliation-packet.md:69` | "dead-code duplicate with stale DEFAULT" (acknowledged) | Add line: "DEDUP-CLOSED by `vb-dedup.5` on 2026-XX-XX; file removed." |
| `.beads/vb-o5zb.5/black-hat-review.md:44,181,246` | Citations to `kani_resource_contract_validation_18_fields.rs:120-134` | Replace with citations to `crates/vb_core/src/workflow/types.rs:167-206` and `crates/vb_core/src/workflow/validation.rs:93-181` (canonical `validate_resource_contract` and the 18-field count). |
| `.beads/vb-o5zb.5/closure-reconciliation-packet.md:73,82,130` | Citations to the dead Kani harness | Same replacement as above. |

**FINDING-R2 (added 2026-06-12 by vb-g6wst):** the references to the deleted
`kani_resource_contract_validation_18_fields.rs` harness in
`.beads/vb-o5zb.5/black-hat-review.md:44,111,183,240` and
`.beads/vb-o5zb.5/closure-reconciliation-packet.md:73,82,130` are
**HISTORICAL audit-trail evidence, not defects to actively fix**. The
audit packet predates DEDUP-10 (`vb-eq7lv`) and is intentionally
preserved per the dedup plan's principle of evidence preservation
(see AGENTS.md "Do not commit `.beads/dolt` ... runtime database state"
— these audit packets are the explicit exception).

**Note on `.beads/` updates**: `.beads/` files are gitignored evidence packets
(see AGENTS.md "Do not commit `.beads/dolt`, `.beads/backup`, `.beads/embeddeddolt`,
locks, or runtime database state"). However, the audit packets
(`vb-o5zb.5/black-hat-review.md` and `closure-reconciliation-packet.md`) are
explicitly checked into git per their bead pattern. Verify with
`git ls-files .beads/vb-o5zb.5/` before editing.

---

## CI Defense-in-Depth: `compile_error!` Pointer Strategy

The user requested `compile_error!` pointers on surviving dead files.
Since all 7 files (D-1..D-7) are slated for deletion, the question is:
should any stub file remain? Two options:

### Option A: Hard delete (recommended)

Simply delete the 7 files. The defense comes from the build graph:
- `lib.rs` does not declare `pub mod nodes;`, `pub mod expressions;`, etc.
- The dead files are not in the build graph.
- A future agent who recreates `nodes.rs` and adds `pub mod nodes;` to `lib.rs` will hit a `use crate::validation::WorkflowError` resolution failure (if they also recreate `validation.rs`), or a `use crate::nodes::CompiledNode` failure (if they reference the dead type). Either way, the compiler catches it.

**Hours.** 0h additional (already in D-1..D-7 estimates).

### Option B: Keep stub with `compile_error!` (defense-in-depth)

For each deleted file, leave a stub like:

```rust
// crates/vb_core/src/nodes.rs
//
// DEAD FILE — canonical location: crates/vb_core/src/workflow/types.rs:520-538
// Do not edit. The 4-field `CompiledNode` defined here is stale; canonical
// has 6 fields including `on_error` and `error_slot`. See bead vb-dedup.1.
//
// This file is NOT declared in `lib.rs` and is NOT compiled. If you are
// reading this, it means someone re-introduced the file. Delete it again.
compile_error!("DEAD FILE: crates/vb_core/src/nodes.rs was a duplicate of workflow::types::CompiledNode. Use vb_core::CompiledNode instead. See bead vb-dedup.1 for history.");
```

This pattern would only fire if `lib.rs` ever declares `pub mod nodes;`.
It serves as a tripwire for that scenario. But it adds 8 files of
`compile_error!` boilerplate that must itself be maintained, and
`compile_error!` is itself a `!`-typed expression which `rustc`
emits as a hard error.

**Recommendation:** **Option A** is sufficient. The 7-file deletion plus
the master contract update plus the new CI check (below) is stronger
defense than 7 `compile_error!` stubs. If a future agent does
re-introduce a dead file, the canonical-types-already-exist type
collision and the `rg` failure in `scripts/check-no-dead-ir-duplicates.sh`
will surface the regression immediately.

**Hours.** 0h (Option A) or +0.5h (Option B, not recommended).

### New CI check: `scripts/check-no-dead-ir-duplicates.sh`

A new script (added under `scripts/` per the AGENTS.md convention) that
fails the build if any of the 7 deleted files re-appear. Wire into
`.moon/tasks/all.yml` (probably alongside the `dead-code-detection`
      or `unsafe-audit` family of checks; check
      `.moon/tasks/all.yml:196-205` per Round 4 phantom-task finding PP-2,
      resolved by bead vb-z1i03).

```bash
#!/usr/bin/env bash
# scripts/check-no-dead-ir-duplicates.sh
# Bead: vb-dedup.*
# Fails if any of the 7 dead IR-type duplicate files re-appear.
set -euo pipefail

DEAD_PATHS=(
  "crates/vb_core/src/nodes.rs"
  "crates/vb_core/src/expressions.rs"
  "crates/vb_core/src/accessors.rs"
  "crates/vb_core/src/validation.rs"
  "crates/vb_core/src/validation"
  "crates/vb_core/src/compiled_workflow.rs"
  "crates/vb_core/src/compiled_workflow.rs.removed"
  "crates/vb_core/src/kani_resource_contract_validation_18_fields.rs"
)

FAILED=0
for path in "${DEAD_PATHS[@]}"; do
  if [[ -e "$path" ]]; then
    echo "REGRESSION: dead IR-type duplicate re-appeared at $path"
    echo "  See bead vb-dedup.* in the to-fix plan and AGENTS.md for canonical locations."
    FAILED=1
  fi
done

if [[ $FAILED -ne 0 ]]; then
  exit 1
fi

echo "OK: no dead IR-type duplicates present"
```

**Hours.** 0.5h (script write, moon task wire-up, runbook update).

---

## Cross-Bead Coordination

| Dependency | Bead | Status | This plan's interaction |
|------------|------|--------|-------------------------|
| `vb-o5zb` (P0 reconcile taint/step-state/resource) | parent, BLOCKED | children all closed; awaiting `vb-o5zb.5` audit | `vb-dedup.5` and `vb-dedup.7` update `.beads/vb-o5zb.5/` audit evidence. No change to `vb-o5zb` parent status. |
| `vb-xi2f.35` (P1 digest covers resource contract) | CLOSED | closed 2026-06-03 | The dead `kani_resource_contract_validation_18_fields.rs` was evidence for this bead's closure. `vb-dedup.7` deletes the harness; audit citations are amended. No re-open needed because the canonical `workflow/types.rs:167-206` is the actual source of truth. |
| `vb-3tew` (P0 master-doc backend compliance audit) | not visible in `bd list` | audit findings are in `to-fix/00-master-doc-audit-summary.md` | This plan resolves the dead-IR-type hygiene findings that `vb-3tew` would have flagged. The master contract update (line 572, 578, 3443, 565) closes the doc/code drift that the audit was tracking. |

---

## Total Work-Hour Estimate

| Bead | Title | Hours |
|------|-------|-------|
| `vb-dedup.1` | Delete `crates/vb_core/src/nodes.rs` | 0.5h |
| `vb-dedup.2` | Delete `crates/vb_core/src/expressions.rs` | 0.25h |
| `vb-dedup.3` | Delete `crates/vb_core/src/accessors.rs` | 0.15h |
| `vb-dedup.4` | Delete `crates/vb_core/src/validation.rs` + `validation/` directory | 1.0h |
| `vb-dedup.5` | Delete `crates/vb_core/src/compiled_workflow.rs` (4 citation updates) | 1.0h |
| `vb-dedup.6` | Delete `crates/vb_core/src/compiled_workflow.rs.removed` | 0.15h |
| `vb-dedup.7` | Delete `crates/vb_core/src/kani_resource_contract_validation_18_fields.rs` (3 audit citation updates) | 0.5h |
| `vb-dedup.8` | Master contract update (lines 565, 572, 578, 3443) | 0.5h |
| `vb-dedup.9` | CI check `scripts/check-no-dead-ir-duplicates.sh` + moon wire-up | 0.5h |
| `vb-dedup.10` | Validation rerun: `cargo check -p vb_core`, `cargo test -p vb_core -- resource`, `cargo check -p vb_core --features kani-diagnostic-codes` | 0.5h |
| **Total** | | **5.05h base + 2.45h buffer (master doc, test reruns, citation ripple) = 7.5h ceiling** |

The base case (no re-evidence needed) is **4.5h**. The ceiling (master
doc reroll, audit packet rewrites, Kani feature-gated re-check) is
**7.5h**.

---

## Definition of Done

A future agent cannot edit any of the 7 dead files without compilation error **or** a CI gate failure. Specifically:

1. **Files do not exist** in the working tree:
   ```bash
   test ! -e crates/vb_core/src/nodes.rs
   test ! -e crates/vb_core/src/expressions.rs
   test ! -e crates/vb_core/src/accessors.rs
   test ! -e crates/vb_core/src/validation.rs
   test ! -e crates/vb_core/src/validation
   test ! -e crates/vb_core/src/compiled_workflow.rs
   test ! -e crates/vb_core/src/compiled_workflow.rs.removed
   test ! -e crates/vb_core/src/kani_resource_contract_validation_18_fields.rs
   ```
   All 8 commands exit 0.

2. **CI check passes** in clean main:
   ```bash
   bash scripts/check-no-dead-ir-duplicates.sh
   ```
   Exits 0. If any deleted file re-appears, exits 1 with a regression message pointing to this plan and the `vb-dedup.*` bead series.

3. **Build is green**:
   ```bash
   rtk cargo check -p vb_core --message-format=short
   rtk cargo test -p vb_core -- resource
   rtk cargo check -p vb_core --features kani-diagnostic-codes
   ```
   All three exit 0. The 64 resource tests (per `.beads/vb-o5zb.5/closure-reconciliation-packet.md:71`) pass.

4. **No `.removed`/`.bak`/`.orig` tombstones remain** under `crates/vb_core/src/`:
   ```bash
   find crates/vb_core/src \( -name "*.removed" -o -name "*.bak" -o -name "*.orig" \) -print
   ```
   Returns no results.

5. **Master contract is correct**:
   ```bash
   rg -n "16 fields controlling hard limits" velvet-ballistics-MASTER.md
   rg -n "id: StepIdx.*output: Option.*next: Option.*kind: CompiledNodeKind" velvet-ballistics-MASTER.md
   rg -n "crates/vb_core/src/nodes\.rs" velvet-ballistics-MASTER.md
   ```
   The first two return 0 matches (the stale 16-field and 4-field descriptions are gone). The third returns 0 matches (the dead path is no longer cited).

6. **Cross-document citations are updated**:
   ```bash
   rg -n "vb_core::compiled_workflow\b" --exclude-dir=.beads --exclude-dir=.git --exclude-dir=target
   rg -n "kani_resource_contract_validation_18_fields" --exclude-dir=.beads --exclude-dir=.git --exclude-dir=target
   ```
   Both return 0 matches outside `.beads/`. Inside `.beads/`, the remaining matches in `vb-o5zb.5/{black-hat-review,closure-reconciliation-packet}.md` have a "DEDUP-CLOSED" annotation pointing to this plan and the canonical `workflow/types.rs:167-206` location.

7. **Beads are closed** with proper reason text:
   ```bash
   bd show vb-dedup.1
   bd show vb-dedup.2
   bd show vb-dedup.3
   bd show vb-dedup.4
   bd show vb-dedup.5
   bd show vb-dedup.6
   bd show vb-dedup.7
   bd show vb-dedup.8
   bd show vb-dedup.9
   bd show vb-dedup.10
   ```
   All 10 beads show `✓ CLOSED` with close reason naming the deleted paths and citing `velvet-ballistics-MASTER.md` master line 494 ("code wins; doc drift is non-behavior hygiene").

8. **Git push succeeds**:
   ```bash
   git status  # working tree clean
   git pull --rebase
   bd dolt push
   git push
   ```
   Mandatory per AGENTS.md session completion. The 7-deletion commit should be a single atomic commit titled `DEDUP: remove dead IR-type duplicates (vb-dedup.1..7)` with a body that lists the deleted paths, the master contract updates, the cross-document citation updates, and the new CI check.

---

## Per-File Summary Table (one-page reference)

| # | File:line | LOC | Master cite | Action | Risk | Hours | Bead |
|---|-----------|-----|-------------|--------|------|-------|------|
| D-1 | `crates/vb_core/src/nodes.rs:1-190` | 190 | line 572, 3443 | DELETE | Low | 0.5 | `vb-dedup.1` |
| D-2 | `crates/vb_core/src/expressions.rs:1-179` | 179 | (none) | DELETE | Low | 0.25 | `vb-dedup.2` |
| D-3 | `crates/vb_core/src/accessors.rs:1-24` | 24 | (none) | DELETE | Low | 0.15 | `vb-dedup.3` |
| D-4 | `crates/vb_core/src/validation.rs:1-184` + `validation/{graph,nodes,resource,targets}.rs` (1,081 LOC total) | 1,081 | (none) | DELETE (entire tree) | Medium | 1.0 | `vb-dedup.4` |
| D-5 | `crates/vb_core/src/compiled_workflow.rs:1-228` | 228 | line 578; `proof-to-rust-map.md:50`; `rust-refinement-obligations.jsonl:56-57` | DELETE (4 citation updates) | Medium | 1.0 | `vb-dedup.5` |
| D-6 | `crates/vb_core/src/compiled_workflow.rs.removed:1-228` | 228 | (none) | DELETE | Low | 0.15 | `vb-dedup.6` |
| D-7 | `crates/vb_core/src/kani_resource_contract_validation_18_fields.rs:1-167` | 167 | `.beads/vb-o5zb.5/{black-hat-review,closure-reconciliation-packet}.md` | DELETE (3 audit citation updates) | Medium | 0.5 | `vb-dedup.7` |
| — | `velvet-ballistics-MASTER.md:565,572,578,3443` | 4 lines | n/a | UPDATE | Low | 0.5 | `vb-dedup.8` |
| — | `scripts/check-no-dead-ir-duplicates.sh` + moon wire-up | new | n/a | CREATE | Low | 0.5 | `vb-dedup.9` |
| — | Validation rerun (cargo check, cargo test, kani feature) | n/a | n/a | EXECUTE | Low | 0.5 | `vb-dedup.10` |
| **Total** | **2,097 LOC dead + 4 master lines + 1 CI script** | | | | | **5.05h base, 7.5h ceiling** | 10 beads |
