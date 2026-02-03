#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuCommand {
    SelectServer,
    ConfigureHotkeys,
    ConfigureToggleMode,
    ConfigureClickMode,
    ConfigureCPS,
    ShowSettings,
    StartRAC,
    ConfigureAutoUpdate,
    Exit,
}

impl MenuCommand {
    pub fn from_input(input: &str) -> Option<Self> {
        match input.trim() {
            "1" => Some(Self::SelectServer),
            "2" => Some(Self::ConfigureHotkeys),
            "3" => Some(Self::ConfigureToggleMode),
            "4" => Some(Self::ConfigureClickMode),
            "5" => Some(Self::ConfigureCPS),
            "6" => Some(Self::ShowSettings),
            "7" => Some(Self::StartRAC),
            "8" => Some(Self::ConfigureAutoUpdate),
            "0" => Some(Self::Exit),
            _ => None,
        }
    }

    pub fn description(&self) -> &str {
        match self {
            Self::SelectServer => "Select Server (Craftrise/Sonoyuncu/Custom)",
            Self::ConfigureHotkeys => "Configure Hotkeys",
            Self::ConfigureToggleMode => "Configure Toggle Mode",
            Self::ConfigureClickMode => "Configure Click Mode",
            Self::ConfigureCPS => "Configure CPS Settings",
            Self::ShowSettings => "Show Current Settings",
            Self::StartRAC => "Start RAC",
            Self::ConfigureAutoUpdate => "Configure Auto-Update",
            Self::Exit => "Exit",
        }
    }
}
