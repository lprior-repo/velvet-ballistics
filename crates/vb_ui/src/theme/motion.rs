#![forbid(unsafe_code)]
//! UI Motion / Animation System
//!
//! Shader-based animation system for velvet-ballastics Makepad front-end.
//! Provides GPU-friendly animation primitives for packet dots, node glow,
//! timeline scrubbing, event pulses, failure path focus, taint overlays,
//! queue pressure shimmer, and certificate check cascades.
//!
//! # Budget constraints
//! - Max 256 visible animated nodes
//! - Max 512 active packet dots
//! - Max 2000 timeline events
//!
//! # Thread safety
//! All types in this module are `Send + Sync` only when the UI thread is the
//! sole writer. The `MotionManager` is designed to be owned by the UI thread.

use crate::theme::colors::neon;

pub use error::MotionError;

const MAX_VISIBLE_NODES: u32 = 256;
const MAX_PACKET_DOTS: u32 = 512;
const MAX_TIMELINE_EVENTS: u32 = 2000;

const TAINT_DEFAULT: [f32; 4] = neon::PURPLE;

fn u32_to_usize_saturating(value: u32) -> usize {
    usize::try_from(value).unwrap_or(usize::MAX)
}

fn usize_to_f32_saturating(value: usize) -> f32 {
    value.to_string().parse::<f32>().unwrap_or(f32::MAX)
}

fn u64_to_f32_saturating(value: u64) -> f32 {
    value.to_string().parse::<f32>().unwrap_or(f32::MAX)
}

fn is_unit_interval(value: f32) -> bool {
    (0.0..=1.0).contains(&value)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FrameBudget {
    Unlocked,
    AtLeast(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum GlowMode {
    Selected,
    Running,
    Failed,
    Tainted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum GateStatus {
    Pending,
    Passed,
    Failed,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MotionConfig {
    pub frame_budget: FrameBudget,
    pub max_visible_nodes: u32,
    pub max_packet_dots: u32,
    pub max_timeline_events: u32,
    pub failure_overlay_max_alpha: f32,
    pub taint_color: [f32; 4],
}

impl MotionConfig {
    pub fn new(frame_budget: FrameBudget) -> Self {
        Self {
            frame_budget,
            max_visible_nodes: MAX_VISIBLE_NODES,
            max_packet_dots: MAX_PACKET_DOTS,
            max_timeline_events: MAX_TIMELINE_EVENTS,
            failure_overlay_max_alpha: 0.6,
            taint_color: TAINT_DEFAULT,
        }
    }

    pub fn validate(&self) -> Result<(), MotionError> {
        if self.max_visible_nodes > MAX_VISIBLE_NODES {
            return Err(MotionError::NodeBudgetExceeded {
                requested: u32_to_usize_saturating(self.max_visible_nodes),
                limit: u32_to_usize_saturating(MAX_VISIBLE_NODES),
            });
        }
        if self.max_packet_dots > MAX_PACKET_DOTS {
            return Err(MotionError::PacketBudgetExceeded {
                requested: u32_to_usize_saturating(self.max_packet_dots),
                limit: u32_to_usize_saturating(MAX_PACKET_DOTS),
            });
        }
        if self.max_timeline_events > MAX_TIMELINE_EVENTS {
            return Err(MotionError::TimelineBudgetExceeded {
                requested: u32_to_usize_saturating(self.max_timeline_events),
                limit: u32_to_usize_saturating(MAX_TIMELINE_EVENTS),
            });
        }
        if !is_unit_interval(self.failure_overlay_max_alpha) {
            return Err(MotionError::StateCorrupted {
                detail: "failure_overlay_max_alpha must be in [0.0, 1.0]".into(),
            });
        }
        for ch in &self.taint_color {
            if !is_unit_interval(*ch) {
                return Err(MotionError::StateCorrupted {
                    detail: "taint_color channels must be in [0.0, 1.0]".into(),
                });
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EdgeKey {
    pub from: u16,
    pub to: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PacketDot {
    pub edge_key: EdgeKey,
    pub start_offset: f32,
    pub elapsed_ms: u64,
    pub speed: f32,
    pub color: [f32; 4],
}

impl PacketDot {
    pub fn position(&self, edge_path: &edge_path::EdgePath) -> Option<[f32; 2]> {
        if self.is_complete() {
            return None;
        }
        let total_offset = self.start_offset + u64_to_f32_saturating(self.elapsed_ms) * self.speed;
        let t = total_offset.clamp(0.0, 1.0);
        Some(edge_path.sample_at(t))
    }

    pub fn is_complete(&self) -> bool {
        let total_offset = self.start_offset + u64_to_f32_saturating(self.elapsed_ms) * self.speed;
        total_offset >= 1.0
    }
}

#[derive(Debug, Clone, Default)]
pub struct PacketSystem {
    active_dots: Vec<PacketDot>,
    max_dots: u32,
}

impl PacketSystem {
    pub fn new(max_dots: u32) -> Self {
        Self {
            active_dots: Vec::with_capacity(u32_to_usize_saturating(max_dots)),
            max_dots,
        }
    }

    pub fn spawn_packet_dot(
        &mut self,
        edge: EdgeKey,
        start_offset: f32,
        speed: f32,
        color: [f32; 4],
    ) -> Result<(), MotionError> {
        if self.active_dots.len() >= u32_to_usize_saturating(self.max_dots) {
            return Err(MotionError::PacketBudgetExceeded {
                requested: self.active_dots.len().saturating_add(1),
                limit: u32_to_usize_saturating(self.max_dots),
            });
        }
        if !is_unit_interval(start_offset) {
            return Err(MotionError::StateCorrupted {
                detail: "start_offset must be in [0.0, 1.0]".into(),
            });
        }
        if speed < 0.0 {
            return Err(MotionError::StateCorrupted {
                detail: "speed must be non-negative".into(),
            });
        }
        self.active_dots.push(PacketDot {
            edge_key: edge,
            start_offset,
            elapsed_ms: 0,
            speed,
            color,
        });
        Ok(())
    }

    pub fn tick(&mut self, delta_ms: u64) -> Result<(), MotionError> {
        for dot in &mut self.active_dots {
            dot.elapsed_ms = dot.elapsed_ms.saturating_add(delta_ms);
        }
        self.active_dots.retain(|d| !d.is_complete());
        Ok(())
    }

    pub fn active_packet_count(&self) -> usize {
        self.active_dots.len()
    }

    pub fn dots(&self) -> &[PacketDot] {
        &self.active_dots
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActiveNodeGlow {
    pub node_key: u16,
    pub mode: GlowMode,
    pub base_intensity: f32,
    pub pulse_amplitude: f32,
    pub pulse_frequency_hz: f32,
    local_elapsed_ms: u64,
}

impl ActiveNodeGlow {
    pub fn new(node_key: u16, mode: GlowMode) -> Self {
        let (base_intensity, pulse_amplitude, pulse_frequency_hz) = match mode {
            GlowMode::Selected => (0.6, 0.0, 0.0),
            GlowMode::Running => (0.4, 0.6, 2.0),
            GlowMode::Failed => (0.8, 0.0, 0.0),
            GlowMode::Tainted => (0.5, 0.3, 1.5),
        };
        Self {
            node_key,
            mode,
            base_intensity,
            pulse_amplitude,
            pulse_frequency_hz,
            local_elapsed_ms: 0,
        }
    }

    pub fn intensity(&self, ui_elapsed_ms: u64) -> f32 {
        let t_secs = u64_to_f32_saturating(ui_elapsed_ms) / 1000.0;
        let sine = (2.0 * std::f32::consts::PI * self.pulse_frequency_hz * t_secs).sin();
        let normalized = (sine + 1.0) * 0.5;
        (self.base_intensity + self.pulse_amplitude * normalized).clamp(0.0, 1.0)
    }

    pub fn tick(&mut self, delta_ms: u64) {
        self.local_elapsed_ms = self.local_elapsed_ms.saturating_add(delta_ms);
    }
}

#[derive(Debug, Clone, Default)]
pub struct GraphAnimator {
    nodes: Vec<ActiveNodeGlow>,
    max_visible: u32,
}

impl GraphAnimator {
    pub fn new(max_visible: u32) -> Self {
        Self {
            nodes: Vec::with_capacity(u32_to_usize_saturating(max_visible)),
            max_visible,
        }
    }

    pub fn start_node_animation(
        &mut self,
        node_key: u16,
        mode: GlowMode,
    ) -> Result<(), MotionError> {
        if self.nodes.len() >= u32_to_usize_saturating(self.max_visible) {
            return Err(MotionError::NodeBudgetExceeded {
                requested: self.nodes.len().saturating_add(1),
                limit: u32_to_usize_saturating(self.max_visible),
            });
        }
        if self.nodes.iter().any(|n| n.node_key == node_key) {
            return Err(MotionError::StateCorrupted {
                detail: format!("node {node_key} already has an active animation"),
            });
        }
        self.nodes.push(ActiveNodeGlow::new(node_key, mode));
        Ok(())
    }

    pub fn stop_node_animation(&mut self, node_key: u16) {
        self.nodes.retain(|n| n.node_key != node_key);
    }

    pub fn tick(&mut self, delta_ms: u64) {
        for node in &mut self.nodes {
            node.tick(delta_ms);
        }
    }

    pub fn visible_node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn glows(&self) -> &[ActiveNodeGlow] {
        &self.nodes
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TimelineEventDescriptor {
    pub seq: u64,
    pub event_kind: String,
    pub color: [f32; 4],
    pub step_id: Option<u16>,
}

#[derive(Debug, Clone, Default)]
pub struct TimelineScrubber {
    events: Vec<TimelineEventDescriptor>,
    cursor_seq: Option<u64>,
    cursor_index: Option<usize>,
}

impl TimelineScrubber {
    pub fn new(max_events: u32) -> Self {
        Self {
            events: Vec::with_capacity(u32_to_usize_saturating(max_events)),
            cursor_seq: None,
            cursor_index: None,
        }
    }

    pub fn load_events(&mut self, events: &[TimelineEventDescriptor]) -> Result<(), MotionError> {
        if events.len() > u32_to_usize_saturating(MAX_TIMELINE_EVENTS) {
            return Err(MotionError::TimelineBudgetExceeded {
                requested: events.len(),
                limit: u32_to_usize_saturating(MAX_TIMELINE_EVENTS),
            });
        }
        self.events.clear();
        self.events.extend_from_slice(events);
        self.events.sort_by_key(|e| e.seq);
        Ok(())
    }

    pub fn scrub_to(&mut self, seq: u64) -> Result<(), MotionError> {
        let idx = self.events.iter().position(|e| e.seq == seq);
        match idx {
            Some(i) => {
                self.cursor_seq = Some(seq);
                self.cursor_index = Some(i);
                Ok(())
            }
            None => Err(MotionError::SeqNotFound { seq }),
        }
    }

    pub fn cursor(&self) -> Option<(u64, usize)> {
        self.cursor_seq.zip(self.cursor_index)
    }

    pub fn events_in_range(&self, start_idx: usize, count: usize) -> &[TimelineEventDescriptor] {
        let end = start_idx.saturating_add(count).min(self.events.len());
        let start = start_idx.min(self.events.len());
        self.events.get(start..end).unwrap_or(&[])
    }

    pub fn total_events(&self) -> usize {
        self.events.len()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EventPulse {
    pub seq: u64,
    pub center: [f32; 2],
    pub min_radius: f32,
    pub max_radius: f32,
    pub frequency_hz: f32,
    pub color: [f32; 4],
    local_elapsed_ms: u64,
}

impl EventPulse {
    pub fn new(seq: u64, center: [f32; 2], color: [f32; 4]) -> Self {
        Self {
            seq,
            center,
            min_radius: 4.0,
            max_radius: 24.0,
            frequency_hz: 2.0,
            color,
            local_elapsed_ms: 0,
        }
    }

    pub fn tick(&mut self, delta_ms: u64) {
        self.local_elapsed_ms = self.local_elapsed_ms.saturating_add(delta_ms);
    }

    pub fn radius(&self, ui_elapsed_ms: u64) -> f32 {
        let t_secs = u64_to_f32_saturating(ui_elapsed_ms) / 1000.0;
        let sine = (2.0 * std::f32::consts::PI * self.frequency_hz * t_secs).sin();
        let normalized = (sine + 1.0) * 0.5;
        self.min_radius + (self.max_radius - self.min_radius) * normalized
    }

    pub fn alpha(&self, ui_elapsed_ms: u64) -> f32 {
        let t_secs = u64_to_f32_saturating(ui_elapsed_ms) / 1000.0;
        let period = 1.0 / self.frequency_hz;
        (1.0 - t_secs / period).clamp(0.0, 1.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FailurePathFocus {
    pub failure_node_key: u16,
    overlay_alpha: f32,
    max_alpha: f32,
    pub glow_color: [f32; 4],
    local_elapsed_ms: u64,
    phase: FailurePhase,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum FailurePhase {
    FadeIn,
    Hold,
    FadeOut,
}

impl FailurePathFocus {
    pub fn new(failure_node_key: u16, max_alpha: f32) -> Self {
        Self {
            failure_node_key,
            overlay_alpha: 0.0,
            max_alpha: max_alpha.clamp(0.0, 1.0),
            glow_color: neon::RED,
            local_elapsed_ms: 0,
            phase: FailurePhase::FadeIn,
        }
    }

    pub fn tick(&mut self, delta_ms: u64) {
        self.local_elapsed_ms = self.local_elapsed_ms.saturating_add(delta_ms);
        match self.phase {
            FailurePhase::FadeIn => {
                let progress =
                    (u64_to_f32_saturating(self.local_elapsed_ms) / 300.0).clamp(0.0, 1.0);
                self.overlay_alpha = progress * self.max_alpha;
                if self.local_elapsed_ms >= 300 {
                    self.phase = FailurePhase::Hold;
                    self.local_elapsed_ms = 0;
                }
            }
            FailurePhase::Hold => {
                self.overlay_alpha = self.max_alpha;
                if self.local_elapsed_ms >= 800 {
                    self.phase = FailurePhase::FadeOut;
                    self.local_elapsed_ms = 0;
                }
            }
            FailurePhase::FadeOut => {
                let progress =
                    (u64_to_f32_saturating(self.local_elapsed_ms) / 600.0).clamp(0.0, 1.0);
                self.overlay_alpha = (1.0 - progress) * self.max_alpha;
            }
        }
    }

    pub fn overlay_alpha(&self) -> f32 {
        self.overlay_alpha
    }

    pub fn is_active(&self) -> bool {
        self.overlay_alpha > 0.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TaintOverlay {
    pub taint_color: [f32; 4],
    pub shimmer_frequency_hz: f32,
    pub shimmer_amplitude: f32,
    shimmer_phase: f32,
    is_paused: bool,
    local_elapsed_ms: u64,
}

impl TaintOverlay {
    pub fn new(taint_color: [f32; 4]) -> Self {
        Self {
            taint_color,
            shimmer_frequency_hz: 1.5,
            shimmer_amplitude: 0.3,
            shimmer_phase: 0.0,
            is_paused: false,
            local_elapsed_ms: 0,
        }
    }

    pub fn tick(&mut self, delta_ms: u64) {
        if self.is_paused {
            return;
        }
        self.local_elapsed_ms = self.local_elapsed_ms.saturating_add(delta_ms);
        let t_secs = u64_to_f32_saturating(self.local_elapsed_ms) / 1000.0;
        let raw = 2.0 * std::f32::consts::PI * self.shimmer_frequency_hz * t_secs;
        self.shimmer_phase = raw.rem_euclid(2.0 * std::f32::consts::PI);
    }

    pub fn color(&self, _ui_elapsed_ms: u64) -> [f32; 4] {
        let sine = self.shimmer_phase.sin();
        let normalized = (sine + 1.0) * 0.5;
        let alpha_mod = 0.5 + self.shimmer_amplitude * normalized;
        let [red, green, blue, _alpha] = self.taint_color;
        [red, green, blue, alpha_mod.clamp(0.0, 1.0)]
    }

    pub fn shimmer_phase(&self) -> f32 {
        self.shimmer_phase
    }

    pub fn pause(&mut self) {
        self.is_paused = true;
    }

    pub fn resume(&mut self) {
        self.is_paused = false;
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QueuePressureShimmer {
    pub pressure_ratio: f32,
    shimmer_phase: f32,
    is_paused: bool,
    local_elapsed_ms: u64,
}

impl QueuePressureShimmer {
    pub fn new() -> Self {
        Self {
            pressure_ratio: 0.0,
            shimmer_phase: 0.0,
            is_paused: false,
            local_elapsed_ms: 0,
        }
    }

    pub fn set_pressure(&mut self, ratio: f32) {
        self.pressure_ratio = ratio.clamp(0.0, 1.0);
    }

    pub fn tick(&mut self, delta_ms: u64) {
        if self.is_paused {
            return;
        }
        self.local_elapsed_ms = self.local_elapsed_ms.saturating_add(delta_ms);
        let t_secs = u64_to_f32_saturating(self.local_elapsed_ms) / 1000.0;
        let freq = 0.5 + 2.0 * self.pressure_ratio;
        self.shimmer_phase =
            (2.0 * std::f32::consts::PI * freq * t_secs) % (2.0 * std::f32::consts::PI);
    }

    pub fn shimmer_phase(&self) -> f32 {
        self.shimmer_phase
    }

    pub fn pause(&mut self) {
        self.is_paused = true;
    }

    pub fn resume(&mut self) {
        self.is_paused = false;
    }
}

#[derive(Debug, Clone, Default)]
pub struct CertificateCheckCascade {
    gates: Vec<GateStatus>,
}

impl CertificateCheckCascade {
    pub fn new(gate_count: usize) -> Self {
        Self {
            gates: vec![GateStatus::Pending; gate_count],
        }
    }

    pub fn set_gate(&mut self, ordinal: usize, status: GateStatus) -> Result<(), MotionError> {
        match self.gates.get_mut(ordinal) {
            Some(gate) => {
                *gate = status;
                Ok(())
            }
            None => Err(MotionError::StateCorrupted {
                detail: format!(
                    "gate ordinal {ordinal} out of range [0, {})",
                    self.gates.len()
                ),
            }),
        }
    }

    pub fn gate_passes(&self, ordinal: usize) -> Option<GateStatus> {
        self.gates.get(ordinal).copied()
    }

    pub fn tick(&mut self, _delta_ms: u64) {}

    pub fn current_gate(&self) -> Option<usize> {
        self.gates.iter().position(|g| *g == GateStatus::Pending)
    }

    pub fn all_passed(&self) -> bool {
        self.gates.iter().all(|g| *g == GateStatus::Passed)
    }
}

#[derive(Debug, Clone)]
pub struct AnimationState {
    is_paused: bool,
    wall_elapsed_ms: u64,
    ui_elapsed_ms: u64,
    warmed_up: bool,
}

impl Default for AnimationState {
    fn default() -> Self {
        Self {
            is_paused: true,
            wall_elapsed_ms: 0,
            ui_elapsed_ms: 0,
            warmed_up: false,
        }
    }
}

impl AnimationState {
    pub fn warm_up(&mut self) -> Result<(), MotionError> {
        if self.warmed_up {
            return Err(MotionError::StateCorrupted {
                detail: "warm_up already called".into(),
            });
        }
        self.warmed_up = true;
        Ok(())
    }

    pub fn tick(&mut self, delta_ms: u64) -> Result<(), MotionError> {
        if !self.warmed_up {
            return Err(MotionError::NotWarmedUp);
        }
        self.wall_elapsed_ms = self.wall_elapsed_ms.saturating_add(delta_ms);
        if !self.is_paused {
            self.ui_elapsed_ms = self.ui_elapsed_ms.saturating_add(delta_ms);
        }
        Ok(())
    }

    pub fn pause(&mut self) {
        self.is_paused = true;
    }

    pub fn resume(&mut self) {
        self.is_paused = false;
    }

    pub fn is_paused(&self) -> bool {
        self.is_paused
    }

    pub fn ui_elapsed_ms(&self) -> u64 {
        self.ui_elapsed_ms
    }

    pub fn is_warmed_up(&self) -> bool {
        self.warmed_up
    }
}

#[derive(Debug, Clone)]
pub struct MotionManager {
    config: MotionConfig,
    anim_state: AnimationState,
    packets: PacketSystem,
    graph: GraphAnimator,
    scrubber: TimelineScrubber,
    taint_overlay: TaintOverlay,
    queue_shimmer: QueuePressureShimmer,
    cascade: CertificateCheckCascade,
}

impl MotionManager {
    pub fn new(config: MotionConfig) -> Result<Self, MotionError> {
        config.validate()?;
        Ok(Self {
            config,
            anim_state: AnimationState::default(),
            packets: PacketSystem::new(MAX_PACKET_DOTS),
            graph: GraphAnimator::new(MAX_VISIBLE_NODES),
            scrubber: TimelineScrubber::new(MAX_TIMELINE_EVENTS),
            taint_overlay: TaintOverlay::new(TAINT_DEFAULT),
            queue_shimmer: QueuePressureShimmer::new(),
            cascade: CertificateCheckCascade::default(),
        })
    }

    pub fn warm_up(&mut self) -> Result<(), MotionError> {
        self.anim_state.warm_up()
    }

    pub fn tick(&mut self, delta_ms: u64) -> Result<(), MotionError> {
        self.anim_state.tick(delta_ms)?;
        self.packets.tick(delta_ms)?;
        self.graph.tick(delta_ms);
        self.taint_overlay.tick(delta_ms);
        self.queue_shimmer.tick(delta_ms);
        self.cascade.tick(delta_ms);
        Ok(())
    }

    pub fn pause(&mut self) {
        self.anim_state.pause();
        self.taint_overlay.pause();
        self.queue_shimmer.pause();
    }

    pub fn resume(&mut self) {
        self.anim_state.resume();
        self.taint_overlay.resume();
        self.queue_shimmer.resume();
    }

    pub fn is_paused(&self) -> bool {
        self.anim_state.is_paused()
    }

    pub fn is_warmed_up(&self) -> bool {
        self.anim_state.is_warmed_up()
    }

    pub fn packets_mut(&mut self) -> &mut PacketSystem {
        &mut self.packets
    }

    pub fn graph_mut(&mut self) -> &mut GraphAnimator {
        &mut self.graph
    }

    pub fn scrubber_mut(&mut self) -> &mut TimelineScrubber {
        &mut self.scrubber
    }

    pub fn taint_overlay_mut(&mut self) -> &mut TaintOverlay {
        &mut self.taint_overlay
    }

    pub fn queue_shimmer_mut(&mut self) -> &mut QueuePressureShimmer {
        &mut self.queue_shimmer
    }

    pub fn cascade_mut(&mut self) -> &mut CertificateCheckCascade {
        &mut self.cascade
    }

    pub fn config(&self) -> &MotionConfig {
        &self.config
    }
}

impl Default for QueuePressureShimmer {
    fn default() -> Self {
        Self::new()
    }
}
pub mod edge_path {
    #[derive(Debug, Clone, PartialEq)]
    pub struct EdgePath {
        points: Vec<[f32; 2]>,
    }

    impl EdgePath {
        pub fn new(points: Vec<[f32; 2]>) -> Self {
            Self { points }
        }

        pub fn sample_at(&self, t: f32) -> [f32; 2] {
            if self.points.is_empty() {
                return [0.0, 0.0];
            }
            if self.points.len() == 1 {
                return self.points.first().copied().unwrap_or([0.0, 0.0]);
            }
            let t_clamped = t.clamp(0.0, 1.0);
            let Some(last_index) = self.points.len().checked_sub(1) else {
                return [0.0, 0.0];
            };
            let total = super::usize_to_f32_saturating(last_index);
            let scaled = t_clamped * total;
            let mut i0 = 0usize;
            while i0 < last_index {
                let next = i0.saturating_add(1);
                if scaled <= super::usize_to_f32_saturating(next) {
                    break;
                }
                i0 = next;
            }
            let i1 = i0.saturating_add(1).min(last_index);
            let frac = (scaled - super::usize_to_f32_saturating(i0)).clamp(0.0, 1.0);
            let Some([x0, y0]) = self.points.get(i0).copied() else {
                return [0.0, 0.0];
            };
            let Some([x1, y1]) = self.points.get(i1).copied() else {
                return [x0, y0];
            };
            [x0 * (1.0 - frac) + x1 * frac, y0 * (1.0 - frac) + y1 * frac]
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn edge_path_sample_at_bounds() {
            let path = EdgePath::new(vec![[0.0, 0.0], [1.0, 1.0]]);
            assert_eq!(path.sample_at(0.0), [0.0, 0.0]);
            assert_eq!(path.sample_at(1.0), [1.0, 1.0]);
        }

        #[test]
        fn edge_path_sample_at_midpoint() {
            let path = EdgePath::new(vec![[0.0, 0.0], [2.0, 2.0]]);
            let mid = path.sample_at(0.5);
            assert!((mid[0] - 1.0).abs() < 0.001, "mid[0] = {mid:?}");
            assert!((mid[1] - 1.0).abs() < 0.001, "mid[1] = {mid:?}");
        }

        #[test]
        fn edge_path_empty_returns_zero() {
            let path = EdgePath::new(vec![]);
            assert_eq!(path.sample_at(0.5), [0.0, 0.0]);
        }

        #[test]
        fn edge_path_single_point_returns_that_point() {
            let path = EdgePath::new(vec![[3.0, 4.0]]);
            assert_eq!(path.sample_at(0.0), [3.0, 4.0]);
            assert_eq!(path.sample_at(0.5), [3.0, 4.0]);
            assert_eq!(path.sample_at(1.0), [3.0, 4.0]);
        }
    }
}

pub mod error {
    #[derive(Debug, Clone, PartialEq)]
    #[non_exhaustive]
    pub enum MotionError {
        NodeBudgetExceeded { requested: usize, limit: usize },
        PacketBudgetExceeded { requested: usize, limit: usize },
        TimelineBudgetExceeded { requested: usize, limit: usize },
        NotWarmedUp,
        SeqNotFound { seq: u64 },
        EdgeNotFound { from_step: u16, to_step: u16 },
        NodeNotFound { step_idx: u16 },
        StateCorrupted { detail: String },
        FrameBudgetUnknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOL: f32 = 0.001;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < TOL
    }

    #[test]
    fn motion_config_default_is_valid() {
        let cfg = MotionConfig::new(FrameBudget::Unlocked);
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn motion_config_rejects_node_budget_exceeded() {
        let mut cfg = MotionConfig::new(FrameBudget::Unlocked);
        cfg.max_visible_nodes = MAX_VISIBLE_NODES + 1;
        let r = cfg.validate();
        assert!(matches!(r, Err(MotionError::NodeBudgetExceeded { .. })));
    }

    #[test]
    fn animation_state_starts_paused() {
        let state = AnimationState::default();
        assert!(state.is_paused());
    }

    #[test]
    fn animation_state_warm_up_once() {
        let mut state = AnimationState::default();
        assert!(state.warm_up().is_ok());
        assert!(state.is_warmed_up());
        let r = state.warm_up();
        assert!(matches!(r, Err(MotionError::StateCorrupted { .. })));
    }

    #[test]
    fn animation_state_tick_requires_warm_up() {
        let mut state = AnimationState::default();
        let r = state.tick(16);
        assert!(matches!(r, Err(MotionError::NotWarmedUp)));
    }

    #[test]
    fn animation_state_tick_advances_elapsed() {
        let mut state = AnimationState::default();
        state.warm_up().unwrap();
        state.resume();
        state.tick(100).unwrap();
        assert_eq!(state.ui_elapsed_ms(), 100);
    }

    #[test]
    fn animation_state_pause_freezes_elapsed() {
        let mut state = AnimationState::default();
        state.warm_up().unwrap();
        state.resume();
        state.tick(100).unwrap();
        state.pause();
        state.tick(100).unwrap();
        assert_eq!(state.ui_elapsed_ms(), 100);
        assert!(state.is_paused());
    }

    #[test]
    fn animation_state_resume_continues_elapsed() {
        let mut state = AnimationState::default();
        state.warm_up().unwrap();
        state.resume();
        state.tick(100).unwrap();
        state.pause();
        state.tick(100).unwrap();
        state.resume();
        state.tick(50).unwrap();
        assert_eq!(state.ui_elapsed_ms(), 150);
        assert!(!state.is_paused());
    }

    #[test]
    fn packet_dot_is_complete() {
        let dot = PacketDot {
            edge_key: EdgeKey { from: 0, to: 1 },
            start_offset: 0.0,
            elapsed_ms: 0,
            speed: 0.001,
            color: neon::CYAN,
        };
        assert!(!dot.is_complete());
        let mut complete_dot = dot.clone();
        complete_dot.elapsed_ms = 2000;
        assert!(complete_dot.is_complete());
    }

    #[test]
    fn packet_system_enforces_max_dots() {
        let mut ps = PacketSystem::new(2);
        ps.spawn_packet_dot(EdgeKey { from: 0, to: 1 }, 0.0, 0.001, neon::CYAN)
            .unwrap();
        ps.spawn_packet_dot(EdgeKey { from: 1, to: 2 }, 0.0, 0.001, neon::CYAN)
            .unwrap();
        let r = ps.spawn_packet_dot(EdgeKey { from: 2, to: 3 }, 0.0, 0.001, neon::CYAN);
        assert!(matches!(r, Err(MotionError::PacketBudgetExceeded { .. })));
    }

    #[test]
    fn packet_system_tick_advances_elapsed() {
        let mut ps = PacketSystem::new(512);
        ps.spawn_packet_dot(EdgeKey { from: 0, to: 1 }, 0.0, 0.001, neon::CYAN)
            .unwrap();
        ps.tick(100).unwrap();
        assert_eq!(ps.dots()[0].elapsed_ms, 100);
    }

    #[test]
    fn packet_system_removes_completed() {
        let mut ps = PacketSystem::new(512);
        ps.spawn_packet_dot(EdgeKey { from: 0, to: 1 }, 0.0, 0.01, neon::CYAN)
            .unwrap();
        ps.tick(200).unwrap();
        assert_eq!(ps.active_packet_count(), 0);
    }

    #[test]
    fn graph_animator_enforces_max_nodes() {
        let mut ga = GraphAnimator::new(2);
        ga.start_node_animation(0, GlowMode::Selected).unwrap();
        ga.start_node_animation(1, GlowMode::Running).unwrap();
        let r = ga.start_node_animation(2, GlowMode::Failed);
        assert!(matches!(r, Err(MotionError::NodeBudgetExceeded { .. })));
    }

    #[test]
    fn graph_animator_stop_node() {
        let mut ga = GraphAnimator::new(256);
        ga.start_node_animation(5, GlowMode::Running).unwrap();
        assert_eq!(ga.visible_node_count(), 1);
        ga.stop_node_animation(5);
        assert_eq!(ga.visible_node_count(), 0);
    }

    #[test]
    fn glow_mode_selected_has_steady_intensity() {
        let glow = ActiveNodeGlow::new(0, GlowMode::Selected);
        assert!(approx_eq(glow.intensity(0), 0.6));
        assert!(approx_eq(glow.intensity(1000), 0.6));
    }

    #[test]
    fn glow_mode_running_oscillates() {
        let glow = ActiveNodeGlow::new(0, GlowMode::Running);
        let lo = glow.intensity(0);
        let hi = glow.intensity(125);
        assert!(lo < hi, "expected oscillation: lo={lo}, hi={hi}");
    }

    #[test]
    fn timeline_scrubber_scrub_to_finds_seq() {
        let events = vec![
            TimelineEventDescriptor {
                seq: 10,
                event_kind: "A".into(),
                color: neon::CYAN,
                step_id: None,
            },
            TimelineEventDescriptor {
                seq: 20,
                event_kind: "B".into(),
                color: neon::GREEN,
                step_id: None,
            },
        ];
        let mut scrub = TimelineScrubber::new(2000);
        scrub.load_events(&events).unwrap();
        scrub.scrub_to(20).unwrap();
        assert_eq!(scrub.cursor(), Some((20, 1)));
    }

    #[test]
    fn timeline_scrubber_scrub_to_errs_on_missing_seq() {
        let events = vec![TimelineEventDescriptor {
            seq: 10,
            event_kind: "A".into(),
            color: neon::CYAN,
            step_id: None,
        }];
        let mut scrub = TimelineScrubber::new(2000);
        scrub.load_events(&events).unwrap();
        let r = scrub.scrub_to(999);
        assert!(matches!(r, Err(MotionError::SeqNotFound { seq: 999 })));
    }

    #[test]
    fn event_pulse_radius_oscillates() {
        let mut ep = EventPulse::new(0, [0.0, 0.0], neon::CYAN);
        let r0 = ep.radius(0);
        ep.tick(125);
        let r1 = ep.radius(125);
        assert_ne!(r0, r1, "radius should oscillate");
    }

    #[test]
    fn event_pulse_alpha_fades() {
        let mut ep = EventPulse::new(0, [0.0, 0.0], neon::CYAN);
        let a0 = ep.alpha(0);
        ep.tick(500);
        let a1 = ep.alpha(500);
        assert!(a1 < a0, "alpha should fade: {a0} -> {a1}");
    }

    #[test]
    fn failure_path_focus_fades_in_hold_fade_out() {
        let mut fpf = FailurePathFocus::new(0, 0.5);
        assert!(approx_eq(fpf.overlay_alpha(), 0.0));
        fpf.tick(150);
        assert!(fpf.overlay_alpha() > 0.0);
        fpf.tick(300);
        assert!(approx_eq(fpf.overlay_alpha(), 0.5));
        fpf.tick(800);
        assert!(approx_eq(fpf.overlay_alpha(), 0.5));
        fpf.tick(600);
        assert!(fpf.overlay_alpha() < 0.5);
    }

    #[test]
    fn failure_path_focus_alpha_capped() {
        let fpf = FailurePathFocus::new(0, 0.3);
        assert!(fpf.overlay_alpha() <= 0.3);
    }

    #[test]
    fn taint_overlay_shimmer_oscillates() {
        let mut to = TaintOverlay::new(neon::PURPLE);
        let ph0 = to.shimmer_phase();
        to.tick(200);
        let ph1 = to.shimmer_phase();
        assert_ne!(ph0, ph1, "phase should advance");
        assert!(to.shimmer_phase() >= 0.0);
        assert!(to.shimmer_phase() < 2.0 * std::f32::consts::PI);
    }

    #[test]
    fn taint_overlay_pause_freezes_phase() {
        let mut to = TaintOverlay::new(neon::PURPLE);
        to.tick(100);
        let ph = to.shimmer_phase();
        to.pause();
        to.tick(1000);
        assert_eq!(to.shimmer_phase(), ph);
    }

    #[test]
    fn taint_overlay_color_in_unit_range() {
        let to = TaintOverlay::new(neon::PURPLE);
        let c = to.color(0);
        for ch in 0..4 {
            assert!(
                c[ch] >= 0.0 && c[ch] <= 1.0,
                "channel {ch} = {} out of range",
                c[ch]
            );
        }
    }

    #[test]
    fn queue_pressure_shimmer_monotonic() {
        let mut qs = QueuePressureShimmer::new();
        qs.set_pressure(0.5);
        qs.tick(100);
        let ph0 = qs.shimmer_phase();
        qs.tick(100);
        let ph1 = qs.shimmer_phase();
        assert!(ph1 > ph0, "phase should advance monotonically");
    }

    #[test]
    fn queue_pressure_shimmer_pause_freezes() {
        let mut qs = QueuePressureShimmer::new();
        qs.tick(100);
        let ph = qs.shimmer_phase();
        qs.pause();
        qs.tick(1000);
        assert_eq!(qs.shimmer_phase(), ph);
    }

    #[test]
    fn certificate_check_cascade_gate_order() {
        let mut cc = CertificateCheckCascade::new(3);
        cc.set_gate(0, GateStatus::Passed).unwrap();
        cc.set_gate(1, GateStatus::Failed).unwrap();
        cc.set_gate(2, GateStatus::Pending).unwrap();
        assert_eq!(cc.gate_passes(0), Some(GateStatus::Passed));
        assert_eq!(cc.gate_passes(1), Some(GateStatus::Failed));
        assert_eq!(cc.gate_passes(2), Some(GateStatus::Pending));
        assert_eq!(cc.current_gate(), Some(2));
        assert!(!cc.all_passed());
    }

    #[test]
    fn motion_manager_new_validates_config() {
        let cfg = MotionConfig::new(FrameBudget::Unlocked);
        let mm = MotionManager::new(cfg);
        assert!(mm.is_ok());
    }

    #[test]
    fn motion_manager_tick_pauses_freeze_others() {
        let cfg = MotionConfig::new(FrameBudget::Unlocked);
        let mut mm = MotionManager::new(cfg).unwrap();
        mm.warm_up().unwrap();
        mm.tick(100).unwrap();
        mm.pause();
        mm.tick(1000).unwrap();
        assert!(mm.is_paused());
    }
}
