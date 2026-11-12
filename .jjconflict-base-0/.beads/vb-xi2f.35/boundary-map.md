# Boundary Map: ResourceContract Digest Coverage

## Bead

`vb-xi2f.35` — P1: digest covers resource contract semantics

## Architecture Boundary Principle

Per hexagonal architecture and Holzman Rust: the pure functional core computes digests, validates contracts, and enforces invariants. All I/O (YAML parsing, postcard serialization, journal writes) happens at the shell boundary. The ResourceContract digest gap is a core violation: a pure computation (hash) is missing a critical input.

## Boundaries

```
┌─────────────────────────────────────────────────────────────────────┐
│ IMPERATIVE SHELL (impure, I/O, parsing, serialization)             │
│                                                                     │
│  vb_yaml::ast::parse                  YAML bytes → WorkflowSource  │
│  vb_compile::mod_compile_core         emit_compiled_artifact       │
│  vb_storage::admission                submit_artifact, journal I/O │
│  vb_compile::compile_source           6 entry point (orchestrator) │
│                                                                     │
├─────────────────────────────────────────────────────────────────────┤
│ PURE FUNCTIONAL CORE (no I/O, deterministic)                       │
│                                                                     │
│ ★ vb_compile::canonical_digest()      source + contract → digest   │
│   [CURRENT BUG: contract not hashed]                               │
│                                                                     │
│  vb_core::workflow::ResourceContract   value object (17 fields)    │
│  vb_core::compiled_workflow::ResourceContract  DUPLICATE (15 fld)  │
│  vb_core::ids::WorkflowDigest          digest wrapper              │
│  vb_core::validation::resource         contract validation         │
│  vb_core::budget::CapBudget            transition budget           │
│  vb_core::budget::validate_budget_limit max_transitions check      │
│                                                                     │
├─────────────────────────────────────────────────────────────────────┤
│ RUNTIME SHELL (stateful, I/O, async)                               │
│                                                                     │
│  vb_runtime::shard::lifecycle::chunk_002  handle_ask_answer        │
│     → checks allows_secret_results                                 │
│     → checks max_ipc_payload_bytes                                 │
│  vb_runtime::admission                   runtime admission gates   │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

## Boundary Details

### Boundary 1: YAML Parser → Pure Core

**File**: `crates/vb_yaml/src/ast/parse.rs`

**Current flow**:
```
Raw bytes → saphyr::Yaml → parse_workflow_from_yaml() → WorkflowSource
```

**Contract boundary**: The parser is the trust boundary. It must:
- Reject unknown top-level fields (whitelist)
- Validate field shapes
- **NEW**: Parse optional `resource_contract` section if present
- **NEW**: Validate all contract fields against their types (u16, u8, bool, etc.)

**Illegal states at this boundary**:
- String where u16 is expected in contract fields → `YamlError::InvalidResourceContract`
- Unknown fields inside the contract section → rejected by whitelist
- Missing `resource_contract` → return `None`, caller uses DEFAULT

### Boundary 2: Compilation Entry Points → Pure Core

**Files**: `part_01.rs`, `part_05.rs`, `part_08.rs`, `compile/mod.rs`

**Current flow**:
```
WorkflowSource → compile_source() → [canonical_digest(source)] → [hardcoded DEFAULT] → WorkflowParts
```

**Required flow**:
```
WorkflowSource + ResourceContract → compile_source() → canonical_digest(source, contract) → WorkflowParts
```

**Contract**: The compilation entry point is the **orchestrator** that wires together:
1. Source parsing (shell)
2. Contract resolution (from YAML/API/DEFAULT)
3. Digest computation (pure)
4. Part construction (pure)
5. Validation (pure)
6. Artifact emission (shell)

### Boundary 3: Canonical Digest → All Consumers

**Files**: `part_05.rs:116`, `compile/mod.rs:220`

**Contract**: `canonical_digest()` is pure. It produces a deterministic hash from:
- Version string
- Name string
- Trigger AST
- Step IDs and primitives
- **NEW: All 17 ResourceContract fields**

**Invariant at this boundary**: The digest MUST be a function of ALL semantic inputs. Currently it is a partial function — missing the contract.

### Boundary 4: Validation → Resource Contract Fields

**Files**: `crates/vb_core/src/validation/resource.rs`

**Current flow**:
```
WorkflowParts → validate_resource_contract() → checks 6 contract fields (via 15-field type)
```

**Required flow**:
```
WorkflowParts → validate_resource_contract() → checks ALL 17 contract fields (via canonical type)
```

**Contract**: Validation must vet ALL contract fields, not just the 6 currently checked. Missing validation on `max_transitions_per_tick` and `allows_secret_results` creates a gap.

### Boundary 5: Runtime → Contract Enforcement

**File**: `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs`

**Flow**:
```
AskAnswer → handle_ask_answer() → contract.allows_secret_results check → drive_run()
```

**Contract**: The runtime enforces resource contract limits at the I/O boundary:
- `allows_secret_results`: gates secret-tainted answers
- `max_ipc_payload_bytes`: gates answer payload size
- `max_step_budget_per_tick` / `max_transitions_per_tick`: gates per-tick budget

These are the **actual behavior-affecting uses** of the contract. The digest must be sensitive to all fields that affect behavior at this boundary.

### Boundary 6: Admission → Policy Digest

**File**: `crates/vb_storage/src/admission.rs`

**Flow**:
```
CompiledWorkflow → compute_policy_digest() → postcard serialize contract → blake3 → WorkflowDigest
```

**Contract**: `compute_policy_digest()` computes a digest of the contract alone. This is a **separate** operation from the canonical digest. Both must agree on the contract identity, but they serve different purposes:
- `policy_digest`: "which contract governs this admitted artifact?"
- `canonical_digest`: "what is the complete semantic identity of this workflow?"

## Duplicate Type Boundary

**Critical boundary issue**: `compiled_workflow.rs` and `workflow/mod.rs` both define `ResourceContract`, `WorkflowParts`, `CompiledWorkflow`. This creates an **invisible boundary** where:
- `validation/resource.rs` imports from `compiled_workflow` (15-field contract)
- `vb_compile` imports from `workflow` via `vb_core::` re-exports (17-field contract)
- These are different types that share the same name

**Resolution**: This boundary must be eliminated. Exactly one definition must exist.

## Pure Core / Impure Shell Split

| Component | Pure/Impure | Reason |
|-----------|-------------|--------|
| `ResourceContract` | Pure | Value object, no I/O |
| `WorkflowDigest` | Pure | Value object, no I/O |
| `canonical_digest()` | Pure | Deterministic hash, no I/O |
| `validate_resource_contract()` | Pure | Checks limits against data, no I/O |
| `parse_workflow_from_yaml()` | Impure | Parses external bytes |
| `compile_source()` | Impure (orchestrator) | Wires pure + impure components |
| `compute_policy_digest()` | Pure | Deterministic hash of contract bytes |
| `compute_compiled_digest()` | Pure | Deterministic hash of raw bytes |
| `emit_compiled_artifact()` | Pure | postcard serialization (pure) |
| `submit_artifact()` | Impure | Journal I/O |
| `handle_ask_answer()` | Impure | State mutation + journal I/O |

## Unsafe Code Boundaries

**No unsafe code exists in the digest, contract, or validation paths.** All affected code is safe Rust. The `#![forbid(unsafe_code)]` attribute on all relevant modules confirms this.

## Storage Boundaries

| Storage | Purpose | Contract Sensitivity |
|---------|---------|---------------------|
| `compiled_ir` keyspace (vb_storage) | Persists `AcceptedArtifact` | Contains `policy_digest` (contract hash) and `digest` (workflow digest) |
| Journal (vb_storage) | Event log | Records runtime events |
| In-memory `CompiledWorkflow` | Hot runtime reference | Contains `resource_contract` (15-field type currently) |

## Network/API Boundaries

| Boundary | Contract Relevance |
|----------|-------------------|
| IPC answer handling | Checks `allows_secret_results` and `max_ipc_payload_bytes` |
| Artifact submission API | Accepts `CompiledWorkflow` with embedded contract |
| Admission verification | Checks `policy_digest` matches expected |

## Time Boundaries

No time-dependent operations exist in the pure core. The `handle_ask_answer` function at the runtime boundary is time-aware (timers) but contract checks are deterministic and time-independent.
