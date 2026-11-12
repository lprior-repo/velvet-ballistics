//! Allocation-free checkpoint inspection and serialization for cold transport.
mod budget;
mod cursor;
mod scan;
mod view;
pub use budget::{CheckpointTransportBudget, CheckpointTransportError, TransportUsage};
pub use cursor::CheckpointWireCursor;
pub use scan::{CheckpointShape, preflight_checkpoint};
pub use view::ValueStoreCheckpointViewV1;
