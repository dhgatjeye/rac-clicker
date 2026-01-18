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

pub fn display_progress(current: u64, total: u64) {
    if total > 0 && current <= total {
        let percent = ((current as f64 / total as f64) * 100.0).min(100.0);
        let mb_current = (current as f64 / 1024.0 / 1024.0).max(0.0);
        let mb_total = (total as f64 / 1024.0 / 1024.0).max(0.0);

        print!(
            "\rDownloading: {:.1}% ({:.2}/{:.2} MB)   ",
            percent, mb_current, mb_total
        );
        io::stdout().flush().ok();
    } else if total == 0 {
        print!("\rDownloading: calculating...   ");
        io::stdout().flush().ok();
    }
}

pub fn format_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    const TB: f64 = GB * 1024.0;

    if bytes == 0 {
        return "0 B".to_string();
    }

    let size = bytes as f64;

    if !size.is_finite() {
        return format!("{} B", bytes);
    }

    match size {
        s if s >= TB => {
            let result = s / TB;
            if result.is_finite() {
                format!("{:.2} TB", result)
            } else {
                format!("{} B", bytes)
            }
        }
        s if s >= GB => format!("{:.1} GB", s / GB),
        s if s >= MB => format!("{:.1} MB", s / MB),
        s if s >= KB => format!("{:.0} KB", s / KB),
        _ => format!("{} B", bytes),
    }
}
