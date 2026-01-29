use rac_clicker::app::{flush_console_input, ui};
use rac_clicker::{
    ConsoleMenu, RacApp, RacError, RacResult, check_and_update, has_configured_hotkeys,
    is_first_instance,
};

fn main() -> RacResult<()> {
    if !is_first_instance() {
        flush_console_input();
        ui::show_already_running_error();
        std::process::exit(1);
    }

    if let Err(RacError::UpdateRestart) = check_and_update() {
        return Ok(());
    }

    run_main_loop()
}

fn run_main_loop() -> RacResult<()> {
    loop {
        flush_console_input();

        let mut menu = ConsoleMenu::new()?;

        match menu.show_main_menu() {
            Ok(()) => {}
            Err(RacError::UserExit) => return Ok(()),
            Err(e) => return Err(e),
        }

        let profile = menu.profile().clone();

        if !has_configured_hotkeys(&profile) {
            flush_console_input();
            ui::show_no_hotkeys_error();
            continue;
        }

        let mut app = RacApp::new(profile)?;
        app.run()?;
    }
}
