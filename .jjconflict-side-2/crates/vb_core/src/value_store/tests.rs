use super::{ObjectField, ValueStore};
use crate::errors::CoreError;
use crate::ids::SymbolId;
use crate::ids::{BlobId, ListId, ObjectId};
use crate::limits::{
    MAX_BLOB_BYTES_PER_VALUE, MAX_LIST_ITEMS_PER_VALUE, MAX_OBJECT_FIELDS_PER_VALUE,
    MAX_SYMBOL_BYTES_PER_VALUE,
};
use crate::value::{SlotValue, Taint};
use bytes::Bytes;

include!("tests/insert_bounds_and_accessors.rs");

include!("tests/adversarial_bdd.rs");

include!("tests/phase2_adversarial_and_arena_cap.rs");

include!("tests/security_regression.rs");

include!("tests/blackhat.rs");

include!("tests/additional_coverage_taint_blob_arena.rs");

include!("tests/additional_coverage_validators_default.rs");
