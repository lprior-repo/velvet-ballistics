# Post integration canonical CI

Command: RUSTC_WRAPPER= CARGO_BUILD_JOBS=2 timeout240 moon ci --force --concurrency2 in isolated root mwutouml,20G high/24G max memory. Exit124, interrupted at4m1s. Reported12 completed,9failed,1skipped. Full raw log and current ciReport retained; no full CI pass or v1 readiness claim. Source integration checkpoint5601ba50.

Additional full CLI package attempt exited101 at compile: crates/velvet_ballistics/tests/ir_artifact_admission.rs WorkflowParts fixture lacks input_slots. Targeted reviewed27tests remaingreen; raw cli-package.log. This compile failure blocks full CLI test execution, no runtime result inferred.
