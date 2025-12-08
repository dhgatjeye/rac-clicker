use super::utils::ScreenUtils;
use crate::config::{ConfigProfile, SettingsManager};
use crate::core::{ClickMode, RacResult};

pub struct ClickModeConfigScreen;

impl ClickModeConfigScreen {
    pub fn show(
        profile: &mut ConfigProfile,
        settings_manager: &mut SettingsManager,
    ) -> RacResult<()> {
        ScreenUtils::clear_console();
        println!("╔════════════════════════════════════════════╗");
        println!("║         CONFIGURE CLICK MODE               ║");
        println!("╚════════════════════════════════════════════╝");
        println!();
        println!("Current: {}", profile.settings.click_mode);
        println!();
        println!("1. Left Click Only");
        println!("2. Right Click Only");
        println!("3. Both (Left + Right simultaneously)");
        println!("4. Back to Main Menu");
        println!();

        let input = ScreenUtils::prompt("Select mode: ")?;

        match input.trim() {
            "1" => {
                profile.settings.click_mode = ClickMode::LeftOnly;
                settings_manager.save(&profile.settings)?;
                println!("\n✓ Click mode set to: Left Click Only");
                println!("✓ Settings saved!");
            }
            "2" => {
                profile.settings.click_mode = ClickMode::RightOnly;
                settings_manager.save(&profile.settings)?;
                println!("\n✓ Click mode set to: Right Click Only");
                println!("✓ Settings saved!");
            }
            "3" => {
                profile.settings.click_mode = ClickMode::Both;
                settings_manager.save(&profile.settings)?;
                println!("\n✓ Click mode set to: Both (Left + Right)");
                println!("✓ Settings saved!");
            }
            "4" => return Ok(()),
            _ => {
                println!("\n✗ Invalid option!");
            }
        }

        ScreenUtils::press_enter_to_continue();
        Ok(())
    }
}
