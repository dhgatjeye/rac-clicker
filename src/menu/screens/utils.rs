use std::io::{self, Write};
use windows::Win32::System::Console::{
    CONSOLE_SCREEN_BUFFER_INFO, COORD, FillConsoleOutputAttribute, FillConsoleOutputCharacterA,
    GetConsoleScreenBufferInfo, GetStdHandle, STD_OUTPUT_HANDLE, SetConsoleCursorPosition,
};

pub struct ScreenUtils;

impl ScreenUtils {
    pub fn clear_console() {
        unsafe {
            let console_handle = GetStdHandle(STD_OUTPUT_HANDLE).ok();
            if let Some(handle) = console_handle {
                let mut csbi = CONSOLE_SCREEN_BUFFER_INFO::default();

                if GetConsoleScreenBufferInfo(handle, &mut csbi).is_ok() {
                    let console_size = (csbi.dwSize.X as u32) * (csbi.dwSize.Y as u32);
                    let home_coords = COORD { X: 0, Y: 0 };
                    let mut written = 0;

                    let _ = FillConsoleOutputCharacterA(
                        handle,
                        b' ' as i8,
                        console_size,
                        home_coords,
                        &mut written,
                    );

                    let _ = FillConsoleOutputAttribute(
                        handle,
                        csbi.wAttributes.0,
                        console_size,
                        home_coords,
                        &mut written,
                    );

                    let _ = SetConsoleCursorPosition(handle, home_coords);
                }
            }
        }
    }

    pub fn press_enter_to_continue() {
        unsafe {
            use windows::Win32::System::Console::{
                FlushConsoleInputBuffer, GetStdHandle, STD_INPUT_HANDLE,
            };
            if let Ok(handle) = GetStdHandle(STD_INPUT_HANDLE) {
                let _ = FlushConsoleInputBuffer(handle);
            }
        }

        println!("\nPress Enter to continue...");
        let mut _input = String::new();
        let _ = io::stdin().read_line(&mut _input);
    }

    pub fn read_input() -> io::Result<String> {
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        Ok(input)
    }

    pub fn prompt(message: &str) -> io::Result<String> {
        print!("{}", message);
        io::stdout().flush()?;
        Self::read_input()
    }
}
