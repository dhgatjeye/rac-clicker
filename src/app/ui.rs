use std::io::{self, Write};

pub fn print_banner(title: &str) {
    println!("\n╔════════════════════════════════════════════╗");
    println!("║  {:^40}  ║", title);
    println!("╚════════════════════════════════════════════╝\n");
}

pub fn print_error_banner() {
    println!("╔════════════════════════════════════════════╗");
    println!("║              ERROR                         ║");
    println!("╚════════════════════════════════════════════╝");
}

pub fn wait_for_enter(message: &str) {
    println!("{}", message);
    let mut input = String::new();
    let _ = io::stdin().read_line(&mut input);
}

pub fn show_no_hotkeys_error() {
    print_error_banner();
    println!();
    println!("✗ No hotkeys configured!");
    println!();
    println!("You must configure at least one hotkey:");
    println!("  • Toggle hotkey (global on/off), OR");
    println!("  • Left/Right click hotkeys (direct control)");
    println!();
    println!("Please configure hotkeys from the menu.");
    println!();
    wait_for_enter("Press Enter to return to menu...");
}

pub fn show_already_running_error() {
    eprintln!("✗ RAC v2 is already running!");
    eprintln!();
    wait_for_enter("Press Enter to exit...");
}

pub fn print_dots_animation(checking: &std::sync::atomic::AtomicBool) {
    use std::sync::atomic::Ordering;
    use std::thread;
    use std::time::Duration;

    while checking.load(Ordering::Relaxed) {
        for _ in 0..3 {
            if !checking.load(Ordering::Relaxed) {
                break;
            }
            print!(".");
            io::stdout().flush().ok();
            thread::sleep(Duration::from_millis(300));
        }
        if checking.load(Ordering::Relaxed) {
            print!("\rChecking for updates   ");
            io::stdout().flush().ok();
        }
    }
}

pub fn prompt_yes_no(default_yes: bool) -> bool {
    print!("> ");
    io::stdout().flush().ok();

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_ok() {
        let answer = input.trim().to_lowercase();
        if default_yes {
            answer.is_empty() || answer == "y" || answer == "yes"
        } else {
            answer == "y" || answer == "yes"
        }
    } else {
        default_yes
    }
}

pub fn create_download_progress_callback() -> impl Fn(u64, u64) {
    move |current: u64, total: u64| {
        if total > 0 {
            let percent = (current as f64 / total as f64) * 100.0;
            let mb_current = current as f64 / 1024.0 / 1024.0;
            let mb_total = total as f64 / 1024.0 / 1024.0;
            print!(
                "\rDownloading: {:.1}% ({:.2}/{:.2} MB)   ",
                percent, mb_current, mb_total
            );
            io::stdout().flush().ok();
        }
    }
}
