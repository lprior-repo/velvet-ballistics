# Proof Strategy — vb-hs9m (State 4: Observability & Evidence Packaging)

## Bead Overview

| Field | Value |
|-------|-------|
| Bead | vb-hs9m |
| Focus | Observability, TraceRing, EvidenceBundle, BDD catalog, artifact packaging, audit trails |
| Workspace | `/home/lewis/src/vb-hs9m-workspace` |
| Source checkout | `/home/lewis/src/velvet-ballistics` |

---

## Risk Classification

| Risk | Primary lane | Secondary lane | Rationale |
|------|-------------|----------------|-----------|
| Rust-local invariant (TraceRing boundedness, FIFO, monotonicity) | **Kani** | unit-test | Bounded state machine; Kani exhausts all push/drain paths |
| Rust-local invariant (EvidenceBundle validation) | **Kani** | unit-test | Pure function over product type; Kani proves all-field coverage |
| Panic freedom (parse, write, read) | **Kani** | — | Arbitrary string input → bounded model check |
| Serialization round-trip (YAML/JSON/Postcard) | **proptest** | — | Broad input space; 1000 iterations kills mutant paths |
| Postcard UB | **Miri** | — | Binary codec with raw byte representation |
| TraceRing UB (SPSC operations) | **Miri** | — | Note: `trace.rs` is `#![forbid(unsafe_code)]`; Miri still runs as belt-and-suspenders |
| Catalog uniqueness / non-empty fields | **unit-test** | integration-test | Static compile-time slice; exhaustively testable |
| Evidence path formatting | **unit-test** | — | Deterministic string construction |
| Concurrency | **not_applicable** | — | SPSC ring buffer only; no Mutex/RwLock/Atomic in scope |
| Temporal / protocol / workflow | **waived** | — | Explicit TLA+ waiver: no state-over-time or interleaving behavior |
| Theorem kernel (Lean/Aeneas/Hax) | **waived** | — | No algebraic properties requiring proof assistant extraction |
| Verus refinement types | **waived** | — | Rust-local invariants provable by Kani + unit-test |

---

## Verifier Lane Budget

| Lane | Allocated obligations | Budget rationale |
|------|---------------------|-----------------|
| Kani | 9 | Core safety proofs (boundedness, panic freedom, validation correctness) |
| Miri | 2 | Belt-and-suspenders UB check (TraceRing, Postcard) |
| proptest | 3 | Round-trip property tests across 3 serialization formats |
| unit-test | 10 | Catalog validation, path formatting, overflow adversarial |
| integration-test | 5 | Catalog gate, evidence write/read cycle |
| **Total** | **26 + 3 waived** | |

---

## Waiver Registry

| Clause | Waived lane | Reason | Compensating evidence | Follow-up trigger |
|--------|------------|--------|----------------------|-------------------|
| TLA+ overall | `tla-plus` | No temporal/protocol/workflow behavior in scope; TraceRing is pure local state; EvidenceBundle is static container; BDD catalog is compile-time static | Kani bounded model check + unit tests | If workflow orchestration or lease/claim logic added to scope |
| Lean/Aeneas/Hax | `theorem` | No algebraic theorem requiring proof-assistant extraction; all properties expressible as Kani/UT | Kani + proptest | If symbolic algebraic proof required by security audit |
| Verus (signals_invariant, run_frame_invariant) | `verus` | Pre-existing artifacts outside vb-hs9m scope; not authored by this bead | Existing Verus proofs in `verification/verus/` | If vb-hs9m code gains Verus-annotated dependencies |
| Concurrency | `loom` / `tla-plus` | SPSC ring buffer is lock-free by design; rtrb crate guarantees single-producer/single-consumer; no concurrent writer conflicts | Kani + Miri | If multi-producer or shared-channel concurrency added |

---

## TraceRing Defense-in-Depth

1. **Kani** — bounded model check: `len <= capacity`, `dropped` monotonic, FIFO ordering, `has_terminal_event_for_run` correctness
2. **unit-test** — adversarial overflow, drain empties ring, terminal event detection
3. **Miri** — UB check across all trace operations (belt-and-suspenders; code is `forbid(unsafe_code)`)

## EvidenceBundle Defense-in-Depth

1. **Kani** — panic freedom for `parse_bundle_schema_version`, `validate_bundle`, `write_bundle`/`read_bundle`
2. **proptest** — YAML/JSON/Postcard round-trip identity (1000 iterations)
3. **Miri** — Postcard binary serialization UB check

## Catalog Defense-in-Depth

1. **unit-test** — `validate_catalog` error mapping for all `CatalogValidationError` variants
2. **integration-test** — BDD catalog gate: scenario-to-behavior coverage, orphan detection

## Evidence Persistence Defense-in-Depth

1. **unit-test** — `evidence_path` and `bundle_path` format correctness
2. **integration-test** — write/read round-trip preserves `GateEvidence`

---

## Verification Artifacts (existing + planned)

| Artifact | Status | Path |
|----------|--------|------|
| `contract.md` | ✅ accepted | `.beads/vb-hs9m/contract.md` |
| `traceability-matrix.jsonl` | ✅ accepted | `.beads/vb-hs9m/traceability-matrix.jsonl` |
| `verification-layers.md` | ✅ accepted | `.beads/vb-hs9m/verification-layers.md` |
| `proof-obligations.jsonl` (planned) | 🔄 this artifact | `.beads/vb-hs9m/proof-obligations.planned.jsonl` |
| `proof-strategy.md` | 🔄 this artifact | `.beads/vb-hs9m/proof-strategy.md` |
| `proof-plan-review-input.md` | 🔄 this artifact | `.beads/vb-hs9m/proof-plan-review-input.md` |

---

## Assumptions & Bounds

- `rtrb` crate (SPSC ring buffer) is trusted; not re-verified in vb-hs9m scope
- `serde_yaml`, `serde_json`, `postcard` crates are trusted; Miri checks for UB only
- `catalog()` returns a static compile-time slice; no dynamic loading
- `TraceRing` capacity is bounded by `usize::MAX`; Kani harnesses bound capacity to reasonable limits (1–64) for exhaustiveness
- All `EvidenceBundle` fields are product types; Kani can construct arbitrary field combinations exhaustively
- `cargo +nightly miri` is available in CI; if not, `OBL-TRC-007` and `OBL-BND-007` degrade to `blocked_tooling`

---

## Waiver Candidates (not applied — documented for review)

| Potential waiver | Reason not applied | Owner decision |
|-----------------|-------------------|----------------|
| `OBL-EVN-001` (evidence_path format) — `required: false` | Low risk; unit-test already covers; not blocking release | Keep as planned |
| `OBL-EVN-002` (bundle_path format) — `required: false` | Low risk; format is deterministic string concat | Keep as planned |
