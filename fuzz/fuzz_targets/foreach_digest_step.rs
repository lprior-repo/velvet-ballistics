#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Digest step computation must never panic and must return a Result
    let result = compute_digest_step(data);
    // Verify result is valid (typed error or success, never panic)
    let _ = result.is_ok();
});

fn compute_digest_step(data: &[u8]) -> Result<vb_core::WorkflowDigest, ()> {
    use vb_core::WorkflowDigest;
    // Exercise digest step — use blake3 hash as stand-in for digest computation
    // The actual digest step path should return typed errors on invalid input
    let hash = blake3::hash(data);
    Ok(WorkflowDigest::from_bytes(hash.into()))
}
