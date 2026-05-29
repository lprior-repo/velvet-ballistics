#![cfg(kani)]
#![forbid(unsafe_code)]

//! PO-VB-DYBJ-013: nonempty trailing suffixes are rejected by exact Postcard decode.

use vb_core::WorkflowDigest;

fn exact_workflow_digest_from_postcard(bytes: &[u8]) -> Result<WorkflowDigest, postcard::Error> {
    match postcard::take_from_bytes::<WorkflowDigest>(bytes) {
        Ok((digest, remaining)) if remaining.is_empty() => Ok(digest),
        Ok((_digest, _remaining)) => Err(postcard::Error::DeserializeUnexpectedEnd),
        Err(error) => Err(error),
    }
}

#[kani::proof]
#[kani::unwind(9)]
fn kani_vb_dybj_trailing_bytes_rejected() {
    let suffix_len: usize = kani::any();
    kani::assume((1_usize..=8_usize).contains(&suffix_len));
    let suffix_byte: u8 = kani::any();

    let digest_bytes: [u8; 32] = kani::any();
    let mut candidate = [0_u8; 40];
    candidate[..32].copy_from_slice(&digest_bytes);
    for idx in 0_usize..8_usize {
        if idx < suffix_len {
            candidate[32_usize + idx] = suffix_byte;
        }
    }
    let total_len = 32_usize + suffix_len;
    let decoded = exact_workflow_digest_from_postcard(&candidate[..total_len]);
    assert!(decoded.is_err());
}
