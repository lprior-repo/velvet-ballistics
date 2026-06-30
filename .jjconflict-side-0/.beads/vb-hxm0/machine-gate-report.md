bead_id: vb-hxm0
phase: 11
attempt: 1-of-7

STATUS: PASS_WITH_DEFERRED_GLOBAL

Commands:
- rtk cargo test -p velvet-ballistics-workspace-tests --test vb_hxm0_acceptance_catalog: PASS (4 passed).
- rtk cargo check -p velvet-ballistics-workspace-tests: PASS.
- moon run velvet-ballistics:verify-standard: FAIL; first failure ignored fallible results in crates/vb_runtime/src/action_queue/tests/bounded_queue_tests.rs, crates/vb_storage/src/admission.rs, crates/vb_cli/src/app_impl.rs, crates/vb_cli/src/commands_ai_context.rs, crates/vb_ipc/src/client.rs, crates/vb_ipc/src/server/dispatch.rs. None are touched files.
- moon ci: FAIL; fmt debt in unrelated benches/tests/fuzz and unused variables in crates/vb_expr/src/eval.rs. None are touched files.
