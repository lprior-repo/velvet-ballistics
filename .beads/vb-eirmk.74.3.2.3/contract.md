# Error validator functional traversal contract

Replace the six imperative production loops in the error-exhaustiveness BIN and parser with finite fallible iterators. Preserve check/oracle order, first write/parse error handling, first-seen distinct variant/mention order, lazy successful wildcard matching and all CLI diagnostics. No public API, dependency, test, assertion or gate changes.

Verify the unchanged 56 error-validator cases and combined 165-case suite, strict source Clippy, pinned formatting, source/function bounds, production-token scan and the unchanged integrity outcomes. Separate independent reviews must inspect the delta before delivery; closure source64d stays frozen.
