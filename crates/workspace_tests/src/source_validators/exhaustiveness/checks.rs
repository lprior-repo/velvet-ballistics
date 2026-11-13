// Error exhaustiveness check definitions.
// Maps each production error enum to its corresponding fuzz oracle locations.

use std::path::{Path, PathBuf};

pub(crate) struct Check {
    pub type_name: &'static str,
    pub enum_path: &'static str,
    pub oracles: &'static [Oracle],
}

pub(crate) struct Oracle {
    pub path: &'static str,
    pub function: &'static str,
}

pub(crate) const CHECKS: [Check; 3] = [
    Check {
        type_name: "JournalError",
        enum_path: "crates/vb_storage/src/error/mod.rs",
        oracles: &[
            Oracle {
                path: "fuzz/src/journal_target/errors.rs",
                function: "assert_typed_journal_error",
            },
            Oracle {
                path: "fuzz/fuzz_targets/decode_record.rs",
                function: "assert_typed_journal_error",
            },
            Oracle {
                path: "fuzz/fuzz_targets/journal_decode.rs",
                function: "assert_typed_journal_error",
            },
            Oracle {
                path: "fuzz/tests/proptest_journal_error_exhaustiveness.rs",
                function: "assert_known_journal_error",
            },
        ],
    },
    Check {
        type_name: "IpcError",
        enum_path: "crates/vb_ipc/src/error.rs",
        oracles: &[
            Oracle {
                path: "fuzz/src/ipc_target.rs",
                function: "assert_typed_ipc_error",
            },
            Oracle {
                path: "fuzz/tests/proptest_ipc_error_exhaustiveness.rs",
                function: "assert_known_ipc_error",
            },
        ],
    },
    Check {
        type_name: "ValidationError",
        enum_path: "crates/vb_validate/src/validation_errors.rs",
        oracles: &[Oracle {
            path: "fuzz/src/validation_target/verifier.rs",
            function: "assert_typed_validation_error",
        }],
    },
];

pub(crate) fn join_path(root: &Path, rel: &str) -> PathBuf {
    root.join(rel)
}
