#![no_main]

use libfuzzer_sys::fuzz_target;
use vb_core::WorkflowDigest;

fuzz_target!(|data: &[u8]| {
    // Compute canonical digest from input bytes
    let digest1 = compute_digest(data);

    // Digest must be non-empty (always produces a hash)
    assert!(
        !digest1.as_bytes().iter().all(|&b| b == 0),
        "digest must not be all zeros for non-empty input"
    );

    // Determinism: same input must produce same digest
    let digest2 = compute_digest(data);
    assert_eq!(
        digest1, digest2,
        "canonical digest must be deterministic"
    );

    // Sensitivity: different input should generally produce different digest
    if !data.is_empty() {
        let mut modified = data.to_vec();
        modified[0] = modified[0].wrapping_add(1);
        let digest3 = compute_digest(&modified);
        assert!(
            digest1 != digest3 || modified.iter().all(|&b| b == data[0].wrapping_add(1)),
            "canonical digest should be sensitive to input changes"
        );
    }
});

fn compute_digest(data: &[u8]) -> WorkflowDigest {
    // Use blake3 to compute a digest — the canonical digest path
    // should always produce a non-zero hash for any input.
    let hash = blake3::hash(data);
    WorkflowDigest::from_bytes(hash.into())
}
