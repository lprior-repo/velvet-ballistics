use super::engine::ReplayEngine;
use super::types::PlaybackSpeed;
use crate::ipc_bridge::{IpcBridge, IpcReply, IpcRequest};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaybackState {
    Stopped,
    Playing { speed: PlaybackSpeed },
    Paused { position: u32 },
}

pub struct ReplayController {
    engine: Option<ReplayEngine>,
    bridge: IpcBridge,
    state: PlaybackState,
    current_position: u32,
    total_events: u32,
}

impl ReplayController {
    pub fn new() -> Self {
        Self {
            engine: None,
            bridge: IpcBridge::new(),
            state: PlaybackState::Stopped,
            current_position: 0,
            total_events: 0,
        }
    }

    pub fn play(&mut self) {
        if self.engine.is_some() && self.state == PlaybackState::Stopped {
            self.state = PlaybackState::Playing {
                speed: PlaybackSpeed::Normal,
            };
        }
    }

    pub fn pause(&mut self) {
        if let PlaybackState::Playing { .. } = self.state {
            self.state = PlaybackState::Paused {
                position: self.current_position,
            };
        }
    }

    pub fn step_forward(&mut self) {
        if self.engine.is_some() && self.current_position < self.total_events.saturating_sub(1) {
            self.current_position += 1;
            self.state = PlaybackState::Paused {
                position: self.current_position,
            };
        }
    }

    pub fn step_backward(&mut self) {
        if self.engine.is_some() && self.current_position > 0 {
            self.current_position -= 1;
            self.state = PlaybackState::Paused {
                position: self.current_position,
            };
        }
    }

    pub fn jump_to_failure(&mut self) {
        if let Some(ref engine) = self.engine {
            if let Some(pos) = engine.find_failure() {
                let pos = u32::try_from(pos).unwrap_or(u32::MAX);
                self.current_position = pos;
                self.state = PlaybackState::Paused { position: pos };
            }
        }
    }

    pub fn jump_to_position(&mut self, pos: u32) {
        if self.engine.is_some() && pos < self.total_events {
            self.current_position = pos;
            self.state = PlaybackState::Paused { position: pos };
        }
    }

    pub fn set_speed(&mut self, speed: PlaybackSpeed) {
        if let PlaybackState::Playing { .. } = self.state {
            self.state = PlaybackState::Playing { speed };
        }
    }

    pub fn current_position(&self) -> u32 {
        self.current_position
    }

    pub fn total_events(&self) -> u32 {
        self.total_events
    }

    pub fn playback_state(&self) -> &PlaybackState {
        &self.state
    }

    pub fn is_loaded(&self) -> bool {
        self.engine.is_some()
    }

    pub fn poll(&mut self) -> Vec<IpcReply> {
        self.bridge.poll()
    }

    pub fn send(&mut self, request: IpcRequest) {
        let _ = self.bridge.send(request);
    }
}
