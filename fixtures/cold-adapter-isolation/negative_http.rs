// negative_http fixture for the cold-adapter-isolation scanner.
//
// This fixture is deliberately contaminated: it contains active
// `use hyper::...` and `use reqwest::...` imports. The scanner MUST
// report BOTH tokens at file:line and exit 1.
//
// Master quote: "HTTP and JSON are excluded from the v1 runtime core.
// Any future adapter must be a separate cold-path adapter crate and
// must not enter vb_core, vb_runtime, vb_storage, or vb_ipc."

#![forbid(unsafe_code)]
#![allow(dead_code)]

use hyper::body::Body;
use reqwest::Client;

#[derive(Debug, Clone, PartialEq, Eq)]
struct HttpAdapterShape {
    body: Body,
    client: Client,
}

fn build_http_adapter() -> HttpAdapterShape {
    HttpAdapterShape {
        body: Body::empty(),
        client: Client::new(),
    }
}
