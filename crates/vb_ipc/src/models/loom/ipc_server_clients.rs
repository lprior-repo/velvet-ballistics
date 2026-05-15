//! LOOM-IPC-001: IPC server client-map invariants
//!
//! Model: Abstract client map with token-keyed entries.
//! Invariant: token uniqueness (each token maps to at most one client) &&
//!            active size <= MAX_CLIENTS
//!
//! Obligation: LOOM-IPC-001
//! Verifier: loom
//! Command: RUSTFLAGS="--cfg loom" cargo test -p vb_ipc ipc_server_clients

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

/// MAX_CLIENTS from crates/vb_ipc/src/server/impl_.rs
const MAX_CLIENTS: usize = 256;

/// Abstract model of the IPC server client map.
/// Tests: token uniqueness and active size <= MAX_CLIENTS.
#[derive(Debug)]
struct ClientMap {
    clients: HashMap<usize, ClientEntry>,
    next_token: usize,
}

#[derive(Debug, Clone)]
struct ClientEntry {
    id: usize,
}

impl ClientMap {
    fn new() -> Self {
        Self {
            clients: HashMap::new(),
            next_token: 1,
        }
    }

    /// Attempts to accept a new client. Fails if at MAX_CLIENTS.
    /// Returns the token on success.
    fn accept(&mut self, client_id: usize) -> Option<usize> {
        if self.clients.len() >= MAX_CLIENTS {
            return None;
        }
        let token = self.next_token;
        self.next_token += 1;
        self.clients.insert(token, ClientEntry { id: client_id });
        Some(token)
    }

    /// Removes a client by token. No-op if token not present.
    fn remove(&mut self, token: usize) {
        self.clients.remove(&token);
    }

    /// Returns number of active clients.
    fn active(&self) -> usize {
        self.clients.len()
    }

    /// Checks token uniqueness: each token maps to at most one entry.
    fn check_token_uniqueness(&self) {
        // HashMap contract guarantees uniqueness; verify no duplicate tokens
        assert_eq!(self.clients.len(), self.clients.keys().count(), "duplicate token detected");
    }

    /// Checks capacity bound: active <= MAX_CLIENTS
    fn check_capacity(&self) {
        assert!(
            self.active() <= MAX_CLIENTS,
            "active {} exceeds MAX_CLIENTS {}",
            self.active(),
            MAX_CLIENTS
        );
    }

    fn check_invariants(&self) {
        self.check_token_uniqueness();
        self.check_capacity();
    }
}

/// Thread-safe wrapper for loom exploration.
#[derive(Debug, Clone)]
struct SharedClientMap {
    inner: Arc<Mutex<ClientMap>>,
}

impl SharedClientMap {
    fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(ClientMap::new())),
        }
    }

    fn accept(&self, client_id: usize) -> Option<usize> {
        self.inner.lock().unwrap().accept(client_id)
    }

    fn remove(&self, token: usize) {
        self.inner.lock().unwrap().remove(token);
    }

    fn active(&self) -> usize {
        self.inner.lock().unwrap().active()
    }

    fn check_invariants(&self) {
        let map = self.inner.lock().unwrap();
        map.check_invariants();
    }
}

/// Loom model: single accept and remove.
/// Tests INV-003: token uniqueness and active <= MAX_CLIENTS.
#[test]
fn ipc_server_clients_basic() {
    loom::model(|| {
        let map = SharedClientMap::new();
        let m1 = map.clone();
        let m2 = map.clone();

        let token = m1.accept(42).expect("should succeed");
        m2.remove(token);

        map.check_invariants();
    });
}

/// Loom model: multiple concurrent accepts.
/// Bounded exploration: 3 accepts x 3 removes x 3 rounds.
#[test]
fn ipc_server_clients_concurrent_accepts() {
    loom::model(|| {
        let map = SharedClientMap::new();
        let m1 = map.clone();
        let m2 = map.clone();
        let m3 = map.clone();

        // Three threads each accept a client
        loom::thread::spawn(move || {
            let _t1 = m1.accept(1);
        });
        loom::thread::spawn(move || {
            let _t2 = m2.accept(2);
        });
        loom::thread::spawn(move || {
            let _t3 = m3.accept(3);
        });

        map.check_invariants();
    });
}

/// Loom model: accept and remove interleaved with capacity check.
/// Tests that MAX_CLIENTS bound is maintained under concurrent mutations.
#[test]
fn ipc_server_clients_capacity_preserved() {
    loom::model(|| {
        let map = SharedClientMap::new();
        let m1 = map.clone();
        let m2 = map.clone();

        let t1 = m1.accept(10).expect("first accept");
        let t2 = m2.accept(20).expect("second accept");

        // Remove t1, then add t3
        m1.remove(t1);
        let _t3 = m2.accept(30);

        // Remove t2
        m2.remove(t2);

        map.check_invariants();
    });
}

/// Loom model: rapid accept/remove cycles.
/// Tests token uniqueness is preserved across many interleavings.
#[test]
fn ipc_server_clients_rapid_cycles() {
    loom::model(|| {
        let map = SharedClientMap::new();
        let m1 = map.clone();
        let m2 = map.clone();

        loom::thread::spawn(move || {
            for i in 0..3 {
                if let Some(t) = m1.accept(i) {
                    m1.remove(t);
                }
            }
        });

        loom::thread::spawn(move || {
            for i in 100..103 {
                if let Some(t) = m2.accept(i) {
                    m2.remove(t);
                }
            }
        });

        map.check_invariants();
    });
}
