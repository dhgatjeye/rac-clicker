use rac_clicker::{
    check_and_update, check_single_instance, ConsoleMenu, RacApp, RacError, RacResult,
};

fn main() -> RacResult<()> {
    if !check_single_instance() {
        show_already_running_error();
        std::process::exit(1);
    }

    if let Err(RacError::UpdateRestart) = check_and_update() {
        return Ok(());
    }

    run_main_loop()
}

fn show_already_running_error() {
    flush_console_input();
    eprintln!("✗ RAC v2 is already running!");
    eprintln!("\nPress Enter to exit...");
    wait_for_enter();
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

        if !has_any_hotkey_configured(&profile) {
            show_no_hotkey_error();
            continue;
        }

        let mut app = RacApp::new(profile)?;
        app.run()?;
    }
}

fn has_any_hotkey_configured(profile: &rac_clicker::ConfigProfile) -> bool {
    profile.settings.toggle_hotkey != 0
        || profile.settings.left_hotkey != 0
        || profile.settings.right_hotkey != 0
}

fn show_no_hotkey_error() {
    flush_console_input();

    println!("╔════════════════════════════════════════════╗");
    println!("║              ERROR                         ║");
    println!("╚════════════════════════════════════════════╝");
    println!();
    println!("✗ No hotkeys configured!");
    println!();
    println!("You must configure at least one hotkey:");
    println!("  • Toggle hotkey (global on/off), OR");
    println!("  • Left/Right click hotkeys (direct control)");
    println!();
    println!("Please configure hotkeys from the menu.");
    println!();
    println!("Press Enter to return to menu...");

    wait_for_enter();
}

fn flush_console_input() {
    unsafe {
        use windows::Win32::System::Console::{
            FlushConsoleInputBuffer, GetStdHandle, STD_INPUT_HANDLE,
        };
        if let Ok(handle) = GetStdHandle(STD_INPUT_HANDLE) {
            let _ = FlushConsoleInputBuffer(handle);
        }
    }
}

fn wait_for_enter() {
    let mut input = String::new();
    let _ = std::io::stdin().read_line(&mut input);
}