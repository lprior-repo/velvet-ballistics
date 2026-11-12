# Contract Specification — vb-hs9m

## Context

- **Bead:** vb-hs9m
- **Focus:** Observability, trace ring, evidence collection, artifact packaging, audit trails, BDD execution evidence
- **Source Checkout:** `/home/lewis/src/velvet-ballistics`
- **Isolated Workspace:** `/home/lewis/src/vb-hs9m-workspace`

---

## Domain Terms

| Term | Definition |
|------|------------|
| `TraceRing` | Bounded SPSC ring buffer holding `TraceEvent` values; backed by `rtrb` crate |
| `TraceEvent` | Enum with 11 variants capturing runtime lifecycle events (StepStarted, StepEnded, SlotWritten, ActionScheduled, ActionCompleted, ActionFailed, AskAnswered, RunSubmitted, RunFinished, RunFailed, RunCancelled) |
| `EvidenceBundle` | Top-level evidence container linking gate execution records to a bead ID |
| `GateEvidence` | Individual gate execution record: kind, gate_name, command, exit_code, log, status, why_failed |
| `Scenario` | BDD scenario struct with Given/When/Then/PublicSurface/Fixture/ExpectedOutcome fields |
| `catalog()` | Returns static slice of all `Scenario` definitions |
| `validate_catalog()` | Validates scenario catalog for required fields, uniqueness, and consistency |
| `RunId` | Identifier for a workflow run |
| `SPSC` | Single-producer single-consumer lock-free ring buffer pattern |

---

## Assumptions

- `TraceRing` is used in production hot path; overflow behavior (drop count) is observable
- `EvidenceBundle` is the canonical evidence artifact for release gates and AI audits
- `catalog()` returns a static slice populated at compile time; no dynamic loading
- Evidence files are written atomically (no partial writes visible to downstream consumers)
- Secret redaction in release artifacts is checked against a static denylist at validation time
- The SPSC ring buffer is the only concurrent data structure in this bead scope (no other concurrency primitives)

---

## Preconditions

- **PRE-001:** `TraceRing::new(capacity)` requires `capacity > 0`
- **PRE-002:** `TraceRing::push(event)` requires the ring to be in a valid state (not already poisoned)
- **PRE-003:** `EvidenceBundle::validate_bundle(&bundle)` requires `bundle` to be constructed via the public API (not a `#[repr(C)]` or raw bytes overlay)
- **PRE-004:** `parse_bundle_schema_version(input)` accepts any `&str`; caller handles `Err` result

---

## Postconditions

- **POST-001:** `TraceRing::new(capacity)` returns a ring with `capacity == capacity`, `len() == 0`, `dropped == 0`, and both producer/consumer ends valid
- **POST-002:** `TraceRing::push(event)` returns `true` iff the event was successfully enqueued; when `false`, `dropped` is incremented by 1 (saturated)
- **POST-003:** `TraceRing::drain()` returns all events currently in the ring and leaves the ring empty
- **POST-004:** `TraceRing::drain_for_run(run_id, limit)` returns only events belonging to `run_id`, up to `limit`, preserving insertion order
- **POST-005:** `TraceRing::has_terminal_event_for_run(run_id)` returns `true` iff at least one terminal event (RunFinished, RunFailed, RunCancelled) exists for the given `run_id`
- **POST-006:** `EvidenceBundle::validate_bundle(&bundle)` returns an empty `Vec` iff all required fields are non-empty; each missing field produces exactly one `MissingRequiredField` error
- **POST-007:** `parse_bundle_schema_version(input)` returns `Ok(v)` iff input matches `^(0|[1-9][0-9])\.(0|[1-9][0-9])$`; all other inputs return `Err(SchemaVersionParseFailed)`
- **POST-008:** `EvidenceBundle` round-trip (write + read) in Yaml/Json/Postcard format produces a bundle equal to the original
- **POST-009:** `catalog()` returns a non-empty slice with unique scenario IDs
- **POST-010:** `validate_catalog(scenarios)` returns `Ok(())` iff every scenario has non-empty `id`, `master_behavior`, `given`, `when`, `then`, and unique IDs

---

## Invariants

- **INV-001:** `TraceRing` invariants:
  - `capacity > 0` always holds after construction
  - `dropped` is monotonically non-decreasing across all `push` calls
  - `len() <= capacity` always holds
  - Events in the ring are stored in insertion order (FIFO)
  - `has_terminal_event_for_run(run_id)` is stable: once `true`, subsequent calls for the same `run_id` remain `true` unless the ring is drained
- **INV-002:** `EvidenceBundle` invariants:
  - `schema_version` is non-empty and parses to `major.minor` format
  - `linked_bead_id` is non-empty
  - `executor_context.agent` is non-empty
  - `gates` contains at least one entry per required gate name
- **INV-003:** `Scenario` catalog invariants:
  - All scenario IDs are unique
  - No scenario has empty `given`, `when`, or `then` fields
  - Each scenario's `executable_evidence_target` path, if present, points to a resolvable artifact
- **INV-004:** Evidence path invariants:
  - `evidence_path(bead_id, gate_name)` returns `.evidence/<bead_id>/<gate_name>.yaml`
  - `bundle_path(bead_id, format)` returns `.evidence/<bead_id>/bundle.<ext>`

---

## Error Taxonomy

### TraceRing Errors
- `TraceError::RingFull` — returned as `false` from `push`; observable via drop count
- No panic variants for `push`, `drain`, `drain_into`, `drain_for_run`, `snapshot_for_run`

### EvidenceBundle Errors
- `Error::SchemaVersionParseFailed` — input does not match `major.minor` format
- `Error::MissingRequiredField { field }` — required field is empty
- `Error::GateTimeout` — gate execution exceeded timeout
- `Error::GateFailed { exit_code }` — gate exited with non-zero
- `Error::MissingEvidence` — evidence file absent at validation time
- `Error::EvidenceWriteFailed` — file I/O error during evidence write
- `Error::SubcommandNotFound` — gate command not found in PATH
- `Error::BeadDirectoryCreationFailed` — cannot create `.evidence/<bead_id>/`
- `Error::YamlSerializationFailed` — serde_yaml error during bundle write
- `Error::BundleSerializationFailed` — general serialization error
- `Error::UpstreamMoonFailed` — moon build system failure
- `Error::UpstreamJustFailed` — just build system failure

### Catalog Validation Errors
- `CatalogValidationError::EmptyCatalog` — catalog returns empty slice
- `CatalogValidationError::MissingGivenWhenThen { scenario_id }` — required BDD fields absent
- `CatalogValidationError::MissingExactAssertion { scenario_id }` — no `expected_outcome` or `expected_error`
- `CatalogValidationError::MissingEvidenceDisposition { scenario_id }` — no `executable_evidence_target`
- `CatalogValidationError::ConflictingEvidenceDisposition { scenario_id }` — contradictory evidence fields
- `CatalogValidationError::DuplicateScenarioId { scenario_id }` — ID appears more than once

---

## TLA+-Owned Clauses

**Non-applicability rationale:** The trace ring (`TraceRing`) is a pure local data structure with no temporal/protocol/workflow behavior. Push returns a boolean (full/not-full), drain removes events — there are no liveness properties, fairness requirements, concurrent writer conflicts (SPSC guarantee), deadlock possibilities, or state-machine transitions. Evidence bundles are static containers with no asynchronous state transitions. The BDD catalog is a static compile-time data structure validated synchronously. There are no schedulers, queues, retry logic, claim/lease protocols, lifecycle state machines, distributed coordination, or inter-agent orchestration in this bead scope.

TLA+ is explicitly waived for all vb-hs9m contract clauses. The bounded ring buffer capacity and drop-count are purely local invariants provable by unit tests and Kani.

---

## Verus-Owned Clauses

The following pure Rust-local invariants are owned by Verus (or Kani as complementary implementation evidence):

- **INV-001 (TraceRing boundedness):** `len() <= capacity` and `dropped` monotonicity — provable by Verus spec functions and also checked by `cargo test` with adversarial BDD-style tests
- **INV-002 (EvidenceBundle required fields):** non-empty validation is a pure function `validate_bundle(&EvidenceBundle) -> Vec<Error>` — checkable by unit tests and Kani
- **INV-003 (Scenario uniqueness):** `validate_catalog` is a pure function over a static slice — checkable by unit tests

See `verification-layers.md` for the full verification assignment.

---

## Theorem-Owned Clauses

No Lean/Aeneas/Hax theorem kernels are required. The algebraic properties of the bounded ring buffer (monotonic drop count, FIFO ordering, capacity bound) are expressible as unit-test properties and Kani harnesses without a proof assistant.

---

## Non-goals

- No TLA+ model for the trace ring or evidence bundle (explicit waiver above)
- No Lean/Aeneas/Hax theorem kernels for this bead
- No performance benchmarking for the trace ring (not a hot-path claim in this bead scope)
- No formal proof of the BDD runner execution timeline — covered by integration tests
- No coverage of the UI trace display (`execution_details.rs`) — covered by UI integration tests
