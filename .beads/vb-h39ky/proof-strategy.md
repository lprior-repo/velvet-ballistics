# Proof Strategy — vb-h39ky

**Bead:** vb-h39ky — register 296 unregistered Verus files
**Master sections:** §40, §44
**Status:** PARTIAL → planning full triage closure

## 1. Problem Frame

The bead's close reason (PARTIAL) establishes:
- 329 total production `verus!` blocks across the workspace.
- 33 explicitly registered via `files:` lists in `proof_obligations.yaml`.
- 296 are local `#[cfg(verus)]` blocks inside production source files
  (e.g. `workflow/lifecycle/mod.rs`, `action/proof.rs`).
- No vacuum files were found.

The bead itself is meta — it does not produce new proof evidence. It
produces a **triage table** that classifies each of the 296 blocks into
one of three buckets:

| Bucket | Meaning | v0.2.0 disposition |
|---|---|---|
| `register_in_v0_2_0` | Will receive an explicit obligation row | Add to `proof_obligations.yaml` |
| `defer_to_v0_2_0_with_obligation` | Production-bound but needs contract work first | Document with linked_obligations |
| `retire_as_vacuum_model` | Standalone sanity model, NOT bound to production | Add to `verus_registry_targets` retire notes |

## 2. Anti-Laundering Mandate

Per the planner skill's ANTI-VERIFICATION LAUNDERING MANDATE, every
registered obligation must bind to production Rust. The triage rule is:

> A `#[cfg(verus)]` block is registerable ONLY if it annotates a specific
> production function whose body it documents. A standalone `verus!{}`
> block in `verification/verus/*.rs` that does not include or `path=`
> production code is RETIRED.

This applies to 14 triage groups below. Group 14 (vacuum-spec-only-sketches)
catches the 4 already-RETIRED files plus 6 additional similar artifacts
discovered during group enumeration.

## 3. Lane Selections (meta-task)

| Lane | Required? | Rationale |
|---|---|---|
| Verus (L4) | NO | Triage is registry work; existing Verus obligations already use L4. |
| Kani (L3) | NO | Triage is registry work; existing Kani obligations already use L3. |
| proptest (L1) | NO | Triage is registry work. |
| Flux | NO | Registry work. |
| Loom | NO | Registry work. |
| cargo-fuzz | NO | Registry work. |
| TLA+ | NO | Registry work. |

The lane decisions for the **groups** are inherited from their underlying
obligations. The triage only changes the **registry location**, not the
verifier lane policy.

## 4. Risk Tags (from seed)

- `registry`: triage of catalog items.
- `vacuum-model-elimination`: forbid standalone `verus!{}` blocks from
  counting as proof evidence.
- `production-binding`: every register decision must cite a production
  source path.

## 5. Triage Decision Tree

For each `#[cfg(verus)]` block, ask:

1. Does it annotate a specific production fn (in the same file or via
   `#[path]`)? → YES → register.
2. Is there an existing obligation row whose `files:` list contains the
   production fn? → YES → register.
3. Is the block a standalone spec-only sketch with phantom constants or
   self-contained math? → YES → retire.
4. Otherwise → defer (needs contract work).

## 6. Execution Order

1. Run `rg -l 'verus!|cfg\(verus\)' crates/ verification/` to enumerate
   every block. Save to `.beads/vb-h39ky/file_list.txt`.
2. Group files by directory pattern (eval/, lexer/, parser/, bytecode/,
   workflow/, action/, journal/, recovery/, runtime/, proof_kernels/,
   classify/, normalize/, spec_only/).
3. For each group, apply the decision tree.
4. Write `triage_table.md` with all decisions and rationale.
5. Update `contracts/proof_obligations.yaml`
   `verus_file_triage_2026_06_19` block with the new counts.
6. Add the 40 retire entries to `verus_registry_targets` notes.

## 7. Out of Scope

- Per-file obligation row creation in v0.2.0 (the 132 register and 124
  defer entries are placeholders pointing to v0.2.0 work).
- Modifying any production code.