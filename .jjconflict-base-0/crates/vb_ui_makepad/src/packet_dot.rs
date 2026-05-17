#![forbid(unsafe_code)]

use crate::tokens::color;

const MAX_PACKET_DOTS: usize = 512;

#[derive(Debug, Clone)]
pub struct PacketDot {
    pub edge_id: String,
    pub t: f64,
    pub speed: f64,
    pub active: bool,
}

impl PacketDot {
    pub fn new(edge_id: String) -> Self {
        Self {
            edge_id,
            t: 0.0,
            speed: 0.2,
            active: true,
        }
    }

    pub fn position_along_bezier(
        t: f64,
        start: [f64; 2],
        cp1: [f64; 2],
        cp2: [f64; 2],
        end: [f64; 2],
    ) -> [f64; 2] {
        let t2 = t * t;
        let t3 = t2 * t;
        let mt = 1.0 - t;
        let mt2 = mt * mt;
        let mt3 = mt2 * mt;

        [
            mt3 * start[0] + 3.0 * mt2 * t * cp1[0] + 3.0 * mt * t2 * cp2[0] + t3 * end[0],
            mt3 * start[1] + 3.0 * mt2 * t * cp1[1] + 3.0 * mt * t2 * cp2[1] + t3 * end[1],
        ]
    }

    pub fn color(&self) -> [f32; 4] {
        color::active_cyan()
    }

    pub fn size(&self) -> f32 {
        6.0
    }

    pub fn reset(&mut self) {
        self.t = 0.0;
        self.active = true;
    }

    pub fn finish(&mut self) {
        self.t = 1.0;
        self.active = false;
    }
}

#[derive(Debug, Clone)]
pub struct AnimationTick {
    pub delta_ms: f64,
}

impl AnimationTick {
    pub fn new(delta_ms: f64) -> Self {
        Self { delta_ms }
    }

    pub fn normalized_delta(&self) -> f64 {
        self.delta_ms / 1000.0
    }
}

pub struct PacketDotManager {
    dots: Vec<PacketDot>,
    max_dots: usize,
}

impl PacketDotManager {
    pub fn new() -> Self {
        Self {
            dots: Vec::with_capacity(MAX_PACKET_DOTS),
            max_dots: MAX_PACKET_DOTS,
        }
    }

    pub fn add_dot(&mut self, edge_id: String) {
        if self.dots.len() >= self.max_dots {
            self.dots.remove(0);
        }
        self.dots.push(PacketDot::new(edge_id));
    }

    pub fn animate(&mut self, delta_ms: f64) {
        let tick = AnimationTick::new(delta_ms);
        let nd = tick.normalized_delta();

        for dot in &mut self.dots {
            if dot.active {
                dot.t += dot.speed * nd;
                if dot.t >= 1.0 {
                    dot.t = 1.0;
                    dot.active = false;
                }
            }
        }
    }

    pub fn active_count(&self) -> usize {
        self.dots.iter().filter(|d| d.active).count()
    }

    pub fn total_count(&self) -> usize {
        self.dots.len()
    }

    pub fn clear(&mut self) {
        self.dots.clear();
    }

    pub fn reset_all(&mut self) {
        for dot in &mut self.dots {
            dot.reset();
        }
    }
}

impl Default for PacketDotManager {
    fn default() -> Self {
        Self::new()
    }
}
