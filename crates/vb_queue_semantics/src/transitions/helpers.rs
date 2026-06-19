//! Verus-shared helper functions used across the transition subsystem.
//!
//! These are pure `const fn` routes that feed the zero-allocation decision kernels
//! and production wrappers. They operate on `usize` primitives only.

/// Verus-shared helper route for enqueue admission decisions.
#[must_use]
#[cfg_attr(flux, flux_rs::sig(fn(capacity: usize, len: usize) -> bool[len < capacity]))]
pub const fn helper_enqueue_accepts(capacity: usize, len: usize) -> bool {
    crate::state::helper_queue_is_full(capacity, len)
}

/// Verus-shared helper route for command pop decisions.
#[must_use]
#[cfg_attr(flux, flux_rs::sig(fn(capacity: usize, len: usize) -> bool[len > 0 && capacity > 0]))]
pub const fn helper_command_pop_is_pop_front(capacity: usize, len: usize) -> bool {
    len > 0 && capacity > 0
}

/// Verus-shared helper route for shard tick pop decisions.
#[must_use]
#[cfg_attr(flux, flux_rs::sig(fn(capacity: usize, len: usize) -> bool[len > 0 && capacity > 0]))]
pub const fn helper_shard_tick_is_pop_front(capacity: usize, len: usize) -> bool {
    helper_command_pop_is_pop_front(capacity, len)
}

/// Verus-shared helper route for public runtime QueueFull mapping.
#[must_use]
#[cfg_attr(flux, flux_rs::sig(fn(depth: usize, capacity: usize) -> bool[depth >= capacity]))]
pub const fn helper_runtime_queue_full_maps(depth: usize, capacity: usize) -> bool {
    crate::state::helper_queue_is_full(capacity, depth)
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- helper_enqueue_accepts --------------------------------------------------

    #[test]
    fn helper_enqueue_accepts_empty_yes() {
        assert!(helper_enqueue_accepts(4, 0));
    }

    #[test]
    fn helper_enqueue_accepts_full_no() {
        assert!(!helper_enqueue_accepts(4, 4));
    }

    #[test]
    fn helper_enqueue_accepts_over_no() {
        assert!(!helper_enqueue_accepts(4, 5));
    }

    #[test]
    fn helper_enqueue_accepts_below_full_yes() {
        assert!(helper_enqueue_accepts(4, 3));
    }

    #[test]
    fn helper_enqueue_accepts_zero_capacity_no() {
        assert!(!helper_enqueue_accepts(0, 0));
    }

    // -- helper_command_pop_is_pop_front -----------------------------------------

    #[test]
    fn helper_command_pop_is_pop_front_empty_no_capacity_yes() {
        // condition: len > 0 && capacity > 0
        // 0 > 0 false → false
        assert!(!helper_command_pop_is_pop_front(4, 0));
    }

    #[test]
    fn helper_command_pop_is_pop_front_one_yes() {
        assert!(helper_command_pop_is_pop_front(4, 1));
    }

    #[test]
    fn helper_command_pop_is_pop_front_zero_capacity() {
        assert!(!helper_command_pop_is_pop_front(0, 1));
    }

    #[test]
    fn helper_command_pop_is_pop_front_zero_both() {
        assert!(!helper_command_pop_is_pop_front(0, 0));
    }

    #[test]
    fn helper_command_pop_is_pop_front_max_both() {
        assert!(helper_command_pop_is_pop_front(100, 100));
    }

    // -- helper_shard_tick_is_pop_front ------------------------------------------

    #[test]
    fn helper_shard_tick_matches_command() {
        for cap in 0..6 {
            for len in 0..6 {
                assert_eq!(
                    helper_shard_tick_is_pop_front(cap, len),
                    helper_command_pop_is_pop_front(cap, len),
                );
            }
        }
    }

    #[test]
    fn helper_shard_tick_zero_both() {
        assert!(!helper_shard_tick_is_pop_front(0, 0));
    }

    #[test]
    fn helper_shard_tick_nonempty_with_capacity() {
        assert!(helper_shard_tick_is_pop_front(4, 1));
    }

    #[test]
    fn helper_shard_tick_no_capacity_never_pops() {
        assert!(!helper_shard_tick_is_pop_front(0, 5));
    }

    #[test]
    fn helper_shard_tick_is_const_callable() {
        // Pure const fn returning bool is callable in const context
        assert!(helper_shard_tick_is_pop_front(4, 2));
    }

    // -- helper_runtime_queue_full_maps ------------------------------------------

    #[test]
    fn helper_runtime_queue_full_maps_at_depth_eq_capacity() {
        assert!(helper_runtime_queue_full_maps(4, 4));
    }

    #[test]
    fn helper_runtime_queue_full_maps_below_capacity() {
        assert!(!helper_runtime_queue_full_maps(3, 4));
    }

    #[test]
    fn helper_runtime_queue_full_maps_above_capacity() {
        assert!(helper_runtime_queue_full_maps(5, 4));
    }

    #[test]
    fn helper_runtime_queue_full_maps_zero_zero() {
        // depth >= capacity → 0 >= 0 → true
        assert!(helper_runtime_queue_full_maps(0, 0));
    }

    #[test]
    fn helper_runtime_queue_full_maps_consistent_with_queue_is_full() {
        // Note arg order: helper_runtime_queue_full_maps(depth, capacity)
        //                helper_queue_is_full(capacity, len)
        for cap in 0..6 {
            for depth in 0..6 {
                assert_eq!(
                    helper_runtime_queue_full_maps(depth, cap),
                    super::super::super::state::helper_queue_is_full(cap, depth),
                );
            }
        }
    }
}
