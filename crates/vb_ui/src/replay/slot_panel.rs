//! Slot diff panel -- shows what changed at each replay event boundary.

use vb_core::ids::SlotIdx;
use vb_core::value::{SlotValue, Taint};

// ---------------------------------------------------------------------------
// Slot change
// ---------------------------------------------------------------------------

/// Describes how a slot's value changed between two replay states.
#[derive(Debug, Clone, PartialEq)]
pub enum SlotChange {
    /// Slot was written with a new value.
    Written {
        /// Previous value, or `None` if the slot was previously unset.
        before: Option<SlotValue>,
        /// New value after the write.
        after: SlotValue,
    },
    /// Slot was cleared (value removed).
    Cleared {
        /// Value that was present before clearing.
        before: SlotValue,
    },
    /// No change to the slot value.
    Unchanged,
}

// ---------------------------------------------------------------------------
// Taint change
// ---------------------------------------------------------------------------

/// Describes how a slot's taint changed between two replay states.
#[derive(Debug, Clone, PartialEq)]
pub enum TaintChange {
    /// Taint level changed.
    Changed {
        /// Taint before the transition.
        before: Taint,
        /// Taint after the transition.
        after: Taint,
    },
    /// No change to the taint level.
    Unchanged,
}

// ---------------------------------------------------------------------------
// Slot diff (panel entry)
// ---------------------------------------------------------------------------

/// A single slot's value and taint transition at a replay boundary.
///
/// This is the panel-specific diff type. It differs from
/// [`super::types::SlotDiff`] which carries serialized string values
/// for engine-level diffs; this type carries rich typed values for
/// the UI panel display.
#[derive(Debug, Clone)]
pub struct SlotDiffEntry {
    /// Which slot changed.
    pub slot: SlotIdx,
    /// How the value changed.
    pub value_change: SlotChange,
    /// How the taint changed.
    pub taint_change: TaintChange,
}

// ---------------------------------------------------------------------------
// Slot diff panel
// ---------------------------------------------------------------------------

/// Panel model for displaying slot diffs at a replay boundary.
pub struct SlotDiffPanel {
    diffs: Vec<SlotDiffEntry>,
    selected: Option<usize>,
}

impl SlotDiffPanel {
    /// Creates an empty panel with no selection.
    #[must_use]
    pub fn new() -> Self {
        Self {
            diffs: Vec::new(),
            selected: None,
        }
    }

    /// Creates a panel pre-populated with diffs.
    #[must_use]
    pub fn from_changes(diffs: Vec<SlotDiffEntry>) -> Self {
        Self {
            diffs,
            selected: None,
        }
    }

    /// Returns all diffs in the panel.
    #[must_use]
    pub fn diffs(&self) -> &[SlotDiffEntry] {
        &self.diffs
    }

    /// Returns the currently selected diff, if any.
    #[must_use]
    pub fn selected(&self) -> Option<&SlotDiffEntry> {
        self.selected.and_then(|idx| self.diffs.get(idx))
    }

    /// Sets the selection to the given index.
    ///
    /// Clamps to the valid range. Set to `None` if the panel is empty.
    pub fn select(&mut self, idx: usize) {
        if self.diffs.is_empty() {
            self.selected = None;
            return;
        }
        self.selected = Some(idx.min(self.diffs.len().saturating_sub(1)));
    }

    /// Clears the selection.
    pub fn clear_selection(&mut self) {
        self.selected = None;
    }

    /// Returns diffs where taint changed to a non-Clean value.
    pub fn tainted_diffs(&self) -> impl Iterator<Item = &SlotDiffEntry> {
        self.diffs.iter().filter(|d| {
            matches!(
                d.taint_change,
                TaintChange::Changed {
                    after: Taint::Secret | Taint::DerivedFromSecret,
                    ..
                }
            )
        })
    }

    /// Returns a human-readable string for a single diff entry.
    #[must_use]
    pub fn format_diff(diff: &SlotDiffEntry) -> String {
        let value_str = match &diff.value_change {
            SlotChange::Written { before, after } => {
                let before_display = before
                    .as_ref()
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| String::from("<unset>"));
                format!("{} -> {after}", before_display)
            }
            SlotChange::Cleared { before } => {
                format!("{before} -> <cleared>")
            }
            SlotChange::Unchanged => String::from("(no change)"),
        };

        let taint_str = match &diff.taint_change {
            TaintChange::Changed { before, after } => {
                format!("{before:?} -> {after:?}")
            }
            TaintChange::Unchanged => String::from("(no change)"),
        };

        format!("slot[{}]: value={} taint={}", diff.slot.get(), value_str, taint_str)
    }
}

impl Default for SlotDiffPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_written_diff(slot: u16, before: Option<SlotValue>, after: SlotValue) -> SlotDiffEntry {
        SlotDiffEntry {
            slot: SlotIdx::new(slot),
            value_change: SlotChange::Written { before, after },
            taint_change: TaintChange::Unchanged,
        }
    }

    fn make_tainted_diff(slot: u16, taint: Taint) -> SlotDiffEntry {
        SlotDiffEntry {
            slot: SlotIdx::new(slot),
            value_change: SlotChange::Written {
                before: None,
                after: SlotValue::I64(42),
            },
            taint_change: TaintChange::Changed {
                before: Taint::Clean,
                after: taint,
            },
        }
    }

    // -- Construction --

    #[test]
    fn new_panel_is_empty() {
        let panel = SlotDiffPanel::new();
        assert!(panel.diffs().is_empty());
        assert!(panel.selected().is_none());
    }

    #[test]
    fn default_is_same_as_new() {
        let panel = SlotDiffPanel::default();
        assert!(panel.diffs().is_empty());
    }

    #[test]
    fn from_changes_populates_diffs() {
        let diffs = vec![make_written_diff(0, None, SlotValue::Bool(true))];
        let panel = SlotDiffPanel::from_changes(diffs);
        assert_eq!(panel.diffs().len(), 1);
    }

    // -- Selection --

    #[test]
    fn select_on_empty_panel_is_none() {
        let mut panel = SlotDiffPanel::new();
        panel.select(0);
        assert!(panel.selected().is_none());
    }

    #[test]
    fn select_clamps_to_last_index() {
        let diffs = vec![
            make_written_diff(0, None, SlotValue::I64(1)),
            make_written_diff(1, None, SlotValue::I64(2)),
        ];
        let mut panel = SlotDiffPanel::from_changes(diffs);
        panel.select(100);
        assert_eq!(
            panel.selected().map(|d| d.slot.get()),
            Some(SlotIdx::new(1).get())
        );
    }

    #[test]
    fn select_valid_index_returns_diff() {
        let diffs = vec![
            make_written_diff(0, None, SlotValue::I64(1)),
            make_written_diff(1, None, SlotValue::I64(2)),
        ];
        let mut panel = SlotDiffPanel::from_changes(diffs);
        panel.select(0);
        assert_eq!(panel.selected().map(|d| d.slot.get()), Some(0));
    }

    #[test]
    fn clear_selection_resets_to_none() {
        let diffs = vec![make_written_diff(0, None, SlotValue::I64(1))];
        let mut panel = SlotDiffPanel::from_changes(diffs);
        panel.select(0);
        assert!(panel.selected().is_some());
        panel.clear_selection();
        assert!(panel.selected().is_none());
    }

    // -- tainted_diffs --

    #[test]
    fn tainted_diffs_returns_only_non_clean() {
        let diffs = vec![
            make_written_diff(0, None, SlotValue::I64(1)),
            make_tainted_diff(1, Taint::Secret),
            make_tainted_diff(2, Taint::DerivedFromSecret),
            make_tainted_diff(3, Taint::Clean),
        ];
        let panel = SlotDiffPanel::from_changes(diffs);
        let tainted: Vec<_> = panel.tainted_diffs().collect();
        assert_eq!(tainted.len(), 2);
        assert_eq!(tainted[0].slot.get(), 1);
        assert_eq!(tainted[1].slot.get(), 2);
    }

    #[test]
    fn tainted_diffs_returns_empty_when_none() {
        let diffs = vec![make_written_diff(0, None, SlotValue::I64(1))];
        let panel = SlotDiffPanel::from_changes(diffs);
        let tainted: Vec<_> = panel.tainted_diffs().collect();
        assert!(tainted.is_empty());
    }

    // -- format_diff --

    #[test]
    fn format_written_diff_with_no_prior() {
        let diff = SlotDiffEntry {
            slot: SlotIdx::new(5),
            value_change: SlotChange::Written {
                before: None,
                after: SlotValue::Bool(true),
            },
            taint_change: TaintChange::Unchanged,
        };
        let formatted = SlotDiffPanel::format_diff(&diff);
        assert!(formatted.contains("slot[5]"));
        assert!(formatted.contains("<unset> -> true"));
        assert!(formatted.contains("(no change)"));
    }

    #[test]
    fn format_written_diff_with_prior() {
        let diff = SlotDiffEntry {
            slot: SlotIdx::new(3),
            value_change: SlotChange::Written {
                before: Some(SlotValue::I64(10)),
                after: SlotValue::I64(20),
            },
            taint_change: TaintChange::Changed {
                before: Taint::Clean,
                after: Taint::Secret,
            },
        };
        let formatted = SlotDiffPanel::format_diff(&diff);
        assert!(formatted.contains("slot[3]"));
        assert!(formatted.contains("10 -> 20"));
        assert!(formatted.contains("Clean -> Secret"));
    }

    #[test]
    fn format_cleared_diff() {
        let diff = SlotDiffEntry {
            slot: SlotIdx::new(7),
            value_change: SlotChange::Cleared {
                before: SlotValue::Null,
            },
            taint_change: TaintChange::Unchanged,
        };
        let formatted = SlotDiffPanel::format_diff(&diff);
        assert!(formatted.contains("<cleared>"));
    }

    #[test]
    fn format_unchanged_diff() {
        let diff = SlotDiffEntry {
            slot: SlotIdx::new(0),
            value_change: SlotChange::Unchanged,
            taint_change: TaintChange::Unchanged,
        };
        let formatted = SlotDiffPanel::format_diff(&diff);
        assert!(formatted.contains("(no change)"));
    }

    // -- Equality for SlotChange / TaintChange --

    #[test]
    fn slot_change_equality() {
        let a = SlotChange::Written {
            before: None,
            after: SlotValue::I64(1),
        };
        let b = SlotChange::Written {
            before: None,
            after: SlotValue::I64(1),
        };
        assert_eq!(a, b);
    }

    #[test]
    fn slot_change_inequality_different_after() {
        let a = SlotChange::Written {
            before: None,
            after: SlotValue::I64(1),
        };
        let b = SlotChange::Written {
            before: None,
            after: SlotValue::I64(2),
        };
        assert_ne!(a, b);
    }

    #[test]
    fn taint_change_equality() {
        assert_eq!(TaintChange::Unchanged, TaintChange::Unchanged);
        assert_eq!(
            TaintChange::Changed {
                before: Taint::Clean,
                after: Taint::Secret
            },
            TaintChange::Changed {
                before: Taint::Clean,
                after: Taint::Secret
            }
        );
    }

    #[test]
    fn taint_change_inequality() {
        assert_ne!(
            TaintChange::Changed {
                before: Taint::Clean,
                after: Taint::Secret
            },
            TaintChange::Changed {
                before: Taint::Clean,
                after: Taint::DerivedFromSecret
            }
        );
    }
}
