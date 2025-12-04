use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use rac_clicker::{
    ConfigProfile,
    RacResult, RacError, MouseButton,
    ThreadManager, ClickWorker, WorkerConfig,
    ClickExecutor, ClickController, DelayCalculator,
    WindowFinder, WindowHandle,
    InputMonitor, HotkeyManager,
    ConsoleMenu,
    UpdateManager, Version,
};

struct RacApp {
    profile: ConfigProfile,
    thread_manager: Arc<Mutex<ThreadManager>>,
    window_handle: Arc<WindowHandle>,
    window_finder: Arc<WindowFinder>,
}

impl Drop for RacApp {
    fn drop(&mut self) {
        if let Ok(mut tm) = self.thread_manager.lock() {
            let _ = tm.stop_all();
        }
    }
}

impl RacApp {
    fn new(profile: ConfigProfile) -> RacResult<Self> {
        let target_process = profile.get_target_process()?;

        Ok(Self {
            profile,
            thread_manager: Arc::new(Mutex::new(ThreadManager::new())),
            window_handle: Arc::new(WindowHandle::new()),
            window_finder: Arc::new(WindowFinder::new(&target_process)),
        })
    }

    fn initialize_workers(&mut self) -> RacResult<()> {
        let server_config = self.profile.server_registry.get_active()?;

        let left_cps = self.profile.get_left_cps()?;
        let right_cps = self.profile.get_right_cps()?;

        if self.profile.settings.click_mode.is_left_active() {
            let mut pattern = server_config.left_click;
            pattern.max_cps = left_cps;

            let config = WorkerConfig::left_click(pattern);
            let worker = ClickWorker::new(config);

            let mut tm = self.thread_manager.lock()
                .map_err(|e| RacError::SyncError(format!("Failed to lock thread manager: {}", e)))?;
            tm.register_worker(worker);
        }

        if self.profile.settings.click_mode.is_right_active() {
            let mut pattern = server_config.right_click;
            pattern.max_cps = right_cps;

            let config = WorkerConfig::right_click(pattern);
            let worker = ClickWorker::new(config);

            let mut tm = self.thread_manager.lock()
                .map_err(|e| RacError::SyncError(format!("Failed to lock thread manager: {}", e)))?;
            tm.register_worker(worker);
        }

        Ok(())
    }

    fn start_workers(&mut self) -> RacResult<()> {
        let toggle_mode = self.profile.settings.toggle_mode;
        let server_type = self.profile.settings.active_server;

        if self.profile.settings.click_mode.is_left_active() {
            let window_handle = Arc::clone(&self.window_handle);
            let controller = ClickController::new(toggle_mode, ClickExecutor::new());

            let tm = self.thread_manager.lock()
                .map_err(|e| RacError::SyncError(e.to_string()))?;

            if let Some(worker) = tm.get_worker(MouseButton::Left) {
                let delay_calc = DelayCalculator::new(worker.config().pattern, MouseButton::Left, server_type)?;

                worker.signal().pause();

                drop(tm);

                let mut tm_mut = self.thread_manager.lock()
                    .map_err(|e| RacError::SyncError(e.to_string()))?;

                tm_mut.start_worker(MouseButton::Left, move |worker| {
                    let mut delay = delay_calc;
                    controller.run_loop(worker, &mut delay, || window_handle.get());
                })?;
            }
        }

        if self.profile.settings.click_mode.is_right_active() {
            let window_handle = Arc::clone(&self.window_handle);
            let controller = ClickController::new(toggle_mode, ClickExecutor::new());

            let tm = self.thread_manager.lock()
                .map_err(|e| RacError::SyncError(e.to_string()))?;

            if let Some(worker) = tm.get_worker(MouseButton::Right) {
                let delay_calc = DelayCalculator::new(worker.config().pattern, MouseButton::Right, server_type)?;

                worker.signal().pause();

                drop(tm);

                let mut tm_mut = self.thread_manager.lock()
                    .map_err(|e| RacError::SyncError(e.to_string()))?;

                tm_mut.start_worker(MouseButton::Right, move |worker| {
                    let mut delay = delay_calc;
                    controller.run_loop(worker, &mut delay, || window_handle.get());
                })?;
            }
        }

        Ok(())
    }

    fn start_window_finder(&self) {
        let window_finder = Arc::clone(&self.window_finder);
        let window_handle = Arc::clone(&self.window_handle);

        thread::spawn(move || {
            loop {
                let _ = window_finder.find_window(&window_handle);
                thread::sleep(Duration::from_secs(2));
            }
        });
    }

    fn start_input_monitor(&self) -> RacResult<()> {
        let settings = &self.profile.settings;

        let mut input_monitor = InputMonitor::new(
            settings.toggle_mode,
            settings.click_mode,
            settings.toggle_hotkey,
            settings.left_hotkey,
            settings.right_hotkey,
        );

        let thread_manager = Arc::clone(&self.thread_manager);

        thread::spawn(move || {
            input_monitor.monitor_loop(thread_manager);
        });

        Ok(())
    }

    fn run(&mut self) -> RacResult<()> {
        println!("\n╔════════════════════════════════════════════╗");
        println!("║         RAC v2 - Starting...               ║");
        println!("╚════════════════════════════════════════════╝\n");

        println!("→ Initializing workers...");
        self.initialize_workers()?;

        println!("→ Starting worker threads...");
        self.start_workers()?;

        println!("→ Starting window finder...");
        self.start_window_finder();

        println!("→ Starting input monitor...");
        self.start_input_monitor()?;

        println!("\n✓ RAC v2 is now running!");
        println!("\nConfiguration:");
        println!("  Server:       {}", self.profile.server_registry.active_server_type());
        println!("  Toggle Mode:  {}", self.profile.settings.toggle_mode);
        println!("  Click Mode:   {}", self.profile.settings.click_mode);
        println!("  Toggle Key:   {}", HotkeyManager::key_name(self.profile.settings.toggle_hotkey));

        if self.profile.settings.click_mode.is_left_active() {
            let left_cps = self.profile.get_left_cps()?;
            println!("  Left CPS:     {}", left_cps);
        }
        if self.profile.settings.click_mode.is_right_active() {
            let right_cps = self.profile.get_right_cps()?;
            println!("  Right CPS:    {}", right_cps);
        }

        println!("\nPress Ctrl+Q to return to main menu...\n");

        use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_CONTROL, VK_Q};

        loop {
            thread::sleep(Duration::from_millis(50));

            unsafe {
                let ctrl_pressed = GetAsyncKeyState(VK_CONTROL.0 as i32) < 0;
                let q_pressed = GetAsyncKeyState(VK_Q.0 as i32) < 0;

                if ctrl_pressed && q_pressed {
                    println!("\n✓ Ctrl+Q detected - Stopping RAC v2...");

                    if let Ok(mut tm) = self.thread_manager.lock() {
                        let _ = tm.stop_all();
                    }

                    thread::sleep(Duration::from_millis(500));
                    println!("✓ RAC v2 stopped successfully!");
                    println!("✓ Returning to main menu...\n");

                    return Ok(());
                }
            }
        }
    }
}

fn check_single_instance() -> bool {
    use windows::Win32::Foundation::ERROR_ALREADY_EXISTS;
    use windows::Win32::System::Threading::CreateMutexW;
    use windows::core::w;

    unsafe {
        let mutex_name = w!("Global\\RACv2ApplicationMutex");
        let _ = CreateMutexW(None, true, mutex_name);
        windows::Win32::Foundation::GetLastError() != ERROR_ALREADY_EXISTS
    }
}

fn bring_existing_instance_to_front() -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, SetForegroundWindow, ShowWindow,
        SW_RESTORE, IsIconic
    };
    use windows::Win32::Foundation::{HWND, LPARAM};

    unsafe {
        let mut found_hwnd: HWND = HWND::default();

        let _ = EnumWindows(
            Some(enum_window_callback),
            LPARAM(&mut found_hwnd as *mut HWND as isize),
        );

        if !found_hwnd.is_invalid() && found_hwnd != HWND::default() {
            if IsIconic(found_hwnd).as_bool() {
                let _ = ShowWindow(found_hwnd, SW_RESTORE);
            }
            let _ = SetForegroundWindow(found_hwnd);
            return true;
        }

        false
    }
}

unsafe extern "system" fn enum_window_callback(
    hwnd: windows::Win32::Foundation::HWND,
    lparam: windows::Win32::Foundation::LPARAM
) -> windows::core::BOOL {
    use windows::Win32::UI::WindowsAndMessaging::{GetWindowThreadProcessId, IsWindowVisible};
    use windows::Win32::System::Threading::GetCurrentProcessId;

    unsafe {
        if !IsWindowVisible(hwnd).as_bool() {
            return true.into();
        }

        let mut window_pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut window_pid));

        let current_pid = GetCurrentProcessId();
        if window_pid == current_pid {
            return true.into();
        }

        let window_exe = get_process_path(window_pid);

        let current_exe_name = std::env::current_exe()
            .ok()
            .and_then(|path| path.file_name().map(|n| n.to_string_lossy().to_string()))
            .unwrap_or_else(|| "rac-clicker.exe".to_string());

        if window_exe.contains(&current_exe_name) {
            let result_ptr = lparam.0 as *mut windows::Win32::Foundation::HWND;
            if !result_ptr.is_null() {
                *result_ptr = hwnd;
                return false.into();
            }
        }

        true.into()
    }
}

unsafe fn get_process_path(pid: u32) -> String {
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};
    use windows::Win32::System::ProcessStatus::GetModuleFileNameExW;

    unsafe {
        let process = match OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid) {
            Ok(p) => p,
            Err(_) => return String::new(),
        };

        let mut path_buffer = [0u16; 260];
        let len = GetModuleFileNameExW(Some(process), None, &mut path_buffer);

        if len > 0 {
            String::from_utf16_lossy(&path_buffer[..len as usize])
        } else {
            String::new()
        }
    }
}

fn check_and_update() {
    use std::io::{self, Write};

    print!("Checking for updates");
    io::stdout().flush().ok();

    let checking = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let checking_clone = checking.clone();

    let animation_handle = thread::spawn(move || {
        while checking_clone.load(std::sync::atomic::Ordering::Relaxed) {
            for _ in 0..3 {
                if !checking_clone.load(std::sync::atomic::Ordering::Relaxed) {
                    break;
                }
                print!(".");
                io::stdout().flush().ok();
                thread::sleep(Duration::from_millis(300));
            }
            if checking_clone.load(std::sync::atomic::Ordering::Relaxed) {
                print!("\rChecking for updates   ");
                io::stdout().flush().ok();
            }
        }
    });

    match UpdateManager::new() {
        Ok(update_mgr) => {
            match update_mgr.check_for_updates() {
                Ok(Some(release)) => {
                    checking.store(false, std::sync::atomic::Ordering::Relaxed);
                    let _ = animation_handle.join();

                    println!("\r                                              ");
                    println!("\n╔══════════════════════════════════════════╗");
                    println!("║           NEW UPDATE AVAILABLE!            ║");
                    println!("╚════════════════════════════════════════════╝");
                    println!("\nCurrent Version:  v{}", Version::current());
                    println!("New Version:      v{}", release.version);
                    println!("Release Name:     {}", release.release_name);
                    println!("File Size:        {:.2} MB", release.asset_size as f64 / 1024.0 / 1024.0);

                    if !release.release_notes.is_empty() {
                        println!("\nRelease Notes:");
                        println!("─────────────────────────────────────────────");
                        println!("{}", release.release_notes);
                        println!("─────────────────────────────────────────────");
                    }

                    println!("\nInstall update? [Y/n]: ");
                    print!("> ");
                    io::stdout().flush().ok();

                    let mut input = String::new();
                    if io::stdin().read_line(&mut input).is_ok() {
                        let answer = input.trim().to_lowercase();

                        if answer == "y" || answer == "yes" || answer.is_empty() {
                            println!("\nDownloading update...\n");

                            let progress_cb = Arc::new(|current: u64, total: u64| {
                                if total > 0 {
                                    let percent = (current as f64 / total as f64) * 100.0;
                                    let mb_current = current as f64 / 1024.0 / 1024.0;
                                    let mb_total = total as f64 / 1024.0 / 1024.0;
                                    print!("\rDownloading: {:.1}% ({:.2}/{:.2} MB)   ",
                                        percent, mb_current, mb_total);
                                    io::stdout().flush().ok();
                                }
                            });

                            match update_mgr.download_and_install(&release, Some(progress_cb)) {
                                Ok(_) => {
                                    println!("\n\nUpdate downloaded successfully!");
                                    println!("Restarting application...\n");
                                    thread::sleep(Duration::from_secs(1));
                                }
                                Err(e) => {
                                    println!("\n\nUpdate failed: {}", e);
                                    println!("RAC will continue with current version.\n");
                                    thread::sleep(Duration::from_secs(2));
                                }
                            }
                        } else {
                            println!("\n⏭Update skipped. Starting RAC...\n");
                            thread::sleep(Duration::from_millis(800));
                        }
                    }
                }
                Ok(None) => {
                    checking.store(false, std::sync::atomic::Ordering::Relaxed);
                    let _ = animation_handle.join();
                    println!("\rYou're up to date! (v{})        ", Version::current());
                    thread::sleep(Duration::from_millis(500));
                    println!();
                }
                Err(e) => {
                    checking.store(false, std::sync::atomic::Ordering::Relaxed);
                    let _ = animation_handle.join();
                    println!("\r  Could not check for updates: {}        ", e);
                    println!("   Starting RAC normally...\n");
                    thread::sleep(Duration::from_millis(800));
                }
            }
        }
        Err(e) => {
            checking.store(false, std::sync::atomic::Ordering::Relaxed);
            let _ = animation_handle.join();
            println!("\r   Could not initialize update system: {}        ", e);
            println!("   Starting RAC normally...\n");
            thread::sleep(Duration::from_millis(800));
        }
    }
}

fn main() -> RacResult<()> {
    if !check_single_instance() {
        if bring_existing_instance_to_front() {
            std::process::exit(0);
        } else {
            unsafe {
                use windows::Win32::System::Console::{GetStdHandle, FlushConsoleInputBuffer, STD_INPUT_HANDLE};
                if let Ok(handle) = GetStdHandle(STD_INPUT_HANDLE) {
                    let _ = FlushConsoleInputBuffer(handle);
                }
            }

            eprintln!("✗ RAC v2 is already running!");
            eprintln!("  (Couldn't find the window to bring to front)");
            eprintln!("\nPress Enter to exit...");

            let mut input = String::new();
            let _ = std::io::stdin().read_line(&mut input);
            std::process::exit(1);
        }
    }

    check_and_update();

    loop {
        unsafe {
            use windows::Win32::System::Console::{GetStdHandle, FlushConsoleInputBuffer, STD_INPUT_HANDLE};
            if let Ok(handle) = GetStdHandle(STD_INPUT_HANDLE) {
                let _ = FlushConsoleInputBuffer(handle);
            }
        }

        let mut menu = ConsoleMenu::new()?;

        match menu.show_main_menu() {
            Ok(()) => {}
            Err(RacError::UserExit) => {
                return Ok(());
            }
            Err(e) => {
                return Err(e);
            }
        }

        let profile = menu.profile().clone();

        let has_toggle = profile.settings.toggle_hotkey != 0;
        let has_left = profile.settings.left_hotkey != 0;
        let has_right = profile.settings.right_hotkey != 0;

        if !has_toggle && !has_left && !has_right {
            unsafe {
                use windows::Win32::System::Console::{GetStdHandle, FlushConsoleInputBuffer, STD_INPUT_HANDLE};
                if let Ok(handle) = GetStdHandle(STD_INPUT_HANDLE) {
                    let _ = FlushConsoleInputBuffer(handle);
                }
            }

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

            let mut input = String::new();
            let _ = std::io::stdin().read_line(&mut input);

            continue;
        }

        let mut app = RacApp::new(profile)?;
        app.run()?;
    }
}