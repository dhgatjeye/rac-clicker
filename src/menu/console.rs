use crate::config::{ConfigProfile, SettingsManager};
use crate::core::{RacError, RacResult};
use crate::menu::MenuCommand;
use crate::menu::screens::{
    AutoUpdateConfigScreen, ClickModeConfigScreen, CpsConfigScreen, HotkeyConfigScreen,
    ScreenUtils, ServerSelectionScreen, SettingsDisplayScreen, ToggleModeConfigScreen,
};
use std::io::{self, Write};
use windows::Win32::System::Console::SetConsoleTitleA;
use windows::core::PCSTR;

pub struct ConsoleMenu {
    profile: ConfigProfile,
    settings_manager: SettingsManager,
}

impl ConsoleMenu {
    pub fn new() -> RacResult<Self> {
        let settings_manager = SettingsManager::new()?;
        let settings = settings_manager.load()?;

        let profile = ConfigProfile {
            settings,
            ..Default::default()
        };

        Ok(Self {
            profile,
            settings_manager,
        })
    }

    pub fn show_main_menu(&mut self) -> RacResult<()> {
        loop {
            self.set_console_title("RAC v2 Main Menu")?;
            ScreenUtils::clear_console();

            self.display_main_menu();

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            match MenuCommand::from_input(&input) {
                Some(MenuCommand::SelectServer) => {
                    ServerSelectionScreen::show(&mut self.profile, &mut self.settings_manager)?
                }
                Some(MenuCommand::ConfigureHotkeys) => {
                    HotkeyConfigScreen::show(&mut self.profile, &mut self.settings_manager)?
                }
                Some(MenuCommand::ConfigureToggleMode) => {
                    ToggleModeConfigScreen::show(&mut self.profile, &mut self.settings_manager)?
                }
                Some(MenuCommand::ConfigureClickMode) => {
                    ClickModeConfigScreen::show(&mut self.profile, &mut self.settings_manager)?
                }
                Some(MenuCommand::ConfigureCPS) => {
                    CpsConfigScreen::show(&mut self.profile, &mut self.settings_manager)?
                }
                Some(MenuCommand::ShowSettings) => SettingsDisplayScreen::show(&self.profile)?,
                Some(MenuCommand::StartRAC) => {
                    self.settings_manager.save(&self.profile.settings)?;
                    ScreenUtils::clear_console();
                    return Ok(());
                }
                Some(MenuCommand::ConfigureAutoUpdate) => {
                    AutoUpdateConfigScreen::show(&mut self.profile, &mut self.settings_manager)?
                }
                Some(MenuCommand::Exit) => {
                    println!("\n✓ Exiting RAC v2...");
                    println!("✓ Cleaning up resources...");

                    let _ = self.settings_manager.save(&self.profile.settings);

                    return Err(RacError::UserExit);
                }
                None => {
                    println!("\n✗ Invalid option!");
                    ScreenUtils::press_enter_to_continue();
                }
            }
        }
    }

    fn display_main_menu(&self) {
        use crate::menu::{Align, DoubleMenu};

        let active_server = self.profile.server_registry.active_server_type();
        let active_info = format!("  Active Server: {}", active_server);

        let menu = DoubleMenu::new(46)
            .header("RAC v2", Align::Center)
            .and_then(|m| m.blank())
            .and_then(|m| m.plain(&active_info))
            .and_then(|m| m.blank())
            .and_then(|m| m.box_start())
            .and_then(|m| m.line("MAIN MENU", Align::Center))
            .and_then(|m| m.divider())
            .and_then(|m| m.line("  1. Select Server", Align::Left))
            .and_then(|m| m.line("  2. Configure Hotkeys", Align::Left))
            .and_then(|m| m.line("  3. Configure Toggle Mode", Align::Left))
            .and_then(|m| m.line("  4. Configure Click Mode", Align::Left))
            .and_then(|m| m.line("  5. Configure CPS Settings", Align::Left))
            .and_then(|m| m.line("  6. Show Current Settings", Align::Left))
            .and_then(|m| m.line("  7. Start RAC", Align::Left))
            .and_then(|m| m.line("  8. Configure Auto-Update", Align::Left))
            .and_then(|m| m.line("  0. Exit", Align::Left))
            .and_then(|m| m.box_end())
            .and_then(|m| m.blank());

        if let Ok(m) = menu {
            let _ = m.finish(&mut io::stdout());
        }

        print!("Select option: ");
        io::stdout().flush().ok();
    }

    fn set_console_title(&self, title: &str) -> RacResult<()> {
        use std::ffi::CString;

        for (idx, c) in title.chars().enumerate() {
            if !c.is_ascii() {
                return Err(RacError::InvalidInput(format!(
                    "Title contains non-ASCII character at position {}",
                    idx
                )));
            }

            if c.is_ascii_control() {
                return Err(RacError::InvalidInput(format!(
                    "Title contains control character at position {}",
                    idx
                )));
            }
        }

        if title.len() > 256 {
            return Err(RacError::InvalidInput(
                "Title exceeds maximum length of 256 characters".into(),
            ));
        }

        let title_cstring = CString::new(title)
            .map_err(|_| RacError::InvalidInput("Title contains null byte".into()))?;

        unsafe {
            SetConsoleTitleA(PCSTR::from_raw(title_cstring.as_ptr() as *const u8)).map_err(
                |e| RacError::WindowError(format!("Failed to set console title: {}", e)),
            )?;
        }
        Ok(())
    }

    pub fn profile(&self) -> &ConfigProfile {
        &self.profile
    }
}
