// Action completion and failure preflight helpers. Implementation is split
// across focused chunks under `chunk_003_chunks/`. All chunks share this
// module's `use` declarations and are `include!`-d into the lifecycle shell
// to keep the public surface and tests unchanged. Splitting by domain
// responsibility:
//
// - `chunk_001_preflight_completion` - `use` declarations, completion
//   preflight (`preflight_action_completion`) plus every size, taint, and
//   contract-validation rejector.
// - `chunk_002_failure_preflight` - failure preflight / apply split
//   (`ActionFailurePreflight`, `preflight_action_failure`,
//   `apply_action_failure_preflight`) and `write_failure_slot`.

include!("chunk_003_chunks/chunk_001_preflight_completion.rs");
include!("chunk_003_chunks/chunk_002_failure_preflight.rs");