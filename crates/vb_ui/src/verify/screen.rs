#![forbid(unsafe_code)]
//! Verification/Certificate View screen layout model (Phase 2).
//!
//! Provides the data skeleton for the Makepad 2.0 Splash DSL layout
//! that renders the Verification/Certificate screen.  This is a
//! LAYOUT-ONLY implementation -- all data is placeholder; no IPC
//! wiring yet.
//!
//! Layout structure:
//! ```text
//! +---------------------------------------------------------------+
//! | vb -- Verification  [Workflow: issue-triage]                  |
//! +----------------------------+----------------------------------+
//! | Certificate Cards          |  Detail Inspector                |
//! | [Structural Validity  PASS]|  +- Taint Flow Overlay ---------+|
//! | [Bounded Transitions  PASS]|  | Source: Step(0) Secret       ||
//! | [Secret-to-Result     PASS]|  | Sink:   Step(5) Finish       ||
//! | [Durability          WARN] |  | Path:   0 -> 2 -> 5          ||
//! | [Idempotency          PASS]|  +-------------------------------+|
//! | [Memory Budget        PASS]|  +- Resource Bounds Panel -------+|
//! | [Max Transitions      PASS]|  | slots: 4/1024  frames: 3/128 ||
//! | [Max Action Calls     PASS]|  | payload: 1024  result: 512   ||
//! +----------------------------+  +-------------------------------+|
//! |                            |  +- Action Policy Panel ---------+|
//! |                            |  | Do#1: DeterministicPure, S    ||
//! |                            |  | Do#2: AtLeastOnce, !S         ||
//! |                            |  +-------------------------------+|
//! +----------------------------+----------------------------------+
//! ```

use crate::verify::action_policy::{ActionPolicyReport, IdempotencyClass};
use crate::verify::certificates::{Certificate, CertificateStatus, VerificationResult};
use crate::verify::resources::{ResourceBoundsPanel, ResourceStatus};
use crate::verify::taint_overlay::TaintOverlayResult;

// ---------------------------------------------------------------------------
// Color constants -- cyberpunk palette (mirrors replay/screen.rs)
// ---------------------------------------------------------------------------

/// Panel background: `#12121f`.
pub const PANEL_BG: &str = "#12121f";
/// Card background: `#16162a`.
pub const CARD_BG: &str = "#16162a";
/// Border color: `#2a2a4a`.
pub const BORDER: &str = "#2a2a4a";
/// Primary text: `#e8e8ff`.
pub const TEXT_PRIMARY: &str = "#e8e8ff";
/// Secondary text: `#8888aa`.
pub const TEXT_SECONDARY: &str = "#8888aa";
/// Neon cyan accent: `#00f5ff`.
pub const NEON_CYAN: &str = "#00f5ff";
/// Neon green accent: `#39ff14`.
pub const NEON_GREEN: &str = "#39ff14";
/// Neon red accent: `#ff073a`.
pub const NEON_RED: &str = "#ff073a";
/// Neon orange accent: `#ff6b00`.
pub const NEON_ORANGE: &str = "#ff6b00";
/// Text dim / label color: `#555577`.
pub const TEXT_DIM: &str = "#555577";
/// Canvas background: `#0a0a12`.
pub const CANVAS_BG: &str = "#0a0a12";
/// Neon magenta for secret sources: `#ff00ff`.
pub const NEON_MAGENTA: &str = "#ff00ff";
/// Neon teal for safe finish: `#00e5c7`.
pub const NEON_TEAL: &str = "#00e5c7";
/// Neon purple for authentication: `#b14dff`.
pub const NEON_PURPLE: &str = "#b14dff";

// ---------------------------------------------------------------------------
// CertificateCard -- one per verification check
// ---------------------------------------------------------------------------

/// Status badge shown on a certificate card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CardStatus {
    /// Verification check passed.
    Pass,
    /// Verification check failed.
    Fail,
    /// Verification check passed with a warning.
    Warning,
}

impl CardStatus {
    /// Returns the display label for this status badge.
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Fail => "FAIL",
            Self::Warning => "WARN",
        }
    }

    /// Returns the hex color for this status badge.
    #[must_use]
    pub fn color(&self) -> &'static str {
        match self {
            Self::Pass => NEON_GREEN,
            Self::Fail => NEON_RED,
            Self::Warning => NEON_ORANGE,
        }
    }
}

/// One certificate card in the verification panel.
#[derive(Debug, Clone)]
pub struct CertificateCard {
    /// Card title, e.g. "Structural Validity".
    pub title: String,
    /// Current status badge.
    pub status: CardStatus,
    /// Short detail string, e.g. "4 nodes, entry in bounds".
    pub detail: String,
    /// Whether the card is expanded (showing extra content).
    pub expanded: bool,
    /// Title text color (hex).
    pub title_color: String,
    /// Background color for the card (hex).
    pub bg_color: String,
    /// Border color for the card (hex).
    pub border_color: String,
}

impl CertificateCard {
    /// Creates a new certificate card with the given fields.
    #[must_use]
    pub fn new(title: &str, status: CardStatus, detail: &str) -> Self {
        let border_color = match status {
            CardStatus::Pass => NEON_GREEN,
            CardStatus::Fail => NEON_RED,
            CardStatus::Warning => NEON_ORANGE,
        };
        Self {
            title: String::from(title),
            status,
            detail: String::from(detail),
            expanded: false,
            title_color: String::from(TEXT_PRIMARY),
            bg_color: String::from(CARD_BG),
            border_color: String::from(border_color),
        }
    }

    /// Toggles the expanded state.
    pub fn toggle_expand(&mut self) {
        self.expanded = !self.expanded;
    }
}

// ---------------------------------------------------------------------------
// TaintFlowOverlay -- model for secret flow visualization
// ---------------------------------------------------------------------------

/// A single source node in the taint flow overlay.
#[derive(Debug, Clone)]
pub struct TaintSourceNode {
    /// Step index of the source.
    pub step_idx: u16,
    /// Human-readable label, e.g. "WaitEvent".
    pub label: String,
    /// Hex color for the node.
    pub color: String,
}

/// A single sink path from source to sink.
#[derive(Debug, Clone)]
pub struct TaintSinkPath {
    /// Source step index.
    pub source_step: u16,
    /// Sink step index (Finish node).
    pub sink_step: u16,
    /// Intermediate step indices along the path.
    pub intermediate: Vec<u16>,
    /// Whether this path reaches a forbidden sink.
    pub is_forbidden: bool,
    /// Hex color for the path line.
    pub path_color: String,
    /// Status label, e.g. "Dangerous" or "Warning".
    pub status_label: String,
}

/// Complete taint flow overlay model for rendering.
#[derive(Debug, Clone)]
pub struct TaintFlowOverlay {
    /// Source nodes (where secrets enter).
    pub sources: Vec<TaintSourceNode>,
    /// Paths from sources to sinks.
    pub sink_paths: Vec<TaintSinkPath>,
    /// Whether the Finish node is safe (no secret reaches it).
    pub finish_safe: bool,
    /// Summary text, e.g. "2 sources, 0 leaks".
    pub summary: String,
}

impl TaintFlowOverlay {
    /// Creates a placeholder taint flow overlay for the default screen.
    #[must_use]
    pub fn new_placeholder() -> Self {
        Self {
            sources: vec![TaintSourceNode {
                step_idx: 0,
                label: String::from("SetConst"),
                color: String::from(NEON_MAGENTA),
            }],
            sink_paths: vec![TaintSinkPath {
                source_step: 0,
                sink_step: 5,
                intermediate: vec![2],
                is_forbidden: false,
                path_color: String::from(NEON_ORANGE),
                status_label: String::from("Warning"),
            }],
            finish_safe: true,
            summary: String::from("1 source, 0 leaks"),
        }
    }

    /// Creates an empty overlay with no sources or paths.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            sources: Vec::new(),
            sink_paths: Vec::new(),
            finish_safe: true,
            summary: String::from("0 sources, 0 leaks"),
        }
    }

    /// Returns the number of source nodes.
    #[must_use]
    pub fn source_count(&self) -> usize {
        self.sources.len()
    }

    /// Returns the number of sink paths.
    #[must_use]
    pub fn path_count(&self) -> usize {
        self.sink_paths.len()
    }

    /// Returns the number of forbidden paths.
    #[must_use]
    pub fn forbidden_path_count(&self) -> usize {
        self.sink_paths.iter().filter(|p| p.is_forbidden).count()
    }
}

// ---------------------------------------------------------------------------
// ResourceBoundsPanel -- slot count, max frame size, etc.
// ---------------------------------------------------------------------------

/// One row in the resource bounds display.
#[derive(Debug, Clone)]
pub struct ResourceRow {
    /// Human-readable label, e.g. "Slots".
    pub label: String,
    /// Current/used value.
    pub current: u64,
    /// Maximum/contract value.
    pub maximum: u64,
    /// Unit suffix, e.g. "bytes".
    pub unit: String,
    /// Status of this resource row.
    pub status: ResourceRowStatus,
}

/// Status of a resource row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ResourceRowStatus {
    /// Within bounds.
    Ok,
    /// At or near the limit.
    Warning,
    /// Exceeds the limit.
    Over,
}

impl ResourceRowStatus {
    /// Returns the hex color for this status.
    #[must_use]
    pub fn color(&self) -> &'static str {
        match self {
            Self::Ok => NEON_GREEN,
            Self::Warning => NEON_ORANGE,
            Self::Over => NEON_RED,
        }
    }
}

/// Complete resource bounds panel for rendering.
#[derive(Debug, Clone)]
pub struct ResourceBoundsDisplay {
    /// Rows of resource metrics.
    pub rows: Vec<ResourceRow>,
    /// Whether all resources are within bounds.
    pub all_ok: bool,
    /// Summary text, e.g. "7/8 metrics within bounds".
    pub summary: String,
}

impl ResourceBoundsDisplay {
    /// Creates a placeholder resource bounds display.
    #[must_use]
    pub fn new_placeholder() -> Self {
        let rows = vec![
            ResourceRow {
                label: String::from("Slots"),
                current: 4,
                maximum: 1024,
                unit: String::from(""),
                status: ResourceRowStatus::Ok,
            },
            ResourceRow {
                label: String::from("Max Frame Size"),
                current: 3,
                maximum: 128,
                unit: String::from(""),
                status: ResourceRowStatus::Ok,
            },
            ResourceRow {
                label: String::from("Action Payload"),
                current: 1024,
                maximum: 1024,
                unit: String::from("bytes"),
                status: ResourceRowStatus::Warning,
            },
            ResourceRow {
                label: String::from("Result Size"),
                current: 512,
                maximum: 512,
                unit: String::from("bytes"),
                status: ResourceRowStatus::Warning,
            },
            ResourceRow {
                label: String::from("Queue Depth"),
                current: 1,
                maximum: 64,
                unit: String::from(""),
                status: ResourceRowStatus::Ok,
            },
        ];
        Self {
            all_ok: false,
            summary: String::from("3/5 metrics within bounds"),
            rows,
        }
    }

    /// Returns the number of rows.
    #[must_use]
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }
}

// ---------------------------------------------------------------------------
// ActionPolicyPanel -- per-Do-node policy details
// ---------------------------------------------------------------------------

/// Per-Do-node policy card in the action policy panel.
#[derive(Debug, Clone)]
pub struct ActionPolicyCard {
    /// Action ID.
    pub action_id: u16,
    /// Idempotency class label, e.g. "DeterministicPure".
    pub idempotency_label: String,
    /// Hex color for idempotency label.
    pub idempotency_color: String,
    /// Whether the action has a timeout.
    pub has_timeout: bool,
    /// Timeout in ms, if configured.
    pub timeout_ms: Option<u32>,
    /// Whether strict mode is eligible.
    pub strict_eligible: bool,
    /// Hex color for strict eligibility badge.
    pub strict_color: String,
    /// Capability labels, e.g. "net:http", "fs:read".
    pub capabilities: Vec<String>,
    /// Policy issue labels.
    pub issues: Vec<String>,
    /// Card background color.
    pub bg_color: String,
    /// Border color.
    pub border_color: String,
}

/// Complete action policy panel for rendering.
#[derive(Debug, Clone)]
pub struct ActionPolicyDisplay {
    /// Per-action policy cards.
    pub cards: Vec<ActionPolicyCard>,
    /// Whether all actions are strict-eligible.
    pub all_strict_eligible: bool,
    /// Summary text.
    pub summary: String,
}

impl ActionPolicyDisplay {
    /// Creates a placeholder action policy display.
    #[must_use]
    pub fn new_placeholder() -> Self {
        let cards = vec![
            ActionPolicyCard {
                action_id: 1,
                idempotency_label: String::from("DeterministicPure"),
                idempotency_color: String::from(NEON_GREEN),
                has_timeout: true,
                timeout_ms: Some(5000),
                strict_eligible: true,
                strict_color: String::from(NEON_GREEN),
                capabilities: vec![String::from("net:http")],
                issues: Vec::new(),
                bg_color: String::from(CARD_BG),
                border_color: String::from(NEON_GREEN),
            },
            ActionPolicyCard {
                action_id: 2,
                idempotency_label: String::from("AtLeastOnce"),
                idempotency_color: String::from(NEON_ORANGE),
                has_timeout: true,
                timeout_ms: Some(3000),
                strict_eligible: false,
                strict_color: String::from(NEON_RED),
                capabilities: Vec::new(),
                issues: vec![String::from("MissingIdempotency")],
                bg_color: String::from(CARD_BG),
                border_color: String::from(NEON_ORANGE),
            },
        ];
        Self {
            all_strict_eligible: false,
            summary: String::from("2 actions, 1 strict-eligible"),
            cards,
        }
    }

    /// Returns the number of action policy cards.
    #[must_use]
    pub fn card_count(&self) -> usize {
        self.cards.len()
    }
}

// ---------------------------------------------------------------------------
// VerificationPanel -- top-level container
// ---------------------------------------------------------------------------

/// Top-level data model for the Verification/Certificate screen layout.
///
/// Contains all the placeholder data needed to render the four
/// quadrants of the Verification View:
///
/// 1. **Top bar** -- workflow name
/// 2. **Left** -- certificate cards (one per check)
/// 3. **Right top** -- taint flow overlay
/// 4. **Right middle** -- resource bounds panel
/// 5. **Right bottom** -- action policy panel
pub struct VerificationPanel {
    // -- Top bar --
    /// Displayed workflow name.
    pub workflow_name: String,

    // -- Left: certificate cards --
    /// Certificate cards (one per verification check).
    pub certificate_cards: Vec<CertificateCard>,

    // -- Right: taint flow overlay --
    /// Taint flow visualization model.
    pub taint_overlay: TaintFlowOverlay,

    // -- Right: resource bounds --
    /// Resource bounds panel model.
    pub resource_bounds: ResourceBoundsDisplay,

    // -- Right: action policy --
    /// Action policy panel model.
    pub action_policy: ActionPolicyDisplay,
}

impl VerificationPanel {
    /// Create a new panel populated with placeholder data matching the
    /// Phase 2 Verification layout spec.
    #[must_use]
    pub fn new() -> Self {
        let workflow_name = String::from("issue-triage");

        // -- Certificate cards (8 checks) --
        let certificate_cards = vec![
            CertificateCard::new(
                "Structural Validity",
                CardStatus::Pass,
                "4 nodes, entry in bounds, IDs match positions",
            ),
            CertificateCard::new(
                "Bounded Transitions",
                CardStatus::Pass,
                "max_steps=10000, max_slots=1024 within limits",
            ),
            CertificateCard::new(
                "Secret-to-Result Leak",
                CardStatus::Pass,
                "1 source, 0 leaks detected",
            ),
            CertificateCard::new(
                "Durability",
                CardStatus::Warning,
                "1 Do node without on_error handler: step 3",
            ),
            CertificateCard::new(
                "Idempotency",
                CardStatus::Pass,
                "all Do nodes have idempotency declarations",
            ),
            CertificateCard::new(
                "Memory Budget",
                CardStatus::Pass,
                "slot_count=4 <= max_slots=1024",
            ),
            CertificateCard::new(
                "Max Transitions",
                CardStatus::Pass,
                "6 transitions <= max_steps=10000",
            ),
            CertificateCard::new(
                "Max Action Calls",
                CardStatus::Pass,
                "2 Do nodes, retry budget=6",
            ),
        ];

        // -- Right panels (placeholder) --
        let taint_overlay = TaintFlowOverlay::new_placeholder();
        let resource_bounds = ResourceBoundsDisplay::new_placeholder();
        let action_policy = ActionPolicyDisplay::new_placeholder();

        Self {
            workflow_name,
            certificate_cards,
            taint_overlay,
            resource_bounds,
            action_policy,
        }
    }

    /// Returns the formatted top-bar title string.
    #[must_use]
    pub fn title_text(&self) -> String {
        String::from("vb")
    }

    /// Returns the formatted page title string.
    #[must_use]
    pub fn page_title(&self) -> String {
        String::from("Verification")
    }

    /// Returns the formatted workflow badge text.
    #[must_use]
    pub fn workflow_name_text(&self) -> String {
        self.workflow_name.clone()
    }

    /// Returns the certificate panel header label.
    #[must_use]
    pub fn certificate_header_text(&self) -> String {
        String::from("CERTIFICATES")
    }

    /// Returns the taint overlay header label.
    #[must_use]
    pub fn taint_header_text(&self) -> String {
        String::from("TAINT FLOW")
    }

    /// Returns the resource bounds header label.
    #[must_use]
    pub fn resource_header_text(&self) -> String {
        String::from("RESOURCE BOUNDS")
    }

    /// Returns the action policy header label.
    #[must_use]
    pub fn action_policy_header_text(&self) -> String {
        String::from("ACTION POLICY")
    }

    /// Returns the number of certificate cards.
    #[must_use]
    pub fn certificate_count(&self) -> usize {
        self.certificate_cards.len()
    }

    /// Returns the number of passing certificates.
    #[must_use]
    pub fn pass_count(&self) -> usize {
        self.certificate_cards
            .iter()
            .filter(|c| c.status == CardStatus::Pass)
            .count()
    }

    /// Returns the number of failing certificates.
    #[must_use]
    pub fn fail_count(&self) -> usize {
        self.certificate_cards
            .iter()
            .filter(|c| c.status == CardStatus::Fail)
            .count()
    }

    /// Returns the number of warning certificates.
    #[must_use]
    pub fn warn_count(&self) -> usize {
        self.certificate_cards
            .iter()
            .filter(|c| c.status == CardStatus::Warning)
            .count()
    }

    /// Returns a reference to the certificate cards.
    #[must_use]
    pub fn certificate_cards(&self) -> &[CertificateCard] {
        &self.certificate_cards
    }

    /// Returns a reference to the taint overlay.
    #[must_use]
    pub fn taint_overlay(&self) -> &TaintFlowOverlay {
        &self.taint_overlay
    }

    /// Returns a reference to the resource bounds display.
    #[must_use]
    pub fn resource_bounds(&self) -> &ResourceBoundsDisplay {
        &self.resource_bounds
    }

    /// Returns a reference to the action policy display.
    #[must_use]
    pub fn action_policy(&self) -> &ActionPolicyDisplay {
        &self.action_policy
    }

    /// Returns a summary badge text, e.g. "6/8 passed".
    #[must_use]
    pub fn summary_badge(&self) -> String {
        let passes = self.pass_count();
        let total = self.certificate_count();
        format!("{}/{} passed", passes, total)
    }
}

impl Default for VerificationPanel {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Conversion helpers: from domain types to screen types
// ---------------------------------------------------------------------------

/// Converts a `CertificateStatus` into a screen `CardStatus`.
#[must_use]
pub fn cert_status_to_card_status(status: &CertificateStatus) -> CardStatus {
    match status {
        CertificateStatus::Pass => CardStatus::Pass,
        CertificateStatus::Fail(_) => CardStatus::Fail,
        CertificateStatus::Warn(_) => CardStatus::Warning,
    }
}

/// Converts a `Certificate` into a screen `CertificateCard`.
#[must_use]
pub fn certificate_to_card(cert: &Certificate) -> CertificateCard {
    let status = cert_status_to_card_status(&cert.status);
    CertificateCard::new(&format!("{:?}", cert.kind), status, &cert.details)
}

/// Converts a `VerificationResult` into a `Vec<CertificateCard>`.
#[must_use]
pub fn verification_result_to_cards(result: &VerificationResult) -> Vec<CertificateCard> {
    result
        .certificates
        .iter()
        .map(certificate_to_card)
        .collect()
}

/// Converts a `TaintOverlayResult` into a `TaintFlowOverlay`.
#[must_use]
pub fn taint_overlay_to_display(result: &TaintOverlayResult) -> TaintFlowOverlay {
    let sources: Vec<TaintSourceNode> = result
        .sources
        .iter()
        .map(|&step| TaintSourceNode {
            step_idx: step.get(),
            label: format!("Step({})", step.get()),
            color: String::from(NEON_MAGENTA),
        })
        .collect();

    let sink_paths: Vec<TaintSinkPath> = result
        .flow_paths
        .iter()
        .map(|path| {
            let is_forbidden = path.is_forbidden;
            let path_color = if is_forbidden {
                String::from(NEON_RED)
            } else {
                String::from(NEON_ORANGE)
            };
            let status_label = if is_forbidden {
                String::from("Dangerous")
            } else {
                String::from("Warning")
            };
            let intermediate: Vec<u16> = path
                .path_nodes
                .iter()
                .filter(|&&s| s != path.source_step && s != path.sink_step)
                .map(|s| s.get())
                .collect();
            TaintSinkPath {
                source_step: path.source_step.get(),
                sink_step: path.sink_step.get(),
                intermediate,
                is_forbidden,
                path_color,
                status_label,
            }
        })
        .collect();

    let source_count = sources.len();
    let leak_count = sink_paths.iter().filter(|p| p.is_forbidden).count();
    let summary = format!(
        "{} source{}, {} leak{}",
        source_count,
        if source_count == 1 { "" } else { "s" },
        leak_count,
        if leak_count == 1 { "" } else { "s" }
    );

    TaintFlowOverlay {
        sources,
        sink_paths,
        finish_safe: result.finish_safe,
        summary,
    }
}

/// Converts a `ResourceBoundsPanel` into a `ResourceBoundsDisplay`.
#[must_use]
pub fn resource_panel_to_display(panel: &ResourceBoundsPanel) -> ResourceBoundsDisplay {
    let rows: Vec<ResourceRow> = panel
        .metrics()
        .iter()
        .map(|m| {
            let status = match m.status {
                ResourceStatus::WithinBounds => ResourceRowStatus::Ok,
                ResourceStatus::AtLimit => ResourceRowStatus::Warning,
                ResourceStatus::ExceedsLimit => ResourceRowStatus::Over,
            };
            ResourceRow {
                label: String::from(m.label),
                current: m.computed_value,
                maximum: m.contract_value,
                unit: String::new(),
                status,
            }
        })
        .collect();

    let ok_count = rows
        .iter()
        .filter(|r| r.status == ResourceRowStatus::Ok)
        .count();
    let total = rows.len();
    let all_ok = ok_count == total;
    let summary = format!("{}/{} metrics within bounds", ok_count, total);

    ResourceBoundsDisplay {
        rows,
        all_ok,
        summary,
    }
}

/// Converts a slice of `ActionPolicyReport` into an `ActionPolicyDisplay`.
#[must_use]
pub fn action_reports_to_display(reports: &[ActionPolicyReport]) -> ActionPolicyDisplay {
    let cards: Vec<ActionPolicyCard> = reports
        .iter()
        .map(|r| {
            let (idem_label, idem_color) = match r.idempotency_class {
                IdempotencyClass::DeterministicPure => {
                    (String::from("DeterministicPure"), String::from(NEON_GREEN))
                }
                IdempotencyClass::AtLeastOnce => {
                    (String::from("AtLeastOnce"), String::from(NEON_ORANGE))
                }
                IdempotencyClass::Unknown => (String::from("Unknown"), String::from(NEON_RED)),
            };

            let strict_color = if r.strict_eligible {
                String::from(NEON_GREEN)
            } else {
                String::from(NEON_RED)
            };

            let border_color = if r.issues.is_empty() && r.strict_eligible {
                String::from(NEON_GREEN)
            } else if r.issues.is_empty() {
                String::from(NEON_ORANGE)
            } else {
                String::from(NEON_RED)
            };

            let issues: Vec<String> = r.issues.iter().map(|i| format!("{:?}", i)).collect();

            ActionPolicyCard {
                action_id: r.action_id,
                idempotency_label: idem_label,
                idempotency_color: idem_color,
                has_timeout: r.has_timeout,
                timeout_ms: r.timeout_ms,
                strict_eligible: r.strict_eligible,
                strict_color,
                capabilities: Vec::new(),
                issues,
                bg_color: String::from(CARD_BG),
                border_color,
            }
        })
        .collect();

    let strict_count = cards.iter().filter(|c| c.strict_eligible).count();
    let total = cards.len();
    let all_strict = strict_count == total && total > 0;
    let summary = format!(
        "{} action{}, {} strict-eligible",
        total,
        if total == 1 { "" } else { "s" },
        strict_count
    );

    ActionPolicyDisplay {
        cards,
        all_strict_eligible: all_strict,
        summary,
    }
}

// ---------------------------------------------------------------------------
// Error types for fallible constructors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    VerificationIncomplete,
    ArtifactDigestMissing,
    WorkflowCorrupted,
    PanelRenderError,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::VerificationIncomplete => write!(f, "verification report has no certificates"),
            Self::ArtifactDigestMissing => write!(f, "artifact digest field is empty"),
            Self::WorkflowCorrupted => write!(f, "workflow entry index out of bounds"),
            Self::PanelRenderError => write!(f, "UI panel failed to construct from data"),
        }
    }
}

// ---------------------------------------------------------------------------
// VerificationBanner -- top banner with pass/fail status
// ---------------------------------------------------------------------------

const BANNER_GREEN: &str = "#10B981";
const BANNER_RED: &str = "#EF4444";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BannerStatus {
    Pass,
    Fail,
}

impl BannerStatus {
    #[must_use]
    pub fn color(&self) -> &'static str {
        match self {
            Self::Pass => BANNER_GREEN,
            Self::Fail => BANNER_RED,
        }
    }
}

#[derive(Debug, Clone)]
pub struct VerificationBanner {
    pub status: BannerStatus,
    pub message: String,
}

impl VerificationBanner {
    #[must_use]
    pub fn new(passed: bool) -> Self {
        if passed {
            Self {
                status: BannerStatus::Pass,
                message: String::from("Verification passed"),
            }
        } else {
            Self {
                status: BannerStatus::Fail,
                message: String::from("Verification failed"),
            }
        }
    }

    #[must_use]
    pub fn status(&self) -> BannerStatus {
        self.status
    }

    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

// ---------------------------------------------------------------------------
// VerificationGate -- one gate in the 9-gate pipeline
// ---------------------------------------------------------------------------

const GATE_PENDING_COLOR: &str = "#98A2B3";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum GateStatus {
    Pass,
    Fail,
    Warning,
    Pending,
}

impl GateStatus {
    #[must_use]
    pub fn color(&self) -> &'static str {
        match self {
            Self::Pass => NEON_GREEN,
            Self::Fail => NEON_RED,
            Self::Warning => NEON_ORANGE,
            Self::Pending => GATE_PENDING_COLOR,
        }
    }
}

#[derive(Debug, Clone)]
pub struct VerificationGate {
    pub name: String,
    pub status: GateStatus,
    pub index: usize,
}

impl VerificationGate {
    #[must_use]
    pub fn new(name: &str, status: GateStatus, index: usize) -> Self {
        Self {
            name: String::from(name),
            status,
            index,
        }
    }
}

// ---------------------------------------------------------------------------
// ArtifactPanel -- side panel with artifact metadata and digests
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ArtifactPanel {
    pub artifact_version: String,
    pub workflow_version: String,
    pub ir_digest: String,
    pub action_abi_digest: String,
    pub policy_digest: String,
    pub verified_timestamp: String,
    pub warnings: Vec<String>,
}

impl ArtifactPanel {
    #[must_use]
    pub fn new(
        artifact_version: &str,
        workflow_version: &str,
        ir_digest: &str,
        action_abi_digest: &str,
        policy_digest: &str,
        verified_timestamp: &str,
        warnings: Vec<String>,
    ) -> Self {
        Self {
            artifact_version: String::from(artifact_version),
            workflow_version: String::from(workflow_version),
            ir_digest: String::from(ir_digest),
            action_abi_digest: String::from(action_abi_digest),
            policy_digest: String::from(policy_digest),
            verified_timestamp: String::from(verified_timestamp),
            warnings,
        }
    }

    #[must_use]
    pub fn warning_count(&self) -> usize {
        self.warnings.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ir_digest.is_empty()
            && self.action_abi_digest.is_empty()
            && self.policy_digest.is_empty()
    }
}

// ---------------------------------------------------------------------------
// ProofSummary -- five boolean badges for verification outcome
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofSummary {
    pub bounded: bool,
    pub taint_safe: bool,
    pub retry_safe: bool,
    pub durable: bool,
    pub replayable: bool,
}

impl ProofSummary {
    #[must_use]
    pub fn new(
        bounded: bool,
        taint_safe: bool,
        retry_safe: bool,
        durable: bool,
        replayable: bool,
    ) -> Self {
        Self {
            bounded,
            taint_safe,
            retry_safe,
            durable,
            replayable,
        }
    }

    #[must_use]
    pub fn badge_count(&self) -> usize {
        5
    }

    #[must_use]
    pub fn all_pass(&self) -> bool {
        self.bounded && self.taint_safe && self.retry_safe && self.durable && self.replayable
    }

    #[must_use]
    pub fn pass_count(&self) -> usize {
        let mut count = 0usize;
        if self.bounded {
            count = count.saturating_add(1);
        }
        if self.taint_safe {
            count = count.saturating_add(1);
        }
        if self.retry_safe {
            count = count.saturating_add(1);
        }
        if self.durable {
            count = count.saturating_add(1);
        }
        if self.replayable {
            count = count.saturating_add(1);
        }
        count
    }
}

// ---------------------------------------------------------------------------
// VerificationCertificateView -- top-level container for Screen 4
// ---------------------------------------------------------------------------

pub struct VerificationCertificateView {
    pub banner: VerificationBanner,
    pub certificate_cards: Vec<CertificateCard>,
    pub gates: Vec<VerificationGate>,
    pub artifact_panel: ArtifactPanel,
    pub proof_summary: ProofSummary,
}

impl VerificationCertificateView {
    #[must_use]
    pub fn new(
        banner: VerificationBanner,
        certificate_cards: Vec<CertificateCard>,
        gates: Vec<VerificationGate>,
        artifact_panel: ArtifactPanel,
        proof_summary: ProofSummary,
    ) -> Self {
        Self {
            banner,
            certificate_cards,
            gates,
            artifact_panel,
            proof_summary,
        }
    }

    #[must_use]
    pub fn gate_count(&self) -> usize {
        self.gates.len()
    }

    #[must_use]
    pub fn card_count(&self) -> usize {
        self.certificate_cards.len()
    }
}

impl Default for VerificationCertificateView {
    fn default() -> Self {
        let banner = VerificationBanner::new(true);
        let certificate_cards = Vec::new();
        let gates = Vec::new();
        let artifact_panel = ArtifactPanel::new("", "", "", "", "", "", Vec::new());
        let proof_summary = ProofSummary::new(false, false, false, false, false);
        Self {
            banner,
            certificate_cards,
            gates,
            artifact_panel,
            proof_summary,
        }
    }
}

// ---------------------------------------------------------------------------
// 9-gate pipeline builder
// ---------------------------------------------------------------------------

/// Build the standard 9-gate pipeline in order.
/// Gates: Parse, Graph check, Policy, Resources, Taint, Durability, Idempotency, Capability, Result
#[must_use]
pub fn build_gate_pipeline() -> Vec<VerificationGate> {
    let gate_names = [
        "Parse",
        "Graph check",
        "Policy",
        "Resources",
        "Taint",
        "Durability",
        "Idempotency",
        "Capability",
        "Result",
    ];
    gate_names
        .iter()
        .enumerate()
        .map(|(i, name)| VerificationGate::new(name, GateStatus::Pending, i))
        .collect()
}

/// Returns the 8 certificate card titles in the contract-specified order.
#[must_use]
pub fn certificate_card_titles() -> Vec<&'static str> {
    vec![
        "Structure",
        "Boundedness",
        "Resources",
        "Taint/Secrets",
        "Action policy",
        "Durability",
        "Idempotency",
        "Capability",
    ]
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // VerificationPanel::new() basic tests
    // ========================================================================

    #[test]
    fn new_panel_has_placeholder_workflow_name() {
        let panel = VerificationPanel::new();
        assert_eq!(panel.workflow_name, "issue-triage");
    }

    #[test]
    fn default_matches_new() {
        let from_new = VerificationPanel::new();
        let from_default = VerificationPanel::default();
        assert_eq!(from_new.workflow_name, from_default.workflow_name);
        assert_eq!(
            from_new.certificate_count(),
            from_default.certificate_count()
        );
    }

    #[test]
    fn title_text_returns_vb() {
        let panel = VerificationPanel::new();
        assert_eq!(panel.title_text(), "vb");
    }

    #[test]
    fn page_title_returns_verification() {
        let panel = VerificationPanel::new();
        assert_eq!(panel.page_title(), "Verification");
    }

    #[test]
    fn workflow_name_text_returns_workflow() {
        let panel = VerificationPanel::new();
        assert_eq!(panel.workflow_name_text(), "issue-triage");
    }

    #[test]
    fn certificate_header_text() {
        let panel = VerificationPanel::new();
        assert_eq!(panel.certificate_header_text(), "CERTIFICATES");
    }

    #[test]
    fn taint_header_text() {
        let panel = VerificationPanel::new();
        assert_eq!(panel.taint_header_text(), "TAINT FLOW");
    }

    #[test]
    fn resource_header_text() {
        let panel = VerificationPanel::new();
        assert_eq!(panel.resource_header_text(), "RESOURCE BOUNDS");
    }

    #[test]
    fn action_policy_header_text() {
        let panel = VerificationPanel::new();
        assert_eq!(panel.action_policy_header_text(), "ACTION POLICY");
    }

    // ========================================================================
    // Certificate cards
    // ========================================================================

    #[test]
    fn new_panel_has_eight_certificate_cards() {
        let panel = VerificationPanel::new();
        assert_eq!(panel.certificate_count(), 8);
    }

    #[test]
    fn certificate_card_titles_from_panel() {
        let panel = VerificationPanel::new();
        let cards = panel.certificate_cards();
        assert_eq!(cards[0].title, "Structural Validity");
        assert_eq!(cards[1].title, "Bounded Transitions");
        assert_eq!(cards[2].title, "Secret-to-Result Leak");
        assert_eq!(cards[3].title, "Durability");
        assert_eq!(cards[4].title, "Idempotency");
        assert_eq!(cards[5].title, "Memory Budget");
        assert_eq!(cards[6].title, "Max Transitions");
        assert_eq!(cards[7].title, "Max Action Calls");
    }

    #[test]
    fn certificate_card_statuses() {
        let panel = VerificationPanel::new();
        assert_eq!(panel.certificate_cards()[0].status, CardStatus::Pass);
        assert_eq!(panel.certificate_cards()[1].status, CardStatus::Pass);
        assert_eq!(panel.certificate_cards()[2].status, CardStatus::Pass);
        assert_eq!(panel.certificate_cards()[3].status, CardStatus::Warning);
        assert_eq!(panel.certificate_cards()[4].status, CardStatus::Pass);
        assert_eq!(panel.certificate_cards()[5].status, CardStatus::Pass);
        assert_eq!(panel.certificate_cards()[6].status, CardStatus::Pass);
        assert_eq!(panel.certificate_cards()[7].status, CardStatus::Pass);
    }

    #[test]
    fn pass_fail_warn_counts() {
        let panel = VerificationPanel::new();
        assert_eq!(panel.pass_count(), 7);
        assert_eq!(panel.fail_count(), 0);
        assert_eq!(panel.warn_count(), 1);
    }

    #[test]
    fn summary_badge_text() {
        let panel = VerificationPanel::new();
        assert_eq!(panel.summary_badge(), "7/8 passed");
    }

    #[test]
    fn certificate_card_details_not_empty() {
        let panel = VerificationPanel::new();
        for (i, card) in panel.certificate_cards().iter().enumerate() {
            assert!(
                !card.detail.is_empty(),
                "card {} detail should not be empty",
                i
            );
        }
    }

    #[test]
    fn certificate_card_default_not_expanded() {
        let panel = VerificationPanel::new();
        for (i, card) in panel.certificate_cards().iter().enumerate() {
            assert!(
                !card.expanded,
                "card {} should not be expanded by default",
                i
            );
        }
    }

    // ========================================================================
    // CertificateCard
    // ========================================================================

    #[test]
    fn certificate_card_new_pass() {
        let card = CertificateCard::new("Test", CardStatus::Pass, "ok");
        assert_eq!(card.title, "Test");
        assert_eq!(card.status, CardStatus::Pass);
        assert_eq!(card.detail, "ok");
        assert!(!card.expanded);
        assert_eq!(card.border_color, NEON_GREEN);
    }

    #[test]
    fn certificate_card_new_fail() {
        let card = CertificateCard::new("Test", CardStatus::Fail, "broken");
        assert_eq!(card.status, CardStatus::Fail);
        assert_eq!(card.border_color, NEON_RED);
    }

    #[test]
    fn certificate_card_new_warning() {
        let card = CertificateCard::new("Test", CardStatus::Warning, "caution");
        assert_eq!(card.status, CardStatus::Warning);
        assert_eq!(card.border_color, NEON_ORANGE);
    }

    #[test]
    fn certificate_card_toggle_expand() {
        let mut card = CertificateCard::new("Test", CardStatus::Pass, "ok");
        assert!(!card.expanded);
        card.toggle_expand();
        assert!(card.expanded);
        card.toggle_expand();
        assert!(!card.expanded);
    }

    #[test]
    fn certificate_card_clone_roundtrip() {
        let card = CertificateCard::new("Clone Test", CardStatus::Fail, "detail");
        let cloned = card.clone();
        assert_eq!(cloned.title, card.title);
        assert_eq!(cloned.status, card.status);
        assert_eq!(cloned.detail, card.detail);
        assert_eq!(cloned.border_color, card.border_color);
    }

    // ========================================================================
    // CardStatus
    // ========================================================================

    #[test]
    fn card_status_labels() {
        assert_eq!(CardStatus::Pass.label(), "PASS");
        assert_eq!(CardStatus::Fail.label(), "FAIL");
        assert_eq!(CardStatus::Warning.label(), "WARN");
    }

    #[test]
    fn card_status_colors() {
        assert_eq!(CardStatus::Pass.color(), NEON_GREEN);
        assert_eq!(CardStatus::Fail.color(), NEON_RED);
        assert_eq!(CardStatus::Warning.color(), NEON_ORANGE);
    }

    #[test]
    fn card_status_equality() {
        assert_eq!(CardStatus::Pass, CardStatus::Pass);
        assert_ne!(CardStatus::Pass, CardStatus::Fail);
        assert_ne!(CardStatus::Fail, CardStatus::Warning);
    }

    // ========================================================================
    // TaintFlowOverlay
    // ========================================================================

    #[test]
    fn taint_overlay_placeholder_has_one_source() {
        let overlay = TaintFlowOverlay::new_placeholder();
        assert_eq!(overlay.source_count(), 1);
    }

    #[test]
    fn taint_overlay_placeholder_has_one_path() {
        let overlay = TaintFlowOverlay::new_placeholder();
        assert_eq!(overlay.path_count(), 1);
    }

    #[test]
    fn taint_overlay_placeholder_finish_safe() {
        let overlay = TaintFlowOverlay::new_placeholder();
        assert!(overlay.finish_safe);
    }

    #[test]
    fn taint_overlay_placeholder_no_forbidden_paths() {
        let overlay = TaintFlowOverlay::new_placeholder();
        assert_eq!(overlay.forbidden_path_count(), 0);
    }

    #[test]
    fn taint_overlay_empty_has_nothing() {
        let overlay = TaintFlowOverlay::empty();
        assert_eq!(overlay.source_count(), 0);
        assert_eq!(overlay.path_count(), 0);
        assert!(overlay.finish_safe);
        assert_eq!(overlay.summary, "0 sources, 0 leaks");
    }

    #[test]
    fn taint_source_node_fields() {
        let node = TaintSourceNode {
            step_idx: 3,
            label: String::from("Ask"),
            color: String::from(NEON_MAGENTA),
        };
        assert_eq!(node.step_idx, 3);
        assert_eq!(node.label, "Ask");
    }

    #[test]
    fn taint_sink_path_fields() {
        let path = TaintSinkPath {
            source_step: 0,
            sink_step: 5,
            intermediate: vec![2, 3],
            is_forbidden: true,
            path_color: String::from(NEON_RED),
            status_label: String::from("Dangerous"),
        };
        assert!(path.is_forbidden);
        assert_eq!(path.intermediate.len(), 2);
    }

    #[test]
    fn taint_overlay_clone_roundtrip() {
        let overlay = TaintFlowOverlay::new_placeholder();
        let cloned = overlay.clone();
        assert_eq!(cloned.source_count(), overlay.source_count());
        assert_eq!(cloned.path_count(), overlay.path_count());
        assert_eq!(cloned.finish_safe, overlay.finish_safe);
    }

    #[test]
    fn taint_overlay_panel_accessor() {
        let panel = VerificationPanel::new();
        assert_eq!(panel.taint_overlay().source_count(), 1);
        assert_eq!(panel.taint_overlay().path_count(), 1);
    }

    // ========================================================================
    // ResourceBoundsDisplay
    // ========================================================================

    #[test]
    fn resource_bounds_placeholder_has_five_rows() {
        let display = ResourceBoundsDisplay::new_placeholder();
        assert_eq!(display.row_count(), 5);
    }

    #[test]
    fn resource_bounds_placeholder_not_all_ok() {
        let display = ResourceBoundsDisplay::new_placeholder();
        assert!(!display.all_ok);
    }

    #[test]
    fn resource_bounds_placeholder_summary() {
        let display = ResourceBoundsDisplay::new_placeholder();
        assert_eq!(display.summary, "3/5 metrics within bounds");
    }

    #[test]
    fn resource_bounds_row_labels() {
        let display = ResourceBoundsDisplay::new_placeholder();
        assert_eq!(display.rows[0].label, "Slots");
        assert_eq!(display.rows[1].label, "Max Frame Size");
        assert_eq!(display.rows[2].label, "Action Payload");
        assert_eq!(display.rows[3].label, "Result Size");
        assert_eq!(display.rows[4].label, "Queue Depth");
    }

    #[test]
    fn resource_bounds_row_statuses() {
        let display = ResourceBoundsDisplay::new_placeholder();
        assert_eq!(display.rows[0].status, ResourceRowStatus::Ok);
        assert_eq!(display.rows[1].status, ResourceRowStatus::Ok);
        assert_eq!(display.rows[2].status, ResourceRowStatus::Warning);
        assert_eq!(display.rows[3].status, ResourceRowStatus::Warning);
        assert_eq!(display.rows[4].status, ResourceRowStatus::Ok);
    }

    #[test]
    fn resource_row_status_colors() {
        assert_eq!(ResourceRowStatus::Ok.color(), NEON_GREEN);
        assert_eq!(ResourceRowStatus::Warning.color(), NEON_ORANGE);
        assert_eq!(ResourceRowStatus::Over.color(), NEON_RED);
    }

    #[test]
    fn resource_bounds_panel_accessor() {
        let panel = VerificationPanel::new();
        assert_eq!(panel.resource_bounds().row_count(), 5);
    }

    #[test]
    fn resource_row_clone_roundtrip() {
        let row = ResourceRow {
            label: String::from("Test"),
            current: 10,
            maximum: 100,
            unit: String::from("bytes"),
            status: ResourceRowStatus::Ok,
        };
        let cloned = row.clone();
        assert_eq!(cloned.label, row.label);
        assert_eq!(cloned.current, row.current);
        assert_eq!(cloned.maximum, row.maximum);
    }

    // ========================================================================
    // ActionPolicyDisplay
    // ========================================================================

    #[test]
    fn action_policy_placeholder_has_two_cards() {
        let display = ActionPolicyDisplay::new_placeholder();
        assert_eq!(display.card_count(), 2);
    }

    #[test]
    fn action_policy_placeholder_not_all_strict() {
        let display = ActionPolicyDisplay::new_placeholder();
        assert!(!display.all_strict_eligible);
    }

    #[test]
    fn action_policy_placeholder_summary() {
        let display = ActionPolicyDisplay::new_placeholder();
        assert_eq!(display.summary, "2 actions, 1 strict-eligible");
    }

    #[test]
    fn action_policy_first_card_is_pure() {
        let display = ActionPolicyDisplay::new_placeholder();
        let card = display.cards.first().expect("first card");
        assert_eq!(card.action_id, 1);
        assert_eq!(card.idempotency_label, "DeterministicPure");
        assert!(card.strict_eligible);
        assert!(card.has_timeout);
        assert_eq!(card.timeout_ms, Some(5000));
        assert!(card.issues.is_empty());
    }

    #[test]
    fn action_policy_second_card_is_at_least_once() {
        let display = ActionPolicyDisplay::new_placeholder();
        let card = display.cards.get(1).expect("second card");
        assert_eq!(card.action_id, 2);
        assert_eq!(card.idempotency_label, "AtLeastOnce");
        assert!(!card.strict_eligible);
        assert!(!card.issues.is_empty());
    }

    #[test]
    fn action_policy_panel_accessor() {
        let panel = VerificationPanel::new();
        assert_eq!(panel.action_policy().card_count(), 2);
    }

    #[test]
    fn action_policy_card_clone_roundtrip() {
        let card = ActionPolicyCard {
            action_id: 5,
            idempotency_label: String::from("Unknown"),
            idempotency_color: String::from(NEON_RED),
            has_timeout: false,
            timeout_ms: None,
            strict_eligible: false,
            strict_color: String::from(NEON_RED),
            capabilities: Vec::new(),
            issues: vec![String::from("MissingTimeout")],
            bg_color: String::from(CARD_BG),
            border_color: String::from(NEON_RED),
        };
        let cloned = card.clone();
        assert_eq!(cloned.action_id, card.action_id);
        assert_eq!(cloned.issues.len(), card.issues.len());
    }

    // ========================================================================
    // Color constants
    // ========================================================================

    #[test]
    fn color_constants_match_spec() {
        assert_eq!(PANEL_BG, "#12121f");
        assert_eq!(CARD_BG, "#16162a");
        assert_eq!(BORDER, "#2a2a4a");
        assert_eq!(TEXT_PRIMARY, "#e8e8ff");
        assert_eq!(TEXT_SECONDARY, "#8888aa");
        assert_eq!(NEON_CYAN, "#00f5ff");
        assert_eq!(NEON_GREEN, "#39ff14");
        assert_eq!(NEON_RED, "#ff073a");
        assert_eq!(NEON_ORANGE, "#ff6b00");
        assert_eq!(TEXT_DIM, "#555577");
        assert_eq!(CANVAS_BG, "#0a0a12");
        assert_eq!(NEON_MAGENTA, "#ff00ff");
        assert_eq!(NEON_TEAL, "#00e5c7");
        assert_eq!(NEON_PURPLE, "#b14dff");
    }

    // ========================================================================
    // Certificate card color assignments
    // ========================================================================

    #[test]
    fn certificate_cards_have_nonempty_colors() {
        let panel = VerificationPanel::new();
        for (i, card) in panel.certificate_cards().iter().enumerate() {
            assert!(
                !card.title_color.is_empty(),
                "empty title_color for card {}",
                i
            );
            assert!(!card.bg_color.is_empty(), "empty bg_color for card {}", i);
            assert!(
                !card.border_color.is_empty(),
                "empty border_color for card {}",
                i
            );
        }
    }

    // ========================================================================
    // cert_status_to_card_status conversion
    // ========================================================================

    #[test]
    fn cert_status_pass_to_card_pass() {
        assert_eq!(
            cert_status_to_card_status(&CertificateStatus::Pass),
            CardStatus::Pass
        );
    }

    #[test]
    fn cert_status_fail_to_card_fail() {
        assert_eq!(
            cert_status_to_card_status(&CertificateStatus::Fail(String::from("reason"))),
            CardStatus::Fail
        );
    }

    #[test]
    fn cert_status_warn_to_card_warning() {
        assert_eq!(
            cert_status_to_card_status(&CertificateStatus::Warn(String::from("caution"))),
            CardStatus::Warning
        );
    }

    // ========================================================================
    // ResourceRowStatus
    // ========================================================================

    #[test]
    fn resource_row_status_equality() {
        assert_eq!(ResourceRowStatus::Ok, ResourceRowStatus::Ok);
        assert_ne!(ResourceRowStatus::Ok, ResourceRowStatus::Warning);
        assert_ne!(ResourceRowStatus::Warning, ResourceRowStatus::Over);
    }

    // ========================================================================
    // Integration: all panels accessible from VerificationPanel
    // ========================================================================

    #[test]
    fn all_panel_accessors_return_data() {
        let panel = VerificationPanel::new();
        assert!(!panel.certificate_cards().is_empty());
        assert!(panel.taint_overlay().source_count() > 0);
        assert!(panel.resource_bounds().row_count() > 0);
        assert!(panel.action_policy().card_count() > 0);
    }

    #[test]
    fn certificate_card_border_matches_status() {
        let pass_card = CertificateCard::new("P", CardStatus::Pass, "ok");
        assert_eq!(pass_card.border_color, NEON_GREEN);
        let fail_card = CertificateCard::new("F", CardStatus::Fail, "bad");
        assert_eq!(fail_card.border_color, NEON_RED);
        let warn_card = CertificateCard::new("W", CardStatus::Warning, "meh");
        assert_eq!(warn_card.border_color, NEON_ORANGE);
    }

    // ========================================================================
    // VerificationBanner
    // ========================================================================

    #[test]
    fn banner_pass() {
        let banner = VerificationBanner::new(true);
        assert_eq!(banner.status(), BannerStatus::Pass);
        assert_eq!(banner.message(), "Verification passed");
        assert_eq!(banner.status().color(), BANNER_GREEN);
    }

    #[test]
    fn banner_fail() {
        let banner = VerificationBanner::new(false);
        assert_eq!(banner.status(), BannerStatus::Fail);
        assert_eq!(banner.message(), "Verification failed");
        assert_eq!(banner.status().color(), BANNER_RED);
    }

    // ========================================================================
    // VerificationGate
    // ========================================================================

    #[test]
    fn gate_pass() {
        let gate = VerificationGate::new("Parse", GateStatus::Pass, 0);
        assert_eq!(gate.name, "Parse");
        assert_eq!(gate.status, GateStatus::Pass);
        assert_eq!(gate.index, 0);
        assert_eq!(gate.status.color(), NEON_GREEN);
    }

    #[test]
    fn gate_fail() {
        let gate = VerificationGate::new("Result", GateStatus::Fail, 8);
        assert_eq!(gate.name, "Result");
        assert_eq!(gate.status, GateStatus::Fail);
        assert_eq!(gate.index, 8);
        assert_eq!(gate.status.color(), NEON_RED);
    }

    #[test]
    fn gate_pending() {
        let gate = VerificationGate::new("Taint", GateStatus::Pending, 4);
        assert_eq!(gate.status, GateStatus::Pending);
        assert_eq!(gate.status.color(), GATE_PENDING_COLOR);
    }

    // ========================================================================
    // ArtifactPanel
    // ========================================================================

    #[test]
    fn artifact_panel_fields() {
        let panel = ArtifactPanel::new(
            "v1.0.0",
            "wf-v2",
            "abc123",
            "def456",
            "ghi789",
            "2026-05-09T12:00:00Z",
            vec![String::from("warn1"), String::from("warn2")],
        );
        assert_eq!(panel.artifact_version, "v1.0.0");
        assert_eq!(panel.workflow_version, "wf-v2");
        assert_eq!(panel.ir_digest, "abc123");
        assert_eq!(panel.action_abi_digest, "def456");
        assert_eq!(panel.policy_digest, "ghi789");
        assert_eq!(panel.verified_timestamp, "2026-05-09T12:00:00Z");
        assert_eq!(panel.warning_count(), 2);
        assert!(!panel.is_empty());
    }

    #[test]
    fn artifact_panel_empty() {
        let panel = ArtifactPanel::new("", "", "", "", "", "", Vec::new());
        assert!(panel.is_empty());
        assert_eq!(panel.warning_count(), 0);
    }

    // ========================================================================
    // ProofSummary
    // ========================================================================

    #[test]
    fn proof_summary_all_pass() {
        let summary = ProofSummary::new(true, true, true, true, true);
        assert!(summary.all_pass());
        assert_eq!(summary.badge_count(), 5);
        assert_eq!(summary.pass_count(), 5);
    }

    #[test]
    fn proof_summary_all_fail() {
        let summary = ProofSummary::new(false, false, false, false, false);
        assert!(!summary.all_pass());
        assert_eq!(summary.pass_count(), 0);
    }

    #[test]
    fn proof_summary_partial() {
        let summary = ProofSummary::new(true, false, true, false, true);
        assert!(!summary.all_pass());
        assert_eq!(summary.pass_count(), 3);
    }

    // ========================================================================
    // VerificationCertificateView
    // ========================================================================

    #[test]
    fn certificate_view_default() {
        let view = VerificationCertificateView::default();
        assert_eq!(view.card_count(), 0);
        assert_eq!(view.gate_count(), 0);
        assert!(view.proof_summary.badge_count() == 5);
    }

    #[test]
    fn certificate_view_new() {
        let banner = VerificationBanner::new(true);
        let cards = vec![CertificateCard::new("Test", CardStatus::Pass, "ok")];
        let gates = build_gate_pipeline();
        let artifact = ArtifactPanel::new("v1", "wf1", "digest", "", "", "ts", Vec::new());
        let proof = ProofSummary::new(true, true, false, true, false);
        let view = VerificationCertificateView::new(banner, cards, gates, artifact, proof);
        assert_eq!(view.card_count(), 1);
        assert_eq!(view.gate_count(), 9);
        assert_eq!(view.proof_summary.pass_count(), 3);
    }

    // ========================================================================
    // Gate pipeline and card title invariants
    // ========================================================================

    #[test]
    fn gate_pipeline_has_9_gates() {
        let gates = build_gate_pipeline();
        assert_eq!(gates.len(), 9);
        let expected_names = [
            "Parse",
            "Graph check",
            "Policy",
            "Resources",
            "Taint",
            "Durability",
            "Idempotency",
            "Capability",
            "Result",
        ];
        for (i, gate) in gates.iter().enumerate() {
            assert_eq!(gate.index, i);
            assert_eq!(gate.name, expected_names[i]);
        }
    }

    #[test]
    fn certificate_card_titles_invariant() {
        let titles = super::certificate_card_titles();
        assert_eq!(titles.len(), 8);
        assert_eq!(titles[0], "Structure");
        assert_eq!(titles[1], "Boundedness");
        assert_eq!(titles[2], "Resources");
        assert_eq!(titles[3], "Taint/Secrets");
        assert_eq!(titles[4], "Action policy");
        assert_eq!(titles[5], "Durability");
        assert_eq!(titles[6], "Idempotency");
        assert_eq!(titles[7], "Capability");
    }

    // ========================================================================
    // Error display
    // ========================================================================

    #[test]
    fn error_display() {
        assert_eq!(
            format!("{}", Error::VerificationIncomplete),
            "verification report has no certificates"
        );
        assert_eq!(
            format!("{}", Error::ArtifactDigestMissing),
            "artifact digest field is empty"
        );
        assert_eq!(
            format!("{}", Error::WorkflowCorrupted),
            "workflow entry index out of bounds"
        );
        assert_eq!(
            format!("{}", Error::PanelRenderError),
            "UI panel failed to construct from data"
        );
    }

    // ========================================================================
    // BannerStatus and GateStatus colors match contract
    // ========================================================================

    #[test]
    fn banner_pass_color_is_contract_green() {
        assert_eq!(BannerStatus::Pass.color(), "#10B981");
    }

    #[test]
    fn banner_fail_color_is_contract_red() {
        assert_eq!(BannerStatus::Fail.color(), "#EF4444");
    }

    #[test]
    fn gate_status_colors() {
        assert_eq!(GateStatus::Pass.color(), NEON_GREEN);
        assert_eq!(GateStatus::Fail.color(), NEON_RED);
        assert_eq!(GateStatus::Warning.color(), NEON_ORANGE);
        assert_eq!(GateStatus::Pending.color(), GATE_PENDING_COLOR);
    }
}
