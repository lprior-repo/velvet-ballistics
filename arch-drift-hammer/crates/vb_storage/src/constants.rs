#![forbid(unsafe_code)]
//! Keyspace names, magic constants, and size constants for vb_storage.
//!
//! These constants define the storage contract between the runtime and
//! the durable Fjall key-value backend.

/// Immutable YAML source records by digest.
pub const KEYSPACE_WORKFLOW_SOURCE: &str = "workflow_source";
/// Compiled workflow IR records by digest.
pub const KEYSPACE_COMPILED_IR: &str = "compiled_ir";
/// Run metadata and status records.
pub const KEYSPACE_RUN_HEADER: &str = "run_header";
/// Compact binary event journal records.
pub const KEYSPACE_RUN_EVENT: &str = "run_event";
/// Compact binary run snapshot records.
pub const KEYSPACE_RUN_SNAPSHOT: &str = "run_snapshot";
/// Large input, output, and action payload blobs.
pub const KEYSPACE_BLOB: &str = "blob";
/// Status/time index records.
pub const KEYSPACE_INDEX_STATUS: &str = "index_status";
/// Workflow/run index records.
pub const KEYSPACE_INDEX_WORKFLOW: &str = "index_workflow";
/// Pending action index records.
pub const KEYSPACE_INDEX_ACTION: &str = "index_action";

/// `workflow_source` key prefix.
pub const PREFIX_WORKFLOW_SOURCE: u8 = 0x01;
/// `compiled_ir` key prefix.
pub const PREFIX_COMPILED_IR: u8 = 0x02;
/// `run_header` key prefix.
pub const PREFIX_RUN_HEADER: u8 = 0x10;
/// `run_event` key prefix.
pub const PREFIX_RUN_EVENT: u8 = 0x11;
/// `run_snapshot` key prefix.
pub const PREFIX_RUN_SNAPSHOT: u8 = 0x12;
/// `blob` key prefix.
pub const PREFIX_BLOB: u8 = 0x20;
/// `index_status` key prefix.
pub const PREFIX_INDEX_STATUS: u8 = 0x30;
/// `index_workflow` key prefix.
pub const PREFIX_INDEX_WORKFLOW: u8 = 0x31;
/// `index_action` key prefix.
pub const PREFIX_INDEX_ACTION: u8 = 0x32;

/// Record envelope header length.
pub const RECORD_HEADER_LEN: u32 = 60;
/// Current record schema version.
pub const CURRENT_SCHEMA_VERSION: u16 = 1;
/// Compiled artifact magic, ASCII `VBIR`.
pub const MAGIC_COMPILED_ARTIFACT: u32 = 0x5642_4952;
/// Journal event magic, ASCII `VBJE`.
pub const MAGIC_JOURNAL_EVENT: u32 = 0x5642_4A45;
/// Snapshot magic, ASCII `VBSN`.
pub const MAGIC_SNAPSHOT: u32 = 0x5642_534E;
/// Blob record magic, ASCII `VBBL`.
pub const MAGIC_BLOB: u32 = 0x5642_424C;
/// IPC frame magic, ASCII `VBLT`.
pub const MAGIC_IPC_FRAME: u32 = 0x5642_4C54;
/// Workflow source magic, ASCII `VBSR`.
pub const MAGIC_WORKFLOW_SOURCE: u32 = 0x5642_5352;
/// Index record magic, ASCII `VBIX`.
pub const MAGIC_INDEX_RECORD: u32 = 0x5642_4958;

pub(crate) const JOURNAL_KEY_BYTES: usize = 17;
pub(crate) const DIGEST_KEY_BYTES: usize = 33;
pub(crate) const RUN_ONLY_KEY_BYTES: usize = 9;
pub(crate) const INDEX_STATUS_KEY_BYTES: usize = 18;
pub(crate) const INDEX_WORKFLOW_KEY_BYTES: usize = 13;
pub(crate) const INDEX_ACTION_KEY_BYTES: usize = 13;
pub(crate) const _RUN_EVENT_PREFIX_BYTES: usize = 9;
/// Digest byte width used by storage keys and record payload checksums.
pub const DIGEST_BYTES: usize = 32;
/// Record header bytes length.
pub const RECORD_HEADER_BYTES: usize = 60;
/// CRC32C checksum offset in header.
pub const CRC_OFFSET: usize = 56;
/// Maximum journal event payload accepted by the default journal APIs.
pub const MAX_JOURNAL_EVENT_PAYLOAD_BYTES: u32 = 1_048_576;
/// Maximum source bytes accepted by the default workflow source APIs.
pub const MAX_WORKFLOW_SOURCE_BYTES: u32 = 1_048_576;
/// Maximum compiled IR bytes accepted by the default compiled artifact APIs.
pub const MAX_COMPILED_IR_BYTES: u32 = 16_777_216;
/// Maximum run header payload bytes accepted by the default header APIs.
pub const MAX_RUN_HEADER_BYTES: u32 = 65_536;
/// Maximum snapshot payload bytes accepted by the default snapshot APIs.
pub const MAX_SNAPSHOT_BYTES: u32 = 67_108_864;
/// Maximum blob payload bytes accepted by the default blob APIs.
pub const MAX_BLOB_BYTES: u32 = 67_108_864;
/// Maximum number of events permitted in a single journal write batch.
pub const MAX_BATCH_COUNT: usize = 10_000;
const _PAYLOAD_LEN_CONVERSION_MAX: u32 = 4_294_967_295;
