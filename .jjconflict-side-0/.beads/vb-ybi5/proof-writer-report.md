STATUS: COMPLETE

Changed verification artifact `crates/vb_storage/src/kani_recovery_hydrate.rs` only. Replaced two ignored `Err(_) => {}` arms with explicit unexpected-error branches that assert false. Repaired digest construction/imports so the harness no longer relies on private fields or nonexistent `WorkflowDigest::ZERO`.
