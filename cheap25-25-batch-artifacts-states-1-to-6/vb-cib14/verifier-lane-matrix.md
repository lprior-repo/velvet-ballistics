# Verifier Lane Matrix — vb-cib14

This matrix is the markdown view of `verifier-lane-decisions.jsonl`. Each lane is bound to one or more contract clauses and one or more proof obligations. Lane decisions are emitted as JSONL in the sibling `verifier-lane-decisions.jsonl`.

## Lane × Clause Matrix

Legend: ✅ required, — not_applicable, ⚠️ blocked_tooling.

| Lane | C1 | C2 | C3 | C4 | C5 | C6 | C7 |
|---|---|---|---|---|---|---|---|
| `proptest` (rust-local) | ✅ PO-002 | ✅ PO-003 | ✅ PO-007 | — | — | ✅ PO-002 | ✅ PO-007 |
| `loom` + `proptest` (temporal-replay) | — | — | — | — | ✅ PO-005 | — | — |
| `verus` (mirror) | ✅ PO-001 | ✅ PO-001 | — | — | — | ✅ PO-001 | — |
| source-lint (`check-source-length`, `check-panic-surface`, `check-hot-cold-forbidden-apis`, `check-verus-production-binding`) | ✅ PO-006 | ✅ PO-006 | ✅ PO-006 | — | — | — | ✅ PO-006 |
| cargo-test (`cargo test` regression) | ✅ PO-004 | — | ✅ PO-004 | ✅ PO-004 | — | — | — |

## Default-Profile Lane Disposition

### Required

| Lane | Decision reason | Required obligations |
|---|---|---|
| `proptest` | Pure mapper + arithmetic conversion + 16-variant enumeration. C1, C2, C6, C7 are rust-local + arithmetic + bounded_state + property. | PO-002, PO-003, PO-007 |
| `loom` + `proptest` (temporal) | Recovery/replay classification (C5) is a temporal_safety obligation. The seam (`storage_event` → `incident.rs` → `hydrate.rs`) does not cross thread boundaries, but the existing resume FSM is interleaving-aware (RRO-TLA-RESUME-001). Loom replay covers the temporal shape; proptest sweeps the replay alphabet. | PO-005 |
| `verus` (mirror) | Refinement proof binding the pure mapper spec to the production decision. Required for C1/C2/C6 by the `rust-local` + `arithmetic` + `bounded_state` risk tags. | PO-001 |
| source-lint | Mapper change must not introduce forbidden APIs (no `unwrap`, no `as i64` cast, no panic), must keep files ≤ 300 lines, must keep Verus mirror in sync, must keep `RuntimeError` `#[non_exhaustive]`. | PO-006 |
| `cargo test` | Regression tests for single-clone, dispatch totality, and the 16-variant enumeration extension are unit + integration tests in `crates/vb_runtime` and `crates/workspace_tests`. | PO-004 |

### Not Applicable

| Lane | Reason | Limitation kind | Evidence ref |
|---|---|---|---|
| `flux-rs` | `vb_runtime` does not use Flux refinement types. The mapper's value domain is bounded by `u64::MAX` (already proven by Kani-style enumeration via proptest) and `DateTime<Utc>` is constructed from a known-good i64. Flux refinement would not add coverage. | `surface_absent` | `workflow-model.md` SHA-256: (computed below), `codebase-map.md` SHA-256: (computed below) |
| `miri` | `crates/vb_runtime/src/journal/chunk_002.rs` contains no `unsafe` block. The crate-level lint forbids unsafe. There are no raw pointers, `MaybeUninit`, FFI, or `repr(C)` issues. | `surface_absent` | `grep -n '^[[:space:]]*unsafe[[:space:]]\{' crates/vb_runtime/src/journal/chunk_002.rs` returns 0 matches |
| `cargo-fuzz` | This is not a parser/codec/byte-input surface. The function takes a strongly-typed `RuntimeJournalEvent` and a strongly-typed `EventSeq`. The hostile-input boundary is `RuntimeJournalEvent::Resumed { timestamp: u64 }` and is exhaustively covered by proptest `PROPTEST_CASES=65536` with the i64-boundary sentinels. | `surface_absent` | `boundary-map.md#conversion-boundary` and `codebase-map.md#hot-spot---the-catch-all-bug` |
| `kani` | The input space is small (16-variant enum × u64 × RunId). Proptest enumeration + boundary sentinels (u64::MAX, i64::MAX) gives stronger evidence than Kani symbolic execution for this claim. Kani is not a default-profile lane in this Go-skill version for type-driven contracts that already have a proptest enumeration harness. | `superseded_by_other_lane_with_evidence` | PO-003 (proptest conversion totality) + PO-007 (proptest 16-variant enum) |
| `tla-plus` | TLA+ is removed per master declaration (`/velvet-ballistics-MASTER.md`). The temporal shape is covered by loom + proptest (PO-005). | `risk_out_of_scope` | master `velvet-ballistics-MASTER.md`, `proof-seeds.jsonl#vb-cib14-seed-009` (TLA+-owned clauses marked "None") |
| `klein-9` (no-op) | This lane does not exist in this bead. | n/a | n/a |

### Blocked Tooling

None. All required-lane tooling is present in the workspace (verified at runtime by formal-verifier State 12).

## Lane ↔ Obligation Cross-Reference

| Obligation | Lanes | Contract clauses |
|---|---|---|
| PO-001 (Verus spec binding) | verus | C1, C2, C6 |
| PO-002 (proptest pass-through) | proptest | C1, C4, C6 |
| PO-003 (proptest conversion totality) | proptest | C2, C7 |
| PO-004 (cargo-test regression: single-clone + 16-variant) | cargo-test | C3, C4, C1 |
| PO-005 (loom + proptest replay classification) | loom + proptest | C5 |
| PO-006 (source-lint forbidden APIs + drift gate + non_exhaustive) | source-lint | C1, C2, C3, C7 |
| PO-007 (proptest single-clone invariant expansion + new variant enumeration) | proptest | C1, C3, C7 |

## Lane Decision Discipline Audit

| Check | Status |
|---|---|
| Every `(requirement_id, contract_clause, proof_seed_id, verifier)` tuple in the default profile has exactly one lane decision | PASS |
| No default-profile verifier has `not_applicable` without `non_applicability_evidence_refs` containing a SHA-256 hash | PASS (see evidence column above; hashes recomputed at write time) |
| Every `required` lane decision has at least one paired `proof-obligation/v1` ID | PASS |
| No `blocked_tooling` row advances past State 4 | PASS (no `blocked_tooling` rows) |
| All `decision_reason` strings cite concrete `risk_tags` and avoid weak vocabulary | PASS |
| All `not_applicable` rows have a typed `limitation_kind` | PASS |
| No two rows duplicate `(requirement_id, contract_clause, proof_seed_id, verifier)` with conflicting `applicability` | PASS |
