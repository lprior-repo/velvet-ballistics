# Domain Model — vb-282my

**Bead:** vb-282my (P1)
**Title:** Add refinement harnesses or waivers for repaired TLA bridge (7 RRO rows)
**Domain:** TLA bridge refinement
**Date:** 2026-05-29

## Ubiquitous Language

| Term | Definition | Rust/TLA Mapping |
|------|-----------|-----------------|
| **Refinement Obligation (RRO)** | A `rust-refinement-obligation/v1` row linking a TLA+ model to Rust sources, behavior tests, and refinement harnesses. Each row must close with either an independent refinement harness or an approved proportional waiver. | `verification/tla/rust-refinement-obligations.jsonl` |
| **TLA+ Model** | A temporal specification checked by TLC under bounded configuration. Provides temporal-design evidence. Does NOT provide Rust implementation proof by itself. | `.tla` files under `verification/tla/` and `specs/` |
| **Rust Source Reference** | A specific production symbol (function, method, type) annotated with exact file and line range. Must be named, not just file-scoped. | e.g., `crates/vb_runtime/src/shard/helpers.rs:273-294::record_retry_attempt` |
| **Behavior Test** | A Rust `#[test]` that exercises production behavior. Necessary but not sufficient for refinement closure — behavior tests exercise public APIs; a distinct refinement harness independently verifies model-Rust alignment. | e.g., `crates/vb_runtime/src/shard/tests/chunk_014.rs:102-207` |
| **Refinement Harness** | An implementation-bound verification artifact distinct from behavior tests: Kani proof, Flux refinement, Verus spec binding, or proptest property. Provides independent evidence that the Rust code satisfies the TLA+ model's claim. | Kani `#[kani::proof]`, Flux `#[sig]`, Verus `proof fn`, proptest `proptest!` |
| **Proportional Waiver** | A documented, reviewer-approved justification that the TLC + behavior-test combination is sufficient for a specific RRO row without a separate refinement harness. Behavior-affecting claims CANNOT be waived. | `formal-waiver/v1` in `proof-findings.jsonl` |
| **Bridge Verdict** | The overall proof-to-rust review status: PASS (all rows closed), REJECTED (gaps exist), PARTIAL (some rows open). | `verification/tla/proof-to-rust-review.md` |
| **Mapping Status** | Per-RRO status: `planned` (not yet materialized), `materialized` (code exists), `verified` (reviewer approved), `partial` (incomplete). | Field `mapping_status` in RRO JSONL |
| **TLC Evidence** | Bounded model-check output (states, distinct states, depth, exit 0) confirming the TLA+ spec is consistent. | `verification/tla/proof-to-rust-map.md` |
| **Risk Tag** | Categorization of what kind of correctness risk the RRO addresses: `temporal`, `concurrency`, `persistence`, `public-api`, `user-visible-behavior`. | Delivery scope and RRO rows |
| **Harness Binding** | The precise mapping from a TLA+ claim to a specific Rust harness function. Must be confirmed by reviewer. | Kani harness at specific line range, Flux signature, proptest property |
| **Refinement Harness Gap** | The blocking finding (`TLA-BRIDGE-REFINEMENT-HARNESS-GAP`) that rows have empty `refinement_harness_refs` and no approved waiver. | `proof-to-rust-review.md` |

## Entities

### RefinementObligation (Aggregate Root)

The central entity. Each RRO owns its TLA model, source refs, test refs, harness refs, and evidence commands.

```
RefinementObligation
  ├── id: RroId                    (RRO-TLA-CHOOSE-LOWERING-001, etc.)
  ├── tla_artifact: TlaModel       (path to .tla)
  ├── claim: DomainClaim           (natural-language claim text)
  ├── source_refs: Vec<SourceRef>  (production Rust symbols)
  ├── behavior_test_refs: Vec<TestRef>  (existing behavior tests)
  ├── refinement_harness_refs: Vec<HarnessRef>  (verification artifacts)
  ├── evidence_commands: Vec<EvidenceCommand>   (recorded commands)
  ├── mapping_status: MappingStatus             (planned/materialized/verified/partial)
  ├── required_fix: str                          (natural-language fix description)
  └── risk_tags: Vec<RiskTag>                   (temporal, concurrency, etc.)
```

### TlaModel (Entity)

A bounded TLC-proof temporal specification.

```
TlaModel
  ├── file: TlaPath                 (.tla file)
  ├── cfg: CfgPath                  (.cfg file)
  ├── tlc_result: TlcResult        (states, distinct, depth, exit code)
  └── invariants: Vec<InvariantName>
```

### RefinementHarness (Entity)

An independent verification artifact. Three kinds: formal harness, property harness, or proportional waiver.

```
RefinementHarness
  ├── kind: HarnessKind            (KaniProof | FluxRefinement | VerusSpec | ProptestProperty | ProportionalWaiver)
  ├── file: HarnessPath            (file path)
  ├── line_range: LineRange        (exact line span)
  ├── symbol: SymbolName           (function name)
  ├── binding_status: BindingStatus (unconfirmed | partial | confirmed)
  └── reviewer_disposition: Disposition (pending | accepted | rejected)
```

### ProportionalWaiver (Entity)

A documented waiver when a full verification harness is not feasible or necessary.

```
ProportionalWaiver
  ├── waiver_id: WaiverId
  ├── reason: str                  (why proportional)
  ├── behavior_affecting: bool     (MUST be false for validity)
  ├── compensating_evidence: str   (existing evidence sufficient)
  ├── boundary_proof: str          (what boundary makes this safe)
  ├── expiry: ISO8601
  └── review_status: ReviewStatus  (pending | approved | rejected)
```

## Value Objects

### RroId
- Format: `RRO-TLA-{MODEL}-{NNN}`
- Semantics: Uniquely identifies a refinement obligation
- Validation: matches regex `^RRO-TLA-[A-Z]+-\d{3}$`

### SourceRef
- Format: `{file}:{line_range}::{symbol}`
- Invariant: MUST name a specific symbol, not just a file
- Example: `crates/vb_runtime/src/shard/helpers.rs:273-294::record_retry_attempt`

### TestRef
- Format: `{file}:{line_range}` or `{file}` with test_count
- Invariant: MUST reference tests that would fail if production behavior were deleted

### HarnessRef
- Format: `{file}:{line_range}` or `{file}:{line_range}::{symbol}`
- Invariant: MUST point to an independent verification artifact, not a behavior test

### RiskTag
- Variants: `temporal`, `concurrency`, `persistence`, `public-api`, `user-visible-behavior`
- No row is untagged; all 7 RRO rows carry at least temporal

### MappingStatus
- Variants: `planned`, `materialized`, `verified`, `partial`
- `partial` = not closed; `verified` = reviewer approved
- State 12 closure forbids `planned`

### TlcResult
- fields: states_generated, distinct_states, depth, exit_code
- Invariant: exit_code == 0 for valid TLC evidence

## Invariants

### INV-1: Source Symbol Identity
Every `source_ref` MUST name a specific production symbol (function, method, type). File-only references are invalid.

### INV-2: Independent Harness
A refinement harness MUST be distinct from behavior tests. A test passing `cargo test` is NOT a refinement harness.

### INV-3: No Behavior-Affecting Waivers
A proportional waiver covering a behavior-affecting claim (`behavior_affecting: true`) is INVALID. Any such waiver must be rejected by the reviewer.

### INV-4: Harness-or-Waiver Closure
No RRO row may transition to `verified` without either (a) a non-empty `refinement_harness_refs` with `binding_status: confirmed` and `reviewer_disposition: accepted`, or (b) a `formal-waiver/v1` row with `review_status: approved` and `status: approved`.

### INV-5: Independent Review
The bridge verdict cannot be self-approved. Review artifacts (proof-reviewer, proof-plan-reviewer) MUST be produced by an independent agent invocation. Self-stamping reviewer dispositions is invalid.

### INV-6: TLA Alone Is Not Proof
TLA+ model-check passing under bounded configuration provides temporal-design evidence only. No RRO row may close with TLC evidence as the sole implementation proof.

### INV-7: Behavior Tests Are Necessary But Not Sufficient
Behavior tests pass is required for closure, but behavior tests are not refinement harnesses. Each RRO must have behavior tests AND a distinct refinement harness or waiver.

### INV-8: RetryFSM Kani Binding Completeness
The existing Kani harness for RetryFSM (`kani_shard_lifecycle_harnesses.rs:315-354`) covers monotonicity only. The full RetryFSM claim includes: eventual exhaustion under weak fairness, no retry after max attempts, and exhausted retries remain typed and terminal. The harness binding status is PARTIAL because only monotonicity is covered.

## Forbidden States

| State | Why Illegal | Detection |
|-------|-----------|-----------|
| Empty `refinement_harness_refs` with `mapping_status: verified` | INV-4 violation | RRO JSONL validation |
| `behavior_affecting: true` on a waiver | INV-3 violation | Waiver schema validation |
| File-only `source_ref` without symbol name | INV-1 violation | Source string parsing |
| Behavior test file used as refinement harness ref | INV-2 violation | Path collision detection |
| Self-stamped `reviewer_disposition: accepted` | INV-5 violation | Invocation provenance check |
| `mapping_status: planned` at State 12 | INV-4 + lifecycle violation | State transition guard |
| RetryFSM `binding_status: confirmed` without full claim coverage | INV-8 violation | Claim-coverage analysis |
| RRO with TLC evidence only and `mapping_status: verified` | INV-6 violation | Evidence completeness check |

## 7 RRO Rows — Domain Classification

| RRO ID | Domain Concept | Core Guard | Harness Kind Needed |
|--------|---------------|------------|-------------------|
| RRO-TLA-CHOOSE-LOWERING-001 | Compile-time lowering enforces fanout limit (64), rejects empty branch tables, resolves canonical otherwise labels, and lowers branch targets. | Fanout ≤ 64; empty branches ⟹ otherwise required; label resolution must find step index; branch targets must be in compile bounds. | Kani (fanout/empty-rejection exhaustivity) or proptest (label-resolution/integration) |
| RRO-TLA-CHOOSE-REPLAY-001 | Runtime ChooseSlot replay selects first true branch, falls back to otherwise, errors when no branch and no otherwise. | Branch iteration order is deterministic; boolean-only conditions; otherwise must exist when all false; overflow-safe branch indexing. | Kani (branch exhaustion, overflow) or proptest (branch permutation) |
| RRO-TLA-ASK-ANSWER-001 | Ask-answer requires matching pending timer; emits SlotWritten before AskAnswered; preserves per-run journal sequence monotonicity; pending timer only exposed after AskScheduled journaled. | Pending timer (run, step, kind: Ask) must exist before answer; AskScheduled journal append must succeed before timer insertion; SlotWritten must precede AskAnswered in journal. | Kani (sequence monotonicity, append-before-insert) or Flux (timer→AskScheduled refinement) |
| RRO-TLA-RETRY-FSM-001 | Retryable failures exhaust under weak fairness; no retry after max; exhausted retries remain typed and terminal. | `action_attempts[step] < policy.max_attempts` gate; monotonicity: attempts only increase; overflow fail-closed (checked_add); terminal state is Failed with error kind. | Kani (full claim: monotonicity + exhaustion + terminal typing) — existing partial harness, needs completion |
| RRO-TLA-RETRY-JOURNAL-001 | Journal duplicate identity by (run, seq) storage key: strict reject duplicates; queued unpersisted allows exact idempotent duplicates. | `contains_key(key)` before insert; idempotent path: decode+compare existing event bytes; key encoding is deterministic: `[0x11][run_id_u64_be][seq_u64_be]`. | Kani (key-space injectivity, duplicate-rejection) or proptest (idempotency round-trip) |
| RRO-TLA-RESUME-001 | Resume transitions: Resumed journaled before drive; append failure rolls back to Resumable; drive failure preserves Resumed event but rolls runtime state. | RuntimeState guard: only Resumable→Resuming; journal append before drive returns; rollback path: append failure → ResumeRollback; drive failure → ResumeRollback (preserving journal). | Kani (state-machine transitions, append-then-drive ordering) or Flux (RuntimeState refinement) |
| RRO-TLA-ADMISSION-001 | Admission never acknowledges or allocates live state before durable header persistence; append failures map to AdmissionHeaderPersistenceFailed. | RunSubmitted before RunState insert; RunAdmission before RunState insert; append failure → discard sequence → AdmissionHeaderPersistenceFailed; no live state created on failure path. | Kani (append-before-insert ordering) or proptest (error-path coverage) |

## Open Questions

1. **RetryFSM Kani binding completeness**: Does the existing `kani_retry_attempt_monotonicity` cover the full claim (eventual exhaustion, terminal typing) or only monotonicity? Per INV-8, the binding is currently partial. The downstream proof-writer must either extend the Kani harness or document a proportional waiver covering non-monotonicity subclaims.

2. **Proportional waiver policy**: When is a proportional waiver acceptable vs. mandating a full refinement harness? Current policy: behavior-affecting claims CANNOT be waived. All 7 rows have `behavior_affecting: true` (or unset, default true). This means every row needs a refinement harness unless a specific subclaim is determined to be non-behavior-affecting.

3. **ChooseSlotLowering/ChooseSlotReplay split**: These are separate TLA models but share workflow foundations. A single cross-crate harness could cover both, but would need to span `vb_compile` (compile-time) and `vb_core` (runtime) crates.

4. **Verus/Flux applicability**: Several rows (AskAnswer, Resume, Admission) involve state-machine refinements that could be expressed as Flux refinements or Verus spec bindings. The domain recommends Kani for most rows due to bounded exhaustivity patterns, but final lane decisions belong to proof-planner.
