# vb-qi37.6 Proof Writer Report

STATUS: REPAIRED

## Summary

- Validated the repaired State 3/4 proof ledgers: `proof-obligations.jsonl` and `proof-obligations.planned.jsonl` match exactly on 24 IDs.
- Reran proof-owned State 5 TLA+/Verus evidence against the repaired 24-row plan in `/home/lewis/src/vb-qi37-6` only.
- TLC passed all six `CapabilityLifecycle` configs; each reported `Model checking completed. No error has been found.`, `478 states generated`, and `220 distinct states found`.
- Verus passed `verification/verus/capability_artifact_model.rs` with `verification results:: 8 verified, 0 errors`.
- Kani setup/execution and fuzz setup/execution are recorded as later-owner routed work, not State 5 PASS.

## Commands Run

### 24-Row Ledger Validation

- `mkdir -p .tmp/state5-rerun-24 && jq -r '.id' .beads/vb-qi37.6/proof-obligations.jsonl > .tmp/state5-rerun-24/ids.primary && jq -r '.id' .beads/vb-qi37.6/proof-obligations.planned.jsonl > .tmp/state5-rerun-24/ids.planned && diff -u .tmp/state5-rerun-24/ids.primary .tmp/state5-rerun-24/ids.planned && test "$(wc -l < .tmp/state5-rerun-24/ids.primary)" -eq 24 && jq -e 'select(.status=="PASS")' .beads/vb-qi37.6/proof-obligations.planned.jsonl >/tmp/vb_qi37_6_pass_rows && test ! -s /tmp/vb_qi37_6_pass_rows && jq -e 'select((.id=="PRE-003-FUZZ-SCHEMA") and .owner_state==8 and .after_setup_owner_state==11)' .beads/vb-qi37.6/proof-obligations.planned.jsonl >/dev/null && jq -e 'select((.id=="INV-001-KANI-EXACT-SETUP" or .id=="INV-002-KANI-CARDINALITY-SETUP") and .owner_state==8 and .after_setup_owner_state==11)' .beads/vb-qi37.6/proof-obligations.planned.jsonl >/dev/null && jq -e 'select(.id=="GAUNTLET-010" and .owner_state==11 and (.blocked_until | length == 2))' .beads/vb-qi37.6/proof-obligations.planned.jsonl >/dev/null`: PASS.

### TLA+ Rerun

- `TMPDIR=/home/lewis/src/vb-qi37-6/.tmp JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=/home/lewis/src/vb-qi37-6/.tmp tlc -metadir .tmp/state5-rerun-24/tlc-all -config verification/tla/CapabilityLifecycleAll.cfg verification/tla/CapabilityLifecycle.tla`: PASS.
- `TMPDIR=/home/lewis/src/vb-qi37-6/.tmp JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=/home/lewis/src/vb-qi37-6/.tmp tlc -metadir .tmp/state5-rerun-24/tlc-gate -config verification/tla/CapabilityLifecycleGateMismatch.cfg verification/tla/CapabilityLifecycle.tla`: PASS.
- `TMPDIR=/home/lewis/src/vb-qi37-6/.tmp JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=/home/lewis/src/vb-qi37-6/.tmp tlc -metadir .tmp/state5-rerun-24/tlc-exact -config verification/tla/CapabilityLifecycleExactProfile.cfg verification/tla/CapabilityLifecycle.tla`: PASS.
- `TMPDIR=/home/lewis/src/vb-qi37-6/.tmp JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=/home/lewis/src/vb-qi37-6/.tmp tlc -metadir .tmp/state5-rerun-24/tlc-excess -config verification/tla/CapabilityLifecycleExcessGrant.cfg verification/tla/CapabilityLifecycle.tla`: PASS.
- `TMPDIR=/home/lewis/src/vb-qi37-6/.tmp JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=/home/lewis/src/vb-qi37-6/.tmp tlc -metadir .tmp/state5-rerun-24/tlc-nocontract -config verification/tla/CapabilityLifecycleNoContract.cfg verification/tla/CapabilityLifecycle.tla`: PASS.
- `TMPDIR=/home/lewis/src/vb-qi37-6/.tmp JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=/home/lewis/src/vb-qi37-6/.tmp tlc -metadir .tmp/state5-rerun-24/tlc-legacy -config verification/tla/CapabilityLifecycleLegacyBypass.cfg verification/tla/CapabilityLifecycle.tla`: PASS.

All six TLC runs reported `478 states generated, 220 distinct states found, 0 states left on queue`, complete search depth `3`, and no invariant violation.

### Verus Rerun

- `TMPDIR=/home/lewis/src/vb-qi37-6/.tmp RUSTC_WRAPPER= verus verification/verus/capability_artifact_model.rs`: PASS, `verification results:: 8 verified, 0 errors`.

## 24-ID Obligation Disposition

- `PRE-001-TLA-ENVELOPE`: covered by TLC `CapabilityLifecycleAll.cfg`; State 5 proof evidence PASS for absent-envelope denial/no allocation invariants.
- `PRE-002-TLA-GATE15`: covered by TLC `CapabilityLifecycleGateMismatch.cfg`; State 5 proof evidence PASS for gate-count mismatch denial.
- `PRE-003-FUZZ-SCHEMA`: later-owner routed; owner_state 8 fuzz bin registration, owner_state 11 fuzz execution; no State 5 PASS claimed.
- `PRE-004-API-GRANTS`: later-owner routed to State 10 tests/BDD; no State 5 PASS claimed.
- `PRE-005-TLA-CONTRACT-SLICE`: covered by TLC `CapabilityLifecycleNoContract.cfg`; State 5 proof evidence PASS for no-contract denial and contracted Do exact-grant invariant.
- `PRE-006-UI-SOURCE`: later-owner routed to State 10 UI parity tests; no State 5 PASS claimed.
- `POST-001-VERUS-EXACT`: covered by Verus `capability_artifact_model.rs`; State 5 proof evidence PASS for exact name/action model.
- `POST-002-TLA-GATE-DENIAL`: covered by TLC `CapabilityLifecycleGateMismatch.cfg`; State 5 proof evidence PASS for invalid gate denial/no allocation.
- `POST-003-TLA-CARDINALITY-DENIAL`: covered by TLC `CapabilityLifecycleExcessGrant.cfg`; State 5 proof evidence PASS for cardinality mismatch denial/no allocation.
- `POST-004-TLA-MISSING-EXACT`: covered by TLC `CapabilityLifecycleExactProfile.cfg`; State 5 proof evidence PASS for missing/non-exact grant denial/no allocation.
- `POST-005-TLA-SUCCESS-JOURNAL`: covered by TLC `CapabilityLifecycleAll.cfg`; State 5 proof evidence PASS for journal-after-admission safety invariants.
- `POST-006-TLA-DO-CHECKS`: covered by TLC `CapabilityLifecycleNoContract.cfg`; State 5 proof evidence PASS for contracted Do exact-grant requirement.
- `POST-007-TLA-NO-CONTRACT-DENY`: covered by TLC `CapabilityLifecycleNoContract.cfg`; State 5 proof evidence PASS for no-contract no-AwaitingAction invariant.
- `POST-008-TLA-LEGACY-BYPASS`: covered by TLC `CapabilityLifecycleLegacyBypass.cfg`; State 5 proof evidence PASS for protected legacy bypass denial.
- `POST-009-UI-PARITY`: later-owner routed to State 10 UI projection/parity tests; no State 5 PASS claimed.
- `INV-001-KANI-EXACT-SETUP`: later-owner routed; owner_state 8 Kani module wiring, owner_state 11 Kani execution; no State 5 PASS claimed.
- `INV-002-KANI-CARDINALITY-SETUP`: later-owner routed; owner_state 8 Kani setup dependency, owner_state 11 runtime Kani execution; no State 5 PASS claimed.
- `INV-003-TLA-GATE-CONTRACT`: covered by TLC `CapabilityLifecycleGateMismatch.cfg`; State 5 proof evidence PASS for single gate-count contract fail-closed behavior.
- `INV-004-VERUS-PERSISTENCE`: covered by Verus `capability_artifact_model.rs`; State 5 proof evidence PASS for pure required-capability preservation model.
- `INV-005-TLA-DENIAL-ATOMIC`: covered by TLC `CapabilityLifecycleAll.cfg`; State 5 proof evidence PASS for denial atomicity/no run allocation.
- `INV-006-TLA-SHARD-CONTRACTS`: covered by TLC `CapabilityLifecycleNoContract.cfg`; State 5 proof evidence PASS for shard contract-bypass denial model.
- `INV-007-STATIC-LEGACY`: later-owner routed to State 10 static scan/integration evidence; no State 5 PASS claimed.
- `INV-008-TLA-PUBLIC-GRANTS`: covered by TLC `CapabilityLifecycleExactProfile.cfg`; State 5 proof evidence PASS for empty/non-exact public grant denial model.
- `GAUNTLET-010`: later-owner routed to State 11 gauntlet; remains blocked on State 8 Kani/fuzz setup or approved waivers; no State 5 PASS claimed.

## Deferred Later-Owner Work

- Kani: `INV-001-KANI-EXACT-SETUP` and `INV-002-KANI-CARDINALITY-SETUP` remain owner_state 8 setup with owner_state 11 execution. State 5 did not edit `crates/vb_core/src/kani.rs`, `crates/vb_core/src/kani/mod.rs`, production source, or Kani harness source.
- Fuzz: `PRE-003-FUZZ-SCHEMA` remains owner_state 8 fuzz target registration with owner_state 11 execution. State 5 did not edit `fuzz/Cargo.toml` or fuzz target source.
- Runtime/UI/static/test lanes: `PRE-004-API-GRANTS`, `PRE-006-UI-SOURCE`, `POST-009-UI-PARITY`, and `INV-007-STATIC-LEGACY` remain State 10 evidence.
- Release gauntlet: `GAUNTLET-010` remains State 11 and must consume TLA/Verus evidence plus later Kani/fuzz/test/static evidence or explicit waivers.

## Boundaries And Assumptions

- Workspace used: `/home/lewis/src/vb-qi37-6` only.
- Forbidden source checkout `/home/lewis/src/Velvet-ballistics` was not used.
- Edits in this rerun are limited to `.beads/vb-qi37.6/proof-writer-report.md` and `.beads/vb-qi37.6/proof-evidence.md`.
- TLA model is finite: gate count `{0, 2, 15}`, required/grant counts in bounded small domains, and booleans for artifact/contracts/legacy-path cases.
- TLA evidence is safety-only; no liveness PASS is claimed.
- Verus abstracts capability names/actions and preservation as pure model values; Fjall I/O, postcard bytes, filesystem durability, UI serde shell, and runtime integration remain later evidence boundaries.
