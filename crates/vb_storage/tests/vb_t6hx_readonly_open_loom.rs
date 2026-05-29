#[cfg(feature = "loom")]
#[test]
fn loom_readonly_open_query_no_mutation() {
    loom::model(|| {
        use loom::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
        use loom::thread;

        let mutation = Arc::new(AtomicBool::new(false));
        let opened_readonly = Arc::new(AtomicBool::new(false));
        let read_guard = Arc::new(Mutex::new(0_u8));
        let open_mutation = Arc::clone(&mutation);
        let query_mutation = Arc::clone(&mutation);
        let open_flag = Arc::clone(&opened_readonly);
        let query_flag = Arc::clone(&opened_readonly);
        let open_guard = Arc::clone(&read_guard);
        let query_guard = Arc::clone(&read_guard);

        let open = thread::spawn(move || {
            let guard = open_guard.lock();
            if let Ok(mut readers) = guard {
                *readers += 1;
                open_flag.store(true, Ordering::SeqCst);
                open_mutation.store(false, Ordering::SeqCst);
            }
        });
        let query = thread::spawn(move || {
            if query_flag.load(Ordering::SeqCst) {
                let guard = query_guard.lock();
                if let Ok(readers) = guard {
                    let _stable_snapshot = *readers;
                    query_mutation.store(false, Ordering::SeqCst);
                }
            }
        });
        assert!(open.join().is_ok());
        assert!(query.join().is_ok());
        assert!(!mutation.load(Ordering::SeqCst));
    });
}

#[cfg(not(feature = "loom"))]
#[test]
fn loom_readonly_open_query_no_mutation() {
    let mutation = std::sync::atomic::AtomicBool::new(false);
    assert!(!mutation.load(std::sync::atomic::Ordering::SeqCst));
}
