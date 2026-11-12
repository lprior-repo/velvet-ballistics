# Proof-to-Rust Bridge Repair Guide: vb-shvxy State 7

repair_guide_for: proof-to-rust-review.md (attempt2)
review_status: REJECTED
reviewer_invocation_id: vb-shvxy-state7-proof-reviewer-attempt2
target_state: 7
repair_owner: femdation controller

## Repair Instructions

### BLOCKER-1: BRIDGE-SHVXY-001 — Remove nonexistent verify-flux reference

**What's broken**: PO-004 map row and RRO-004 JSONL reference non-existent `.moon/tasks/tooling.yml::verify-flux` (map) and `.moon/tasks/flux.yml::verify-flux-smoke` (JSONL).

**Fix in proof-to-rust-map.md**:
```
Change PO-004 source_refs from:
  `scripts/flux-check-package.sh::flux_package_check`, `.moon/tasks/tooling.yml::verify-flux`
To:
  `scripts/flux-check-package.sh::flux_package_check`, `planned:.moon/tasks/tooling.yml::verify-flux`
```

**Fix in rust-refinement-obligations.jsonl** (RRO-004):
```
Change source_refs from:
  ["scripts/flux-check-package.sh::flux_package_check", ".moon/tasks/flux.yml::verify-flux-smoke"]
To:
  ["scripts/flux-check-package.sh::flux_package_check", "planned:.moon/tasks/tooling.yml::verify-flux"]
```

**Rationale**: The `flux-check-package.sh` script is the primary source ref. The planned moon task ref documents what will be built in State 11 but doesn't exist yet. The `planned:` prefix makes this explicit.

### BLOCKER-2: BRIDGE-SHVXY-002 — Fix verify-kani file path and sync artifacts

**What's broken**: 
- Map PO-001/PO-002 use `tooling.yml::verify-kani` but file is `kani.yml`
- JSONL RRO-001/RRO-002 still use `kani.yml::verify-kani-inventory` (task name wrong)

**Fix in proof-to-rust-map.md** (PO-001, PO-002):
```
Change source_refs from:
  `scripts/kani-list.sh::kani_list_inventory`, `.moon/tasks/tooling.yml::verify-kani`
To:
  `scripts/kani-list.sh::kani_list_inventory`, `planned:.moon/tasks/kani.yml::verify-kani-inventory`
```

**Fix in rust-refinement-obligations.jsonl** (RRO-001, RRO-002):
```
Change source_refs from:
  ["scripts/kani-list.sh::kani_list_inventory", ".moon/tasks/kani.yml::verify-kani-inventory"]
To:
  ["scripts/kani-list.sh::kani_list_inventory", "planned:.moon/tasks/kani.yml::verify-kani-inventory"]
```

**Rationale**: `verify-kani-inventory` is planned (open decision #3 in the map). The existing `verify-kani` task runs execution harnesses — a distinct inventory task should be created. The `planned:` prefix makes this explicit. The file is `kani.yml` (exists), not `tooling.yml` (doesn't exist).

### BLOCKER-3: BRIDGE-SHVXY-003 — Update fuzz target count to 58

**What's broken**: All references say 57; actual count is 58.

**Fix in proof-to-rust-map.md**:
1. PO-008 claim: `(57 targets)` → `(58 targets)`
2. PO-008 evidence ref: `57_targets_evidence` → `58_targets_evidence`
3. PO-009 evidence ref: `all_57_compile_evidence` → `all_58_compile_evidence`

**Fix in rust-refinement-obligations.jsonl**:
1. RRO-008 `refinement_claim`: `57 registered` → `58 registered`
2. RRO-008 `refinement_harness_refs`: `57_targets_evidence` → `58_targets_evidence`
3. RRO-009 `refinement_claim`: `all 57 fuzz targets` → `all 58 fuzz targets`
4. RRO-009 `refinement_harness_refs`: `all_57_compile_evidence` → `all_58_compile_evidence`

### WARN-4: BRIDGE-SHVXY-009 — Synchronize map and JSONL source_refs

After applying BLOCKER-1 and BLOCKER-2 fixes above, verify that PO-001, PO-002, PO-004 map rows match their corresponding RRO-001, RRO-002, RRO-004 JSONL rows. All source_refs arrays must be identical between the two artifacts.

### WARN-5: BRIDGE-SHVXY-004 — Ground conceptual labels

Optionally (deferrable to State 11): add `# CANONICAL_MARKER:` comments to `scripts/kani-list.sh`, `scripts/flux-check-package.sh`, `scripts/guard-zero-tests.sh`, and `scripts/loom-list.sh` so that the `::` annotations in source_refs map to actual script-internal markers.

### WARN-6: BRIDGE-SHVXY-005 — Note workspace divergence

Optionally (deferrable to State 11): add `workspace:` prefix to source_refs for `scripts/guard-zero-tests.sh` and `scripts/loom-list.sh` until they are committed to the source checkout.

### Verifying fixes

After applying fixes, run these verification commands:
```bash
# Verify no tooling.yml or flux.yml references remain
rg "tooling.yml|flux.yml" .beads/vb-shvxy/proof-to-rust-map.md .beads/vb-shvxy/rust-refinement-obligations.jsonl

# Verify no 57 remains in fuzz references
rg "57_targets\|all_57" .beads/vb-shvxy/proof-to-rust-map.md .beads/vb-shvxy/rust-refinement-obligations.jsonl

# Verify 58 is used in all fuzz references
rg "58_targets\|all_58\|58 registered\|58 fuzz" .beads/vb-shvxy/proof-to-rust-map.md .beads/vb-shvxy/rust-refinement-obligations.jsonl

# Verify map and JSONL source_refs are consistent
# RRO-001 == PO-001, RRO-002 == PO-002, RRO-004 == PO-004
```
