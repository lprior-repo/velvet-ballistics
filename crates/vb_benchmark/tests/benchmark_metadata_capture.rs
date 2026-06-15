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
//   PO-vb-hints-009  (serialized JSON contains all 20 MASTER_METADATA_FIELDS keys)
//   PO-vb-hints-020  (serialized JSON keys are audit-compatible without _ns suffix)
//   PO-vb-hints-022  (commit_hash validation preserved with new parameters)
//
// Production code is implemented in vb_benchmark/src/lib.rs with all
// required types: capture_metadata (10-param), BenchmarkMetadata with
// latency fields, EvidenceError variants, LatencyFieldId enum, and
// MASTER_METADATA_FIELDS constant. Serde/serde_json are dependencies.

use std::time::Duration;
use vb_benchmark::*;

#[cfg(kani)]
mod kani_harnesses {
    use kani::Arbitrary;

    // Derive Arbitrary for BenchmarkMetadata to enable symbolic inputs.
    // This is required by GOD Rule 1: no hardcoded structural inputs.
    impl Arbitrary for BenchmarkMetadata {
        fn arbitrary() -> Self {
            Self {
                name: kani::any::<String>(),
                baseline_us: kani::any(),
                result_us: kani::any(),
                command: kani::any::<String>(),
                commit_hash: kani::any::<String>(),
                environment: kani::any::<String>(),
                budget_us: kani::any(),
                fjall_write_latency_ns: kani::any(),
                direct_api_latency_ns: kani::any(),
                ipc_latency_ns: kani::any(),
            }
        }
    }

    /// Harness: capture_metadata populates all three latency fields from inputs.
    ///
    /// Proves PO-vb-hints-002: for any valid inputs, the returned metadata contains
    /// the latency fields populated with the exact input values.
    #[kani::proof]
    fn proof_capture_metadata_populates_latency_fields() {
        let name: String = kani::any();
        let baseline: Option<Duration> = kani::any();
        let result: Duration = kani::any();
        let command: String = kani::any();
        let commit_hash: String = kani::any();
        let environment: String = kani::any();
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
                kani::assert(
                    metadata.fjall_write_latency_ns == fjall_ns,
                    "fjall_write_latency_ns must equal input fjall_ns",
                );
                kani::assert(
                    metadata.direct_api_latency_ns == api_ns,
                    "direct_api_latency_ns must equal input api_ns",
                );
                kani::assert(
                    metadata.ipc_latency_ns == ipc_ns,
                    "ipc_latency_ns must equal input ipc_ns",
                );
            }
            Err(_) => {
                // Should not happen with valid commit_hash assumption.
                kani::assert(
                    false,
                    "capture_metadata should succeed with valid commit_hash",
                );
            }
        }
    }

    /// Harness: commit_hash validation is preserved with new parameters.
    ///
    /// Proves PO-vb-hints-022: empty and non-hex commit hashes still return
    /// Err(MissingCommit) even when the new latency parameters are present.
    #[kani::proof]
    fn proof_commit_hash_validation_preserved() {
        let name: String = kani::any();
        let baseline: Option<Duration> = kani::any();
        let result: Duration = kani::any();
        let command: String = kani::any();
        let environment: String = kani::any();
        let budget_us: u64 = kani::any();
        let fjall_ns: u64 = kani::any();
        let api_ns: u64 = kani::any();
        let ipc_ns: u64 = kani::any();

        // Test case 1: empty commit hash
        let empty_hash: &str = "";
        let result = capture_metadata(
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
        kani::assert(
            matches!(result, Err(EvidenceError::MissingCommit)),
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
        kani::assert(
            matches!(result2, Err(EvidenceError::MissingCommit)),
            "non-hex commit_hash must return MissingCommit",
        );
    }

    /// Harness: serialized JSON contains all MASTER_METADATA_FIELDS keys.
    ///
    /// Proves PO-vb-hints-009: for any valid metadata, the serialized JSON
    /// representation contains all 20 keys from MASTER_METADATA_FIELDS.
    #[kani::proof]
    fn proof_serialization_completeness() {
        let name: String = kani::any();
        let baseline: Option<Duration> = kani::any();
        let result: Duration = kani::any();
        let command: String = kani::any();
        let commit_hash: String = kani::any();
        let environment: String = kani::any();
        let budget_us: u64 = kani::any();
        let fjall_ns: u64 = kani::any();
        let api_ns: u64 = kani::any();
        let ipc_ns: u64 = kani::any();

        kani::assume(!commit_hash.is_empty());
        kani::assume(commit_hash.bytes().all(|b| b.is_ascii_hexdigit()));

        let metadata = capture_metadata(
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
        )
        .expect("valid inputs should produce Ok(metadata)");

        let json = serde_json::to_string(&metadata).expect("serialization should succeed");
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("roundtrip parse should succeed");

        if let serde_json::Value::Object(map) = parsed {
            for key in &MASTER_METADATA_FIELDS {
                kani::assert(
                    map.contains_key(*key),
                    &format!("JSON must contain key: {}", key),
                );
            }
        } else {
            kani::assert(false, "serialized metadata must be a JSON object");
        }
    }

    /// Harness: serialized JSON keys are audit-compatible (without _ns suffix).
    ///
    /// Proves PO-vb-hints-020: the serialized JSON contains:
    ///   - fjall_write_latency (not fjall_write_latency_ns)
    ///   - direct_api_latency (not direct_api_latency_ns)
    ///   - ipc_latency (not ipc_latency_ns)
    #[kani::proof]
    fn proof_audit_compatible_keys() {
        let name: String = kani::any();
        let baseline: Option<Duration> = kani::any();
        let result: Duration = kani::any();
        let command: String = kani::any();
        let commit_hash: String = kani::any();
        let environment: String = kani::any();
        let budget_us: u64 = kani::any();
        let fjall_ns: u64 = kani::any();
        let api_ns: u64 = kani::any();
        let ipc_ns: u64 = kani::any();

        kani::assume(!commit_hash.is_empty());
        kani::assume(commit_hash.bytes().all(|b| b.is_ascii_hexdigit()));

        let metadata = capture_metadata(
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
        )
        .expect("valid inputs should produce Ok(metadata)");

        let json = serde_json::to_string(&metadata).expect("serialization should succeed");
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("roundtrip parse should succeed");

        if let serde_json::Value::Object(map) = parsed {
            kani::assert(
                map.contains_key("fjall_write_latency"),
                "must contain audit key fjall_write_latency",
            );
            kani::assert(
                map.contains_key("direct_api_latency"),
                "must contain audit key direct_api_latency",
            );
            kani::assert(
                map.contains_key("ipc_latency"),
                "must contain audit key ipc_latency",
            );
        }
    }
}
