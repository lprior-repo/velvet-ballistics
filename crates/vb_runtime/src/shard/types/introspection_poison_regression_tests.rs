// ============================================================================
// Regression tests for RA-014: lock_admission permanently bricks shard
// admission on mutex poison.
//
// Before the fix, `IntrospectionRegistry::register` / `unregister` /
// `unregister_all` / `register_with_overlap_policy` all used the pattern
//   `self.inner.lock().map_err(|_| RuntimeError::JournalPoisoned)?`
// which discards the poisoned guard. Once poisoned, every subsequent call
// hits a still-poisoned mutex and returns `Err(JournalPoisoned)` forever,
// permanently disabling the admission gate for that shard.
//
// After the fix, the registry recovers the inner data via
// `PoisonError::into_inner()` and continues accepting admissions. The
// follow-up tests pin down that behavior.
// ============================================================================
#[cfg(test)]
mod introspection_poison_regression_tests {
    use super::*;
    use crate::RuntimeError;
    use std::panic::AssertUnwindSafe;
    use std::sync::Arc;
    use std::thread;

    /// Spawns a thread that locks the shared mutex and panics, leaving the
    /// mutex poisoned. `catch_unwind` swallows the panic so the test thread
    /// is not poisoned. The function returns once the spawned thread has
    /// finished and the mutex is verifiably poisoned.
    fn poison_arc_mutex<T: Send + 'static>(mutex: Arc<std::sync::Mutex<T>>) {
        let handle = thread::spawn(move || {
            // Lock and immediately panic to poison the mutex. The panic
            // itself is the source of the poison; we use `catch_unwind` to
            // keep the test process alive.
            let _guard = mutex.lock().expect("acquire lock to poison");
            panic!("intentional poison for RA-014 regression test");
        });
        let result = std::panic::catch_unwind(AssertUnwindSafe(move || {
            // Wait for the spawner to finish panicking.
            let _ = handle.join();
        }));
        // The spawned thread panicked; that is the intended outcome.
        let _ = result;
    }

    #[test]
    fn register_recovers_after_mutex_poison() {
        // Given: a fresh registry whose inner mutex is poisoned.
        let mut registry = IntrospectionRegistry::new();
        poison_arc_mutex(registry.inner_arc_for_test());

        // When: register is called after the poison.
        let result = registry.register(RunId::new(1));

        // Then: registration succeeds because the registry recovers
        // the poisoned guard rather than returning JournalPoisoned.
        assert!(
            result.is_ok(),
            "register must recover from mutex poison, got {result:?}"
        );
        assert!(registry.is_visible(RunId::new(1)));
    }

    #[test]
    fn register_with_overlap_recovers_after_mutex_poison() {
        let mut registry = IntrospectionRegistry::new();
        poison_arc_mutex(registry.inner_arc_for_test());

        let result = registry.register_with_overlap_policy(RunId::new(7));
        assert!(
            result.is_ok(),
            "register_with_overlap_policy must recover, got {result:?}"
        );
    }

    #[test]
    fn unregister_recovers_after_mutex_poison() {
        let mut registry = IntrospectionRegistry::new();
        poison_arc_mutex(registry.inner_arc_for_test());

        let result = registry.unregister(RunId::new(3));
        assert!(result.is_ok(), "unregister must recover, got {result:?}");
    }

    #[test]
    fn unregister_all_recovers_after_mutex_poison() {
        let mut registry = IntrospectionRegistry::new();
        poison_arc_mutex(registry.inner_arc_for_test());

        let result = registry.unregister_all();
        assert!(
            result.is_ok(),
            "unregister_all must recover, got {result:?}"
        );
    }

    #[test]
    fn admission_continues_after_poison_recovery() {
        // Given: a registry with one registered run, then a poison event.
        let mut registry = IntrospectionRegistry::new();
        let initial = registry.register(RunId::new(100));
        assert!(initial.is_ok(), "initial register must succeed");

        // Drop the handle so the run state can be re-registered later.
        drop(initial);

        poison_arc_mutex(registry.inner_arc_for_test());

        // When: we register again after the poison.
        let after_poison = registry.register(RunId::new(101));

        // Then: registration succeeds and the second run is visible.
        assert!(
            after_poison.is_ok(),
            "post-poison register must succeed, got {after_poison:?}"
        );
        assert!(registry.is_visible(RunId::new(101)));
    }

    #[test]
    fn register_rejects_run_already_exists_after_poison_recovery() {
        // Given: a registry with an actively-held registration, poisoned,
        // then we try to register the same run again. The handle stays
        // alive so the registry still holds the prior mapping.
        let mut registry = IntrospectionRegistry::new();
        let first = registry.register(RunId::new(42));
        assert!(first.is_ok(), "first register must succeed");

        poison_arc_mutex(registry.inner_arc_for_test());

        // When: we attempt to register the same run again post-recovery.
        let dup = registry.register(RunId::new(42));

        // Then: recovery exposes the actual state, so the duplicate
        // detection still rejects the second registration with the typed
        // RunAlreadyExists error rather than a misleading JournalPoisoned.
        assert!(
            matches!(dup, Err(RuntimeError::RunAlreadyExists)),
            "post-recovery duplicate must surface RunAlreadyExists, got {dup:?}"
        );

        // Keep `first` alive until the assertions are done so the handle's
        // RAII Drop does not race with the recovery test.
        drop(first);
    }
}
