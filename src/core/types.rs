use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Right,
}

impl fmt::Display for MouseButton {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Left => write!(f, "Left"),
            Self::Right => write!(f, "Right"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ServerType {
    Craftrise,
    Sonoyuncu,
    Custom,
}

impl fmt::Display for ServerType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Craftrise => write!(f, "Craftrise"),
            Self::Sonoyuncu => write!(f, "Sonoyuncu"),
            Self::Custom => write!(f, "Custom"),
        }
    }
}

impl ServerType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "craftrise" => Some(Self::Craftrise),
            "sonoyuncu" => Some(Self::Sonoyuncu),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }

    pub fn all() -> Vec<Self> {
        vec![Self::Craftrise, Self::Sonoyuncu, Self::Custom]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToggleMode {
    MouseHold,
    HotkeyHold,
}

impl fmt::Display for ToggleMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MouseHold => write!(f, "Mouse Hold"),
            Self::HotkeyHold => write!(f, "Hotkey Hold"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClickMode {
    LeftOnly,
    RightOnly,
    Both,
}

impl fmt::Display for ClickMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LeftOnly => write!(f, "Left Click Only"),
            Self::RightOnly => write!(f, "Right Click Only"),
            Self::Both => write!(f, "Both (Left + Right)"),
        }
    }
}

impl ClickMode {
    pub fn is_left_active(&self) -> bool {
        matches!(self, Self::LeftOnly | Self::Both)
    }

    pub fn is_right_active(&self) -> bool {
        matches!(self, Self::RightOnly | Self::Both)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ClickPattern {
    pub max_cps: u8,
    pub min_delay_us: u64,
    pub randomize: bool,
    pub jitter_us: i64,
    pub hold_duration_us: u64,
}

impl Default for ClickPattern {
    fn default() -> Self {
        Self {
            max_cps: 15,
            min_delay_us: 200,
            randomize: false,
            jitter_us: 40,
            hold_duration_us: 35
        }
    }
}

impl ClickPattern {
    pub fn from_cps(cps: u8) -> Self {

        let min_delay = match cps {
            17.. => 420,
            15..=16 => 400,
            13..=14 => 360,
            11..=12 => 330,
            _ => 300,
        };

        Self {
            max_cps: cps,
            min_delay_us: min_delay,
            randomize: false,
            jitter_us: 40,
            hold_duration_us: 35,
        }
    }

    pub fn with_randomization(mut self, jitter: i64) -> Self {
        self.randomize = true;
        self.jitter_us = jitter;
        self
    }

    pub fn base_delay_us(&self) -> u64 {
        if self.max_cps == 0 {
            1_000_000
        } else {
            1_000_000 / self.max_cps as u64
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickerState {
    Stopped,
    Idle,
    Active
}

impl fmt::Display for ClickerState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Stopped => write!(f, "Stopped"),
            Self::Idle => write!(f, "Idle"),
            Self::Active => write!(f, "Active"),
        }
    }
}