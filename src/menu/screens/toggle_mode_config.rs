use super::utils::ScreenUtils;
use crate::config::{ConfigProfile, SettingsManager};
use crate::core::{RacResult, ToggleMode};

pub struct ToggleModeConfigScreen;

impl ToggleModeConfigScreen {
    pub fn show(
        profile: &mut ConfigProfile,
        settings_manager: &mut SettingsManager,
    ) -> RacResult<()> {
        ScreenUtils::clear_console();
        println!("╔════════════════════════════════════════════╗");
        println!("║        CONFIGURE TOGGLE MODE               ║");
        println!("╚════════════════════════════════════════════╝");
        println!();
        println!("Current: {}", profile.settings.toggle_mode);
        println!();
        println!("1. Mouse Hold Mode");
        println!("   → Press hotkey once to toggle RAC on/off");
        println!("   → Hold mouse button to click");
        println!();
        println!("2. Hotkey Hold Mode");
        println!("   → Press hotkey once to toggle RAC on/off");
        println!("   → Clicking only while holding hotkey");
        println!();
        println!("3. Back to Main Menu");
        println!();

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
            "3" => return Ok(()),
            _ => {
                println!("\n✗ Invalid option!");
            }
        }

        ScreenUtils::press_enter_to_continue();
        Ok(())
    }
}
