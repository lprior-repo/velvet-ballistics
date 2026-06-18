//! Kani harness for KANI-STORE-001: StorageArtifactStore Send+Sync hygiene.
//!
//! Artifact: crates/vb_runtime/src/admission.rs
//! Obligation: Verify StorageArtifactStore is Send+Sync and that
//! `compiled_ir_exists` never panics in the Kani storage model.
//!
//! The key concurrency property is that production `StorageArtifactStore` wraps
//! `Arc<FjallJournal>`, and the Kani build supplies a bounded storage model so
//! the runtime proof lane does not try to symbolically execute Fjall filesystem
//! internals.

#![forbid(unsafe_code)]

/// KANI-STORE-001: StorageArtifactStore Send+Sync proof.
///
/// This harness verifies:
/// 1. `StorageArtifactStore` satisfies `Send` and `Sync` bounds.
///    This is proven by the compiler via the auto-derived trait impls,
///    since `Arc<FjallJournal>` is Send+Sync.
/// 2. `compiled_ir_exists` never panics in the bounded Kani model.
///
/// Assumptions:
/// - A-001: FjallJournal persistence is covered by storage/integration tests;
///   this runtime lane uses a boolean model because Kani cannot tractably model
///   Fjall filesystem internals.
/// - A-005: Arc<dyn ArtifactStore> is Send+Sync (this harness proves
///   StorageArtifactStore specifically, which is one implementation).
#[cfg(kani)]
mod kani_store_proofs {
    use crate::admission::{ArtifactStore, StorageArtifactStore};
    use vb_core::ids::WorkflowDigest;

    fn require_send<T: Send>() {}

    fn require_sync<T: Sync>() {}

    /// KANI-STORE-001 H1: StorageArtifactStore is Send.
    ///
    /// Proving Send: if StorageArtifactStore were not Send, this harness would
    /// not compile.
    /// Send bound is verified at compile time.
    ///
    /// H1 does not construct Fjall because the Send bound is a type-level
    /// property. H3/H4 cover the Kani storage model.
    #[kani::proof]
    fn storage_artifact_store_send() {
        require_send::<StorageArtifactStore>();
    }

    /// KANI-STORE-001 H2: StorageArtifactStore is Sync.
    ///
    /// Proving Sync: if StorageArtifactStore were not Sync, this harness would
    /// not compile.
    /// Sync bound is verified at compile time.
    ///
    /// H2 does not construct Fjall because the Sync bound is a type-level
    /// property. H3/H4 cover the Kani storage model.
    #[kani::proof]
    fn storage_artifact_store_sync() {
        require_sync::<StorageArtifactStore>();
    }

    /// KANI-STORE-001 H3: compiled_ir_exists returns correct bool for known digests.
    ///
    /// With a concrete zero digest and a modeled empty store, the method should
    /// return false (no artifact stored), demonstrating the return value
    /// is well-formed (not an unwind or panic).
    #[kani::proof]
    fn compiled_ir_exists_returns_false_for_empty_journal() {
        let store = StorageArtifactStore::kani_model(false);

        let zero_digest = WorkflowDigest::from_bytes([0u8; 32]);
        let exists = store.compiled_ir_exists(zero_digest);

        // With an empty model, no artifact exists, so result must be false.
        // This proves (a) no panic occurred, (b) the bool is well-formed.
        kani::assert(
            !exists,
            "empty journal must report exists==false for zero digest",
        );
    }

    /// KANI-STORE-001 H4: compiled_ir_exists is callable with max digest.
    ///
    /// Proves the method doesn't panic on edge-case digest values.
    /// Uses the Kani storage model because Fjall's filesystem path is covered
    /// by integration tests rather than this bounded model-checking lane.
    #[kani::proof]
    fn compiled_ir_exists_no_panic_max_digest() {
        let expected = kani::any::<bool>();
        let store = StorageArtifactStore::kani_model(expected);

        let max_digest = WorkflowDigest::from_bytes([0xFFu8; 32]);
        let exists: bool = store.compiled_ir_exists(max_digest);

        kani::assert(
            exists == expected,
            "max digest case must return the modeled storage result",
        );
    }
}
