// harnesses/kani/benchmark_metadata_gate.rs
//
// Kani bounded model checking harnesses for check_evidence_gate zero-latency validation.
//
// This artifact targets the planned production function:
//   pub fn check_evidence_gate(
//       metadata: &BenchmarkMetadata,
//       threshold_pct: u64,
//   ) -> Result<(), EvidenceError>
//
// With the planned zero-latency checks:
//   - Returns Err(ZeroLatencyField(FjallWrite)) if fjall_write_latency_ns == 0
//   - Returns Err(ZeroLatencyField(DirectApi)) if direct_api_latency_ns == 0
//   - Returns Err(ZeroLatencyField(Ipc)) if ipc_latency_ns == 0
//
// Obligation coverage:
//   PO-vb-hints-007  (check_evidence_gate returns Err(ZeroLatencyField) for any zero latency)
//   PO-vb-hints-011  (non-zero latencies do not trigger ZeroLatencyField error)
//
// Production code is implemented: check_evidence_gate has zero-latency checks,
// EvidenceError::ZeroLatencyField variant exists, LatencyFieldId enum exists.

use vb_benchmark::*;

#[cfg(kani)]
mod kani_harnesses {
    use kani::Arbitrary;

    /// Harness: any zero latency field causes check_evidence_gate to return Err(ZeroLatencyField).
    ///
    /// Proves PO-vb-hints-007: for all (metadata, threshold_pct), if any latency field == 0,
    /// then check_evidence_gate returns Err(ZeroLatencyField(_)).
    ///
    /// Three separate assertions cover each latency field independently.
    #[kani::proof]
    fn proof_gate_rejects_zero_latency_fields() {
        let threshold_pct: u64 = kani::any();

        // Case 1: fjall_write_latency_ns == 0
        {
            let metadata = BenchmarkMetadata {
                name: kani::any(),
                baseline_us: kani::any(),
                result_us: kani::any(),
                command: kani::any(),
                commit_hash: kani::any(),
                environment: kani::any(),
                budget_us: kani::any(),
                fjall_write_latency_ns: 0,
                direct_api_latency_ns: kani::any(),
                ipc_latency_ns: kani::any(),
            };
            let result = check_evidence_gate(&metadata, threshold_pct);
            kani::assert(
                matches!(result, Err(EvidenceError::ZeroLatencyField(_))),
                "zero fjall_write_latency_ns must return ZeroLatencyField error",
            );
        }

        // Case 2: direct_api_latency_ns == 0
        {
            let metadata = BenchmarkMetadata {
                name: kani::any(),
                baseline_us: kani::any(),
                result_us: kani::any(),
                command: kani::any(),
                commit_hash: kani::any(),
                environment: kani::any(),
                budget_us: kani::any(),
                fjall_write_latency_ns: kani::any(),
                direct_api_latency_ns: 0,
                ipc_latency_ns: kani::any(),
            };
            let result = check_evidence_gate(&metadata, threshold_pct);
            kani::assert(
                matches!(result, Err(EvidenceError::ZeroLatencyField(_))),
                "zero direct_api_latency_ns must return ZeroLatencyField error",
            );
        }

        // Case 3: ipc_latency_ns == 0
        {
            let metadata = BenchmarkMetadata {
                name: kani::any(),
                baseline_us: kani::any(),
                result_us: kani::any(),
                command: kani::any(),
                commit_hash: kani::any(),
                environment: kani::any(),
                budget_us: kani::any(),
                fjall_write_latency_ns: kani::any(),
                direct_api_latency_ns: kani::any(),
                ipc_latency_ns: 0,
            };
            let result = check_evidence_gate(&metadata, threshold_pct);
            kani::assert(
                matches!(result, Err(EvidenceError::ZeroLatencyField(_))),
                "zero ipc_latency_ns must return ZeroLatencyField error",
            );
        }
    }

    /// Harness: all non-zero latencies do not trigger ZeroLatencyField error.
    ///
    /// Proves PO-vb-hints-011: for metadata where all three latency fields > 0,
    /// check_evidence_gate does not return Err(ZeroLatencyField).
    #[kani::proof]
    fn proof_gate_allows_nonzero_latencies() {
        let threshold_pct: u64 = kani::any();

        let fjall_ns: u64 = kani::any();
        let api_ns: u64 = kani::any();
        let ipc_ns: u64 = kani::any();
        kani::assume(fjall_ns > 0);
        kani::assume(api_ns > 0);
        kani::assume(ipc_ns > 0);

        let metadata = BenchmarkMetadata {
            name: kani::any(),
            baseline_us: kani::any(),
            result_us: kani::any(),
            command: kani::any(),
            commit_hash: kani::any(),
            environment: kani::any(),
            budget_us: kani::any(),
            fjall_write_latency_ns: fjall_ns,
            direct_api_latency_ns: api_ns,
            ipc_latency_ns: ipc_ns,
        };

        let result = check_evidence_gate(&metadata, threshold_pct);

        // The result should NOT be ZeroLatencyField for any latency.
        // It may be another error (MissingBaseline, EmptyBudget, etc.)
        // but not ZeroLatencyField since all latencies are non-zero.
        kani::assert(
            !matches!(result, Err(EvidenceError::ZeroLatencyField(_))),
            "non-zero latencies should not trigger ZeroLatencyField error",
        );
    }
}
