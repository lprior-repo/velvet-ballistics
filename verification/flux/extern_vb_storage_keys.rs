// Extern companion for vb_w6po5 storage key refinements.
//
// Bead: vb-w6po5
//
// This file documents the production constant bindings. The actual Flux
// refinements in vb_w6po5_storage_key_refinements.rs use literal values
// that match these production constants. The literals are the binding:
// if a constant changes in production, the Flux file must be updated.
//
// PRODUCTION BINDING GATE:
//   verification/flux/vb_w6po5_storage_key_refinements.rs binds to:
//   crates/vb_storage/src/constants.rs (PREFIX_*, DIGEST_*, *_KEY_BYTES)
//
// Binding verified by: grep -c bool\[true\] verification/flux/vb_w6po5_storage_key_refinements.rs
//   (must return 0 — no vacuous bool[true] specs)
//
// Production constants (crates/vb_storage/src/constants.rs):
//   PREFIX_WORKFLOW_SOURCE  = 0x01   (pub)
//   PREFIX_COMPILED_IR      = 0x02   (pub)
//   PREFIX_RUN_HEADER       = 0x10   (pub)
//   PREFIX_RUN_EVENT        = 0x11   (pub)
//   PREFIX_RUN_SNAPSHOT     = 0x12   (pub)
//   PREFIX_BLOB             = 0x20   (pub)
//   PREFIX_INDEX_STATUS     = 0x30   (pub)
//   PREFIX_INDEX_WORKFLOW   = 0x31   (pub)
//   PREFIX_INDEX_ACTION     = 0x32   (pub)
//   DIGEST_BYTES            = 32     (pub)
//   JOURNAL_KEY_BYTES       = 17     (pub(crate))
//   DIGEST_KEY_BYTES        = 33     (pub(crate))
//   RUN_ONLY_KEY_BYTES      = 9      (pub(crate))
//   INDEX_STATUS_KEY_BYTES  = 18     (pub(crate))
//   INDEX_WORKFLOW_KEY_BYTES = 13    (pub(crate))
//   INDEX_ACTION_KEY_BYTES  = 13     (pub(crate))
