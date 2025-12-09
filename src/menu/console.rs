use crate::config::{ConfigProfile, SettingsManager};
use crate::core::{RacError, RacResult};
use crate::menu::MenuCommand;
use crate::menu::screens::*;
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
        println!("╔════════════════════════════════════════════╗");
        println!("║              RAC v2                        ║");
        println!("╚════════════════════════════════════════════╝");
        println!();

        let active_server = self.profile.server_registry.active_server_type();
        println!("  Active Server: {}", active_server);
        println!();
        println!("╔════════════════════════════════════════════╗");
        println!("║              MAIN MENU                     ║");
        println!("╠════════════════════════════════════════════╣");
        println!("║  1. Select Server                          ║");
        println!("║  2. Configure Hotkeys                      ║");
        println!("║  3. Configure Toggle Mode                  ║");
        println!("║  4. Configure Click Mode                   ║");
        println!("║  5. Configure CPS Settings                 ║");
        println!("║  6. Show Current Settings                  ║");
        println!("║  7. Start RAC                              ║");
        println!("║  0. Exit                                   ║");
        println!("╚════════════════════════════════════════════╝");
        println!();
        print!("Select option: ");
        io::stdout().flush().ok();
    }

    fn set_console_title(&self, title: &str) -> RacResult<()> {
        use std::ffi::CString;

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
