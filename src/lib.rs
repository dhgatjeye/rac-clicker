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
    ClickMode, ClickPattern, ClickerState, MouseButton, RacError, RacResult, ServerType,
    ToggleMode,
};

pub use config::{ConfigProfile, ServerConfig, ServerRegistry, Settings, SettingsManager};

pub use thread::{ClickWorker, SyncSignal, ThreadManager, WorkerConfig, WorkerState};

pub use clicker::{ClickController, ClickExecutor, DelayCalculator};

pub use window::{WatcherConfig, WindowFinder, WindowHandle, WindowWatcher};

pub use input::{HotkeyEvent, HotkeyManager, InputMonitor, MonitorConfig};

pub use menu::{ConsoleMenu, MenuCommand};

pub use update::{ReleaseInfo, UpdateManager, Version};

pub use app::{check_and_update, check_single_instance, RacApp};