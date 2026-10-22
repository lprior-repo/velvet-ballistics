# vb-bbf56 Repair Contract — production-inner drift (faithful)

## Problem

`scripts/check-production-inner-drift.sh` exits 1 with 13 drift findings:
5 `production_inner/` mirror files and 8 `extern_*` BINDING LEDGER files carry
stale production line ranges / missing identifiers after the accepted
faithful split (`bfb8bac3`) restructured `vb_core::workflow`, split
`vb_storage::events` into `events.rs` + `events/event.rs`, and added new
types (`InputSlotKind`, `CompiledInputSlot`, `ExprBranch`, `SlotBranch`,
`RetryCheckpoint`, `RunInputBinding`) plus new `WorkflowError` /
`CliExitCode` variants.

## Scope & constraints

- Production Rust (`crates/**`) is READ ONLY. No edits there.
- Edit only: `verification/verus/production_inner/*.rs` mirrors,
  `verification/verus/extern_*.rs` BINDING LEDGER comments, and evidence/docs.
- No weakening of `scripts/check-*.sh`, no gate exceptions, no phantom
  identifiers added solely to satisfy grep. Every added identifier is a real
  production type/variant/field mirrored with correct shape.

## Approach (per finding)

Mirror repairs: regenerate the drifted mirror sections from current
production source — add the genuinely-new production identifiers with exact
field types / enum variants / array lengths, and correct each stale
`Source:`/DRIFT POLICY claim range to the moved production location.
Opaque extern crypto (`blake3`) is recorded as an explicitly TRUSTED
boundary (opaque stub, no fake hash output); no proof closure claimed over it.

Extern ledger repairs: update BINDING LEDGER `<- crates/...` line ranges to
the current production location of the named identifier.

## Verification gates (required)

1. `bash scripts/check-production-inner-drift.sh` must exit 0, run through
   `/tmp/vb-run-gate.py SECONDS PREFIX COMMAND ARGS`, raw output under
   `.beads/vb-bbf56/evidence/faithful*`.
2. `bash scripts/check-verus-production-binding.sh` must exit 0.
3. Affected bounded Verus obligations via `bash scripts/verify-verus.sh`
   (registry-driven), no broad fleet / Kani / full moon CI.

## Acceptance

- Drift gate exit 0 (exit 1 is NOT PASS).
- Production-binding gate exit 0.
- No proof claims invented; trusted boundaries explicitly labelled.
- Bead stays OPEN; root assigns independent review.

## Out of scope / not claimed

- Full Verus proof closure over opaque external crypto/serialization.
- Full CI fleet / Kani / moon ci.
