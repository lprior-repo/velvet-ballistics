// Lifecycle chunk 001: submit, resume, and action completion methods.
// Split into submodules for maintainability:
// - chunk_001_submit.rs: Submit lifecycle methods
// - chunk_001_resume.rs: Resume lifecycle methods
// - chunk_001_action.rs: Action completion and failure methods
// - chunk_001_tests.rs: Test helpers

include!("chunk_001_submit.rs");
include!("chunk_001_resume.rs");
include!("chunk_001_action.rs");
include!("chunk_001_tests.rs");