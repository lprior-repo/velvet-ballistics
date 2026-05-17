# codebase-map.md — vb-qi37.5.4

## Bead
- **bead_id**: vb-qi37.5.4
- **title**: verifier: Idempotency gate evidence tests
- **state**: 2 (Explore and scope)
- **workspace**: /home/lewis/src/vb-qi37-5-4
- **source_checkout**: /home/lewis/src/Velvet-ballistics

---

## Scope Summary

This bead covers **evidence tests for the idempotency gate verifier**: static and runtime checks that action contracts declare compatible idempotency, side-effect, and retry-safety classifications.

---

## Touched Crates and Files

### vb_validate — Static Idempotency Contract Checks
- **`crates/vb_validate/src/idempotency_contract.rs`** (267 lines, #![forbid(unsafe_code)])
  - `IdempotencyContractError` enum (ActionContractMissing, ActionContractOrphan, IdempotencyViolations)
  - `IdempotencyContractViolation` enum (SideEffectingRetryUnsafe, SideEffectingAtLeastOnceExternal, SideEffectingDeterministicPure)
  - `validate_workflow_idempotency_contracts(parts, action_contracts)` → `IdempotencyContractResult<()>`
  - `validate_action_idempotency_contract(contract)` → `Result<(), IdempotencyContractViolation>`
  - `collect_idempotency_contract_violations(action_contracts)` → `Result<(), IdempotencyContractErrors>`
  - `is_statically_idempotent_contract(contract)` → `Result<(), IdempotencyContractViolation>`
  - Internal helpers: `ensure_contract_completeness`, `first_missing_contract`, `first_orphan_contract`, `collect_workflow_idempotency_violations`, `do_action`, `has_contract`, `find_contract`, `has_do_action`

### vb_core — Action ABI Types and Runtime Verification
- **`crates/vb_core/src/action.rs`** (2165 lines, #![forbid(unsafe_code)])
  - `enum Idempotency` — DeterministicPure(0), IdempotentExternal(1), AtLeastOnceExternal(2)
  - `enum SideEffect` — None(0), Writes(1), Sends(2), Creates(3), Destroys(4)
  - `enum RetrySafety` — Safe(0), KeyRequired(1), Unsafe(2)
  - `enum IdempotencyViolation` — MissingKey, SecretInKey(u32), RandomInKey(u32), TimeInKey(u32)
  - `struct ActionContract` — full contract with idempotency, side_effect, retry_safety fields
  - `validate_idempotency_key_ingredients(key_slots, frame)` → `Result<(), IdempotencyViolation>` (line 317)
  - `verify_idempotency(contract, key_slots, frame)` → `Result<(), IdempotencyViolation>` (line 355)
  - 20+ unit tests for verify_idempotency (lines 1061–1498)

### vb_compile — Compile-Time Gate
- **`crates/vb_compile/src/lib.rs`**
  - `check_idempotency_gates(contracts)` (line 754) — compile-time enforcement of idempotency rules
  - Called at line 223 during compilation

### vb_validate tests — Idempotency Contract Tests
- **`crates/vb_validate/tests/idempotency_contract_red.rs`** (837 lines)
  - 28+ unit tests covering all decision-table combinations
  - 2 proptest harnesses covering action IDs 0..128
  - Tests: validate_action_idempotency_contract (5 variants), collect_idempotency_contract_violations (6 variants), is_statically_idempotent_contract (11 variants), runtime verify_idempotency (4 variants), validate_idempotency_key_ingredients (2 variants), validate_workflow_idempotency_contracts (4 variants)

### velvet_ballastics — CLI and Action Table
- **`crates/velvet_ballastics/src/main.rs`**
  - `action_idempotency_name(value)` → maps Idempotency to string (line 638)
  - `action_idempotency_rule(idempotency, retry_safety)` → idempotency rule description (line 678)
  - Action table output with idempotency column (lines 363, 434, 476, 540–545)

### lifecycle integration tests — retry/replay
- **`crates/velvet_ballastics/tests/lifecycle_integration.rs`**
  - `replay(&journal)` calls (lines 116, 160, 203, 246, 290)
  - `retry(run, &journal)` tests for all state transitions

---

## Public APIs in Scope

### vb_validate::idempotency_contract
```
pub type IdempotencyContractResult<T> = Result<T, IdempotencyContractError>;
pub struct IdempotencyContractErrors(pub Box<[IdempotencyContractViolation]>);
pub enum IdempotencyContractError { ActionContractMissing{action_id,node_index}, ActionContractOrphan{action_id}, IdempotencyViolations(IdempotencyContractErrors) }
pub enum IdempotencyContractViolation { SideEffectingRetryUnsafe{action,side_effect,idempotency,retry_safety}, SideEffectingAtLeastOnceExternal{...}, SideEffectingDeterministicPure{...} }
pub fn validate_workflow_idempotency_contracts(parts: &WorkflowParts, action_contracts: &[ActionContract]) -> IdempotencyContractResult<()>
pub fn validate_action_idempotency_contract(contract: &ActionContract) -> Result<(), IdempotencyContractViolation>
pub fn collect_idempotency_contract_violations(action_contracts: &[ActionContract]) -> Result<(), IdempotencyContractErrors>
pub fn is_statically_idempotent_contract(contract: &ActionContract) -> Result<(), IdempotencyContractViolation>
```

### vb_core::action
```
pub enum Idempotency { DeterministicPure, IdempotentExternal, AtLeastOnceExternal }
pub enum SideEffect { None, Writes, Sends, Creates, Destroys }
pub enum RetrySafety { Safe, KeyRequired, Unsafe }
pub enum IdempotencyViolation { MissingKey(SideEffect), SecretInKey(u32), RandomInKey(u32), TimeInKey(u32) }
pub struct ActionContract { id, input_slot_count, output_slot_count, max_input_bytes, max_output_bytes, timeout_ms, idempotency, side_effect, retry_safety, required_capabilities }
pub fn validate_idempotency_key_ingredients(key_slots: &[SlotIdx], frame: &RunFrame) -> Result<(), IdempotencyViolation>
pub fn verify_idempotency(contract: &ActionContract, key_slots: &[SlotIdx], frame: &RunFrame) -> Result<(), IdempotencyViolation>
```

---

## Decision Table (Static Gate)

| side_effect | retry_safety | idempotency | result |
|---|---|---|---|
| None | any | any | Ok |
| any | Unsafe | any (if side_effect != None) | Err(SideEffectingRetryUnsafe) |
| any (if side_effect != None) | any | AtLeastOnceExternal | Err(SideEffectingAtLeastOnceExternal) |
| any (if side_effect != None) | any | DeterministicPure | Err(SideEffectingDeterministicPure) |
| any (if side_effect != None) | Safe\|KeyRequired | IdempotentExternal | Ok |

---

## Changed Dependencies

No dependency changes anticipated for this bead. This bead is test/infrastructure (evidence tests for existing verifier logic). No Cargo.toml, Cargo.lock, feature flags, or dependency policy changes.

---

## Contract Clauses for Idempotency Gate

1. **Side-effecting actions must not be retry-unsafe**: Any action with `side_effect != None` and `retry_safety == Unsafe` is rejected.
2. **Side-effecting actions must not be AtLeastOnceExternal**: Any action with `side_effect != None` and `idempotency == AtLeastOnceExternal` is rejected.
3. **Side-effecting actions must not be DeterministicPure**: Any action with `side_effect != None` and `idempotency == DeterministicPure` is rejected.
4. **KeyRequired actions must provide a non-empty, non-tainted key**: `verify_idempotency` enforces key presence and taint checks at runtime.
5. **No duplicate non-idempotent entries in replay**: `reference/src/replay_model.rs` `check_no_duplicate_non_idempotent`.

---

## Risk Tags

- **temporal**: replay journal ordering, retry ordering
- **concurrency**: journal replay concurrent access to tracker state
- **persistence**: journal replay across process restarts
- **public_api**: ActionContract, Idempotency, verify_idempotency are all public APIs
- **contract_parity**: decision table must match between compile-time gate (vb_compile) and validation module (vb_validate)
- **verification_gap**: No Kani proofs currently cover idempotency gate logic; existing kani/ uses idempotency only as test data

---

## Required Verifier Modes

| Mode | Required | Rationale |
|---|---|---|
| cargo kani | YES | Bounded model checking for idempotency decision table exhaustiveness; existing kani/ has no idempotency coverage |
| cargo proof (Verus) | YES | verify_idempotency runtime logic, key taint propagation, slot validation |
| cargo test | YES | Unit test coverage already exists (837-line test file); evidence run required |
| Miri | YES | detect UB in slot/index arithmetic for key slot validation |
| loom | NO | journal replay is single-threaded sequential; not a concurrency concern |

---

## Build Status

- **vb_validate**: compiles (targeted by this bead)
- **vb_core**: compiles (provides types)
- **vb_compile**: compiles (contains check_idempotency_gates)
- **vb_runtime**: FAILS — missing `crates/vb_runtime/src/runtime/chunk_001.rs` — **DEFERRED_GLOBAL**, outside this bead's scope

---

## Verification Artifacts

| Artifact | Location | Status |
|---|---|---|
| Kani proofs | `kani/` | NO coverage for idempotency gate (only uses idempotency as test data) |
| Verus proofs | `verification/verus/` | lemma_join_idempotent (taint lattice), lemma_terminal_idempotency (state machine) — not gate-specific |
| Unit tests | `crates/vb_validate/tests/idempotency_contract_red.rs` | EXISTS — 28+ tests |
| Integration tests | `crates/velvet_ballastics/tests/lifecycle_integration.rs` | EXISTS — retry/replay coverage |
| Fuzz targets | `fuzz/src/bin/replay_events.rs` | EXISTS |

---

## Open Questions / DISCOVERY_BLOCKED

None. The scope is well-defined from existing source.

---

## Recommended Downstream Owners

- **contract**: rust-contract for ActionContract invariants and IdempotencyViolation error lattice
- **proof**: kani for decision-table exhaustiveness; proof-writer for idempotency gate harness
- **test**: test-planner for gap analysis against existing 28-test suite
- **implementation**: holzman-rust if any gate logic changes are required
