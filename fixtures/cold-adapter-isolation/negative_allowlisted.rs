// negative_allowlisted fixture for the cold-adapter-isolation scanner.
//
// This fixture is deliberately contaminated: it contains an active
// `use reqwest::...` import, but the previous non-blank line is an
// allowlist marker (`# allow-cold-adapter: <reason>`). The scanner
// MUST consume the marker, report the line as `allowlisted:`, keep
// the violation OUT of the active count, and exit 0.
//
// Master quote: "HTTP and JSON are excluded from the v1 runtime core.
// Any future adapter must be a separate cold-path adapter crate and
// must not enter vb_core, vb_runtime, vb_storage, or vb_ipc."
//
// Per-line allowlist is reserved for narrow historical / build-config
// / verification context. Do not use it to hide active release /
// runtime violations.

#![forbid(unsafe_code)]
#![allow(dead_code)]

# allow-cold-adapter: historical example - reqwest shape is here to prove the allowlist path
use reqwest::Client;

#[derive(Debug, Clone, PartialEq, Eq)]
struct AllowlistedShape {
    client: Client,
}

fn build_allowlisted() -> AllowlistedShape {
    AllowlistedShape {
        client: Client::new(),
    }
}
