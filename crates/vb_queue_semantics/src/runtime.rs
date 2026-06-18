//! Runtime surface: command-surface admission mapping.
//!
//! Maps public queue-backed admission failures to exact queue-full transition
//! summaries. This is the imperative-shell boundary that surfaces queue
//! state to production callers.

use crate::transitions::helper_runtime_queue_full_maps;

/// Public runtime queue-backed surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeQueueSurface {
    /// Submit-family command admission.
    Submit,
    /// Cancel command admission.
    Cancel,
    /// Resume command admission.
    Resume,
    /// Inspect command admission.
    Inspect,
}

/// Public runtime queue-full transition summary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeQueueFullTransition {
    /// Surface that reached queue admission.
    pub surface: RuntimeQueueSurface,
    /// Queue capacity at rejection.
    pub capacity: usize,
    /// Queue depth at rejection.
    pub depth: usize,
    /// True only when the rejected command must not be admitted.
    pub rejected_without_admission: bool,
}

/// Maps a public queue-backed admission failure to an exact queue-full transition.
#[must_use]
pub const fn runtime_queue_full_error_transition(
    depth: usize,
    capacity: usize,
    surface: RuntimeQueueSurface,
) -> Option<RuntimeQueueFullTransition> {
    if helper_runtime_queue_full_maps(depth, capacity) {
        return Some(RuntimeQueueFullTransition {
            surface,
            capacity,
            depth,
            rejected_without_admission: true,
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- RuntimeQueueSurface -------------------------------------------------------

    #[test]
    fn runtime_queue_surface_four_variants_eq() {
        assert_eq!(RuntimeQueueSurface::Submit, RuntimeQueueSurface::Submit);
        assert_eq!(RuntimeQueueSurface::Cancel, RuntimeQueueSurface::Cancel);
        assert_eq!(RuntimeQueueSurface::Resume, RuntimeQueueSurface::Resume);
        assert_eq!(RuntimeQueueSurface::Inspect, RuntimeQueueSurface::Inspect);
    }

    #[test]
    fn runtime_queue_surface_variants_distinct() {
        assert_ne!(RuntimeQueueSurface::Submit, RuntimeQueueSurface::Cancel);
        assert_ne!(RuntimeQueueSurface::Cancel, RuntimeQueueSurface::Resume);
        assert_ne!(RuntimeQueueSurface::Resume, RuntimeQueueSurface::Inspect);
        assert_ne!(RuntimeQueueSurface::Submit, RuntimeQueueSurface::Inspect);
    }

    #[test]
    fn runtime_queue_surface_is_copy() {
        let a = RuntimeQueueSurface::Submit;
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn runtime_queue_surface_clone_eq() {
        let a = RuntimeQueueSurface::Cancel;
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn runtime_queue_surface_debug_strings() {
        assert_eq!(format!("{:?}", RuntimeQueueSurface::Submit), "Submit");
        assert_eq!(format!("{:?}", RuntimeQueueSurface::Cancel), "Cancel");
        assert_eq!(format!("{:?}", RuntimeQueueSurface::Resume), "Resume");
        assert_eq!(format!("{:?}", RuntimeQueueSurface::Inspect), "Inspect");
    }

    // -- RuntimeQueueFullTransition ------------------------------------------------

    #[test]
    fn runtime_queue_full_transition_eq_carries_fields() {
        let a = RuntimeQueueFullTransition {
            surface: RuntimeQueueSurface::Submit,
            capacity: 4,
            depth: 4,
            rejected_without_admission: true,
        };
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn runtime_queue_full_transition_ne_on_surface() {
        let a = RuntimeQueueFullTransition {
            surface: RuntimeQueueSurface::Submit,
            capacity: 4,
            depth: 4,
            rejected_without_admission: true,
        };
        let b = RuntimeQueueFullTransition {
            surface: RuntimeQueueSurface::Cancel,
            capacity: 4,
            depth: 4,
            rejected_without_admission: true,
        };
        assert_ne!(a, b);
    }

    #[test]
    fn runtime_queue_full_transition_ne_on_capacity() {
        let a = RuntimeQueueFullTransition {
            surface: RuntimeQueueSurface::Submit,
            capacity: 4,
            depth: 4,
            rejected_without_admission: true,
        };
        let b = RuntimeQueueFullTransition {
            surface: RuntimeQueueSurface::Submit,
            capacity: 8,
            depth: 4,
            rejected_without_admission: true,
        };
        assert_ne!(a, b);
    }

    #[test]
    fn runtime_queue_full_transition_is_copy() {
        let a = RuntimeQueueFullTransition {
            surface: RuntimeQueueSurface::Resume,
            capacity: 1,
            depth: 1,
            rejected_without_admission: true,
        };
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn runtime_queue_full_transition_debug_includes_surface() {
        let t = RuntimeQueueFullTransition {
            surface: RuntimeQueueSurface::Inspect,
            capacity: 3,
            depth: 3,
            rejected_without_admission: true,
        };
        let s = format!("{:?}", t);
        assert!(s.contains("Inspect"));
        assert!(s.contains("capacity: 3"));
    }

    // -- runtime_queue_full_error_transition ---------------------------------------

    #[test]
    fn runtime_queue_full_error_transition_at_depth_capacity() {
        let t = runtime_queue_full_error_transition(4, 4, RuntimeQueueSurface::Submit);
        assert!(t.is_some());
        let t = t.unwrap_or_else(|| unreachable!("unwrap failed"));
        assert_eq!(t.surface, RuntimeQueueSurface::Submit);
        assert_eq!(t.capacity, 4);
        assert_eq!(t.depth, 4);
        assert!(t.rejected_without_admission);
    }

    #[test]
    fn runtime_queue_full_error_transition_below() {
        let t = runtime_queue_full_error_transition(3, 4, RuntimeQueueSurface::Cancel);
        assert!(t.is_none());
    }

    #[test]
    fn runtime_queue_full_error_transition_zero_zero() {
        let t = runtime_queue_full_error_transition(0, 0, RuntimeQueueSurface::Resume);
        // 0 >= 0 → Some
        assert!(t.is_some());
    }

    #[test]
    fn runtime_queue_full_error_transition_above_capacity() {
        let t = runtime_queue_full_error_transition(5, 4, RuntimeQueueSurface::Inspect);
        assert!(t.is_some());
    }

    #[test]
    fn runtime_queue_full_error_transition_preserves_surface() {
        let t = runtime_queue_full_error_transition(2, 2, RuntimeQueueSurface::Inspect)
            .unwrap_or_else(|| unreachable!("unwrap failed"));
        assert_eq!(t.surface, RuntimeQueueSurface::Inspect);
        assert!(t.rejected_without_admission);
    }
}
