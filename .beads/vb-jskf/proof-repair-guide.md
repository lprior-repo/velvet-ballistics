# vb-jskf Proof Repair Guide

## Required repairs

1. **Kani admission sequence (`K01/KANI-PROP-007`)**: create a crate-discoverable harness over production `AcceptedArtifact`, `RunAccepted`/journal event, `RunId`, `EventSeq`, and digest structures. Use `kani::Arbitrary` or safe bounded generators. Assert sentinel/mismatch rejection and exact `accepted_at_seq == RunAccepted.seq` for same run.
2. **Kani pipeline (`K30`)**: replace fixed two-node workflow with bounded generated `WorkflowParts` and contract sets. Check all `validate_with_contracts` postconditions and typed error paths, not only `entry < nodes.len()`.
3. **Verus digest roles (`VERUS-DIG-004/005`)**: either bind specs to production exec/spec functions and concrete digest role wrappers, or keep the file as abstract non-admission evidence and require executable storage/runtime/CLI evidence for validation claims.
4. **Verus atomic admission (`VERUS-PRE-001..VERUS-ERR-006`)**: bind to production accepted artifact/admission structures and functions, or explicitly narrow proof claims to pure taxonomy/precondition modeling only.
5. **TLA `YamlE2eChain`**: replace unbounded/boolean abstractions with finite hardware/resource domains, typed parser/digest/resource failure transitions, and refinement mapping to production journal/storage/CLI traces.
6. **TLA `EngineYamlRecovery`**: model journal order, payload digest, snapshot/tail, finite sequence domains, and overflow/resource failures as typed fail-closed transitions.
7. **TLA `EngineYamlRunLifecycle`**: encode U64 bounded sequence arithmetic and explicit overflow-to-Err/suspend/fail transition instead of only guarding `seq < MaxSeq`.

## Rerun targets after repair

- `cargo kani ...` exact crate-discoverable harness names for repaired Kani artifacts.
- `verus verification/verus/yaml_e2e_digest_roles.rs` and/or production-bound Verus targets with raw output.
- `verus verification/verus/accepted_run_atomic_admission.rs` and/or production-bound Verus targets with raw output.
- `tlc -config verification/tla/YamlE2eChain.cfg verification/tla/YamlE2eChain.tla` after bounded model repair.
- `tlc -config verification/tla/EngineYamlRecovery.cfg verification/tla/EngineYamlRecovery.tla` after bounded recovery repair.
- `tlc -config verification/tla/EngineYamlRunLifecycle.cfg verification/tla/EngineYamlRunLifecycle.tla` after overflow transition repair.

STATUS: REJECTED
