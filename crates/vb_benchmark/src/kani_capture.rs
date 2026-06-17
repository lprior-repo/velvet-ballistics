// harnesses/kani/benchmark_metadata_capture.rs
//
// Kani bounded model checking harnesses for capture_metadata.
//
// This artifact targets the planned production function:
//   pub fn capture_metadata(
//       name: &str,
//       baseline: Option<Duration>,
//       result: Duration,
//       command: &str,
//       commit_hash: &str,
//       environment: &str,
//       budget_us: u64,
//       fjall_write_latency_ns: u64,
//       direct_api_latency_ns: u64,
//       ipc_latency_ns: u64,
//   ) -> Result<BenchmarkMetadata, EvidenceError>
//
// Obligation coverage:
//   PO-vb-hints-002  (capture_metadata populates all three latency fields)
//   PO-vb-hints-004  (capture_metadata postconditions: latency outputs == inputs)
//   PO-vb-hints-022  (commit_hash validation preserved with new parameters)
//
// Note: PO-vb-hints-009 and PO-vb-hints-020 (serialization completeness/audit keys)
// are waived for Kani and covered by proptest (PO-vb-hints-010, PO-vb-hints-021)
// since serde_json is a dev-dependency not available in src/ harness context.
// Waivers: WC-vb-hints-001, WC-vb-hints-002.
//
// Production code is implemented in vb_benchmark/src/lib.rs with all
// required types: capture_metadata (10-param), BenchmarkMetadata with
// latency fields, EvidenceError variants, LatencyFieldId enum, and
// MASTER_METADATA_FIELDS constant. Serde/serde_json are dependencies.

use std::time::Duration;

#[cfg(kani)]
mod kani_harnesses {
    use crate::*;

    /// Harness: capture_metadata populates all three latency fields from inputs.
    ///
    /// Proves PO-vb-hints-002: for any valid inputs, the returned metadata contains
    /// the latency fields populated with the exact input values.
    #[kani::proof]
    fn proof_capture_metadata_populates_latency_fields() {
        // Kani 0.67.0 doesn't implement Arbitrary for String/&str.
        // Use numeric values to construct strings.
        let name = format!("bench_{}", kani::any::<u64>());
        let baseline: Option<Duration> = kani::any();
        let result: Duration = kani::any();
        let command = format!("cmd_{}", kani::any::<u64>());
        let commit_hash = format!("{:016x}", kani::any::<u64>());
        let environment = format!("env_{}", kani::any::<u64>());
        let budget_us: u64 = kani::any();
        let fjall_ns: u64 = kani::any();
        let api_ns: u64 = kani::any();
        let ipc_ns: u64 = kani::any();

        // Assume commit_hash is valid (non-empty ASCII hex)
        kani::assume(!commit_hash.is_empty());
        kani::assume(commit_hash.bytes().all(|b| b.is_ascii_hexdigit()));

        let result = capture_metadata(
            &name,
            baseline,
            result,
            &command,
            &commit_hash,
            &environment,
            budget_us,
            fjall_ns,
            api_ns,
            ipc_ns,
        );

        match result {
            Ok(metadata) => {
                // harnesses/kani/benchmark_metadata_capture.rs
//
// Kani bounded model checking harnesses for capture_metadata.
//
// This artifact targets the planned production function:
//   pub fn capture_metadata(
//       name: &str,
//       baseline: Option<Duration>,
//       result: Duration,
//       command: &str,
//       commit_hash: &str,
//       environment: &str,
//       budget_us: u64,
//       fjall_write_latency_ns: u64,
//       direct_api_latency_ns: u64,
//       ipc_latency_ns: u64,
//   ) -> Result<BenchmarkMetadata, EvidenceError>
//
// Obligation coverage:
//   PO-vb-hints-002  (capture_metadata populates all three latency fields)
//   PO-vb-hints-004  (capture_metadata postconditions: latency outputs == inputs)
//   PO-vb-hints-022  (commit_hash validation preserved with new parameters)
//
// Note: PO-vb-hints-009 and PO-vb-hints-020 (serialization completeness/audit keys)
// are waived for Kani and covered by proptest (PO-vb-hints-010, PO-vb-hints-021)
// since serde_json is a dev-dependency not available in src/ harness context.
// Waivers: WC-vb-hints-001, WC-vb-hints-002.
//
// Production code is implemented in vb_benchmark/src/lib.rs with all
// required types: capture_metadata (10-param), BenchmarkMetadata with
// latency fields, EvidenceError variants, LatencyFieldId enum, and
// MASTER_METADATA_FIELDS constant. Serde/serde_json are dependencies.

use std::time::Duration;

#[cfg(kani)]
mod kani_harnesses {
    use crate::*;

    /// Harness: capture_metadata populates all three latency fields from inputs.
    ///
    /// Proves PO-vb-hints-002: for any valid inputs, the returned metadata contains
    /// the latency fields populated with the exact input values.
    #[kani::proof]
    fn proof_capture_metadata_populates_latency_fields() {
        // Kani 0.67.0 doesn't implement Arbitrary for String/&str.
        // Use numeric values to construct strings.
        let name = format!("bench_{}", kani::any::<u64>());
        let baseline: Option<Duration> = kani::any();
        let result: Duration = kani::any();
        let command = format!("cmd_{}", kani::any::<u64>());
        let commit_hash = format!("{:016x}", kani::any::<u64>());
        let environment = format!("env_{}", kani::any::<u64>());
        let budget_us: u64 = kani::any();
        let fjall_ns: u64 = kani::any();
        let api_ns: u64 = kani::any();
        let ipc_ns: u64 = kani::any();

        // Assume commit_hash is valid (non-empty ASCII hex)
        kani::assume(!commit_hash.is_empty());
        kani::assume(commit_hash.bytes().all(|b| b.is_ascii_hexdigit()));

        let result = capture_metadata(
            &name,
            baseline,
            result,
            &command,
            &commit_hash,
            &environment,
            budget_us,
            fjall_ns,
            api_ns,
            ipc_ns,
        );

        match result {
            Ok(metadata) => {
                kani::assert(metadata.fjall_write_latency_ns == fjall_ns);
                kani::assert(metadata.direct_api_latency_ns == api_ns);
                kani::assert(metadata.ipc_latency_ns == ipc_ns);
            }
            Err(_) => {
                // Should not happen with valid commit_hash assumption.
                kani::assert(false);
            }
        }
    }

    /// Harness: commit_hash validation is preserved with new parameters.
    ///
    /// Proves PO-vb-hints-022: empty and non-hex commit hashes still return
    /// Err(MissingCommit) even when the new latency parameters are present.
    #[kani::proof]
    fn proof_commit_hash_validation_preserved() {
        let name = format!("bench_{}", kani::any::<u64>());
        let baseline: Option<Duration> = kani::any();
        let result: Duration = kani::any();
        let command = format!("cmd_{}", kani::any::<u64>());
        let environment = format!("env_{}", kani::any::<u64>());
        let budget_us: u64 = kani::any();
        let fjall_ns: u64 = kani::any();
        let api_ns: u64 = kani::any();
        let ipc_ns: u64 = kani::any();

        // Test case 1: empty commit hash
        let empty_hash: &str = "";
        let result1 = capture_metadata(
            &name,
            baseline,
            result,
            &command,
            empty_hash,
            &environment,
            budget_us,
            fjall_ns,
            api_ns,
            ipc_ns,
        );
        kani::assert(matches!(result1, Err(EvidenceError::MissingCommit)),
            "empty commit_hash must return MissingCommit",
        );

        // Test case 2: non-hex commit hash
        let non_hex_hash: &str = "xyz123!@#";
        let result2 = capture_metadata(
            &name,
            baseline,
            result,
            &command,
            non_hex_hash,
            &environment,
            budget_us,
            fjall_ns,
            api_ns,
            ipc_ns,
        );
        kani::assert(matches!(result2, Err(EvidenceError::MissingCommit)),
            "non-hex commit_hash must return MissingCommit",
        );
    }
}
