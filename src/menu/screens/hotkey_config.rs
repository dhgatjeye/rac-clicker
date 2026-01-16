use super::utils::ScreenUtils;
use crate::config::{ConfigProfile, SettingsManager};
use crate::core::RacResult;
use crate::input::HotkeyManager;

pub struct HotkeyConfigScreen;

impl HotkeyConfigScreen {
    pub fn show(
        profile: &mut ConfigProfile,
        settings_manager: &mut SettingsManager,
    ) -> RacResult<()> {
        loop {
            Self::display_menu(profile);

            let input = ScreenUtils::prompt("Select option: ")?;

            if Self::handle_menu_selection(input.trim(), profile, settings_manager)? {
                return Ok(());
            }
        }
    }

    fn display_menu(profile: &ConfigProfile) {
        ScreenUtils::clear_console();
        println!("╔════════════════════════════════════════════╗");
        println!("║         CONFIGURE HOTKEYS                  ║");
        println!("╚════════════════════════════════════════════╝");
        println!();
        println!("Current Hotkeys:");
        println!(
            "  Toggle: {} (optional)",
            HotkeyManager::key_name(profile.settings.toggle_hotkey)
        );
        println!(
            "  Left:   {}",
            HotkeyManager::key_name(profile.settings.left_hotkey)
        );
        println!(
            "  Right:  {}",
            HotkeyManager::key_name(profile.settings.right_hotkey)
        );
        println!();
        println!("NOTE: Toggle hotkey is optional if Left/Right hotkeys are set.");
        println!();
        println!("1. Configure Toggle Hotkey (Optional)");
        println!("2. Configure Left Click Hotkey");
        println!("3. Configure Right Click Hotkey");
        println!("4. Reset All Hotkeys");
        println!("5. Back to Main Menu");
        println!();
    }

    fn handle_menu_selection(
        input: &str,
        profile: &mut ConfigProfile,
        settings_manager: &mut SettingsManager,
    ) -> RacResult<bool> {
        match input {
            "1" => {
                Self::configure_toggle_hotkey(profile, settings_manager)?;
                ScreenUtils::press_enter_to_continue();
                Ok(false)
            }
            "2" => {
                Self::configure_left_hotkey(profile, settings_manager)?;
                ScreenUtils::press_enter_to_continue();
                Ok(false)
            }
            "3" => {
                Self::configure_right_hotkey(profile, settings_manager)?;
                ScreenUtils::press_enter_to_continue();
                Ok(false)
            }
            "4" => {
                Self::reset_all_hotkeys(profile, settings_manager)?;
                ScreenUtils::press_enter_to_continue();
                Ok(false)
            }
            "5" => Ok(true),
            _ => {
                println!("\n✗ Invalid option!");
                ScreenUtils::press_enter_to_continue();
                Ok(false)
            }
        }
    }

    fn configure_toggle_hotkey(
        profile: &mut ConfigProfile,
        settings_manager: &mut SettingsManager,
    ) -> RacResult<()> {
        println!("\nPress the key you want to use as toggle hotkey...");
        println!("(Press ESC to cancel, DELETE to disable)");
        println!("NOTE: Toggle is optional if Left/Right hotkeys are set.");

        if let Some(vk_code) = Self::wait_for_key_press()? {
            Self::apply_toggle_hotkey(vk_code, profile, settings_manager)?;
        }
        Ok(())
    }

    fn apply_toggle_hotkey(
        vk_code: i32,
        profile: &mut ConfigProfile,
        settings_manager: &mut SettingsManager,
    ) -> RacResult<()> {
        if vk_code == 0x1B {
            return Ok(());
        }

        if vk_code == 0x2E {
            profile.settings.toggle_hotkey = 0;
            settings_manager.save(&profile.settings)?;
            println!("\n✓ Toggle hotkey disabled (using Left/Right hotkeys only)");
            println!("✓ Settings saved!");
        } else {
            profile.settings.toggle_hotkey = vk_code;
            settings_manager.save(&profile.settings)?;
            println!(
                "\n✓ Toggle hotkey set to: {}",
                HotkeyManager::key_name(vk_code)
            );
            println!("✓ Settings saved!");
        }
        Ok(())
    }

    fn configure_left_hotkey(
        profile: &mut ConfigProfile,
        settings_manager: &mut SettingsManager,
    ) -> RacResult<()> {
        println!("\nPress the key you want to use for left click...");
        println!("(Press ESC to cancel, or 0 to disable)");

        if let Some(vk_code) = Self::wait_for_key_press()?
            && vk_code != 0x1B
        {
            profile.settings.left_hotkey = vk_code;
            settings_manager.save(&profile.settings)?;
            println!(
                "\n✓ Left click hotkey set to: {}",
                HotkeyManager::key_name(vk_code)
            );
            println!("✓ Settings saved!");
        }
        Ok(())
    }

    fn configure_right_hotkey(
        profile: &mut ConfigProfile,
        settings_manager: &mut SettingsManager,
    ) -> RacResult<()> {
        println!("\nPress the key you want to use for right click...");
        println!("(Press ESC to cancel, or 0 to disable)");

        if let Some(vk_code) = Self::wait_for_key_press()?
            && vk_code != 0x1B
        {
            profile.settings.right_hotkey = vk_code;
            settings_manager.save(&profile.settings)?;
            println!(
                "\n✓ Right click hotkey set to: {}",
                HotkeyManager::key_name(vk_code)
            );
            println!("✓ Settings saved!");
        }
        Ok(())
    }

    fn reset_all_hotkeys(
        profile: &mut ConfigProfile,
        settings_manager: &mut SettingsManager,
    ) -> RacResult<()> {
        profile.settings.toggle_hotkey = 0;
        profile.settings.left_hotkey = 0;
        profile.settings.right_hotkey = 0;
        settings_manager.save(&profile.settings)?;

        println!("\n✓ All hotkeys reset to None");
        println!("✓ Settings saved!");
        Ok(())
    }

    fn wait_for_key_press() -> RacResult<Option<i32>> {
        std::thread::sleep(std::time::Duration::from_millis(200));

        for _ in 0..200 {
            std::thread::sleep(std::time::Duration::from_millis(50));

            if let Some(vk_code) = Self::check_mouse_buttons() {
                return Ok(Some(vk_code));
            }

            if let Some(vk_code) = Self::check_keyboard_keys() {
                return Ok(Some(vk_code));
            }
        }

        Ok(None)
    }

    fn check_mouse_buttons() -> Option<i32> {
        use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;

        for vk_code in [0x04, 0x05, 0x06] {
            // MBUTTON, XBUTTON1, XBUTTON2
            unsafe {
                if GetAsyncKeyState(vk_code) < 0 {
                    Self::wait_for_key_release(vk_code);
                    return Some(vk_code);
                }
            }
        }
        None
    }

    fn check_keyboard_keys() -> Option<i32> {
        use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;

        for vk_code in 0x08..=0xFE {
            unsafe {
                if GetAsyncKeyState(vk_code) < 0 {
                    Self::wait_for_key_release(vk_code);
                    return Some(vk_code);
                }
            }
        }
        None
    }

    fn wait_for_key_release(vk_code: i32) {
        use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;

        unsafe {
            while GetAsyncKeyState(vk_code) < 0 {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
    }
}
