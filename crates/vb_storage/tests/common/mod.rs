//! Shared journal fixture for single-journal proptests.
//!
//! nextest runs each test in its own process, so one `FjallJournal` per process
//! replaces a fresh database (12 keyspaces, fsyncs) per case. Cases stay isolated by
//! drawing a distinct run-id namespace from `fresh_scope()` for every case.

use std::sync::LazyLock;
use std::sync::atomic::{AtomicU64, Ordering};

use tempfile::TempDir;
use vb_storage::journal::FjallJournal;

struct SharedJournal {
    _dir: TempDir,
    journal: FjallJournal,
}

static SHARED: LazyLock<SharedJournal> = LazyLock::new(|| {
    let dir = tempfile::tempdir().expect("tempdir");
    let journal = FjallJournal::open(dir.path(), None).expect("journal open");
    SharedJournal { _dir: dir, journal }
});

static NEXT_SCOPE: AtomicU64 = AtomicU64::new(1);

pub(crate) fn shared_journal() -> &'static FjallJournal {
    &SHARED.journal
}

/// Fresh run-id namespace for one proptest case; OR run ids below 2^32 into it.
pub(crate) fn fresh_scope() -> u64 {
    NEXT_SCOPE.fetch_add(1, Ordering::Relaxed) << 32
}
