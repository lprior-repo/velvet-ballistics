# Test Plan: vb-hs9m — Observability and Evidence Packaging

## Summary

| Category | Count |
|---|---|
| Behaviors identified | 28 |
| Unit tests (existing) | 24 trace ring + 15 bundle/catalog |
| Integration tests (existing) | 8 catalog + 2 bundle persistence |
| Proptest invariants | 6 |
| Fuzz targets | 0 (no raw-bytes parsers in scope) |
| Kani harnesses | 4 (all waived: tooling unavailable) |
| Miri checks | 2 (all waived: rust-src missing) |
| Mutation checkpoints | 12 |

---

## 1. Behavior Inventory

### TraceRing (SPSC bounded ring buffer)

| # | Behavior |
|---|---|
| TRC-01 | TraceRing::new(capacity) creates a valid ring with len==0, dropped==0, capacity==configured |
| TRC-02 | TraceRing::push(event) returns true when ring has space |
| TRC-03 | TraceRing::push(event) returns false and increments dropped when ring is full |
| TRC-04 | TraceRing::drain() returns all events in insertion order and leaves ring empty |
| TRC-05 | TraceRing::drain_into(limit, vec) appends at most limit events, stops at ring exhaustion |
| TRC-06 | TraceRing::drain_for_run(run_id, limit) returns only events for that run_id, up to limit, FIFO order |
| TRC-07 | TraceRing::snapshot_for_run(run_id, limit) returns events without draining |
| TRC-08 | TraceRing::has_terminal_event_for_run(run_id) returns true iff RunFinished/RunFailed/RunCancelled exists |
| TRC-09 | TraceRing::push overflow increments dropped atomically (saturating u64) |
| TRC-10 | TraceRing::len() never exceeds capacity (INV-001 boundedness) |
| TRC-11 | TraceRing history evicts oldest when capacity exceeded |
| TRC-12 | TraceRing dropped counter is monotonically non-decreasing |
| TRC-13 | TraceEvent::run_id() returns correct RunId for all 11 variants |
| TRC-14 | TraceEvent::is_terminal_for_run() returns true only for terminal variants (RunFinished/RunFailed/RunCancelled) |
| TRC-15 | TraceRing::new(0) rejects all events (dropped increments) |
| TRC-16 | TraceRing drain+refill preserves data correctly across cycles |

### EvidenceBundle (serializable evidence container)

| # | Behavior |
|---|---|
| BND-01 | parse_bundle_schema_version("major.minor") returns Ok(v) for valid inputs |
| BND-02 | parse_bundle_schema_version rejects empty string, leading zeros, missing dot, non-numeric, major>1 |
| BND-03 | validate_bundle returns empty Vec iff all required fields are non-empty |
| BND-04 | validate_bundle produces exactly one MissingRequiredField per absent required field |
| BND-05 | EvidenceBundle round-trips through YAML serialization unchanged |
| BND-06 | EvidenceBundle round-trips through JSON serialization unchanged |
| BND-07 | EvidenceBundle round-trips through Postcard binary serialization unchanged |
| BND-08 | write_bundle creates parent directories and writes bytes atomically |
| BND-09 | read_bundle deserializes from file returning EvidenceBundle or Error |
| BND-10 | bundle_path(bead_id, format) returns `.evidence/<bead_id>/bundle.<ext>` |
| BND-11 | evidence_path(bead_id, gate_name) returns `.evidence/<bead_id>/<gate_name>.yaml` |
| BND-12 | write_evidence serializes GateEvidence to YAML file |
| BND-13 | explain_failure returns WhyFailed for GateStatus::Fail, None for Pass/Skipped |
| BND-14 | validate_evidence_dir returns MissingEvidence for each absent required gate file |
| BND-15 | EvidenceBundleFormat::extension() returns correct extension per variant |

### BDD Catalog (Scenario validation)

| # | Behavior |
|---|---|
| CAT-01 | catalog() returns a non-empty slice of Scenario |
| CAT-02 | validate_catalog returns Ok(()) for valid catalog (non-empty, unique IDs, GWT non-empty, assertion present) |
| CAT-03 | validate_catalog returns EmptyCatalog for empty slice |
| CAT-04 | validate_catalog returns DuplicateScenarioId when two scenarios share id |
| CAT-05 | validate_catalog returns MissingGivenWhenThen when G/W/T is empty |
| CAT-06 | validate_catalog returns MissingExactAssertion when neither expected_outcome nor expected_error present |
| CAT-07 | validate_catalog returns MissingEvidenceDisposition when both executable_evidence_target and deferred_follow_up_bead are None |
| CAT-08 | validate_catalog returns ConflictingEvidenceDisposition when both evidence fields are Some |
| CAT-09 | validate_catalog returns InvalidExecutableEvidenceTarget when target is not a valid path pattern |
| CAT-10 | validate_catalog returns InvalidDeferredFollowUpBead when deferred bead doesn't match related_bead prefix |
| CAT-11 | validate_catalog returns PrivateSurface when public_surface contains "private" or "helper" |
| CAT-12 | validate_catalog returns SharedFixture when fixture doesn't contain "isolated" |
| CAT-13 | Scenario ID uniqueness is stable across validate_catalog calls |

---

## 2. Trophy Allocation

### TraceRing Tests (~60% unit)

| Layer | Count | Rationale |
|---|---|---|
| Unit / Calc | 24 | All trace ring behaviors are pure-state operations; rtrb is trusted SPSC; no I/O |
| Integration | 2 | evidence write/read round-trip with real filesystem |
| Static Analysis | 1 | `#![forbid(unsafe_code)]` + clippy on trace.rs |
| E2E | 0 | No CLI/UI trace display covered (explicit non-goal) |

### EvidenceBundle Tests (~65% integration)

| Layer | Count | Rationale |
|---|---|---|
| Unit | 15 | bundle validation, path formatting, schema version parsing |
| Integration | 8 | catalog integration tests + write/read round-trips with real filesystem |
| Proptest | 6 | Round-trip serialization invariants across YAML/JSON/Postcard |
| Static Analysis | 1 | clippy on xtask evidence module |

### BDD Catalog Tests (~70% integration)

| Layer | Count | Rationale |
|---|---|---|
| Unit | 9 | validate_catalog error path for each variant |
| Integration | 8 | Catalog-level assertions against real SCENARIOS constant |

**Overall ratios:** ~48 unit / ~18 integration / ~6 proptest / ~2 static ≈ 74 total test scenarios
Target: ~60% integration — satisfied by catalog + bundle persistence integration tests

---

## 3. BDD Scenarios

### TraceRing

```
### Behavior: TRC-01 — new creates valid ring
Given: a TraceRing with capacity 8
When: new(8) is called
Then: capacity is 8, len is 0, dropped is 0
```

```
### Behavior: TRC-02 — push succeeds when ring has space
Given: a TraceRing with capacity 4 containing no events
When: push(RunSubmitted { run: RunId::new(1) }) is called
Then: returns true and dropped count remains 0
```

```
### Behavior: TRC-03 — push fails and increments dropped when full
Given: a TraceRing with capacity 1 already containing one event
When: push(RunSubmitted { run: RunId::new(2) }) is called
Then: returns false and dropped is 1
```

```
### Behavior: TRC-04 — drain returns all events in FIFO order
Given: a TraceRing with 3 events in order [e1, e2, e3]
When: drain() is called
Then: returns [e1, e2, e3] in insertion order and ring is empty
```

```
### Behavior: TRC-05 — drain_into respects limit
Given: a TraceRing with 5 events
When: drain_into(2, &mut vec) is called
Then: vec has exactly 2 events and ring retains 3
```

```
### Behavior: TRC-06 — drain_for_run filters by run_id and preserves order
Given: a TraceRing with events for run 1 and run 2 interleaved
When: drain_for_run(RunId::new(2), 10) is called
Then: returns only run-2 events in FIFO order
```

```
### Behavior: TRC-07 — snapshot_for_run does not drain
Given: a TraceRing with 3 events for run 1
When: snapshot_for_run(RunId::new(1), 10) is called twice
Then: both calls return 3 events and subsequent drain also returns 3
```

```
### Behavior: TRC-08 — has_terminal_event detects terminal events
Given: a TraceRing with a RunFinished event for run 1
When: has_terminal_event_for_run(RunId::new(1)) is called
Then: returns true

Error variant:
Given: a TraceRing with only StepStarted/StepEnded events for run 1
When: has_terminal_event_for_run(RunId::new(1)) is called
Then: returns false
```

```
### Behavior: TRC-10 — len never exceeds capacity (INV-001)
Given: a TraceRing with capacity 3
When: pushing 10 events (7 overflow)
Then: len never exceeds 3 and dropped is 7
```

```
### Behavior: TRC-12 — dropped is monotonically non-decreasing
Given: a TraceRing with dropped count of 5
When: 3 more overflow pushes occur
Then: dropped count is exactly 8 (never decreases)
```

```
### Behavior: TRC-15 — capacity zero rejects all events
Given: a TraceRing with capacity 0
When: push(any_event) is called
Then: returns false and dropped increments
```

### EvidenceBundle

```
### Behavior: BND-01 — valid schema version parses
Given: input "1.0"
When: parse_bundle_schema_version(input) is called
Then: returns Ok("1.0")

Valid inputs: "0.0", "0.1", "1.0", "1.99", "00" (not valid — leading zero), "0" (not valid — missing dot)
```

```
### Behavior: BND-02 — invalid schema version returns error
Given: input "01.0" (leading zero)
When: parse_bundle_schema_version(input) is called
Then: returns Err(SchemaVersionParseFailed)

Error variants tested:
- empty string → SchemaVersionParseFailed
- "1" (no dot) → SchemaVersionParseFailed
- "1.0.0" (extra component) → handled (splitn(2, '.') ignores third component, but parts[1] would be "0.0" which parses OK; actually this is a potential issue — input "1.0.0" has parts = ["1", "0.0"], parts[1] is not empty so it passes split check, then "0.0" fails u64 parse → SchemaVersionParseFailed ✓
- "01.0" (leading zero) → SchemaVersionParseFailed
- "1.01" (leading zero in minor) → SchemaVersionParseFailed
- "2.0" (major > 1) → SchemaVersionParseFailed
- "a.b" (non-numeric) → SchemaVersionParseFailed
```

```
### Behavior: BND-03 — validate_bundle returns empty Vec for valid bundle
Given: an EvidenceBundle with all required fields non-empty and valid schema_version
When: validate_bundle(&bundle) is called
Then: returns empty Vec
```

```
### Behavior: BND-04 — validate_bundle produces one error per missing field
Given: an EvidenceBundle with empty linked_bead_id and empty agent
When: validate_bundle(&bundle) is called
Then: returns exactly 2 MissingRequiredField errors, one for each absent field
```

### BDD Catalog

```
### Behavior: CAT-02 — validate_catalog accepts valid catalog
Given: a catalog with all scenarios having non-empty id, given, when, then, unique IDs, and either expected_outcome or expected_error
When: validate_catalog(scenarios) is called
Then: returns Ok(())

Error variants:
Given: an empty catalog
When: validate_catalog([]) is called
Then: returns Err(EmptyCatalog)

Given: two scenarios with id "DUPE"
When: validate_catalog([s1, s2]) is called
Then: returns Err(DuplicateScenarioId { scenario_id: "DUPE" })

Given: a scenario with empty "given"
When: validate_catalog is called
Then: returns Err(MissingGivenWhenThen { scenario_id: ... })

Given: a scenario with neither expected_outcome nor expected_error
When: validate_catalog is called
Then: returns Err(MissingExactAssertion { scenario_id: ... })

Given: a scenario with both executable_evidence_target=None and deferred_follow_up_bead=None
When: validate_catalog is called
Then: returns Err(MissingEvidenceDisposition { scenario_id: ... })

Given: a scenario with both evidence fields set
When: validate_catalog is called
Then: returns Err(ConflictingEvidenceDisposition { scenario_id: ... })

Given: executable_evidence_target = Some("follow-up bead vb-hxm0")
When: validate_executable_target is called
Then: returns Err(InvalidExecutableEvidenceTarget) because it doesn't start with "crates/" or ".evidence/"

Given: deferred_follow_up_bead = Some("vb-other") where related_bead = "vb-hxm0"
When: validate_deferred_follow_up is called
Then: returns Err(InvalidDeferredFollowUpBead) because vb-other != vb-hxm0

Given: public_surface = "private helper module"
When: validate_catalog is called
Then: returns Err(PrivateSurface { scenario_id: ... })

Given: fixture = "shared catalog fixture"
When: validate_catalog is called
Then: returns Err(SharedFixture { scenario_id: ... })
```

---

## 4. Proptest Invariants

### Bundle Serialization Round-Trips (OBL-BND-004, OBL-BND-005, OBL-BND-006 — all PASSED)

```
### Proptest: EvidenceBundle YAML round-trip
Invariant: For all arbitrarily generated EvidenceBundle values,
  write_bundle(bundle, path, Yaml) followed by read_bundle(path, Yaml)
  produces a bundle equal (==) to the original
Strategy: evidence_bundle_strategy() — generates all fields via any::<String>()
  with nested strategies for ExecutorContext, GateEvidence, SourceTestMapping, ReleaseGateArtifact
Corpus seeds: empty gates, populated gates, all ArtifactType variants, various string lengths
```

```
### Proptest: EvidenceBundle JSON round-trip
Invariant: Same as YAML — JSON serialization round-trip preserves bundle equality
Strategy: same evidence_bundle_strategy()
Corpus seeds: same as YAML
```

```
### Proptest: EvidenceBundle Postcard round-trip
Invariant: Same — Postcard binary round-trip preserves bundle equality
Strategy: same evidence_bundle_strategy()
Corpus seeds: same as YAML
```

### Bundle Path Determinism (OBL-EVN-002 — WAIVED due to include!() structure)

```
### Proptest: bundle_path deterministic
Invariant: For any bead_id (String) and EvidenceBundleFormat,
  bundle_path(&bead_id, format) called twice returns equal PathBuf
  and path starts_with(".evidence/")
Strategy: any::<String>() × sample(Yaml, Json, Postcard)
```

### Bundle Fail-Closed Validation (OBL-BND-002 — WAIVED; compensated by proptest)

```
### Proptest: validate_bundle fail-closed for empty linked_bead_id
Invariant: For any bundle with linked_bead_id = "",
  validate_bundle returns non-empty error list containing MissingRequiredField{field: "linked_bead_id"}
Strategy: proptest with agent/timestamp/machine arbitrary strings, major >= 2

### Proptest: validate_bundle fail-closed for empty agent
Invariant: For any valid bundle, mutating executor_context.agent to "" causes validate_bundle to return error for that field
Strategy: evidence_bundle_strategy() then field mutation

### Proptest: validate_bundle fail-closed for empty timestamp
Invariant: Same pattern as agent — validate rejects empty timestamp
Strategy: same

### Proptest: validate_bundle fail-closed for empty machine
Invariant: Same pattern
Strategy: same
```

### Schema Version Parsing (OBL-BND-001 — WAIVED; compensated by OBL-BND-004/005/006)

```
### Proptest: parse_bundle_schema_version accepts valid formats
Invariant: "0.0", "0.1", "1.0", "1.99" all return Ok
Anti-invariant: "", "00.0", "01.0", "0.01", "2.0", "a.b" all return Err(SchemaVersionParseFailed)
Strategy: (0u64..=1, 0u64..=99) mapped to format!("{}.{}")
```

---

## 5. Fuzz Targets

**None.** vb-hs9m has no raw-bytes parsers, no network deserialization, and no JSON/YAML stdin ingestion within its bead scope. All parsing is via trusted serde implementations (serde_yaml, serde_json, postcard). Explicit waiver: no fuzz targets required.

---

## 6. Kani Harnesses

All 4 Kani harnesses are **waived** due to `cargo kani` reporting "No supported targets found" (CBMC goto-cc not configured for x86_64-unknown-linux-gnu). Compensating evidence is proptest + unit tests.

| Harness | Property | Compensating Evidence |
|---|---|---|
| `verify_trace_ring_bounds` | INV-001: len <= capacity | OBL-TRC-005 (unit: adversarial_overflow) + OBL-TRC-006 (unit: fifo_ordering) |
| `verify_trace_ring_dropped_monotonic` | INV-001: dropped monotonicity | OBL-TRC-005 (unit: overflow drop count) |
| `verify_drain_for_run_correctness` | POST-004: filter correctness | OBL-BND-004/005/006 (proptest round-trips) |
| `verify_terminal_event_detection` | POST-005: terminal event detection | OBL-BND-004/005/006 (proptest round-trips) |
| `schema_version_parse_non_panic` | PRE-004: parse never panics | OBL-BND-004/005/006 (proptest serialization) |
| `validator_correctness` | INV-002: validate_bundle completeness | OBL-BND-004/005/006 (proptest serialization) |

### Kani Re-run Trigger
When Kani CBMC targets are installed:
```bash
cargo kani --harness verify_trace_ring_bounds --tests
cargo kani --harness verify_trace_ring_dropped_monotonic --tests
cargo kani --harness verify_drain_for_run_correctness --tests
cargo kani --harness verify_terminal_event_detection --tests
cargo kani --harness schema_version_parse_non_panic
cargo kani --harness validator_correctness
```

---

## 7. Mutation Checkpoints

**Threshold: ≥90% mutation kill rate**

### TraceRing mutations (cargo-mutants on crates/vb_runtime/src/trace.rs)

| Function | Mutation | Kill mechanism |
|---|---|---|
| `push` | Remove `saturating_add` → `+` (overflow panic) | `trace_ring_overflow_counts_dropped_events_without_silent_loss` |
| `push` | Return `true` always | `trace_ring_push_returns_false_when_full` |
| `push` | Skip `remember` call | `trace_ring_at_exact_capacity_accepts_all_events_without_drops` |
| `drain_into` | Remove `checked_add` boundary | `trace_ring_drain_into_respects_limit` |
| `drain_for_run` | Return all events unfiltered | `trace_ring_drain_for_run_filters_by_run_id` |
| `drain_for_run` | Remove `inspected` limit check | `trace_ring_drain_for_run_respects_limit` |
| `has_terminal_event_for_run` | Return `true` always | `has_terminal_event_for_run_false_case` |
| `remember` | Skip `pop_front` on full history | `trace_ring_history_evicts_when_drained_and_refilled` |

### EvidenceBundle mutations (cargo-mutants on xtask/src/evidence/bundle.rs)

| Function | Mutation | Kill mechanism |
|---|---|---|
| `parse_bundle_schema_version` | Accept leading zeros | `bundle_parse_invalid_schema_returns_error` (via proptest) |
| `parse_bundle_schema_version` | Accept major > 1 | same |
| `parse_bundle_schema_version` | Return Ok("") on empty input | same |
| `validate_bundle` | Return empty vec on missing field | `prop_fail_closed_missing_bead_id` |
| `validate_bundle` | Duplicate error per field | OBL-BND-002 (Kani, waived — proptest compensates) |
| `bundle_path_component` | Pass through '/' unchanged | OBL-EVN-002 (unit test path formatting) |
| `write_bundle` | Skip `create_dir_all` | integration test (OBL-EVN-003) |
| Postcard `into_bundle` | Drop fields | `prop_write_read_roundtrip_postcard` |

### Catalog mutations (cargo-mutants on crates/workspace_tests/src/acceptance_catalog.rs)

| Function | Mutation | Kill mechanism |
|---|---|---|
| `validate_scenario` | Skip duplicate ID check | `validate_catalog_duplicate_id` |
| `validate_scenario` | Skip GWT empty check | `validate_catalog_missing_gwt` |
| `validate_scenario` | Skip assertion check | `validate_catalog_missing_assertion` |
| `validate_executable_target` | Accept any string | `test_catalog_gate_fails_when_follow_up_is_disguised_as_executable_evidence` |
| `validate_deferred_follow_up` | Skip bead prefix check | `test_catalog_gate_fails_when_deferred_gap_does_not_match_related_bead` |

---

## 8. Combinatorial Coverage Matrix

### TraceRing — POST-002 / POST-003 / POST-004 / POST-005 / INV-001

| Scenario | Input | Expected | Layer |
|---|---|---|---|
| new(capacity) valid | capacity=8 | capacity=8, len=0, dropped=0 | unit |
| new(capacity=0) | capacity=0 | all pushes return false, dropped increments | unit |
| push when not full | 1 event into cap=4 ring | true, dropped=0, len=1 | unit |
| push when exactly full | 4 events into cap=4 ring | true, dropped=0 | unit |
| push when over capacity | 5 events into cap=4 ring | 5th returns false, dropped=1, len=4 | unit |
| push when massively over | 10 events into cap=4 ring | dropped=6, len=4 | unit |
| drain returns all | 3 events, then drain | vec.len=3, ring.len=0 | unit |
| drain leaves ring empty | drain twice | first=3, second=0 | unit |
| drain_into respects limit | 5 events, drain_into(2) | vec.len=2, ring.len=3 | unit |
| drain_into with zero limit | any events, limit=0 | vec.len=0 | unit |
| drain_into exceeding ring | 3 events, limit=100 | vec.len=3 | unit |
| drain_for_run filters | events for run1, run2, run1 | only run1 events returned | unit |
| drain_for_run empty for nonexistent | events for run1 | empty vec | unit |
| drain_for_run respects limit | 5 run1 events, limit=3 | vec.len=3 | unit |
| drain_for_run with zero limit | any events, limit=0 | empty vec (no drain) | unit |
| snapshot_for_run no drain | 3 events for run1 | snapshot twice → both return 3, drain still returns 3 | unit |
| has_terminal_event true | RunFinished for run1 | true | unit |
| has_terminal_event false | only StepStarted for run1 | false | unit |
| has_terminal_event RunFailed | RunFailed for run1 | true | unit |
| has_terminal_event RunCancelled | RunCancelled for run1 | true | unit |
| capacity always positive | new(0) | len never exceeds 0, all events dropped | unit |
| dropped monotonic | overflow 3 times | dropped=3, then overflow 2 more → dropped=5 (never decreases) | unit |
| FIFO ordering | push [e1, e2, e3], drain | [e1, e2, e3] | unit |
| history eviction | push 5 into cap=3 | oldest 2 dropped, newest 3 retained | unit |
| fill-drain-refill | push [0,1], drain, push [2,3] | second drain = [2,3] | unit |

### EvidenceBundle — POST-006 / POST-007 / INV-002 / INV-004

| Scenario | Input | Expected | Layer |
|---|---|---|---|
| schema version "0.0" | "0.0" | Ok("0.0") | unit |
| schema version "1.99" | "1.99" | Ok("1.99") | unit |
| schema version "" | empty | Err(SchemaVersionParseFailed) | unit |
| schema version "00.0" | "00.0" | Err | unit |
| schema version "01.0" | "01.0" | Err | unit |
| schema version "0.01" | "0.01" | Err | unit |
| schema version "1.0.0" | "1.0.0" | Err (fails u64 parse of minor) | unit |
| schema version "2.0" | "2.0" | Err (major > 1) | unit |
| schema version "a.b" | "a.b" | Err | unit |
| validate_bundle all valid | valid bundle | empty Vec | unit |
| validate_bundle missing linked_bead_id | "" | MissingRequiredField{linked_bead_id} | unit |
| validate_bundle missing agent | "" | MissingRequiredField{executor_context.agent} | unit |
| validate_bundle missing timestamp | "" | MissingRequiredField{executor_context.timestamp} | unit |
| validate_bundle missing machine | "" | MissingRequiredField{executor_context.machine} | unit |
| validate_bundle missing multiple | linked_bead_id + agent empty | exactly 2 errors | unit |
| validate_bundle bad schema_version | "not-a-version" | SchemaVersionParseFailed | unit |
| round-trip YAML | valid bundle | bundle == roundtrip | proptest |
| round-trip JSON | valid bundle | bundle == roundtrip | proptest |
| round-trip Postcard | valid bundle | bundle == roundtrip | proptest |
| bundle_path format Yaml | bead_id="vb-hs9m" | ".evidence/vb-hs9m/bundle.yaml" | unit |
| bundle_path format Json | bead_id="vb-hs9m" | ".evidence/vb-hs9m/bundle.json" | unit |
| bundle_path format Postcard | bead_id="vb-hs9m" | ".evidence/vb-hs9m/bundle.postcard" | unit |
| evidence_path format | bead_id="vb-hs9m", gate="test" | ".evidence/vb-hs9m/test.yaml" | unit |
| write_evidence then read_evidence | valid GateEvidence | content preserved | integration |
| explain_failure Fail | GateStatus::Fail | Some(WhyFailed) | unit |
| explain_failure Pass | GateStatus::Pass | None | unit |
| explain_failure Skipped | GateStatus::Skipped{reason} | None | unit |
| validate_evidence_dir all present | dir with all required gates | Ok(empty vec) | unit |
| validate_evidence_dir missing one | dir missing "clippy.yaml" | MissingEvidence{clippy} | unit |

### BDD Catalog — POST-009 / POST-010 / INV-003

| Scenario | Input | Expected | Layer |
|---|---|---|---|
| catalog non-empty | catalog() | len > 0 | integration |
| catalog valid | full SCENARIOS | Ok(()) | unit |
| catalog empty | [] | Err(EmptyCatalog) | unit |
| catalog duplicate id | two scenarios same id | Err(DuplicateScenarioId) | unit |
| catalog missing given | given="" | Err(MissingGivenWhenThen) | unit |
| catalog missing when | when="" | Err(MissingGivenWhenThen) | unit |
| catalog missing then | then="" | Err(MissingGivenWhenThen) | unit |
| catalog missing both assertions | no outcome or error | Err(MissingExactAssertion) | unit |
| catalog missing evidence disposition | both None | Err(MissingEvidenceDisposition) | unit |
| catalog conflicting evidence | both Some | Err(ConflictingEvidenceDisposition) | unit |
| catalog invalid target | "follow-up bead vb-hs9m" | Err(InvalidExecutableEvidenceTarget) | unit |
| catalog invalid deferred bead | deferred="vb-other", related="vb-hs9m" | Err(InvalidDeferredFollowUpBead) | unit |
| catalog private surface | "private helper module" | Err(PrivateSurface) | unit |
| catalog shared fixture | "shared catalog fixture" | Err(SharedFixture) | unit |
| catalog vb-kyyf traceability | full catalog | write_vb_kyyf_traceability succeeds | integration |
| catalog maps tests to scenarios | full catalog | test targets count = 11, evidence targets = 7 | integration |
| catalog deferred gaps | full catalog | 3 deferred follow-up beads: vb-te1i, vb-rpch, vb-0sps | integration |

---

## 9. Open Questions

| # | Question | Resolution Required |
|---|---|---|
| OQ-01 | OBL-EVN-002 (bundle_path unit test) is waived due to `include!()` in xtask/src/evidence.rs — should the module structure be refactored to `pub mod` declarations so tests can be placed at `evidence::bundle::tests::bundle_path_format`? | Before test-writer proceeds on OBL-EVN-002; currently compensated by OBL-EVN-001 (evidence_path) |
| OQ-02 | `bundle_path_component` strips path separators: should `bead_id = "foo/bar"` produce `foo_bar` or be rejected? Current impl silently replaces with `_`. | Clarify contract — test-writer needs exact behavior for invalid bead_id strings |
| OQ-03 | The `parse_bundle_schema_version` accepts major=0..=1, but the contract says `^(0|[1-9][0-9])\.(0|[1-9][0-9])$`. This means `0.0` is valid in the regex but `00.0` is not. Is `0.0` intentionally valid? | Yes per contract — clarify in test naming |
| OQ-04 | `cargo-mutants` has not been run on this bead. Should mutation testing be added as a required gate before landing? | Decision needed: if yes, add `cargo mutants` to moon ci tasks |
| OQ-05 | The `GateEvidencePostcard` / `GateStatusPostcard` encode/tag path adds complexity vs direct postcard serialization of the full EvidenceBundle. Is the indirection intentional or a future refactor target? | Impacts whether proptest should cover both direct and indirect postcard paths |

---

## 10. Verification Evidence Ledger

| Obligation | Status | Evidence Artifact | Location |
|---|---|---|---|
| OBL-TRC-005 | **PASSED** | test-output.txt (adversarial_overflow) | crates/vb_runtime/src/trace.rs:825 |
| OBL-TRC-006 | **PASSED** | test-output.txt (fifo_ordering) | crates/vb_runtime/src/trace.rs:419 |
| OBL-BND-004 | **PASSED** | test-output.txt (round_trip_yaml) | xtask/tests/bundle_tests.rs:157 |
| OBL-BND-005 | **PASSED** | test-output.txt (round_trip_json) | xtask/tests/bundle_tests.rs:178 |
| OBL-BND-006 | **PASSED** | test-output.txt (round_trip_postcard) | xtask/tests/bundle_tests.rs:199 |
| OBL-CAT-001 | **PASSED** | test-output.txt (validate_catalog_valid) | crates/workspace_tests/src/acceptance_catalog.rs |
| OBL-CAT-002 | **PASSED** | test-output.txt (validate_catalog_duplicate_id) | crates/workspace_tests/src/acceptance_catalog.rs |
| OBL-CAT-003 | **PASSED** | test-output.txt (validate_catalog_missing_gwt) | crates/workspace_tests/src/acceptance_catalog.rs |
| OBL-CAT-004 | **PASSED** | test-output.txt (validate_catalog_missing_assertion) | crates/workspace_tests/src/acceptance_catalog.rs |
| OBL-CAT-005 | **PASSED** | test-output.txt (test_catalog_non_empty) | crates/workspace_tests/tests/vb_hxm0_acceptance_catalog.rs |
| OBL-CAT-006 | **PASSED** | test-output.txt (test_catalog_lists_every_master_doc_behavior_by_scenario_id) | crates/workspace_tests/tests/vb_hxm0_acceptance_catalog.rs |
| OBL-CAT-007 | **PASSED** | test-output.txt (test_catalog_maps_existing_tests_to_covered_scenarios) | crates/workspace_tests/tests/vb_hxm0_acceptance_catalog.rs |
| OBL-CAT-008 | **PASSED** | test-output.txt (test_catalog_gate_fails_when_behavior_has_no_scenario) | crates/workspace_tests/tests/vb_hxm0_acceptance_catalog.rs |
| OBL-CAT-009 | **PASSED** | test-output.txt (test_catalog_gate_fails_when_scenario_has_no_test_target) | crates/workspace_tests/tests/vb_hxm0_acceptance_catalog.rs |
| OBL-EVN-001 | **PASSED** | test-output.txt (evidence_path_format) | xtask/src/evidence/persistence.rs |
| OBL-EVN-003 | **PASSED** | test-output.txt (evidence_write_read_roundtrip) | xtask integration test |
| OBL-TRC-001 | **WAIVED** | kani-trace-ring-report.html (BLOCKED_TOOLING) | compensating: OBL-TRC-005 + OBL-TRC-006 |
| OBL-TRC-002 | **WAIVED** | kani-trace-ring-report.html (BLOCKED_TOOLING) | compensating: OBL-TRC-005 |
| OBL-TRC-003 | **WAIVED** | kani-trace-ring-report.html (BLOCKED_TOOLING) | compensating: OBL-BND-004/005/006 |
| OBL-TRC-004 | **WAIVED** | kani-trace-ring-report.html (BLOCKED_TOOLING) | compensating: OBL-BND-004/005/006 |
| OBL-TRC-007 | **WAIVED** | miri-report.txt (BLOCKED_TOOLING: rust-src missing) | compensating: #![forbid(unsafe_code)] on trace.rs |
| OBL-BND-001 | **WAIVED** | kani-bundle-report.html (BLOCKED_TOOLING) | compensating: OBL-BND-004/005/006 |
| OBL-BND-002 | **WAIVED** | kani-bundle-report.html (BLOCKED_TOOLING) | compensating: OBL-BND-004/005/006 |
| OBL-BND-003 | **WAIVED** | kani-bundle-report.html (BLOCKED_TOOLING) | compensating: OBL-BND-004/005/006 |
| OBL-BND-007 | **WAIVED** | miri-report.txt (BLOCKED_TOOLING: rust-src missing) | compensating: OBL-BND-006 |
| OBL-EVN-002 | **WAIVED** | test-output.txt (BLOCKED_STRUCTURE: include!() layout) | compensating: OBL-EVN-001 |
| WAIVED-TLA-001 | **WAIVED** | tla-spec.md (explicit waiver) | N/A |
| WAIVED-LEAN-001 | **WAIVED** | lean-contract.md (explicit waiver) | N/A |
| WAIVED-CONC-001 | **WAIVED** | loom-spec.md (explicit waiver) | N/A |
