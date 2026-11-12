# vb-vt2f regression diff

bead_id: vb-vt2f
phase: 11
attempt: 2-of-7
STATUS: PASS_MACHINE_GATES_FORMAL_BLOCK

## Current status

State 10 attempt 6 cleared the machine/test regressions. State 11 attempt 2 recorded:

- Direct runtime API acceptance: PASS, `13 passed`.
- Catalog acceptance: PASS, `13 passed`.
- `moon ci`: PASS, `9015 tests run: 9015 passed, 2 skipped`.

No current machine regression remains. Formal proof approval is still blocked because old TLA+/Verus waivers are void after production runtime/shard/admission changes.

---

# Attempt 1 historical regression

## Historical blocking regression

State 10 changed runtime/admission submit semantics. Focused direct API and catalog acceptance tests now pass, but canonical `moon ci` fails in bead-related runtime/CLI integration:

- `vb_cli::cli_integration cli_run_minimal_workflow_completes`
- `vb_cli::cli_integration cli_run_maps_postcard_slot_values_from_input_bin`

Failure text: `runtime submit error: admission rejected: artifact not found`.

## Historical scope classification

This is not pre-existing unrelated global debt. It is local to the bead because the changed files include runtime/admission components consumed by CLI submit paths.

## Historical repair route

Rerun from State 10 with `holzman-rust`: preserve approved direct API strict admission behavior while keeping CLI `--durability none` minimal workflows from requiring missing accepted-artifact storage.
