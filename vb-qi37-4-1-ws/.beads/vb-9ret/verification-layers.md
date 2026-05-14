# Verification Layers: vb-9ret validate/compile adapters

## Boundary
- Verified kernel: vb_validate and vb_compile adapters, workflow compilation pipeline, adapter preservation during deduplication.
- Excluded from formal proof: vb_core budget computation internals, Makepad rendering, OS filesystem semantics.

## Layer Assignment
- PRE-001 -> unit + integration: adapter trait signatures preserved after deduplication.
- PRE-002 -> unit + integration: compile workflow succeeds with preserved adapters.
- POST-001 -> integration: output artifacts match expected structure.
- INV-001 -> unit + proptest: no adapter state corruption during deduplication.

## Required Commands and Evidence
- Unit/integration: `cargo nextest run -p vb_validate -p vb_compile` -> `.evidence/vb-9ret/nextest.txt`.
- Compile check: `cargo check -p vb_validate -p vb_compile` -> `.evidence/vb-9ret/check.txt`.
- Five-lane gauntlet: `moon run :verify-fast`, `moon run :verify-standard`, `moon run :verify-deep`, `moon run :verify-proof`, `moon run :verify-all` as release-critical evidence boundaries.

## Structured Waivers

### WAIVE-INCLUDE-STR-PATH-ORIGIN-MAIN
- Clause ID: moon-ci (implicit pre-contract gate).
- Waived layer: moon ci full pipeline.
- Reason: `moon ci` fails due to pre-existing `include_str!` path errors in `crates/vb_core/tests/aggregate_resource_budget_*.rs` that exist on origin/main BEFORE this bead branched. These are test-only instrumentation paths (`include_str!("../src/budget.rs")` and `include_str!("../../vb_runtime/src/admission.rs")`) that fail only under specific moon ci file-scanning tasks, not under standard `cargo check`/`cargo test`.
- Compensating evidence: `cargo nextest run -p vb_compile` (246 tests pass), `cargo nextest run -p vb_validate` (all pass), `cargo check -p vb_validate -p vb_compile --tests` (compiles clean), `moon run :verify-fast` (passes), `moon run :verify-standard` (passes).
- Owner: State 8 contract repair agent for bead `vb-9ret`; downstream formal-verifier owns expiry enforcement.
- Expiry/follow-up: expires when origin/main include_str path errors are resolved; follow-up owner is the bead that fixes `crates/vb_core/tests/` instrumentation paths.

## Independent Review Gate
Downstream State 4+ work must not consume these artifacts until an independent reviewer writes `.beads/vb-9ret/contract-verification-review.md` with `STATUS: APPROVED`.
