// positive fixture for the cold-adapter-isolation scanner.
//
// This file is deliberately clean. It uses the canonical runtime-core
// types and paths but does NOT import any HTTP / JSON / YAML /
// adapter-only crate. The scanner must report zero active findings
// and exit 0.
//
// The fixture proves the scanner's happy path:
//   - no `use serde_json` / `use saphyr` / `use reqwest` / `use hyper` /
//     `use axum` / `use ureq` / `use attohttpc` / `use isahc` imports,
//   - no `extern crate serde_json` style imports,
//   - no allowlist markers are needed to pass,
//   - the names of forbidden crates may appear inside identifiers,
//     comments, or string literals without triggering the scanner.

#![forbid(unsafe_code)]
#![allow(dead_code)]

use vb_core::types::event::Event;
use vb_core::types::resource::ResourceId;

#[derive(Debug, Clone, PartialEq, Eq)]
struct CleanLocal {
    event: Event,
    resource: ResourceId,
}

// Strings and comments can mention forbidden tokens without triggering
// the scanner; the scanner only flags `use <forbidden>` /
// `extern crate <forbidden>` import statements.
#[allow(dead_code)]
const FORBIDDEN_DOCS_NOTE: &str = "runtime-core must not import http/json/yaml adapters";

fn make_event() -> Event {
    Event::default()
}
