#![forbid(unsafe_code)]

//! Integration tests for vb-mrwe.5: storage record kind parity for StepSucceeded.
//!
//! Cohesive submodules preserve the MRWE5 behavior contract while keeping each
//! physical test file under the repository source-length limit.

#[path = "vb_mrwe5_integration_tests/common.rs"]
mod common;
#[path = "vb_mrwe5_integration_tests/kind_parity.rs"]
mod kind_parity;
#[path = "vb_mrwe5_integration_tests/parse_event.rs"]
mod parse_event;
#[path = "vb_mrwe5_integration_tests/roundtrip.rs"]
mod roundtrip;
#[path = "vb_mrwe5_integration_tests/semantic_classification.rs"]
mod semantic_classification;
#[path = "vb_mrwe5_integration_tests/validated_record.rs"]
mod validated_record;
