use super::utils::ScreenUtils;
use crate::config::{ConfigProfile, SettingsManager};
use crate::core::{RacResult, ToggleMode};
use crate::menu::{Align, DoubleMenu};

pub struct ToggleModeConfigScreen;

impl ToggleModeConfigScreen {
    pub fn show(
        profile: &mut ConfigProfile,
        settings_manager: &mut SettingsManager,
    ) -> RacResult<()> {
        ScreenUtils::clear_console();

        let current_mode = format!("Current: {}", profile.settings.toggle_mode);

        let menu = DoubleMenu::new(54)
            .header("CONFIGURE TOGGLE MODE", Align::Center)?
            .blank()?
            .plain(&current_mode)?
            .blank()?
            .plain("1. Mouse Hold Mode")?
            .plain("   → Press hotkey once to toggle RAC on/off")?
            .plain("   → Hold mouse button to click")?
            .blank()?
            .plain("2. Hotkey Hold Mode")?
            .plain("   → Press hotkey once to toggle RAC on/off")?
            .plain("   → Clicking only while holding hotkey")?
            .blank()?
            .plain("3. Hotkey Toggle Mode")?
            .plain("   → Press hotkey once to start clicking")?
            .plain("   → Press hotkey again to stop clicking")?
            .blank()?
            .plain("4. Back to Main Menu")?
            .blank()?;

        menu.finish(&mut std::io::stdout())?;

        let input = ScreenUtils::prompt("Select mode: ")?;

        match input.trim() {
            "1" => {
                profile.settings.toggle_mode = ToggleMode::MouseHold;
                settings_manager.save(&profile.settings)?;

                println!("\n✓ Toggle mode set to: Mouse Hold");
                println!("✓ Settings saved!");
            }
            "2" => {
                profile.settings.toggle_mode = ToggleMode::HotkeyHold;
                settings_manager.save(&profile.settings)?;

                println!("\n✓ Toggle mode set to: Hotkey Hold");
                println!("✓ Settings saved!");
            }
            "3" => {
                profile.settings.toggle_mode = ToggleMode::HotkeyToggle;
                settings_manager.save(&profile.settings)?;

                println!("\n✓ Toggle mode set to: Hotkey Toggle");
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
