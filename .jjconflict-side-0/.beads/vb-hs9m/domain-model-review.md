# Domain Model Review — vb-hs9m

## Reviewed Modules and Types

### 1. TraceRing (`crates/vb_runtime/src/trace.rs`)

**Type Shape:**
```rust
pub struct TraceRing {
    producer: rtrb::Producer<TraceEvent>,
    consumer: rtrb::Consumer<TraceEvent>,
    capacity: usize,
    dropped: u64,
    history: VecDeque<TraceEvent>,
}
```

**TraceEvent Enum (11 variants):**
```rust
pub enum TraceEvent {
    StepStarted { run_id: RunId, step_idx: StepIdx, slot_idx: SlotIdx },
    StepEnded { run_id: RunId, step_idx: StepIdx },
    SlotWritten { run_id: RunId, slot_idx: SlotIdx },
    ActionScheduled { run_id: RunId },
    ActionCompleted { run_id: RunId },
    ActionFailed { run_id: RunId, code: ActionFailureCode },
    AskAnswered { run_id: RunId },
    RunSubmitted { run_id: RunId },
    RunFinished { run_id: RunId },
    RunFailed { run_id: RunId },
    RunCancelled { run_id: RunId },
}
```

**Observability API Surface:**
| Method | Signature | Behavior |
|--------|-----------|----------|
| `new` | `pub fn new(capacity: usize) -> Self` | Constructs ring; `capacity > 0` precondition |
| `push` | `pub fn push(&mut self, event: TraceEvent) -> bool` | `true` if enqueued, `false` if full; increments `dropped` on full |
| `drain` | `pub fn drain(&mut self) -> Vec<TraceEvent>` | Returns all events, clears ring |
| `drain_into` | `pub fn drain_into(&mut self, limit: usize, events: &mut Vec<TraceEvent>)` | Drains up to `limit` events |
| `drain_for_run` | `pub fn drain_for_run(&mut self, run_id: RunId, limit: usize) -> Vec<TraceEvent>` | Filters by run_id |
| `snapshot_for_run` | `pub fn snapshot_for_run(&mut self, run_id: RunId, limit: usize) -> Vec<TraceEvent>` | Non-destructive drain for run |
| `has_terminal_event_for_run` | `pub fn has_terminal_event_for_run(&mut self, run_id: RunId) -> bool` | Checks for RunFinished/RunFailed/RunCancelled |
| `capacity` | `pub const fn capacity(&self) -> usize` | Returns ring capacity |
| `len` | `pub fn len(&self) -> usize` | Current event count |
| `is_empty` | `pub fn is_empty(&self) -> bool` | `true` if no events |

**Domain Invariants:**
1. `capacity > 0` always holds (enforced by `new` returning valid ring or panicking on `RingBuffer::new(0)` — rtrb panics on 0 capacity)
2. `len() <= capacity` always holds
3. `dropped` is monotonically non-decreasing
4. Events are stored in insertion order (FIFO)
5. `has_terminal_event_for_run` is stable once `true` for a given `run_id` (unless drained)

**Concurrency Model:** SPSC via `rtrb`. Single producer (write side), single consumer (read side). `push` is only called from the producer side; drain methods are only called from the consumer side. This is enforced by type design, not by runtime checks.

**Reproduction Note:** The `rtrb::Producer` and `rtrb::Consumer` are `!Send` and `!Sync` by design, preventing cross-thread misuse at the type level.

---

### 2. EvidenceBundle (`xtask/src/evidence/bundle.rs`)

**Core Types:**
```rust
pub struct EvidenceBundle {
    pub schema_version: String,
    pub executor_context: ExecutorContext,
    pub linked_bead_id: String,
    pub gates: Vec<GateEvidence>,
    pub source_test_mappings: Vec<SourceTestMapping>,
    pub release_artifacts: Vec<ReleaseGateArtifact>,
}

pub struct ExecutorContext {
    pub agent: String,
    pub timestamp: String,
    pub machine: String,
}

pub struct GateEvidence {
    pub kind: String,
    pub gate_name: String,
    pub command: String,
    pub exit_code: Option<i32>,
    pub log: Option<String>,
    pub status: GateStatus,
    pub why_failed: Option<WhyFailed>,
}

pub enum GateStatus {
    Pass,
    Fail,
    Skipped { reason: String },
}

pub enum EvidenceBundleFormat {
    Yaml,
    Json,
    Postcard,
}

pub enum ArtifactType {
    Benchmark,
    Coverage,
    Mutation,
    SupplyChain,
    Miri,
    Clippy,
    Fmt,
}
```

**Serialization API:**
| Function | Signature | Behavior |
|----------|-----------|----------|
| `parse_bundle_schema_version` | `pub fn parse_bundle_schema_version(&str) -> Result<String, Error>` | Validates `^(0\|[1-9][0-9])\.(0\|[1-9][0-9])$` |
| `validate_bundle` | `pub fn validate_bundle(&EvidenceBundle) -> Vec<Error>` | Returns empty vec iff all required fields non-empty |
| `write_bundle` | `pub fn write_bundle(&EvidenceBundle, &Path, EvidenceBundleFormat) -> Result<()>` | Serializes to YAML/JSON/postcard |
| `read_bundle` | `pub fn read_bundle(&Path, EvidenceBundleFormat) -> Result<EvidenceBundle>` | Deserializes from file |
| `bundle_path` | `pub fn bundle_path(bead_id: &str, format: EvidenceBundleFormat) -> PathBuf` | Returns `.evidence/<bead_id>/bundle.<ext>` |

**Domain Invariants:**
1. `schema_version` is non-empty and matches `major.minor` format
2. `linked_bead_id` is non-empty
3. `executor_context.agent` is non-empty
4. Each gate in `gates` has a non-empty `gate_name`
5. `ArtifactType` enum covers all artifact kinds emitted by gates

**Validation Rules:**
- Empty `schema_version` → `MissingRequiredField { field: "schema_version" }`
- Empty `linked_bead_id` → `MissingRequiredField { field: "linked_bead_id" }`
- Empty `executor_context.agent` → `MissingRequiredField { field: "executor_context.agent" }`
- Parse failure → `SchemaVersionParseFailed`

---

### 3. Scenario / Catalog (`crates/workspace_tests/src/acceptance_catalog.rs`)

**Type Shape:**
```rust
pub struct Scenario {
    pub id: &'static str,
    pub master_behavior: &'static str,
    pub given: &'static str,
    pub when: &'static str,
    pub then: &'static str,
    pub public_surface: &'static str,
    pub fixture: &'static str,
    pub expected_outcome: Option<&'static str>,
    pub expected_error: Option<&'static str>,
    pub durability_profile: &'static str,
    pub related_bead: &'static str,
    pub executable_evidence_target: Option<&'static str>,
    pub deferred_follow_up_bead: Option<&'static str>,
}
```

**Catalog API:**
| Function | Signature | Behavior |
|----------|-----------|----------|
| `catalog` | `pub fn catalog() -> &'static [Scenario]` | Returns static compile-time scenario slice |
| `validate_catalog` | `pub fn validate_catalog(scenarios: &[Scenario]) -> Result<(), CatalogValidationError>` | Validates all scenarios |

**CatalogValidationError Variants (10 total):**
- `EmptyCatalog` — catalog is empty
- `MissingGivenWhenThen { scenario_id }` — `given`, `when`, or `then` is empty
- `MissingExactAssertion { scenario_id }` — neither `expected_outcome` nor `expected_error` set
- `MissingEvidenceDisposition { scenario_id }` — no `executable_evidence_target`
- `ConflictingEvidenceDisposition { scenario_id }` — contradictory evidence fields
- `InvalidExecutableEvidenceTarget { scenario_id }` — path format invalid
- `InvalidDeferredFollowUpBead { scenario_id }` — bead ID format invalid
- `PrivateSurface { scenario_id }` — surface uses private/internal API
- `SharedFixture { scenario_id }` — fixture shared across incompatible scenarios
- `DuplicateScenarioId { scenario_id }` — ID appears more than once

**Catalog Invariants:**
1. All scenario IDs are unique (enforced by `DuplicateScenarioId` error)
2. All `given`, `when`, `then` fields are non-empty (enforced by `MissingGivenWhenThen`)
3. Either `expected_outcome` or `expected_error` is set (enforced by `MissingExactAssertion`)
4. Catalog is non-empty (enforced by `EmptyCatalog`)

---

## Verification Coverage Map

| Type | Invariant | Verus | Kani | Unit Test | Integration Test |
|------|-----------|-------|------|-----------|-----------------|
| TraceRing | `len() <= capacity` | — | OBL-009 | TraceRing BDD tests (1077 lines) | — |
| TraceRing | `dropped` monotonic | — | OBL-009 | TraceRing BDD tests | — |
| TraceRing | FIFO ordering | — | OBL-010 | TraceRing BDD tests | — |
| TraceRing | `has_terminal_event_for_run` stability | — | OBL-011 | TraceRing BDD tests | — |
| EvidenceBundle | parse no panic | — | OBL-001 | — | — |
| EvidenceBundle | validate correctness | — | OBL-002 | — | — |
| EvidenceBundle | write/read no panic | — | OBL-003 | — | — |
| EvidenceBundle | round-trip YAML | — | — | OBL-005 | — |
| EvidenceBundle | round-trip JSON | — | — | OBL-006 | — |
| EvidenceBundle | round-trip postcard | — | — | OBL-007 | — |
| EvidenceBundle | Miri clean | — | OBL-008 | — | — |
| Scenario catalog | validate_catalog | — | — | vb_hxm0 acceptance tests | BDD catalog validation |

---

## Existing Verification Artifacts (Read-Only Reference)

These files exist in the workspace and were written by previous agents. They are **not** part of this bead's contract scope and must not be modified:

- `xtask/tests/bundle_tests.rs` — Kani harnesses OBL-001 through OBL-008, proptest properties OBL-005 through OBL-007
- `verification/verus/run_frame_invariant.rs` — RunFrame invariants
- `verification/verus/signals_invariant.rs` — Signals invariants
- `crates/workspace_tests/tests/vb_hxm0_acceptance_catalog.rs` — Catalog validation tests

---

## Domain Model Adequacy

**TraceRing:** Adequately models the bounded SPSC ring buffer. The `history: VecDeque` is an implementation detail for snapshot/drain operations — the SPSC guarantee is enforced by `rtrb` Producer/Consumer types being `!Send + !Sync`. The `dropped` counter provides observable evidence of overflow events.

**EvidenceBundle:** Adequately models the evidence container as a plain Rust struct with serde serialization. The validation function is pure and total over the type. Postcard format is included for compact binary evidence storage.

**Scenario/Catalog:** Adequately models the BDD scenario as a flat struct with all required Gherkin-style fields. Compile-time static slice ensures catalog is always populated. Validation is exhaustive over all 10 error variants.

**No temporal behavior** is modeled because none exists in this bead's scope. All state transitions are local and synchronous.
