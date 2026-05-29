#![no_main]
#![forbid(unsafe_code)]

use libfuzzer_sys::fuzz_target;
use vb_core::WorkflowDigest;

const DIGEST_ZERO_WITH_TRAILING_BYTE: &[u8] = &[
    0_u8, 0_u8, 0_u8, 0_u8, 0_u8, 0_u8, 0_u8, 0_u8,
    0_u8, 0_u8, 0_u8, 0_u8, 0_u8, 0_u8, 0_u8, 0_u8,
    0_u8, 0_u8, 0_u8, 0_u8, 0_u8, 0_u8, 0_u8, 0_u8,
    0_u8, 0_u8, 0_u8, 0_u8, 0_u8, 0_u8, 0_u8, 0_u8,
    0_u8,
];

fn exact_workflow_digest_from_postcard(bytes: &[u8]) -> Result<WorkflowDigest, postcard::Error> {
    match postcard::take_from_bytes::<WorkflowDigest>(bytes) {
        Ok((digest, remaining)) if remaining.is_empty() => Ok(digest),
        Ok((_digest, _remaining)) => Err(postcard::Error::DeserializeUnexpectedEnd),
        Err(error) => Err(error),
    }
}

fuzz_target!(|data: &[u8]| {
    let candidate = if data.is_empty() {
        DIGEST_ZERO_WITH_TRAILING_BYTE
    } else {
        data
    };
    if candidate.len() > 32_usize {
        let decoded = exact_workflow_digest_from_postcard(candidate);
        assert!(decoded.is_err());
    }
});
