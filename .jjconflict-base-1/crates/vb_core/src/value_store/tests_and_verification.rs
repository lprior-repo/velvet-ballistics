// Tests and formal verification harnesses for value_store.
// 
// This module is conditionally compiled and does not contribute to the production build.


#[cfg(test)]
mod verification_tests {
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
    include!("tests_and_verification/tests_basic.rs");
    include!("tests_and_verification/tests_adversarial.rs");
    include!("tests_and_verification/tests_security_blackhat.rs");
    include!("tests_and_verification/tests_additional_coverage.rs");
}
