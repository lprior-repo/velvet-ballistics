// negative fixture for the cold-adapter-isolation scanner.
//
// This fixture is deliberately contaminated: it contains an active
// `use serde_json::...` import. The scanner MUST report at least one
// file:line finding naming `serde_json` and exit 1.
//
// Master quote: "HTTP and JSON are excluded from the v1 runtime core.
// Any future adapter must be a separate cold-path adapter crate and
// must not enter vb_core, vb_runtime, vb_storage, or vb_ipc."

#![forbid(unsafe_code)]
#![allow(dead_code)]

use serde_json::{json, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
struct ForbiddenShape {
    payload: Value,
}

fn build_forbidden_payload() -> ForbiddenShape {
    ForbiddenShape {
        payload: json!({ "kind": "contaminated" }),
    }
}
