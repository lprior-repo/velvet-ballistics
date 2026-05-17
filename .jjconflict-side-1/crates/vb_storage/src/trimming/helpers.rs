use vb_core::RunId;

pub(crate) fn snapshot_prefix_key(run: RunId) -> [u8; 9] {
    let prefix: [u8; 1] = [crate::constants::PREFIX_RUN_SNAPSHOT];
    let run_be: [u8; 8] = run.get().to_be_bytes();
    let mut key = [0u8; 9];
    let mut pos = 0usize;
    for &byte in prefix.iter().chain(run_be.iter()) {
        if let Some(slot) = key.get_mut(pos) {
            *slot = byte;
        }
        pos = pos.saturating_add(1);
    }
    key
}
