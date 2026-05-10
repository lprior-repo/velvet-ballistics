#![forbid(unsafe_code)]

use vb_storage::recovery::recover_runtime_frame_seed_from_events;

fn main() {
    let events = Vec::new();
    let _result = recover_runtime_frame_seed_from_events(&events);
}
