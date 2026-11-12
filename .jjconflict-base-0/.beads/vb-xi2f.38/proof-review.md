# Proof Review: vb-xi2f.38 — P1: digest covers collect semantics

**reviewer_invocation_id**: proof-reviewer-vb-xi2f.38
**reviewer_skill**: proof-reviewer
**review_date**: 2026-05-24
**bead**: vb-xi2f.38
**source_checkout**: /home/lewis/src/velvet-ballistics
**isolated_workspace**: /home/lewis/src/vb-xi2f.38-ws

---

## Provenance Verification

- `agent-invocation-ledger.jsonl`: 1 entry (vb-xi2f.38-invoke-1), GENESIS origin, skill=go-skill ✅
- `proof-plan-review.md`: APPROVED at state 6 after repair response (Attempt 3) ✅
- `verifier-lane-review.jsonl`: 26 rows, proof-plan-reviewer reviewed all lanes ✅
- **Self-approval check**: reviewer_invocation_id = "proof-reviewer-vb-xi2f.38", not matching any planner or proof-writer ID ✅

---

## Lethal Findings

### FINDING-001: Kani Harness Does Not Call Production Code (CRITICAL)

**severity**: critical
**obligation**: PO-002, PO-020
**artifact**: `verification/kani/collect_field_coverage.rs:247-269`
**evidence**:
The harness defines a LOCAL function `digest_primitive()` which is a copy of the buggy code:
```rust
fn digest_primitive(hasher: &mut blake3::Hasher, primitive: &StepPrimitive) {
    match primitive {
        // ... explicit arms for Set, Finish ...
        other => {
            // BUG: catch-all only hashes name, not fields
            let name = canonical_primitive_name(other);
            hasher.update(name.as_bytes());
        }
    }
}
```
This is NOT a call to `digest_step_primitive` at `part_05.rs:140-162`.

**Required fix**: Import and call `vb_compile::mod_compile_lowering::part_05::digest_step_primitive` directly, or restructure so the harness invokes the actual production function under test.

**GOD RULE violation**: GOD RULE 2 requires proofs bind to actual Rust implementations. A local copy of the (buggy) function proves nothing about production code correctness.

---

### FINDING-002: Kani/Verus BLOCKED_TOOLING Without Formal Waiver (HIGH)

**severity**: high
**obligation**: PO-002, PO-011, PO-013, PO-015, PO-016, PO-020
**artifact**: `trusted-base-ledger.jsonl` TBL-xi2f.38-019, TBL-xi2f.38-020
**evidence**:
```
TBL-xi2f.38-019: Verus toolchain not verified installed (BLOCKED_TOOLING, risk: high)
TBL-xi2f.38-020: Kani workspace with --features verified not verified configured (BLOCKED_TOOLING, risk: high)
```
But `waiver-candidates.jsonl` has NO waiver entries for these tooling blockers.

Per `proof-reviewer` skill rules: "Reject PENDING_FORMAL_EXECUTION without cheap smoke/typecheck evidence." These obligations are in `status: planned` with tooling blocked, meaning the formal verification never ran and no waiver excuses it.

**Required fix**: Either (a) install and verify toolchains and re-run formal verification, or (b) add formal waiver entries to `waiver-candidates.jsonl` with explicit compensating evidence from alternative verification lanes.

---

### FINDING-003: Verus Spec Disconnected from Implementation (HIGH - TBL-xi2f.38-011)

**severity**: high
**obligation**: PO-011
**artifact**: `verification/verus/collect_lowering.rs`
**evidence**:
The Verus file contains spec functions (`spec_collect_start_fields`, `lemma_lower_canonical_collect_emits_4_nodes`) that are purely mathematical models. They do NOT call `lower_canonical_collect` from `part_03.rs:159-212`.

The trust ledger entry TBL-xi2f.38-011 explicitly states:
> "Verus spec functions are mathematical models not directly bound to Rust implementation functions" (risk: high, waiver: needs_proof_reviewer_approval)

GOD RULE 2: "No Vacuum Verus Proofs: Verus `proof fn` and `spec fn` models MUST mathematically bind to the actual Rust implementations (`exec fn`) inside the production codebase."

**No waiver approval found** for this disconnection. The `waiver-candidates.jsonl` has no entry for this.

**Required fix**: Rewrite Verus specs to call actual `lower_canonical_collect` with `requires`/`ensures` contracts, OR obtain explicit proof-reviewer waiver with compensating evidence.

---

### FINDING-004: TLA+ CollectDigestCoverage Invariant Absent (HIGH)

**severity**: high
**obligation**: PO-001
**artifact**: `verification/tla/collect_body_model.tla`
**evidence**:
The TLA+ model file contains NO `CollectDigestCoverage` invariant. The comment at line 90-93 says:
> "PO-001: CollectDigestCoverage - POST-FIX model. The digest function BLAKE3(...) ensures different Collect field values produce different digests. Modeled symbolically here; proven by Kani exhaustively."

But the actual invariants present are only:
- `NodeCountInvariant` (line 106)
- `OffsetInvariant` (line 111)
- `NodeKindInvariant` (line 122)
- `NoOverflowInvariant` (line 132)
- `TypeOK` (line 137)
- `LoweringDeterminism` (line 141)

NONE of these prove that different Collect fields produce different digests. `LoweringDeterminism` only proves the 4-node sequence structure, not digest content-addressing.

The `proof-evidence.md` line 107 claims `CollectDigestCoverage` is an invariant in the TLA+ model, but it is not present.

**Required fix**: Add `CollectDigestCoverage` invariant to TLA+ model that explicitly verifies different Collect field values (variable, source, pages, items, body) contribute to the digest state.

---

### FINDING-005: Proptest Evidence Absent (HIGH)

**severity**: high
**obligation**: PO-003, PO-004, PO-005, PO-006, PO-007
**artifact**: `crates/vb_compile/src/tests/digest_collect_tests.rs`
**evidence**:
`proof-evidence.md` line 149-153 shows:
```markdown
### Proptest (PENDING)
```bash
cargo test -p vb_compile digest_collect -- --nocapture
```
**Expected**: All tests pass showing different fields → different digests
```
Status is PENDING, meaning the test was never executed. No test output log exists.

**Required fix**: Run proptest and provide actual test output showing all digest_collect tests pass.

---

### FINDING-006: PO-013 Harness Uses Hardcoded Data (MEDIUM - TBL-xi2f.38-013)

**severity**: medium
**obligation**: PO-013
**artifact**: `verification/kani/collect_try_from_parts.rs:35-82`
**evidence**:
The harness constructs a hardcoded 4-node Collect workflow:
```rust
let nodes = vec![
    CompiledNode { id: StepIdx::new(0), kind: CompiledNodeKind::CollectStart { source: source_slot, limit: 10, page_size: 5, ... }, ... },
    // ... 3 more hardcoded nodes ...
];
```
No `kani::any()` is used. TBL-xi2f.38-013 notes: "Waiver requested: harness proves panic-freedom for specific valid IR, not exhaustively."

But no waiver exists in `waiver-candidates.jsonl`.

**Required fix**: Either implement `kani::Arbitrary` for `WorkflowParts` and use `kani::any()` for exhaustive testing, or add formal waiver.

---

### FINDING-007: PO-020 kani_harness_uses_any Vacuous (MEDIUM - TBL-xi2f.38-017)

**severity**: medium
**obligation**: PO-020
**artifact**: `verification/kani/collect_field_coverage.rs:223-239`
**evidence**:
```rust
fn kani_harness_uses_any() {
    let collect = kani::any::<StepPrimitive::Collect>();
    let _var = &collect.variable;
    // ... access all fields ...
    kani::assert(true, "GOD RULE: kani::any() generates all Collect fields");
}
```
`kani::assert(true, ...)` always passes. This harness does NOT verify that OTHER harnesses (like `kani_collect_different_pages_different_digest`) use `kani::any()`. It only verifies that `kani::any::<StepPrimitive::Collect>()` produces valid instances — which is trivially true.

**Required fix**: Either (a) make this harness actually inspect other harness functions for hardcoded data, or (b) accept as code-inspection-only and note that GOD RULE enforcement is manual.

---

### FINDING-008: TLA+ StepIdCoverage Uses Length-Only Model (MEDIUM - TBL-xi2f.38-007)

**severity**: medium
**obligation**: PO-008
**artifact**: `verification/tla/collect_body_model.tla`
**evidence**:
TBL-xi2f.38-007 states: "Step ID modeled as length 1..64 integer instead of actual byte content" (risk: medium). The digest contribution is modeled as length only, not actual BLAKE3 of the ID string.

**Acceptable with trust marker** — this is documented in trusted-base-ledger. TLC still exhaustively checks the bounded model.

---

## Non-Vacuity Checks

### TLA+ Counterexample Probe
The TLA+ model checks structural invariants (node count, offsets, kinds) but NOT digest field coverage. This is a weak bound — the model cannot detect whether `digest_step_primitive` correctly hashes Collect fields.

### Kani Non-Vacuity
BLOCKED_TOOLING — cannot verify non-vacuity.

### Verus Non-Vacuity
BLOCKED_TOOLING — cannot verify non-vacuity.

### Proptest Non-Vacuity
PENDING — no evidence of execution.

---

## Waiver Validation

| Waiver ID | Description | Status |
|-----------|-------------|--------|
| WC-001 | Flux not_applicable | ✅ valid |
| WC-002 | Loom not_applicable | ✅ valid |
| WC-003 | Miri not_applicable | ✅ valid |
| WC-004 | Fuzz not_applicable | ✅ valid |
| TBL-xi2f.38-011 | Verus spec disconnection | ❌ needs_proof_reviewer_approval — NO approval found |
| TBL-xi2f.38-013 | PO-013 hardcoded harness | ❌ waiver requested but no waiver entry |
| TBL-xi2f.38-019 | Verus BLOCKED_TOOLING | ❌ no waiver entry |
| TBL-xi2f.38-020 | Kani BLOCKED_TOOLING | ❌ no waiver entry |

---

## Trust Marker Scan

- **TBL-xi2f.38-001**: TLA+ symbolic digest (integer addition) — acceptable model simplification
- **TBL-xi2f.38-002**: String bounds 256 chars — acceptable for model checking tractability
- **TBL-xi2f.38-003, 004, 005**: Kani harness bounds — acceptable
- **TBL-xi2f.38-006**: blake3/postcard trusted base — acceptable
- **TBL-xi2f.38-007, 008**: Step ID / Trigger modeled as length only — documented, acceptable
- **TBL-xi2f.38-009, 010**: blake3/postcard trusted base — acceptable
- **TBL-xi2f.38-011**: Verus spec disconnection — **UNAPPROVED WAIVER** ❌
- **TBL-xi2f.38-012**: Integration test trusted — acceptable
- **TBL-xi2f.38-013**: PO-013 hardcoded harness — **WAIVER MISSING** ❌
- **TBL-xi2f.38-014, 015**: Kani bounds — acceptable
- **TBL-xi2f.38-016**: TLA+ lowering model — acceptable
- **TBL-xi2f.38-017**: PO-020 vacuous — documented but insufficient ❌
- **TBL-xi2f.38-018**: TLC model check result — acceptable, but 20 states is very small
- **TBL-xi2f.38-019**: Verus BLOCKED_TOOLING — **NO WAIVER** ❌
- **TBL-xi2f.38-020**: Kani BLOCKED_TOOLING — **NO WAIVER** ❌

---

## Summary

| Category | Count |
|----------|-------|
| Total obligations | 21 |
| Obligations with BLOCKED_TOOLING | 6 (PO-002, PO-011, PO-013, PO-015, PO-016, PO-020) |
| Obligations with NO execution evidence | 7 (PO-001, PO-003..007, PO-011, PO-013) |
| Critical findings | 1 (FINDING-001: Kani harness doesn't call production code) |
| High findings | 4 (FINDING-002, 003, 004, 005) |
| Medium findings | 3 (FINDING-006, 007, 008) |
| Unapproved waivers | 4 |
| Missing waivers | 2 |

---

## Decision

**STATUS: REJECTED**

**Reason**: Multiple critical and high-severity findings prevent approval:

1. **FINDING-001 (CRITICAL)**: Kani harness for PO-002 does not call production `digest_step_primitive` — it re-implements a copy of the (buggy) code locally. This violates GOD RULE 2 and proves nothing about production correctness.

2. **FINDING-002 (HIGH)**: 6 obligations (PO-002, PO-011, PO-013, PO-015, PO-016, PO-020) have Kani/Verus BLOCKED_TOOLING but no formal waivers in `waiver-candidates.jsonl`.

3. **FINDING-003 (HIGH)**: PO-011 Verus spec is disconnected from implementation (TBL-xi2f.38-011) but has no approved waiver.

4. **FINDING-004 (HIGH)**: TLA+ `CollectDigestCoverage` invariant is referenced in comments but does not exist in the model.

5. **FINDING-005 (HIGH)**: Proptest evidence (PO-003..PO-007) is PENDING with no execution output.

**Cannot approve** until these findings are resolved. The proof artifacts do not provide non-vacuous evidence that the digest bug is fixed in production code.

---

*Review by proof-reviewer-vb-xi2f.38. Bead vb-xi2f.38 state 6.*
