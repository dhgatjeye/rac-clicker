pub mod defaults {
    pub const MIN_MEMORY_MB: u64 = 4096;
    pub const MIN_CPU_CORES: usize = 2;
    pub const MIN_CPU_SPEED_GHZ: f64 = 1.0;
    pub const MAX_WINDOW_FIND_FAILURES: usize = 5;

    pub const RAC_DIR: &str = "RAC";
    pub const RAC_LOG_PATH: &str = "logs.txt";

    pub const TARGET_PROCESS: &str = "craftrise-x64.exe";

    pub const TOGGLE_KEY: i32 = 0;
    pub const LEFT_TOGGLE_KEY: i32 = 0;
    pub const RIGHT_TOGGLE_KEY: i32 = 0;
    pub const HOTKEY_HOLD_MODE: bool = false;

    pub const LEFT_MAX_CPS: u8 = 15;
    pub const RIGHT_MAX_CPS: u8 = 19;
    pub const CLICK_MODE: &str = "LeftClick";

    pub const LEFT_GAME_MODE: &str = "Combo";
    pub const RIGHT_GAME_MODE: &str = "Combo";
    pub const POST_MODE: &str = "Default";

    pub const ADAPTIVE_CPU_MODE: bool = false;
    pub const BURST_MODE: bool = true;
}