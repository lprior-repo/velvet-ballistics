# vb-qi37.6 Proof Repair Guide

STATUS: NO_STATE_5_REPAIR_REQUIRED

## State 5 Proof Artifacts

- No repair is required for the State 5 TLA+/Verus proof artifacts reviewed here.
- `verification/tla/CapabilityLifecycle.tla` plus all six `CapabilityLifecycle*.cfg` review configs passed TLC under repo-local `.tmp/state6-proof-review-rerun/tlc-*` metadirs.
- `verification/verus/capability_artifact_model.rs` passed Verus with `8 verified, 0 errors`.
- The repaired ledgers are byte-identical primary/planned files with 24 rows, zero `PASS` statuses, and zero `BLOCKED_SETUP` placeholders.

## Required Later Repairs

- Kani setup remains required before release-level proof closure. State 8 must fix or correctly gate `crates/vb_core/src/lib.rs:41` so `pub mod kani;` resolves, or otherwise route the intended harnesses through valid module paths.
- After Kani setup repair, State 11 must rerun at least `TMPDIR=/home/lewis/src/vb-qi37-6/.tmp RUSTC_WRAPPER= cargo kani -p vb_core --harness capability_name_grants_harness` and the planned runtime capability harness, then record raw output, unwind/bound details, and PASS/FAIL status.
- Fuzz setup remains required before release-level proof closure. State 8 must register `capability_name_schema` and `capability_contract_schema` in `fuzz/Cargo.toml`, or provide equivalent owning fuzz invocations.
- After fuzz setup repair, State 11 must rerun the planned capability schema fuzz budgets and record raw output with run counts, target names, corpus/seed notes where applicable, and PASS/FAIL status.

## Do Not Launder

- Do not mark Kani as PASS until the harnesses compile and execute successfully.
- Do not mark fuzz as PASS until the target bins are registered or equivalent targets execute successfully.
- Do not reuse the current TLA result as deadlock, liveness, fairness, parser, storage, public API, postcard, Fjall, filesystem, or UI parity proof.
- Do not convert later implementation/integration obligations into State 5 proof PASS rows.

## Optional Strengthening

- If later states need progress claims, add explicit non-vacuous TLA liveness/fairness properties or executable integration tests that prove progress. The current State 5 TLA approval is safety-only.
- If later states need production-level capability schema proof, connect the Verus model to executable validators or rely on repaired fuzz/unit/integration evidence with strong oracles.
