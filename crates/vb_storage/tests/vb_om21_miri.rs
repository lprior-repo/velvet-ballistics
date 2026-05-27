#![forbid(unsafe_code)]

// PO-vb-om21-key-parse-miri: direct integration-test shim so the exact
// planned command `cargo +nightly miri test -p vb_storage vb_om21_key_parse_miri`
// can discover the nested Miri artifact without changing its body.

#[path = "miri/vb_om21_key_parse_miri.rs"]
mod vb_om21_key_parse_miri;
