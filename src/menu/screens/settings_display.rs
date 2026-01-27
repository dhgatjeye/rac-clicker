use super::utils::ScreenUtils;
use crate::config::ConfigProfile;
use crate::core::RacResult;
use crate::input::HotkeyManager;
use crate::menu::{Align, DoubleBoxLayout};
use std::io::{self, Write};

pub struct SettingsDisplayScreen;

impl SettingsDisplayScreen {
    fn print_section(title: &str) {
        const SEPARATOR_CHAR: &str = "═";
        const MIN_PADDING: usize = 3;

        let title_with_spaces = format!(" {} ", title);

        println!(
            "{}{}{}",
            SEPARATOR_CHAR.repeat(MIN_PADDING),
            title_with_spaces,
            SEPARATOR_CHAR.repeat(MIN_PADDING)
        );
    }

    pub fn show(profile: &ConfigProfile) -> RacResult<()> {
        ScreenUtils::clear_console();

        let layout = DoubleBoxLayout::new(46);
        let mut stdout = io::stdout();

        layout.render_header(&mut stdout, "CURRENT SETTINGS", Align::Center)?;
        layout.render_blank(&mut stdout)?;

        let server = profile.server_registry.get_active()?;

        Self::print_section("Server Configuration");
        println!("  Server:        {}", server.server_type);
        println!("  Process:       {}", server.process_name);
        println!("  Left CPS:      {}", server.left_click.max_cps);
        println!("  Right CPS:     {}", server.right_click.max_cps);
        println!();

        Self::print_section("Hotkeys");
        let toggle_key = HotkeyManager::key_name(profile.settings.toggle_hotkey);
        let left_key = HotkeyManager::key_name(profile.settings.left_hotkey);
        let right_key = HotkeyManager::key_name(profile.settings.right_hotkey);

        println!("  Toggle:        {}", toggle_key);
        println!("  Left Click:    {}", left_key);
        println!("  Right Click:   {}", right_key);
        println!();

        Self::print_section("Modes");
        println!("  Toggle Mode:   {}", profile.settings.toggle_mode);
        println!("  Click Mode:    {}", profile.settings.click_mode);
        println!();

        Self::print_section("CPS Overrides");
        let left_cps = profile.get_left_cps()?;
        let right_cps = profile.get_right_cps()?;

        let left_status = if profile.settings.left_cps_override == 0 {
            "(server default)"
        } else {
            "(override)"
        };
        let right_status = if profile.settings.right_cps_override == 0 {
            "(server default)"
        } else {
            "(override)"
        };

        println!("  Left:          {} {}", left_cps, left_status);
        println!("  Right:         {} {}", right_cps, right_status);
        println!();

        stdout.flush()?;
        ScreenUtils::press_enter_to_continue();
        Ok(())
    }
}
