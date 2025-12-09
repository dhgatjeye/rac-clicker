pub mod app;
pub mod clicker;
pub mod config;
pub mod core;
pub mod input;
pub mod menu;
pub mod servers;
pub mod thread;
pub mod update;
pub mod window;

pub use core::{
    ClickMode, ClickPattern, ClickerState, MouseButton, RacError, RacResult, ServerType, ToggleMode,
};

pub use config::{ConfigProfile, ServerConfig, ServerRegistry, Settings, SettingsManager};

pub use thread::{ClickWorker, SyncSignal, ThreadManager, WorkerConfig, WorkerState};

pub use clicker::{ClickController, ClickExecutor, DelayCalculator};

pub use window::{WindowFinder, WindowHandle, WindowWatcher};

pub use input::{HotkeyEvent, HotkeyManager, InputMonitor};

pub use menu::{ConsoleMenu, MenuCommand};

pub use update::{ReleaseInfo, UpdateManager, Version};

pub use app::{
    RacApp, check_and_update, flush_console_input, has_configured_hotkeys, is_first_instance,
};
