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

impl std::str::FromStr for ServerType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "craftrise" => Ok(Self::Craftrise),
            "sonoyuncu" => Ok(Self::Sonoyuncu),
            "custom" => Ok(Self::Custom),
            _ => Err(()),
        }
    }
}

impl ServerType {
    pub const fn all() -> &'static [Self] {
        &[Self::Craftrise, Self::Sonoyuncu, Self::Custom]
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
}

impl Default for ClickPattern {
    fn default() -> Self {
        Self { max_cps: 15 }
    }
}

impl ClickPattern {
    pub fn from_cps(cps: u8) -> Self {
        Self { max_cps: cps }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickerState {
    Stopped,
    Idle,
    Active,
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
