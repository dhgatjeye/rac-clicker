use super::utils::ScreenUtils;
use crate::config::ConfigProfile;
use crate::core::RacResult;
use crate::input::HotkeyManager;

pub struct SettingsDisplayScreen;

impl SettingsDisplayScreen {
    pub fn show(profile: &ConfigProfile) -> RacResult<()> {
        ScreenUtils::clear_console();
        println!("╔════════════════════════════════════════════╗");
        println!("║         CURRENT SETTINGS                   ║");
        println!("╚════════════════════════════════════════════╝");
        println!();

        let server = profile.server_registry.get_active()?;
        println!("═══ Server Configuration ═══");
        println!("  Server:        {}", server.server_type);
        println!("  Process:       {}", server.process_name);
        println!("  Left CPS:      {}", server.left_click.max_cps);
        println!("  Right CPS:     {}", server.right_click.max_cps);
        println!();

        println!("═══ Hotkeys ═══");
        println!(
            "  Toggle:        {}",
            HotkeyManager::key_name(profile.settings.toggle_hotkey)
        );
        println!(
            "  Left Click:    {}",
            HotkeyManager::key_name(profile.settings.left_hotkey)
        );
        println!(
            "  Right Click:   {}",
            HotkeyManager::key_name(profile.settings.right_hotkey)
        );
        println!();

        println!("═══ Modes ═══");
        println!("  Toggle Mode:   {}", profile.settings.toggle_mode);
        println!("  Click Mode:    {}", profile.settings.click_mode);
        println!();

        println!("═══ CPS Overrides ═══");
        let left_cps = profile.get_left_cps()?;
        let right_cps = profile.get_right_cps()?;
        println!(
            "  Left:          {} {}",
            left_cps,
            if profile.settings.left_cps_override == 0 {
                "(server default)"
            } else {
                "(override)"
            }
        );
        println!(
            "  Right:         {} {}",
            right_cps,
            if profile.settings.right_cps_override == 0 {
                "(server default)"
            } else {
                "(override)"
            }
        );
        println!();

        ScreenUtils::press_enter_to_continue();
        Ok(())
    }
}
