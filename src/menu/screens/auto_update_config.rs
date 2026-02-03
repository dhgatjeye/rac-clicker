use super::utils::ScreenUtils;
use crate::config::{ConfigProfile, SettingsManager};
use crate::core::RacResult;
use crate::menu::{Align, DoubleMenu};

pub struct AutoUpdateConfigScreen;

impl AutoUpdateConfigScreen {
    pub fn show(
        profile: &mut ConfigProfile,
        settings_manager: &mut SettingsManager,
    ) -> RacResult<()> {
        ScreenUtils::clear_console();

        let current_status = if profile.settings.auto_update_check {
            "Enabled"
        } else {
            "Disabled"
        };

        let current_info = format!("Current: {}", current_status);

        let menu = DoubleMenu::new(50)
            .header("AUTO-UPDATE SETTINGS", Align::Center)?
            .blank()?
            .plain(&current_info)?
            .blank()?
            .plain("1. Enable Auto-Update Check")?
            .plain("   → Check for updates on startup")?
            .blank()?
            .plain("2. Disable Auto-Update Check")?
            .plain("   → Skip update check on startup")?
            .blank()?
            .plain("3. Back to Main Menu")?
            .blank()?;

        menu.finish(&mut std::io::stdout())?;

        let input = ScreenUtils::prompt("Select option: ")?;

        match input.trim() {
            "1" => {
                profile.settings.auto_update_check = true;
                settings_manager.save(&profile.settings)?;

                println!("\n✓ Auto-update check: Enabled");
                println!("✓ Settings saved!");
            }
            "2" => {
                profile.settings.auto_update_check = false;
                settings_manager.save(&profile.settings)?;

                println!("\n✓ Auto-update check: Disabled");
                println!("✓ Settings saved!");
            }
            "3" => return Ok(()),
            _ => {
                println!("\n✗ Invalid option!");
            }
        }

        ScreenUtils::press_enter_to_continue();
        Ok(())
    }
}
