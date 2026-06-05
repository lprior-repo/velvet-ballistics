#[flux_rs::extern_spec]
impl crate {
    #[flux_rs::sig(fn(capacity: usize, len: usize) -> bool[len < capacity])]
    pub const fn helper_queue_is_full(capacity: usize, len: usize) -> bool;
}