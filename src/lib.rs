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
    RacError, RacResult,
    MouseButton, ServerType, ToggleMode, ClickMode,
    ClickPattern, ClickerState,
};

pub use config::{
    ServerConfig, ServerRegistry, Settings, SettingsManager, ConfigProfile,
};

pub use thread::{
    ThreadManager, ClickWorker, WorkerConfig, SyncSignal, WorkerState,
};

pub use clicker::{
    ClickExecutor, ClickController, DelayCalculator,
};

pub use window::{
    WindowFinder, WindowHandle,
    WindowWatcher, WatcherConfig, WatcherStats,
};

pub use input::{
    HotkeyManager, HotkeyEvent, InputMonitor,
};

pub use menu::{
    ConsoleMenu, MenuCommand,
};

pub use update::{
    UpdateManager, ReleaseInfo, Version,
};

pub use app::{
    RacApp, check_and_update, is_first_instance, 
    flush_console_input, has_configured_hotkeys,
};
