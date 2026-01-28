use super::utils::ScreenUtils;
use crate::config::{ConfigProfile, SettingsManager};
use crate::core::{ClickMode, RacResult};
use crate::menu::{Align, DoubleMenu};

pub struct ClickModeConfigScreen;

impl ClickModeConfigScreen {
    pub fn show(
        profile: &mut ConfigProfile,
        settings_manager: &mut SettingsManager,
    ) -> RacResult<()> {
        ScreenUtils::clear_console();

        let current_mode = format!("Current: {}", profile.settings.click_mode);

        let menu = DoubleMenu::new(50)
            .header("CONFIGURE CLICK MODE", Align::Center)?
            .blank()?
            .plain(&current_mode)?
            .blank()?
            .plain("1. Left Click Only")?
            .plain("2. Right Click Only")?
            .plain("3. Both (Left + Right simultaneously)")?
            .blank()?;

        menu.finish(&mut std::io::stdout())?;

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
            _ => {
                println!("\n✗ Invalid option!");
            }
        }

        ScreenUtils::press_enter_to_continue();
        Ok(())
    }
}
