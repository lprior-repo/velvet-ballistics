#![forbid(unsafe_code)]
//! Cold, keyed, explicitly budgeted carrier wire. No append or runtime authority.
mod budget;
mod cursor;
mod decode;
mod encode;
mod error;
mod header;
mod integrity;
mod owned;
mod prefix;

pub use budget::{ComponentAllowance, ContinuationCarrierBudget};
pub use decode::decode_owned_continuation;
pub use encode::encode_owned_continuation;
pub use error::CarrierError;
pub use header::ContinuationCarrierHeader;
pub use owned::{
    BoundedContinuationRecord, OwnedContinuationCarrier, OwnedContinuationCarrierView,
};
pub use vb_core::value_store::transport::TransportUsage as CarrierUsage;

/// Cap on the entire new-kind record, including the 60-byte envelope.
pub const MAX_CONTINUATION_RECORD_BYTES: usize = 1_048_576;
