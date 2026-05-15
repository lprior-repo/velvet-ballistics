# Test Plan: vb-qi37.6 State 7 Capability Enforcement

## Startup citations

- `/home/lewis/.claude/skills/test-planner/SKILL.md` lines 8-10 require this agent to produce `test-plan.md` only and not write implementation or test code.
- `/home/lewis/.claude/skills/test-planner/SKILL.md` lines 75-93 require every behavior to have Given/When/Then scenarios with exact expected outcomes.
- `/home/lewis/.claude/skills/test-planner/SKILL.md` lines 96-155 require proptest invariants, fuzz targets, Kani harness plans, and mutation checkpoints.
- `/home/lewis/.claude/skills/test-planner/SKILL.md` lines 170-171 and 219-227 reject vague `is_ok()` / `is_err()` assertions and require explicit error/value checks.
- `/home/lewis/.agents/skills/test-planner/SKILL.md` contains the same rules and wins on conflict.
- `/home/lewis/.agents/skills/test-planner/references/testing-philosophy.md` lines 5-10 require behavior testing through public APIs, and lines 82-86 require every important behavior to be automated and hermetic.

## Inputs reviewed

- Approved contract review: `.beads/vb-qi37.6/contract-verification-review.md` (`STATUS: APPROVED`).
- Approved proof review: `.beads/vb-qi37.6/proof-review.md` (`STATUS: APPROVED`).
- Contract: `.beads/vb-qi37.6/contract.md`.
- 24-row traceability matrix: `.beads/vb-qi37.6/traceability-matrix.jsonl`.
- 24-row proof obligation ledger: `.beads/vb-qi37.6/proof-obligations.jsonl`.

## Summary

- Behaviors identified: 24 contract/traceability behaviors.
- Trophy allocation: 7 unit / 11 integration / 2 e2e-BDD / 4 static/formal setup-or-release gates.
- Proptest invariants: 5.
- Fuzz targets: 2 setup-gated targets plus schema seed suite.
- Kani harnesses: 2 setup-gated harness groups.
- Mutation threshold: at least 90% mutation kill rate, with 100% kill required for exact-grant, cardinality, gate-count, no-contract, and persistence denial branches.
- Red/failing-first rule: each scenario below must first fail against the known current facts from `contract.md` line 13 where applicable: storage writes gate count `2`, accepted artifacts default `required_capabilities` empty, public runtime paths use `CapabilitySet::empty()`, and shard drive passes `&[]` action contracts.

## 1. Behavior Inventory

1. Runtime rejects missing accepted-artifact envelope before admission when policy is Strict or Journaled (`PRE-001`).
2. Runtime rejects accepted artifacts whose gate count is not canonical `15` when policy is Strict or Journaled (`PRE-002`, `POST-002`, `INV-003`).
3. Gate 12 derives non-empty persisted `required_capabilities` from validated action contracts when actions require capabilities (`PRE-003`, `INV-004`).
4. Public Runtime submit paths bind caller grants before admission for capability-protected workflows (`PRE-004`, `INV-008`).
5. Shard drive threads validated action-contract slices into Do execution (`PRE-005`, `INV-006`).
6. UI action descriptions source `required_capabilities` from the same action-contract source as storage/runtime (`PRE-006`, `POST-009`).
7. `CapabilitySet::grants` grants only exact `(name, action_id)` pairs (`POST-001`, `INV-001`).
8. `CapabilitySet::grants` denies hierarchical parent grants such as `network` for `network.github` (`POST-001`, `INV-001`).
9. `CapabilitySet::grants` denies child/partial/sibling/empty-name/action-mismatch grants (`POST-001`, `INV-001`).
10. Admission denies grant cardinality mismatch, including excess grants, with no run frame (`POST-003`, `INV-002`, `INV-005`).
11. Admission denies missing grants with no run frame (`POST-003`, `POST-004`, `INV-002`, `INV-005`).
12. Admission denies non-exact grants with no run frame (`POST-004`, `INV-001`, `INV-005`).
13. Successful admission records digest, run id, policy, exact granted capabilities, and journals only after success (`POST-005`).
14. Denied admission appends no `RunAdmission` journal event (`INV-005`).
15. Contracted Do execution checks all required capabilities before emitting an action ticket (`POST-006`).
16. Do execution without action contract fails closed with `CapabilityDenied` requiring `__contract_required__` (`POST-007`).
17. Do execution without action contract produces no `AwaitingAction` / no action ticket (`POST-007`, `INV-006`).
18. Legacy admission/existence-only APIs cannot bypass protected Strict/Journaled submit flows (`POST-008`, `INV-007`).
19. Empty public Runtime grants are valid only when required capability set is empty (`INV-008`).
20. Exact public Runtime grants admit capability-protected workflows if all other admission preconditions hold (`PRE-004`, `INV-008`).
21. UI serialization/roundtrip preserves required capabilities exactly, including non-empty sets (`POST-009`).
22. Kani setup is detectable in State 8 and routes exact-grant harness execution to State 11 (`INV-001-KANI-EXACT-SETUP`).
23. Kani setup is detectable in State 8 and routes cardinality harness execution to State 11 (`INV-002-KANI-CARDINALITY-SETUP`).
24. Release gauntlet routes proof/deep/static/Miri/fuzz checks without laundering deferred Kani/fuzz blockers into PASS (`GAUNTLET-010`).

## 2. Trophy Allocation

| Layer | Count | Behaviors | Rationale |
|---|---:|---|---|
| Unit / calc | 7 | 7, 8, 9, 15, 16, 17, 21 | Exact grant lattice, Do denial value, and UI roundtrip can be checked fast and deterministically with exact values/errors. |
| Integration | 11 | 1, 2, 3, 4, 5, 10, 11, 12, 13, 14, 20 | Admission/storage/runtime/shard persistence behaviors cross component boundaries and must use real in-memory/local stores where possible. |
| E2E / BDD | 2 | 18, 19 | Public Runtime submit and no legacy bypass are user-facing flow properties; keep narrow but black-box. |
| Static/formal/setup | 4 | 22, 23, 24 plus static portion of 18 | Kani/fuzz/Miri/static/release gates are command-verifiable setup/execution routes, not ordinary runtime behavior. |

Deviation from 60/30/5/5: this bead is security-enforcement heavy, so static/formal setup is larger than 5%. Integration remains widest because storage/runtime/shard/UI parity must be proven across real boundaries.

## 3. BDD Scenarios

### Behavior 1: Missing accepted-artifact envelope denies admission

Test name: `fn admit_artifact_run_rejects_missing_accepted_artifact_when_policy_is_strict_or_journaled()`

Given: a Strict and Journaled runtime admission request for a workflow digest with no persisted `AcceptedArtifact` envelope.
When: `admit_artifact_run` is invoked through the public admission boundary.
Then: the exact result is `Err(AdmissionError::ArtifactEnvelopeDecodeFailed)` or the contract-defined missing-envelope denial variant used by implementation.
And: no run frame exists for the requested `RunId`.
And: no `RunAdmission` journal event is appended.
Red expectation: fails first if the legacy artifact-exists path admits or allocates without accepted-artifact load.

Command:

```bash
TMPDIR=/home/lewis/src/vb-qi37-6/.tmp RUSTC_WRAPPER= cargo test -p vb_runtime admit_artifact_run_rejects_missing_accepted_artifact --lib
```

### Behavior 2: Gate count mismatch denies admission

Test name: `fn runtime_rejects_gate_count_2_under_strict_journaled_without_allocation()`

Given: a persisted accepted artifact whose `verification.gate_count` is `2`, plus Strict and Journaled policies.
When: runtime admission is attempted with otherwise exact capability grants.
Then: the exact error is `AdmissionError::ArtifactInvalidGateCount { found: 2, required: 15 }`.
And: no run frame, run state, or `RunAdmission` journal event exists.
Red expectation: fails first against the current storage fact that `submit_artifact` writes gate count `2` while runtime requires `15`.

Command:

```bash
TMPDIR=/home/lewis/src/vb-qi37-6/.tmp RUSTC_WRAPPER= cargo test -p vb_runtime gate_count --lib
```

### Behavior 3: Required capabilities persist from action contracts

Test name: `fn submit_artifact_persists_non_empty_required_capabilities_when_contract_requires_capability()`

Given: a compiled workflow whose validated `ActionContract.required_capabilities` contains `[Capability { name: "network.github", action_id: A }]`.
When: `submit_artifact` persists the accepted artifact and it is reloaded/decoded.
Then: `AcceptedArtifact.required_capabilities` equals exactly `[Capability { name: "network.github", action_id: A }]`.
And: it is not `[]`.
Red expectation: fails first because the approved contract records current storage defaulting `AcceptedArtifact.required_capabilities` to empty.

Command:

```bash
TMPDIR=/home/lewis/src/vb-qi37-6/.tmp RUSTC_WRAPPER= cargo test -p vb_storage required_capabilities --lib
```

### Behavior 4: Public Runtime rejects empty grants for protected workflows

Test name: `fn public_submit_empty_grants_denies_non_empty_requirements_when_policy_is_strict_or_journaled()`

Given: a public `Runtime` submit call for a workflow with non-empty accepted-artifact requirements.
When: the caller provides no grants or the API implicitly uses `CapabilitySet::empty()`.
Then: the exact error maps to `RuntimeError::AdmissionCapabilityDenied` caused by `AdmissionError::CapabilityDenied`.
And: the denial reports the required capability and an empty granted set.
And: no run id is returned.
Red expectation: fails first if public Runtime APIs continue to pass `CapabilitySet::empty()` while admitting protected workflows.

Command:

```bash
TMPDIR=/home/lewis/src/vb-qi37-6/.tmp RUSTC_WRAPPER= cargo test -p vb_runtime public_submit_empty_grants_denies_non_empty_requirements --lib
```

### Behavior 5: Public Runtime accepts exact grants for protected workflows

Test name: `fn public_submit_exact_grants_admits_when_requirements_and_gate_count_are_valid()`

Given: a public `Runtime` submit call for a Strict/Journaled workflow with gate count `15`, a persisted accepted-artifact envelope, and required capability `[network.github:A]`.
When: the caller supplies exactly `[network.github:A]` through the public grant path.
Then: admission returns a concrete run id and `RunAdmission` containing the same digest, run id, policy, and exact granted capability set.
And: no extra grant is present.
Red expectation: fails first if no public API exists to bind non-empty grants.

Command:

```bash
TMPDIR=/home/lewis/src/vb-qi37-6/.tmp RUSTC_WRAPPER= cargo test -p vb_runtime public_submit_exact_grants_admits --lib
```

### Behavior 6: Shard drive threads contracts into Do execution

Test name: `fn shard_drive_threads_contracts_when_do_node_requires_capability()`

Given: a shard state containing a Do node with a validated action contract requiring `[network.github:A]`.
When: the shard drive lifecycle reaches Do execution with exact grants.
Then: the engine receives the non-empty contract slice and returns `AwaitingAction` / action ticket for that Do node.
And: the ticket is absent if the slice is empty.
Red expectation: fails first because current shard drive forwards `&[]` action contracts.

Command:

```bash
TMPDIR=/home/lewis/src/vb-qi37-6/.tmp RUSTC_WRAPPER= cargo test -p vb_runtime shard_drive_threads_contracts --lib
```

### Behavior 7: Exact grant allows capability

Test name: `fn capability_set_grants_exact_name_and_action_when_required_matches_grant()`

Given: a `CapabilitySet` containing exactly `(name = "network.github", action_id = A)`.
When: checking required `("network.github", A)`.
Then: `CapabilitySet::grants` returns exactly `true`.
And: `check_capability` returns `Ok(())` with no error.
Red expectation: must fail if implementation permits broad grants but breaks exact match.

Command:

```bash
TMPDIR=/home/lewis/src/vb-qi37-6/.tmp RUSTC_WRAPPER= cargo test -p vb_core capability_set_grants_exact_name --lib
```

### Behavior 8: Hierarchical parent grant is denied

Test name: `fn capability_set_rejects_hierarchical_parent_prefix_when_required_is_child()`

Given: a grant `(name = "network", action_id = A)`.
When: checking required `(name = "network.github", action_id = A)`.
Then: `CapabilitySet::grants` returns exactly `false`.
And: runtime admission returns `AdmissionError::CapabilityDenied { action: A, required: network.github:A, granted: [network:A] }`.
Red expectation: fails first if any prefix/hierarchical grant semantics survive.

Command:

```bash
TMPDIR=/home/lewis/src/vb-qi37-6/.tmp RUSTC_WRAPPER= cargo test -p vb_core hierarchical_prefix --lib
```

### Behavior 9: Partial, sibling, empty-name, and action mismatch grants are denied

Test name: `fn capability_set_rejects_non_exact_name_or_action_when_required_differs()`

Given: grants covering partial prefix `net`, sibling `network.gitlab`, child `network.github.repo`, empty name `""`, and action mismatch `(network.github:B)`.
When: checking required `(network.github:A)`.
Then: each case returns exactly `false` from `CapabilitySet::grants`.
And: each admission case returns `AdmissionError::CapabilityDenied`, not gate-count or decode errors.
Red expectation: fails first if string-prefix or action-agnostic matching is used.

Command:

```bash
TMPDIR=/home/lewis/src/vb-qi37-6/.tmp RUSTC_WRAPPER= cargo test -p vb_core capability_set_rejects_non_exact --lib
```

### Behavior 10: Excess grants deny admission

Test name: `fn admit_artifact_run_rejects_excess_grants_without_allocation()`

Given: an accepted artifact requiring `[network.github:A]` and caller grants `[network.github:A, filesystem.read:B]`.
When: Strict/Journaled admission runs.
Then: the exact error is `AdmissionError::CapabilityDenied` with required count `1` and granted count `2` represented in the error payload.
And: no run frame or journal event exists.
Red expectation: fails first if admission only checks required subset and ignores extra grants.

Command:

```bash
TMPDIR=/home/lewis/src/vb-qi37-6/.tmp RUSTC_WRAPPER= cargo test -p vb_runtime admit_artifact_run_rejects_excess_grants --lib
```

### Behavior 11: Missing grants deny admission

Test name: `fn admit_artifact_run_rejects_missing_grants_without_allocation()`

Given: an accepted artifact requiring `[network.github:A, filesystem.read:B]` and caller grants `[network.github:A]`.
When: Strict/Journaled admission runs.
Then: the exact error is `AdmissionError::CapabilityDenied` naming missing `filesystem.read:B` or reporting required/granted exact mismatch.
And: no run frame or journal event exists.
Red expectation: fails first if only one capability is checked or empty default requirements hide the missing grant.

Command:

```bash
TMPDIR=/home/lewis/src/vb-qi37-6/.tmp RUSTC_WRAPPER= cargo test -p vb_runtime admit_artifact_run_rejects_missing_grants --lib
```

### Behavior 12: Non-exact grant denies admission

Test name: `fn admit_artifact_run_rejects_non_exact_grant_without_allocation()`

Given: an accepted artifact requiring `[network.github:A]` and caller grant `[network:A]` or `[network.github:B]`.
When: admission runs.
Then: the exact error is `AdmissionError::CapabilityDenied`, not success.
And: no run frame or journal event exists.
Red expectation: fails first if admission does not use `CapabilitySet::grants` exact semantics.

Command:

```bash
TMPDIR=/home/lewis/src/vb-qi37-6/.tmp RUSTC_WRAPPER= cargo test -p vb_runtime admit_artifact_run_rejects_non_exact_grant --lib
```

### Behavior 13: Successful admission journals only after success

Test name: `fn run_admission_journaled_only_after_success_when_requirements_are_exactly_met()`

Given: a valid accepted artifact, gate count `15`, exact required/granted capabilities, and Strict/Journaled policy.
When: admission succeeds.
Then: `RunAdmission` contains exact digest, run id, policy, and granted capabilities.
And: the journal contains exactly one `RunAdmission` event after successful admission.
Red expectation: fails first if journaling happens before validation or omits granted capabilities.

Command:

```bash
TMPDIR=/home/lewis/src/vb-qi37-6/.tmp RUSTC_WRAPPER= cargo test -p vb_runtime run_admission_journaled_only_after_success --lib
```

### Behavior 14: Denied admission is atomic

Test name: `fn denied_admission_appends_no_run_admission_and_allocates_no_run_frame()`

Given: each denial cause: missing envelope, invalid gate count, excess grants, missing grants, non-exact grants.
When: admission returns the specific denial error for that cause.
Then: run state lookup for that run id returns absent.
And: journal contains no `RunAdmission` event for that run id.
Red expectation: fails first if any error branch allocates then returns denial.

Command:

```bash
TMPDIR=/home/lewis/src/vb-qi37-6/.tmp RUSTC_WRAPPER= cargo test -p vb_runtime denied_admission --lib
```

### Behavior 15: Contracted Do checks all required capabilities before ticket

Test name: `fn execute_do_with_contract_checks_all_required_capabilities_before_ticket()`

Given: a Do action contract requiring two capabilities and caller grants both exactly.
When: `execute_do` runs.
Then: the result is exactly `RuntimeSignal::AwaitingAction` containing the expected action id/ticket.
And: removing either grant returns `EngineError::CapabilityDenied` and no ticket.
Red expectation: fails first if Do emits a ticket before checking the full contract.

Command:

```bash
TMPDIR=/home/lewis/src/vb-qi37-6/.tmp RUSTC_WRAPPER= cargo test -p vb_runtime execute_do_with_contract_checks_all_required_capabilities --lib
```

### Behavior 16: No-contract Do denies with sentinel requirement

Test name: `fn execute_do_without_contract_rejects_with_contract_required_sentinel()`

Given: Do execution is invoked without an action contract slice.
When: `execute_do_without_contract` or the no-contract branch runs.
Then: the exact error is `EngineError::CapabilityDenied` requiring capability name `__contract_required__`.
And: the granted set in the error equals the caller grants supplied to that invocation.
Red expectation: fails first if absent contracts are interpreted as zero requirements.

Command:

```bash
TMPDIR=/home/lewis/src/vb-qi37-6/.tmp RUSTC_WRAPPER= cargo test -p vb_runtime execute_do_without_contract_rejects_without_ticket --lib
```

### Behavior 17: No-contract Do produces no ticket

Test name: `fn execute_do_without_contract_produces_no_awaiting_action_when_denied()`

Given: Do execution has no contract.
When: it is driven by shard lifecycle.
Then: no `AwaitingAction` state or action ticket is emitted.
And: the visible state is the exact capability-denied failure state.
Red expectation: fails first if shard drive still passes `&[]` and treats it as permission.

Command:

```bash
TMPDIR=/home/lewis/src/vb-qi37-6/.tmp RUSTC_WRAPPER= cargo test -p vb_runtime no_contract --lib
```

### Behavior 18: Legacy paths do not bypass protected admission

Test name: `fn strict_submit_uses_admit_artifact_run_before_allocation_when_workflow_is_capability_protected()`

Given: a protected workflow submitted through Strict/Journaled public or shard flows.
When: legacy `admit_run` or `compiled_ir_exists` path would otherwise permit existence-only admission.
Then: the protected submit flow calls accepted-artifact admission before allocation and returns the relevant denial when requirements are unmet.
And: no run allocation is observable from the legacy path.
Red expectation: fails first if strict submit can succeed using artifact existence alone.

Commands:

```bash
rg 'admit_run\(|compiled_ir_exists\(' crates/vb_runtime/src crates/velvet_ballastics/tests
TMPDIR=/home/lewis/src/vb-qi37-6/.tmp RUSTC_WRAPPER= cargo test -p vb_runtime strict_submit_uses_admit_artifact_run_before_allocation --lib
```

### Behavior 19: Empty public grants allowed only for zero requirements

Test name: `fn public_submit_empty_grants_admits_only_when_required_capabilities_are_empty()`

Given: two accepted artifacts: one with empty requirements and one with `[network.github:A]`.
When: public Runtime submit is called with empty grants.
Then: the empty-requirement artifact admits successfully with granted capabilities `[]`.
And: the non-empty-requirement artifact returns `RuntimeError::AdmissionCapabilityDenied`.
Red expectation: fails first if public `CapabilitySet::empty()` is accepted for both artifacts.

Command:

```bash
TMPDIR=/home/lewis/src/vb-qi37-6/.tmp RUSTC_WRAPPER= cargo test -p vb_runtime public_submit_empty_grants --lib
```

### Behavior 20: Exact public grants admit protected workflow

Test name: `fn public_runtime_grant_path_records_exact_grants_when_workflow_is_protected()`

Given: a public Runtime API variant in scope for caller grants, a gate-count-15 accepted artifact, and required `[network.github:A]`.
When: public Runtime submit receives exact grant `[network.github:A]`.
Then: the returned run id maps to a `RunAdmission` whose granted set equals exactly `[network.github:A]`.
And: no implicit extra or empty grants are recorded.
Red expectation: fails first if public Runtime capability grants are not implemented in State 10 scope.

Command:

```bash
TMPDIR=/home/lewis/src/vb-qi37-6/.tmp RUSTC_WRAPPER= cargo test -p vb_runtime public_runtime_grant_path_records_exact_grants --lib
```

### Behavior 21: UI required-capability parity and roundtrip

Test name: `fn action_description_view_required_capabilities_roundtrip_matches_action_contract_source()`

Given: an action contract source containing non-empty required capabilities.
When: `ActionDescriptionView` is built and serialized/deserialized.
Then: `ActionDescriptionView.required_capabilities` equals exactly the source contract set in the same canonical order or equivalent canonical multiset.
And: storage/runtime expected requirements for the same action equal the UI set.
Red expectation: fails first if UI carries stale or independently synthesized requirements.

Command:

```bash
TMPDIR=/home/lewis/src/vb-qi37-6/.tmp RUSTC_WRAPPER= cargo test -p vb_ui_model required_capabilities --lib
TMPDIR=/home/lewis/src/vb-qi37-6/.tmp RUSTC_WRAPPER= cargo test -p velvet_ballastics ui_required_capabilities --test admission_evidence_integration
```

### Behavior 22: Kani exact-grant setup routes to State 11 execution

Test name: `fn state8_kani_exact_grant_setup_detects_module_before_state11_execution()`

Given: State 8 is responsible for Kani setup.
When: the setup check runs.
Then: it reports present only if `crates/vb_core/src/kani.rs` or `crates/vb_core/src/kani/mod.rs` exists.
And: absent setup must remain a blocker, not a PASS.
And: after setup, State 11 must execute `cargo kani -p vb_core --harness capability_name_grants_harness`.
Red expectation: currently reports `KANI_SETUP_MISSING`; State 8 must turn this red into setup-present before State 11 execution.

Commands:

```bash
if test -f crates/vb_core/src/kani.rs || test -f crates/vb_core/src/kani/mod.rs; then printf 'KANI_SETUP_PRESENT\n'; else printf 'KANI_SETUP_MISSING\n'; fi
TMPDIR=/home/lewis/src/vb-qi37-6/.tmp RUSTC_WRAPPER= cargo kani -p vb_core --harness capability_name_grants_harness
```

### Behavior 23: Kani cardinality setup routes to State 11 execution

Test name: `fn state8_kani_cardinality_setup_detects_dependency_before_state11_execution()`

Given: runtime Kani cardinality harness depends on vb_core Kani module wiring.
When: the setup check runs in State 8.
Then: missing vb_core Kani module is a blocker for runtime harness execution.
And: after setup, State 11 must execute `cargo kani -p vb_runtime --harness check_capability_grants_exact_match`.
Red expectation: currently reports `KANI_SETUP_MISSING`; no State 7/8 artifact may claim harness PASS.

Commands:

```bash
if test -f crates/vb_core/src/kani.rs || test -f crates/vb_core/src/kani/mod.rs; then printf 'KANI_SETUP_PRESENT\n'; else printf 'KANI_SETUP_MISSING\n'; fi
TMPDIR=/home/lewis/src/vb-qi37-6/.tmp RUSTC_WRAPPER= cargo kani -p vb_runtime --harness check_capability_grants_exact_match
```

### Behavior 24: Release gauntlet preserves deferred blocker routing

Test name: `fn release_gauntlet_reports_kani_fuzz_blockers_until_state8_setup_then_state11_execution()`

Given: Kani setup and fuzz bin setup are State 8 owned and execution is State 11 owned.
When: release gauntlet or formal verifier runs.
Then: `verify-proof`, `verify-deep`, `lint-src`, `miri`, and `fuzz-smoke` evidence is recorded without converting setup blockers into PASS.
And: if Moon lanes are absent, fallback commands run and record scoped evidence.
Red expectation: fails first if `GAUNTLET-010` ignores Kani/fuzz blockers or claims release PASS before State 8/11 evidence.

Commands:

```bash
moon run :verify-proof && moon run :verify-deep && moon run :lint-src && moon run :miri && moon run :fuzz-smoke
bash scripts/rust-verification-gauntlet.sh proof
bash scripts/rust-verification-gauntlet.sh deep
rustup run nightly-2026-04-28 cargo clippy --workspace --lib --bins --examples --all-features -- -D warnings
rustup run nightly-2026-04-28 cargo miri test --quiet -p vb_core --lib --all-features capability
cargo fuzz build --target x86_64-unknown-linux-gnu
```

## 4. Proptest Invariants

### Proptest: `CapabilitySet::grants`

Invariant: for any generated required `(name, action)` and grant set, `grants` is true iff at least one grant has byte-exact same name and action id.
Strategy: generate non-empty UTF-8 capability names including dotted names, siblings, prefixes, suffixes, empty string, Unicode, and random action ids.
Anti-invariant: any parent/child/prefix/sibling/action-mismatch case must return false.

### Proptest: admission cardinality exactness

Invariant: admission succeeds only when required capability multiset equals granted capability multiset and gate count is `15`; any missing or extra grant returns `AdmissionError::CapabilityDenied`.
Strategy: generate required capability vectors length `0..8`, grant vectors with exact, missing, excess, duplicate, and non-exact substitutions.
Anti-invariant: required len != granted len must never admit.

### Proptest: required capability persistence roundtrip

Invariant: validated `ActionContract.required_capabilities` roundtrips through `AcceptedArtifact.required_capabilities` serialization/reload without erasure or addition.
Strategy: generate valid action contracts with `0..8` capabilities, unique names/action ids, then encode/decode accepted artifact.
Anti-invariant: non-empty source becoming empty is always failure.

### Proptest: UI capability parity

Invariant: UI `ActionDescriptionView.required_capabilities` equals the source action-contract required capabilities and storage/runtime required capabilities for the same action.
Strategy: generate action descriptions from shared action-contract fixtures with `0..8` capabilities.
Anti-invariant: UI capability set differing by name, action id, cardinality, or order/canonical multiset fails.

### Proptest: public Runtime empty-grant boundary

Invariant: empty caller grants are admitted iff accepted artifact required capabilities are empty and all other admission preconditions hold.
Strategy: generate artifacts with empty/non-empty requirements and valid gate/envelope values.
Anti-invariant: non-empty requirements plus empty grants must always produce `RuntimeError::AdmissionCapabilityDenied`.

## 5. Fuzz Targets

### Fuzz Target: `capability_name_schema`

Input type: bytes/string capability name.
Risk: invalid UTF-8, empty names, prefix-confusable names, path separators, null bytes, oversized names, Unicode normalization confusion, parser panic/OOM, accepting forbidden wildcard/hierarchical syntax.
Corpus seeds: `""`, `"network"`, `"network.github"`, `"network.github.repo"`, `"network.gitlab"`, `"net"`, `"network*"`, `"*"`, `"__contract_required__"`, names with null byte, very long dotted names, Unicode lookalikes.
Setup command (State 8):

```bash
test -f fuzz/Cargo.toml && rg -n 'name = "capability_name_schema"' fuzz/Cargo.toml
```

Execution command (State 11):

```bash
TMPDIR=/home/lewis/src/vb-qi37-6/.tmp RUSTC_WRAPPER= cargo fuzz run capability_name_schema -- -runs=1000
```

Red expectation: currently setup is expected to report missing until State 8 registers the bin; no fuzz PASS may be claimed before State 11 run output.

### Fuzz Target: `capability_contract_schema`

Input type: bytes/structured action contract payload.
Risk: duplicate capabilities, mismatched action ids, empty/defaulted requirements, malformed JSON/TOML/postcard-like payloads, panic/OOM, accepting invalid contracts before persistence.
Corpus seeds: empty contract, one capability, duplicate capability, same name different action, same action different name, missing required_capabilities field, null required_capabilities, huge list, no-contract sentinel.
Setup command (State 8):

```bash
test -f fuzz/Cargo.toml && rg -n 'name = "capability_contract_schema"' fuzz/Cargo.toml
```

Execution command (State 11):

```bash
TMPDIR=/home/lewis/src/vb-qi37-6/.tmp RUSTC_WRAPPER= cargo fuzz run capability_contract_schema -- -runs=1000
```

Red expectation: currently setup is expected to report missing until State 8 registers the bin; no fuzz PASS may be claimed before State 11 run output.

## 6. Kani Harnesses

### Kani Harness: exact-only capability grants

Property: for all bounded generated names/action ids, `CapabilitySet::grants(required)` is true only for byte-exact same name and exact same action id; parent, child, partial, sibling, empty-name, duplicate, and action-mismatch grants are false.
Bound: capability name length `0..32`, grant set size `0..4`, action id bounded to representative small integer/domain type values.
Rationale: exact least-privilege matching is security-critical and prefix bugs can survive finite example tests.
Setup command (State 8):

```bash
sh -c 'test -f crates/vb_core/src/kani.rs || test -f crates/vb_core/src/kani/mod.rs'
```

Execution command (State 11):

```bash
TMPDIR=/home/lewis/src/vb-qi37-6/.tmp RUSTC_WRAPPER= cargo kani -p vb_core --harness capability_name_grants_harness
```

### Kani Harness: runtime cardinality exactness

Property: admission/check-capability denies when required/granted cardinalities differ and admits only when sets are exactly equal under exact grant semantics.
Bound: required/grant vectors size `0..4`, representative action ids and capability names from bounded enum/symbol table.
Rationale: extra grants are as dangerous as missing grants for least privilege; proptest samples cannot prove all bounded count combinations.
Setup command (State 8):

```bash
sh -c 'test -f crates/vb_core/src/kani.rs || test -f crates/vb_core/src/kani/mod.rs'
```

Execution command (State 11):

```bash
TMPDIR=/home/lewis/src/vb-qi37-6/.tmp RUSTC_WRAPPER= cargo kani -p vb_runtime --harness check_capability_grants_exact_match
```

## 7. Mutation Checkpoints

Threshold: global minimum 90% mutation kill rate; security-critical branches below must have 100% killed mutants.

- Mutate exact name equality to prefix/contains: killed by `capability_set_rejects_hierarchical_parent_prefix_when_required_is_child` and proptest exact-only invariant.
- Mutate action equality to ignore action id: killed by `capability_set_rejects_non_exact_name_or_action_when_required_differs`.
- Remove cardinality equality check: killed by `admit_artifact_run_rejects_excess_grants_without_allocation` and `admit_artifact_run_rejects_missing_grants_without_allocation`.
- Change canonical gate count from `15` to `2` or accept any count: killed by `runtime_rejects_gate_count_2_under_strict_journaled_without_allocation`.
- Default persisted required capabilities to empty: killed by `submit_artifact_persists_non_empty_required_capabilities_when_contract_requires_capability` and persistence proptest.
- Treat missing action contract as empty requirements: killed by `execute_do_without_contract_rejects_with_contract_required_sentinel` and `execute_do_without_contract_produces_no_awaiting_action_when_denied`.
- Emit action ticket before capability check: killed by `execute_do_with_contract_checks_all_required_capabilities_before_ticket`.
- Move journal append before admission success: killed by `denied_admission_appends_no_run_admission_and_allocates_no_run_frame`.
- Use legacy `admit_run`/`compiled_ir_exists` for protected submit: killed by static scan plus `strict_submit_uses_admit_artifact_run_before_allocation_when_workflow_is_capability_protected`.
- Omit public grant parameter or silently use `CapabilitySet::empty()`: killed by public Runtime empty/exact grant scenarios.
- Drop UI required capabilities during serialization: killed by UI parity roundtrip scenario and proptest.

Mutation command:

```bash
cargo mutants --package vb_core --package vb_runtime --package vb_storage --package vb_ui_model --timeout 120 --minimum-test-timeout 30
```

## 8. Combinatorial Coverage Matrix

| Group | Scenario | Input Class | Expected Output | Layer |
|---|---|---|---|---|
| Exact grants | exact name/action | grant = required | `true` / `Ok(())` | unit |
| Exact grants | parent prefix | `network` vs `network.github` | `false` / `CapabilityDenied` | unit + integration |
| Exact grants | child prefix | `network.github.repo` vs `network.github` | `false` / `CapabilityDenied` | unit |
| Exact grants | sibling prefix | `network.gitlab` vs `network.github` | `false` / `CapabilityDenied` | unit |
| Exact grants | partial prefix | `net` vs `network.github` | `false` / `CapabilityDenied` | unit |
| Exact grants | empty name | `""` vs `network.github` | `false` / `CapabilityDenied` | unit |
| Exact grants | action mismatch | `network.github:B` vs `network.github:A` | `false` / `CapabilityDenied` | unit |
| Gate count | canonical | `15` | admission can continue if caps exact | integration |
| Gate count | storage-current mismatch | `2` | `ArtifactInvalidGateCount { found: 2, required: 15 }`, no allocation | integration |
| Gate count | zero/absent | `0`/missing | `ArtifactInvalidGateCount` or missing-envelope denial, no allocation | integration |
| Cardinality | exact equal | required len = grant len and all exact | `RunAdmission` with exact grants | integration |
| Cardinality | missing | grant len < required len | `CapabilityDenied`, no allocation | integration |
| Cardinality | excess | grant len > required len | `CapabilityDenied`, no allocation | integration |
| Cardinality | duplicate grant | same cap repeated | `CapabilityDenied` unless canonical set semantics explicitly dedupe before exact count | integration |
| Persistence | non-empty source | validated contract has caps | decoded artifact caps equal source | integration + proptest |
| Persistence | empty source | contract has no caps | decoded artifact caps `[]` | integration |
| Do execution | contract + exact grants | contract present, grants exact | `AwaitingAction` ticket | unit/integration |
| Do execution | contract + missing grant | one required cap absent | `EngineError::CapabilityDenied`, no ticket | unit/integration |
| Do execution | no contract | contract slice absent | `EngineError::CapabilityDenied` with `__contract_required__`, no ticket | unit/integration |
| Public Runtime | empty grants + empty requirements | no required capabilities | successful run id and `RunAdmission.granted == []` | e2e-BDD |
| Public Runtime | empty grants + non-empty requirements | protected artifact | `RuntimeError::AdmissionCapabilityDenied`, no run id | e2e-BDD |
| Public Runtime | exact grants + non-empty requirements | protected artifact | successful run id and exact grant record | e2e-BDD |
| UI parity | non-empty caps | source/action/UI/storage/runtime | all exact same capability set | unit/integration + proptest |
| Kani setup | missing module | no `kani.rs`/`kani/mod.rs` | setup blocker, no PASS | static/formal |
| Kani setup | present module | module exists | State 11 `cargo kani` command eligible | static/formal |
| Fuzz setup | missing bins | missing fuzz bin names | setup blocker, no PASS | static/formal |
| Fuzz setup | present bins | both fuzz bins registered | State 11 fuzz runs eligible | static/formal |
| Release gauntlet | blockers present | Kani/fuzz setup missing | release PASS denied or deferred with explicit blocker | static/formal |
| Release gauntlet | blockers resolved | Kani/fuzz executed + static/Miri pass | scoped release evidence may PASS | static/formal |

## 9. State 8 setup checks

These checks must be planned as red/failing-first until setup is repaired. They are not State 7 execution evidence.

```bash
if test -f crates/vb_core/src/kani.rs || test -f crates/vb_core/src/kani/mod.rs; then printf 'KANI_SETUP_PRESENT\n'; else printf 'KANI_SETUP_MISSING\n'; fi
if test -f fuzz/Cargo.toml && rg -q 'name = "capability_name_schema"' fuzz/Cargo.toml && rg -q 'name = "capability_contract_schema"' fuzz/Cargo.toml; then printf 'FUZZ_BINS_PRESENT\n'; else printf 'FUZZ_BINS_MISSING\n'; fi
```

Expected failing-first output before State 8 repair: `KANI_SETUP_MISSING` and `FUZZ_BINS_MISSING`, as approved by proof review. Expected State 8 completion: `KANI_SETUP_PRESENT` and `FUZZ_BINS_PRESENT` plus committed setup artifacts. No Kani/fuzz PASS may be claimed in State 8 unless the State 11 execution commands are also run in State 11 and evidence is recorded.

## 10. State 11 execution routing

After State 8 setup succeeds, State 11 must run and record evidence for:

```bash
TMPDIR=/home/lewis/src/vb-qi37-6/.tmp RUSTC_WRAPPER= cargo kani -p vb_core --harness capability_name_grants_harness
TMPDIR=/home/lewis/src/vb-qi37-6/.tmp RUSTC_WRAPPER= cargo kani -p vb_runtime --harness check_capability_grants_exact_match
TMPDIR=/home/lewis/src/vb-qi37-6/.tmp RUSTC_WRAPPER= cargo fuzz run capability_name_schema -- -runs=1000
TMPDIR=/home/lewis/src/vb-qi37-6/.tmp RUSTC_WRAPPER= cargo fuzz run capability_contract_schema -- -runs=1000
TMPDIR=/home/lewis/src/vb-qi37-6/.tmp RUSTC_WRAPPER= verus verification/verus/capability_artifact_model.rs
TMPDIR=/home/lewis/src/vb-qi37-6/.tmp JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=/home/lewis/src/vb-qi37-6/.tmp tlc -metadir .tmp/state11/tlc-all -config verification/tla/CapabilityLifecycleAll.cfg verification/tla/CapabilityLifecycle.tla
TMPDIR=/home/lewis/src/vb-qi37-6/.tmp JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=/home/lewis/src/vb-qi37-6/.tmp tlc -metadir .tmp/state11/tlc-gate -config verification/tla/CapabilityLifecycleGateMismatch.cfg verification/tla/CapabilityLifecycle.tla
TMPDIR=/home/lewis/src/vb-qi37-6/.tmp JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=/home/lewis/src/vb-qi37-6/.tmp tlc -metadir .tmp/state11/tlc-exact -config verification/tla/CapabilityLifecycleExactProfile.cfg verification/tla/CapabilityLifecycle.tla
TMPDIR=/home/lewis/src/vb-qi37-6/.tmp JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=/home/lewis/src/vb-qi37-6/.tmp tlc -metadir .tmp/state11/tlc-excess -config verification/tla/CapabilityLifecycleExcessGrant.cfg verification/tla/CapabilityLifecycle.tla
TMPDIR=/home/lewis/src/vb-qi37-6/.tmp JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=/home/lewis/src/vb-qi37-6/.tmp tlc -metadir .tmp/state11/tlc-nocontract -config verification/tla/CapabilityLifecycleNoContract.cfg verification/tla/CapabilityLifecycle.tla
TMPDIR=/home/lewis/src/vb-qi37-6/.tmp JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=/home/lewis/src/vb-qi37-6/.tmp tlc -metadir .tmp/state11/tlc-legacy -config verification/tla/CapabilityLifecycleLegacyBypass.cfg verification/tla/CapabilityLifecycle.tla
```

State 11 must classify failures as local implementation/proof failures, not reinterpret setup absence as proof PASS.

## 11. Static, Miri, and release gates

### Static scans

```bash
rg 'admit_run\(|compiled_ir_exists\(' crates/vb_runtime/src crates/velvet_ballastics/tests
rg 'CapabilitySet::empty\(\)' crates/vb_runtime/src crates/velvet_ballastics/tests
rg 'required_capabilities' crates/vb_storage/src crates/vb_runtime/src crates/vb_ui_model/src crates/vb_validate/src
rustup run nightly-2026-04-28 cargo clippy --workspace --lib --bins --examples --all-features -- -D warnings
```

Assertions: scans must identify and justify any remaining legacy/empty-grant call sites; protected Strict/Journaled flows must be covered by denial or grant-binding tests. Clippy must pass without warnings for changed crates before release claim.

### Miri

```bash
rustup run nightly-2026-04-28 cargo miri test --quiet -p vb_core --lib --all-features capability
rustup run nightly-2026-04-28 cargo miri test --quiet -p vb_runtime --lib --all-features admission
```

Assertions: no undefined behavior diagnostics in capability/admission pure logic and no ignored Miri failures may be promoted to release PASS without waiver.

### Release/Moon or fallback

```bash
moon run :verify-proof && moon run :verify-deep && moon run :lint-src && moon run :miri && moon run :fuzz-smoke
bash scripts/rust-verification-gauntlet.sh proof
bash scripts/rust-verification-gauntlet.sh deep
```

Assertions: if Moon lanes are unavailable, fallback commands must produce equivalent scoped reports. `GAUNTLET-010` remains blocked until State 8 setup and State 11 execution complete or an approved waiver exists.

## 12. Required evidence artifacts

- `test-report.md`: exact-value/error test results for unit/integration/E2E behavior scenarios.
- `bdd-report.md`: public Runtime empty/exact grant scenarios and legacy-bypass flow evidence.
- `fuzz-report.md`: State 8 bin setup plus State 11 fuzz run outputs for both capability fuzz targets.
- `kani-report.md`: State 8 Kani setup plus State 11 harness output for exact-grant and cardinality harnesses.
- `miri-report.md`: scoped Miri output for capability/admission logic.
- `static-scan-report.md`: legacy path, empty grant, required-capability scan findings with disposition.
- `formal-verification-report.md`: State 11 TLA/Verus/Kani/fuzz/release gauntlet aggregation without laundering deferred blockers.

## Open Questions

- The exact missing-envelope error variant must be confirmed by implementation; contract taxonomy lists `ArtifactEnvelopeDecodeFailed`, while trace row names missing accepted-artifact rejection. Test-writer must assert the final exact variant, not generic `is_err()`.
- Public Runtime grant API may be introduced in a later state. If not in State 10 scope, the empty-grant denial scenario remains mandatory and exact-grant admission is blocked until an approved API path exists.
