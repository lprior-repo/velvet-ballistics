# Full checkpoint secret authority repair

Bead vb-eirmk.13.13.3.4.16.3.3; source-only WIP, parent r585 native two unchanged secret_authority failures; independent CODE/TEST and parent native required before acceptance.

Use shared existing secret metadata validate_runtime_roots on the reconstructed full checkpoint frame during private checkpoint structure validation; map missing root, wrong handle and wrong taint to InvalidNestedState. Preserve identity, dimensions, retry state, admission/contracts, nested DTO and later symbol-prefix error order. Preserve complete frame and all arena entries, IDs, capacities, collect state and attempt. No reconstruction from loose frame, no schema/public API/codec/value/test changes.

Initial production write fence: crates/vb_runtime/src/run_checkpoint/validation.rs only. Read-only diagnosis of shard recovery/continuation must establish the missing RunFinished event cause before extending this fence. Do not edit Lorentz artifact dispatch/version paths or peer IR3. All changed functions <=25 lines including signature/braces, max5 parameters, file<=300.

Required existing negative oracle: all three original missing-root/wrong-taint/wrong-handle variants reject direct validation and Recover with exact typed errors and unchanged journal/boundary/no publication. Required positive oracle: exact restored frame/store, retired unused allocations, secret Symbol1/Secret, attempt2 and exactly one RunFinished after continuation. Test bytes remain unchanged. Focused native unavailable in this inherited conflicted worktree; parent coherent-source rerun is required, never represented as author PASS.
