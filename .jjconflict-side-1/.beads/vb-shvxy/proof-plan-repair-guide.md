# Proof Plan Repair Guide: vb-shvxy

## Review Result: APPROVED (with minor repairs)

The plan is approved. Three non-blocking findings require repair at the identified artifact and state.

### REPAIR-001: Reset waiver review_status (FIND-001)
- **Artifact**: waiver-candidates.jsonl
- **State to rerun**: 4 (proof-plan-reviewer overrides accepted; planner should reset)
- **Fix**: Change `review_status` from `approved` to `candidate` for WC-001 and WC-002.
- **Justification**: Waiver candidates must not self-stamp reviewer fields. The reviewer independently accepts both waivers.
- **Minimal state**: No rerun needed — reviewer override is sufficient.

### REPAIR-002: Fix seed behavior_affecting flag (FIND-002)
- **Artifact**: proof-seeds.jsonl
- **State to rerun**: 3 (rust-contract)
- **Fix**: Set `behavior_affecting` to `false` for all 7 seeds (vb-shvxy-seed-001 through vb-shvxy-seed-007).
- **Justification**: This bead restores tooling infrastructure. No production Rust behavior is affected. The proof-planner correctly set obligations to behavior_affecting: false.
- **Minimal state**: 3

### REPAIR-003: Align traceability seed IDs (FIND-003)
- **Artifact**: traceability-matrix.jsonl
- **State to rerun**: 3 (rust-contract)
- **Fix**: Replace `PS-SHVXY-00X` references with canonical seed IDs from proof-seeds.jsonl (vb-shvxy-seed-001 through vb-shvxy-seed-007).
- **Justification**: Traceability requires verifiable ID references. Disjoint schemes break traceability mapping.
- **Minimal state**: 3

### Downstream Notes

- PO-006 and PO-007 reference `scripts/guard-zero-tests.sh` which must be created by proof-writer at State 5.
- PO-012K/012F/012P/012C/012L are closure obligations (owner_state: 10, rerun_from: 10) and should not be executed at State 6.
- Miri lane is not in scope for this bead. If future seeds add unsafe/FFI risk tags, a Miri lane decision must be added.
