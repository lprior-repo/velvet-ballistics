# Combined Landing Evidence — qi37 all-ready integration

status: READY_TO_PUSH
updated_at: 2026-05-12T03:56:46Z
workspace: `/home/lewis/src/Velvet-ballistics-landing-all-q37`
forbidden_source_checkout: `/home/lewis/src/Velvet-ballistics` not touched

## Integrated Inputs

- Combined ready base: `xuqoyknzslkmrmqkynwqkqxmoxyrvylq` / `cc1e2b3cb1b982145758dc63fdd11cef44288f1e`.
- Durable resume bead `vb-qi37.16.2`: `sxklquuwtnppxxolqttonpmtmrztmvto` / `a9aa4a8cf76928a0cd96c27f12911fd6a2513c83`.
- Lifecycle evidence bead `vb-qi37.16.5`: `nxzzqotvonmzxymuktkulwtqzvxxqwsm` / `a55ef9156699b551be5f483f199ee36ba64c2b59`.

## Conflict / Hygiene Resolution

- Preserved strict split wrappers for `vb_runtime` journal, shard implementation, lifecycle, and runtime modules.
- Kept `body.inc` out of the landed tree.
- Removed imported/generated `states/` scratch from the combined integration.
- Repaired runtime submit admission durability: `submit_direct` now pre-persists `RunSubmitted`, drains that header before acknowledgement, and shard processing records `RunAdmission` without duplicating the run header.

## Commands and Outcomes

- `rtk cargo fmt --all`: PASS.
- `bash scripts/check-source-length.sh && if rg -n 'body\.inc' --glob '!target/**' --glob '!vb-*/**'; then exit 1; else true; fi`: PASS.
- `rtk cargo test -p vb_runtime ask_answer --lib`: PASS — 19 passed, 1323 filtered out.
- `rtk cargo test -p vb_runtime --test durable_resume_red_phase`: PASS — 17 passed.
- `rtk cargo test -p vb_storage --test replay_resume`: PASS — 3 passed.
- `rtk cargo test -p velvet_ballastics --test admission_evidence_integration`: PASS — 8 passed.
- `rtk cargo test -p velvet_ballastics --test lifecycle_integration`: PASS — 43 passed.
- `rtk cargo test -p vb_runtime journal::tests::runtime_shutdown_graceful_drains_owned_queued_journal --lib`: PASS — 1 passed, 1341 filtered out.
- `moon ci`: PASS — 19 tasks completed, 2 cached; 8063 tests passed; duration 1m 56s 290ms.

## Performance Layer

No speed claim made. No benchmark/profiler evidence required for this landing integration.

## Decision

Ready to push/land from the isolated workspace. Per user instruction, this evidence run did not move `main`, push, close beads, or forget/remove workspaces.
