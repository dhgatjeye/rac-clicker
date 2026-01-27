use super::utils::ScreenUtils;
use crate::config::{ConfigProfile, SettingsManager};
use crate::core::{RacResult, ServerType};
use crate::menu::{Align, DoubleMenu};

pub struct ServerSelectionScreen;

impl ServerSelectionScreen {
    pub fn show(
        profile: &mut ConfigProfile,
        settings_manager: &mut SettingsManager,
    ) -> RacResult<()> {
        ScreenUtils::clear_console();

        let menu = DoubleMenu::new(46)
            .header("SELECT SERVER", Align::Center)?
            .blank()?
            .plain("1. Craftrise")?
            .plain("2. Sonoyuncu")?
            .plain("3. Custom")?
            .plain("4. Back to Main Menu")?
            .blank()?;

        menu.finish(&mut std::io::stdout())?;

        let input = ScreenUtils::prompt("Select server: ")?;

        let server_type = match input.trim() {
            "1" => Some(ServerType::Craftrise),
            "2" => Some(ServerType::Sonoyuncu),
            "3" => Some(ServerType::Custom),
            "4" => return Ok(()),
            _ => None,
        };

        if let Some(server) = server_type {
            profile.switch_server(server)?;
            settings_manager.save(&profile.settings)?;

            println!("\n✓ Switched to: {}", server);
            println!("✓ Settings saved!");

            ScreenUtils::press_enter_to_continue();
        } else {
            println!("\n✗ Invalid selection!");
            ScreenUtils::press_enter_to_continue();
        }

        Ok(())
    }
}
