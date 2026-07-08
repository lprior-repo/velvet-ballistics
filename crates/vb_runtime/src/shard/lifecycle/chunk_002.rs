// External-boundary handlers and drive-application helpers are split into
// focused chunks to preserve source-length gates while sharing the lifecycle
// module imports from `chunk_003`.

include!("chunk_002_parts/chunk_000_ask_answer.rs");
include!("chunk_002_parts/chunk_001_boundary_control.rs");
include!("chunk_002_parts/chunk_002_drive_core.rs");
include!("chunk_002_parts/chunk_003_drive_apply.rs");
