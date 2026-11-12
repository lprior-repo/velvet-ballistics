bead_id: vb-ybi5
phase: 2
attempt: 1-of-7

Scoped files/APIs:
- `crates/vb_storage/src/kani_recovery_hydrate.rs`
- `check_action_abi_digests`, `check_policy_digests`, `check_compiled_ir_digest`
- `RecoveryError::{ActionAbiMismatch, PolicyDigestMismatch, CompiledIrDigestMismatch}`
- `WorkflowDigest::from_bytes`, `ActionId::new`, `StepIdx::new`

Scanner:
- `scripts/check-ignored-fallible-results.sh` scans `crates/*/src` and `xtask/src` for ignored fallible results.
