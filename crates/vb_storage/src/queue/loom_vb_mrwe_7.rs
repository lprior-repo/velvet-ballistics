#![cfg(all(test, loom, feature = "loom-vb-mrwe-7"))]
//! bead vb-mrwe.7 — OBL-CONCURRENCY-LOOM.
#[test]
fn vb_mrwe_7_queue_concurrency_loom() {
    loom::model(|| {
        use loom::sync::{Arc, Mutex};
        use loom::thread;
        let state = Arc::new(Mutex::new((0usize, false)));
        let a = Arc::clone(&state);
        let t1 = thread::spawn(move || {
            if let Ok(mut s) = a.lock() {
                if !s.1 && s.0 < 3 {
                    s.0 += 1;
                }
            }
        });
        let b = Arc::clone(&state);
        let t2 = thread::spawn(move || {
            if let Ok(mut s) = b.lock() {
                s.1 = true;
            }
        });
        let _ = t1.join();
        let _ = t2.join();
        if let Ok(s) = state.lock() {
            assert!(s.0 <= 3);
        }
    });
}
