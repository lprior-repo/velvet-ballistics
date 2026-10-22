# vb-h7rex Assembly V8 Handoff

## Scope

ONE production source repair: split oversized `serialize_pinned_admission_payload` in `crates/vb_storage/src/events/wire_serialize_run.rs` (previously 38 lines) into two private serializer helpers with <=25 logic lines each, preserving exact tuple layout, tags 39/41, and all semantics.

## Changes

### Production (1 file)

**`crates/vb_storage/src/events/wire_serialize_run.rs`**

- `serialize_pinned_admission_payload` (lines 121-158) split into dispatcher + 2 helpers
- New `serialize_pinned_unbound_admission` — tag 39 (RunAdmissionPinned), 3 args, ~13 logic lines
- New `serialize_pinned_bound_admission` — tag 41 (RunAdmissionBoundPinned), 4 args, ~13 logic lines
- Dispatcher preserves original match structure; no semantic change
- No new `panic`/`unwrap`/`expect`/`dbg`/`todo`/`unsafe`
- No duplicated schema constants or magic defaults

### Test Fixtures (2 files)

**`crates/vb_storage/src/codec/tests/kill_kind_admission.rs`**

- `kind_28_does_not_affect_known_kinds_elsewhere`: flip 39/41 from unknown→known assertions
- Tag 38 (policy decision) remains unknown — not yet in production on this branch

**`crates/workspace_tests/tests/vb_eepg_bdd_tests.rs`**

- `is_known_record_kind_returns_false_for_invalid_kinds`: remove 39/41 from `truly_unknown_kinds` list
- Update known-kinds comment to include 39/41

## Evidence

Three bounded gates under `.beads/vb-h7rex/evidence/ASSEMBLY_V8_*` via `/tmp/vb-run-gate.py`:

| Gate | Command | Exit | Time |
|------|---------|------|------|
| ASSEMBLY_V8_proptest_journal | `cargo test -p vb_storage --test proptest_journal_error_codes` | 0 | 2.38s |
| ASSEMBLY_V8_eepg_bdd | `cargo test -p velvet-ballistics-workspace-tests --test vb_eepg_bdd_tests` | 0 | 0.12s |
| ASSEMBLY_V8_fmt | `cargo fmt -- --check` | 0 | 1.52s |

Workspace eepg_bdd suite: 33/33 passed, including `is_known_record_kind_returns_true_for_valid_kinds` and `is_known_record_kind_returns_false_for_invalid_kinds`.

## Line Counts

- Original `serialize_pinned_admission_payload`: 38 lines (fn+where+body)
- Dispatcher after split: 18 lines
- `serialize_pinned_unbound_admission`: 13 logic lines, 3 args
- `serialize_pinned_bound_admission`: 13 logic lines, 4 args
- All under 25-line threshold

## Limits

- No formal verifier runs (Kani/Verus/TLA+/Flux) — per task scope
- No broad workspace test suite — only pinned eepg_bdd focused suite
- No JJs/Git mutation — root will freeze/review/gate integration
- Pre-existing lib test compile errors (`policy_pin` missing in `recovery_unit_tests.rs`, `tests.rs`) not in scope

## Verification Checklist

- [x] Cargo fmt clean on changed file and workspace
- [x] `vb_storage` crate compiles (`cargo check -p vb_storage`)
- [x] Workspace eepg_bdd_tests 33/33 pass
- [x] Proptest journal error codes pass
- [x] Tag 38 still unknown (storage child not landed)
- [x] Tags 39/41 now known in test fixtures

## Root Review Required

Integration into main branch, final CI gate (`moon ci`), and bead closure.