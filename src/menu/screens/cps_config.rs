use super::utils::ScreenUtils;
use crate::config::{ConfigProfile, SettingsManager};
use crate::core::RacResult;

pub struct CpsConfigScreen;

impl CpsConfigScreen {
    pub fn show(
        profile: &mut ConfigProfile,
        settings_manager: &mut SettingsManager,
    ) -> RacResult<()> {
        use crate::menu::{Align, DoubleMenu};

        loop {
            ScreenUtils::clear_console();

            let server = profile.server_registry.get_active()?;

            let menu = DoubleMenu::new(50)
                .header("CONFIGURE CPS SETTINGS", Align::Center)?
                .blank()?
                .plain("Server defaults:")?
                .plain(&format!("  Left CPS:  {}", server.left_click.max_cps))?
                .plain(&format!("  Right CPS: {}", server.right_click.max_cps))?
                .blank()?
                .plain("Current overrides:")?
                .plain(&format!(
                    "  Left:  {} (0 = use server default)",
                    profile.settings.left_cps_override
                ))?
                .plain(&format!(
                    "  Right: {} (0 = use server default)",
                    profile.settings.right_cps_override
                ))?
                .blank()?
                .plain("1. Set Left Click CPS")?
                .plain("2. Set Right Click CPS")?
                .plain("3. Reset to Server Defaults")?
                .plain("4. Back to Main Menu")?
                .blank()?;

            menu.finish(&mut std::io::stdout())?;

            let input = ScreenUtils::prompt("Select option: ")?;

            match input.trim() {
                "1" => {
                    Self::configure_left_cps(profile, settings_manager)?;
                    ScreenUtils::press_enter_to_continue();
                }
                "2" => {
                    Self::configure_right_cps(profile, settings_manager)?;
                    ScreenUtils::press_enter_to_continue();
                }
                "3" => {
                    Self::reset_to_defaults(profile, settings_manager)?;
                    ScreenUtils::press_enter_to_continue();
                }
                "4" => return Ok(()),
                _ => {
                    println!("\n✗ Invalid option!");
                    ScreenUtils::press_enter_to_continue();
                }
            }
        }
    }

    fn configure_left_cps(
        profile: &mut ConfigProfile,
        settings_manager: &mut SettingsManager,
    ) -> RacResult<()> {
        let server = profile.server_registry.get_active()?;
        let server_default = server.left_click.max_cps;

        let cps_input = ScreenUtils::prompt("\nEnter Left CPS (1-30, 0 for server default): ")?;

        if let Ok(cps) = cps_input.trim().parse::<u8>() {
            if cps <= 30 {
                let final_cps = if cps == server_default { 0 } else { cps };

                profile.settings.left_cps_override = final_cps;
                settings_manager.save(&profile.settings)?;

                if final_cps == 0 {
                    println!("✓ Left CPS set to: server default ({})", server_default);
                } else {
                    println!("✓ Left CPS set to: {}", final_cps);
                }
                println!("✓ Settings saved!");
            } else {
                println!("✗ CPS must be between 0-30");
            }
        } else {
            println!("✗ Invalid number");
        }

        Ok(())
    }

    fn configure_right_cps(
        profile: &mut ConfigProfile,
        settings_manager: &mut SettingsManager,
    ) -> RacResult<()> {
        let server = profile.server_registry.get_active()?;
        let server_default = server.right_click.max_cps;

        let cps_input = ScreenUtils::prompt("\nEnter Right CPS (1-30, 0 for server default): ")?;

        if let Ok(cps) = cps_input.trim().parse::<u8>() {
            if cps <= 30 {
                let final_cps = if cps == server_default { 0 } else { cps };

                profile.settings.right_cps_override = final_cps;
                settings_manager.save(&profile.settings)?;

                if final_cps == 0 {
                    println!("✓ Right CPS set to: server default ({})", server_default);
                } else {
                    println!("✓ Right CPS set to: {}", final_cps);
                }
                println!("✓ Settings saved!");
            } else {
                println!("✗ CPS must be between 0-30");
            }
        } else {
            println!("✗ Invalid number");
        }

        Ok(())
    }

    fn reset_to_defaults(
        profile: &mut ConfigProfile,
        settings_manager: &mut SettingsManager,
    ) -> RacResult<()> {
        profile.settings.left_cps_override = 0;
        profile.settings.right_cps_override = 0;
        settings_manager.save(&profile.settings)?;
        println!("\n✓ Reset to server defaults");
        println!("✓ Settings saved!");
        Ok(())
    }
}
