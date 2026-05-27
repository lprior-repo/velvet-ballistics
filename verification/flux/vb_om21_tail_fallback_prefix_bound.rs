#![forbid(unsafe_code)]
// Obligation: PO-vb-om21-prefix-bound-flux
// Flux sublane artifact: refinement-shaped Rust kernel for vb_om21_prefix_bound.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RunEventKey {
    pub bytes: [u8; 17],
}
pub fn has_valid_prefix(key: &RunEventKey, run: u64) -> bool {
    let rb = run.to_be_bytes();
    key.bytes[0] == 0x11 && key.bytes[1..9] == rb
}
pub fn decode_seq_when_valid(key: &RunEventKey, run: u64) -> Option<u64> {
    if !has_valid_prefix(key, run) {
        return None;
    }
    Some(u64::from_be_bytes([
        key.bytes[9],
        key.bytes[10],
        key.bytes[11],
        key.bytes[12],
        key.bytes[13],
        key.bytes[14],
        key.bytes[15],
        key.bytes[16],
    ]))
}
pub fn vb_om21_prefix_bound_flux_kernel(key: &RunEventKey, run: u64) -> bool {
    match decode_seq_when_valid(key, run) {
        Some(_) => has_valid_prefix(key, run),
        None => true,
    }
}
