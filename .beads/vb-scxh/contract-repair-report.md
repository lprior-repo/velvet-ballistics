# Contract Repair Report: vb-scxh

## Repairs made

- Added explicit contract clauses for exact 12 false-closure capture, safety anchor primary evidence, TLA path consistency, scope separation, and non-laundering evidence integrity.
- Repaired TLA target contract to canonical `.beads/vb-scxh/tla/ScxhRecovery.tla` and `.beads/vb-scxh/tla/ScxhRecovery.cfg`.
- Added primary `proof-obligations.jsonl` rows for all missing pre/post/invariant clauses and all `Error::*` variants.
- Added primary waiver rows for Verus, Lean/Aeneas/Hax, Kani, Flux, Loom/Shuttle, Miri/cargo-careful, proptest/fuzz, and performance/API/release-provenance non-goals.
- Made safety bundle/bookmark verification a primary obligation and recorded current bundle-open failure as `BLOCK_LOCAL`.
- Made exact false-closure ID capture and reopened/linked BD status a primary raw-evidence obligation.
- Preserved generated parity gaps as deferrals to `vb-gvmt` / `vb-qi37.10`; referenced generated parity artifacts are scope-control inputs only.
- Replaced tautological subagent evidence claim with a TLA obligation requiring `AttemptLaunderSubagentEvidence`, `NoAcceptanceFromSubagentRequiredEvidence`, and `LaunderingAttemptRejected`.
- Repaired contract-review schema issues after State 6 rejection: `SAFETY-SCXH-001` and `ERR-SCXH-006` now use `status: planned` and preserve downstream blocker semantics with `failure_classification: BLOCK_LOCAL`.
- Added mandatory TLA metadata fields to `TLA-SCXH-003` and `TLA-SCXH-004` using canonical `.beads/vb-scxh/tla/ScxhRecovery.tla` and `.beads/vb-scxh/tla/ScxhRecovery.cfg` paths.
- Added explicit `assumptions` and `waiver` fields to every proof-obligation row; waiver rows carry machine-readable waiver details, non-waiver rows use `null`.

## Downstream blockers retained

- State 5 must update/rerun TLA artifacts if existing model does not include strengthened laundering/safety/path obligations.
- State 11 must produce raw BD exact-12 audit, safety anchor report, CI audit, mutation audit, and scope-control audit; safety anchor raw verification remains a real downstream `BLOCK_LOCAL` anchor if bundle/ref checks fail.
- State 12 must block final close/unblock unless all required lanes pass or have approved waivers; safety anchor failure remains `failure_classification: BLOCK_LOCAL` even though State 3 ledger row status is `planned`.
