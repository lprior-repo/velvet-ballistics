//! Unit tests for vb_boundary_inventory
//!
//! Test organization:
//! - api_tests:      Tests for 5 pub fns in api module
//! - parser_tests:    Tests for parse_inventory
//! - validation_tests: Tests for validate_evidence_reference_bytes
//! - error_tests:     Tests for all 13 BoundaryInventoryError variants
//! - property_tests:  Proptest property-based tests

mod api_tests;
mod error_tests;
mod parser_tests;
mod property_tests;
mod validation_tests;
