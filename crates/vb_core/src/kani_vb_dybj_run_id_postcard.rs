#![cfg(kani)]
#![forbid(unsafe_code)]

//! PO-VB-DYBJ-002: symbolic RunId postcard roundtrip harness.

use crate::RunId;

#[kani::proof]
fn kani_vb_dybj_run_id_postcard_roundtrip() {
    let value: u64 = kani::any();
    let run_id = RunId::new(value);
    assert!(run_id.get() == value);

    let encoded = postcard::to_allocvec(&run_id);
    assert!(encoded.is_ok());

    if let Ok(bytes) = encoded {
        let decoded = postcard::from_bytes::<RunId>(&bytes);
        assert!(decoded.is_ok());
        if let Ok(decoded_run_id) = decoded {
            assert!(decoded_run_id == run_id);
            assert!(decoded_run_id.get() == value);
        }
    }

    let zero = RunId::ZERO;
    assert!(zero.get() == 0_u64);
    let max = RunId::new(u64::MAX);
    assert!(max.get() == u64::MAX);
}
