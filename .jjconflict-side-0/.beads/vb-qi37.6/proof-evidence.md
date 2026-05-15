# vb-qi37.6 Proof Evidence

## Scope

- Workspace used: `/home/lewis/src/vb-qi37-6`.
- Forbidden checkout `/home/lewis/src/Velvet-ballistics` was not used.
- Current State 5 rerun status after repaired 24-row plan: REPAIRED.
- Evidence edits are limited to proof-owned report/evidence files.
- Kani and fuzz are later-owner routed; no Kani or fuzz PASS is claimed by State 5.

## 24-Row Plan Validation

Command:

```bash
mkdir -p .tmp/state5-rerun-24 && jq -r '.id' .beads/vb-qi37.6/proof-obligations.jsonl > .tmp/state5-rerun-24/ids.primary && jq -r '.id' .beads/vb-qi37.6/proof-obligations.planned.jsonl > .tmp/state5-rerun-24/ids.planned && diff -u .tmp/state5-rerun-24/ids.primary .tmp/state5-rerun-24/ids.planned && test "$(wc -l < .tmp/state5-rerun-24/ids.primary)" -eq 24 && jq -e 'select(.status=="PASS")' .beads/vb-qi37.6/proof-obligations.planned.jsonl >/tmp/vb_qi37_6_pass_rows && test ! -s /tmp/vb_qi37_6_pass_rows && jq -e 'select((.id=="PRE-003-FUZZ-SCHEMA") and .owner_state==8 and .after_setup_owner_state==11)' .beads/vb-qi37.6/proof-obligations.planned.jsonl >/dev/null && jq -e 'select((.id=="INV-001-KANI-EXACT-SETUP" or .id=="INV-002-KANI-CARDINALITY-SETUP") and .owner_state==8 and .after_setup_owner_state==11)' .beads/vb-qi37.6/proof-obligations.planned.jsonl >/dev/null && jq -e 'select(.id=="GAUNTLET-010" and .owner_state==11 and (.blocked_until | length == 2))' .beads/vb-qi37.6/proof-obligations.planned.jsonl >/dev/null
```

Status: PASS.

Evidence summary:

- `proof-obligations.jsonl` and `proof-obligations.planned.jsonl` contain exactly 24 IDs.
- ID order and membership match exactly between primary and planned ledgers.
- No planned row has status `PASS`.
- `PRE-003-FUZZ-SCHEMA` is routed to owner_state 8 setup and owner_state 11 execution.
- `INV-001-KANI-EXACT-SETUP` and `INV-002-KANI-CARDINALITY-SETUP` are routed to owner_state 8 setup and owner_state 11 execution.
- `GAUNTLET-010` remains owner_state 11 and blocked on later Kani/fuzz setup or approved waivers.

## TLA+ Evidence

Commands:

```bash
TMPDIR=/home/lewis/src/vb-qi37-6/.tmp JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=/home/lewis/src/vb-qi37-6/.tmp tlc -metadir .tmp/state5-rerun-24/tlc-all -config verification/tla/CapabilityLifecycleAll.cfg verification/tla/CapabilityLifecycle.tla
TMPDIR=/home/lewis/src/vb-qi37-6/.tmp JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=/home/lewis/src/vb-qi37-6/.tmp tlc -metadir .tmp/state5-rerun-24/tlc-gate -config verification/tla/CapabilityLifecycleGateMismatch.cfg verification/tla/CapabilityLifecycle.tla
TMPDIR=/home/lewis/src/vb-qi37-6/.tmp JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=/home/lewis/src/vb-qi37-6/.tmp tlc -metadir .tmp/state5-rerun-24/tlc-exact -config verification/tla/CapabilityLifecycleExactProfile.cfg verification/tla/CapabilityLifecycle.tla
TMPDIR=/home/lewis/src/vb-qi37-6/.tmp JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=/home/lewis/src/vb-qi37-6/.tmp tlc -metadir .tmp/state5-rerun-24/tlc-excess -config verification/tla/CapabilityLifecycleExcessGrant.cfg verification/tla/CapabilityLifecycle.tla
TMPDIR=/home/lewis/src/vb-qi37-6/.tmp JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=/home/lewis/src/vb-qi37-6/.tmp tlc -metadir .tmp/state5-rerun-24/tlc-nocontract -config verification/tla/CapabilityLifecycleNoContract.cfg verification/tla/CapabilityLifecycle.tla
TMPDIR=/home/lewis/src/vb-qi37-6/.tmp JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=/home/lewis/src/vb-qi37-6/.tmp tlc -metadir .tmp/state5-rerun-24/tlc-legacy -config verification/tla/CapabilityLifecycleLegacyBypass.cfg verification/tla/CapabilityLifecycle.tla
```

Status: PASS for all six configs.

Evidence summary:

- TLC version: `TLC2 Version 2.19 of 08 August 2024`.
- Each config reported `Model checking completed. No error has been found.`
- Each config reported `478 states generated, 220 distinct states found, 0 states left on queue`.
- Each config reported complete state graph search depth `3`.
- Each config used an isolated `-metadir` under `.tmp/state5-rerun-24`.

Config-to-ID mapping:

- `CapabilityLifecycleAll.cfg`: `PRE-001-TLA-ENVELOPE`, `POST-005-TLA-SUCCESS-JOURNAL`, `INV-005-TLA-DENIAL-ATOMIC`.
- `CapabilityLifecycleGateMismatch.cfg`: `PRE-002-TLA-GATE15`, `POST-002-TLA-GATE-DENIAL`, `INV-003-TLA-GATE-CONTRACT`.
- `CapabilityLifecycleExactProfile.cfg`: `POST-004-TLA-MISSING-EXACT`, `INV-008-TLA-PUBLIC-GRANTS`.
- `CapabilityLifecycleExcessGrant.cfg`: `POST-003-TLA-CARDINALITY-DENIAL`.
- `CapabilityLifecycleNoContract.cfg`: `PRE-005-TLA-CONTRACT-SLICE`, `POST-006-TLA-DO-CHECKS`, `POST-007-TLA-NO-CONTRACT-DENY`, `INV-006-TLA-SHARD-CONTRACTS`.
- `CapabilityLifecycleLegacyBypass.cfg`: `POST-008-TLA-LEGACY-BYPASS`.

## Verus Evidence

Command:

```bash
TMPDIR=/home/lewis/src/vb-qi37-6/.tmp RUSTC_WRAPPER= verus verification/verus/capability_artifact_model.rs
```

Status: PASS.

Evidence summary:

- Output: `verification results:: 8 verified, 0 errors`.

ID mapping:

- `POST-001-VERUS-EXACT`: exact capability name/action model, prefix denial, empty-name denial, action mismatch denial.
- `INV-004-VERUS-PERSISTENCE`: pure required-capability preservation model across accepted-artifact profile extraction.

## Later-Owner Routed IDs

- `PRE-003-FUZZ-SCHEMA`: owner_state 8 setup for fuzz target registration; owner_state 11 execution for `capability_name_schema` and `capability_contract_schema`; State 5 records routing only.
- `PRE-004-API-GRANTS`: owner_state 10 tests/BDD; State 5 records routing only.
- `PRE-006-UI-SOURCE`: owner_state 10 UI source parity tests; State 5 records routing only.
- `POST-009-UI-PARITY`: owner_state 10 UI projection/parity tests; State 5 records routing only.
- `INV-001-KANI-EXACT-SETUP`: owner_state 8 Kani module setup; owner_state 11 `cargo kani -p vb_core --harness capability_name_grants_harness`; State 5 records routing only.
- `INV-002-KANI-CARDINALITY-SETUP`: owner_state 8 upstream Kani setup; owner_state 11 `cargo kani -p vb_runtime --harness check_capability_grants_exact_match`; State 5 records routing only.
- `INV-007-STATIC-LEGACY`: owner_state 10 static scan/integration evidence; State 5 records routing only.
- `GAUNTLET-010`: owner_state 11 release gauntlet; blocked until State 8 Kani/fuzz setup is repaired or waivers are approved; State 5 records routing only.

## Complete 24-ID Accounting

- `PRE-001-TLA-ENVELOPE`: State 5 TLA PASS via `CapabilityLifecycleAll.cfg`.
- `PRE-002-TLA-GATE15`: State 5 TLA PASS via `CapabilityLifecycleGateMismatch.cfg`.
- `PRE-003-FUZZ-SCHEMA`: later-owner routed, no State 5 PASS.
- `PRE-004-API-GRANTS`: later-owner routed, no State 5 PASS.
- `PRE-005-TLA-CONTRACT-SLICE`: State 5 TLA PASS via `CapabilityLifecycleNoContract.cfg`.
- `PRE-006-UI-SOURCE`: later-owner routed, no State 5 PASS.
- `POST-001-VERUS-EXACT`: State 5 Verus PASS via `capability_artifact_model.rs`.
- `POST-002-TLA-GATE-DENIAL`: State 5 TLA PASS via `CapabilityLifecycleGateMismatch.cfg`.
- `POST-003-TLA-CARDINALITY-DENIAL`: State 5 TLA PASS via `CapabilityLifecycleExcessGrant.cfg`.
- `POST-004-TLA-MISSING-EXACT`: State 5 TLA PASS via `CapabilityLifecycleExactProfile.cfg`.
- `POST-005-TLA-SUCCESS-JOURNAL`: State 5 TLA PASS via `CapabilityLifecycleAll.cfg`.
- `POST-006-TLA-DO-CHECKS`: State 5 TLA PASS via `CapabilityLifecycleNoContract.cfg`.
- `POST-007-TLA-NO-CONTRACT-DENY`: State 5 TLA PASS via `CapabilityLifecycleNoContract.cfg`.
- `POST-008-TLA-LEGACY-BYPASS`: State 5 TLA PASS via `CapabilityLifecycleLegacyBypass.cfg`.
- `POST-009-UI-PARITY`: later-owner routed, no State 5 PASS.
- `INV-001-KANI-EXACT-SETUP`: later-owner routed, no State 5 PASS.
- `INV-002-KANI-CARDINALITY-SETUP`: later-owner routed, no State 5 PASS.
- `INV-003-TLA-GATE-CONTRACT`: State 5 TLA PASS via `CapabilityLifecycleGateMismatch.cfg`.
- `INV-004-VERUS-PERSISTENCE`: State 5 Verus PASS via `capability_artifact_model.rs`.
- `INV-005-TLA-DENIAL-ATOMIC`: State 5 TLA PASS via `CapabilityLifecycleAll.cfg`.
- `INV-006-TLA-SHARD-CONTRACTS`: State 5 TLA PASS via `CapabilityLifecycleNoContract.cfg`.
- `INV-007-STATIC-LEGACY`: later-owner routed, no State 5 PASS.
- `INV-008-TLA-PUBLIC-GRANTS`: State 5 TLA PASS via `CapabilityLifecycleExactProfile.cfg`.
- `GAUNTLET-010`: later-owner routed to State 11, no State 5 PASS.

## Assumptions And Bounds

- TLA evidence is finite-state safety evidence only; no liveness PASS is claimed.
- TLA bounds: `CanonicalGate = 15`, gate count cases `{0, 2, 15}`, bounded required/grant count cases, booleans for artifact presence, contracts, journal state, allocation state, and legacy path.
- Verus evidence is pure-model evidence only; runtime I/O, Fjall storage, postcard serialization, filesystem durability, UI serde shell, and integration behavior remain later evidence boundaries.
- Kani and fuzz setup/execution remain later-owner routed and are not State 5 proof-writer failures after the repaired 24-row plan.
