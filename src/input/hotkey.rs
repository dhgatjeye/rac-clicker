use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyEvent {
    Pressed,
    Released,
    Held,
}

#[repr(C, align(64))]
pub struct HotkeyManager {
    key_states: [bool; 256],
    registered: [i32; 8],
    count: u8,
}

impl Default for HotkeyManager {
    fn default() -> Self {
        Self::new()
    }
}

impl HotkeyManager {
    #[inline]
    pub fn new() -> Self {
        Self {
            key_states: [false; 256],
            registered: [0; 8],
            count: 0,
        }
    }

    #[inline]
    pub fn register(&mut self, vk_code: i32) {
        if vk_code <= 0 || vk_code > 255 {
            return;
        }

        for i in 0..self.count as usize {
            if self.registered[i] == vk_code {
                return;
            }
        }

        if (self.count as usize) < self.registered.len() {
            self.registered[self.count as usize] = vk_code;
            self.count += 1;
        }
    }

    #[inline(always)]
    pub fn is_pressed(&self, vk_code: i32) -> bool {
        if vk_code <= 0 || vk_code > 255 {
            return false;
        }

        unsafe { GetAsyncKeyState(vk_code) < 0 }
    }

    #[inline]
    pub fn poll(&mut self, vk_code: i32) -> Option<HotkeyEvent> {
        if vk_code <= 0 || vk_code > 255 {
            return None;
        }

        let current_state = self.is_pressed(vk_code);
        let last_state = unsafe { *self.key_states.get_unchecked(vk_code as usize) };

        unsafe {
            *self.key_states.get_unchecked_mut(vk_code as usize) = current_state;
        }

        match (last_state, current_state) {
            (false, true) => Some(HotkeyEvent::Pressed),
            (true, false) => Some(HotkeyEvent::Released),
            (true, true) => Some(HotkeyEvent::Held),
            (false, false) => None,
        }
    }

    #[inline]
    pub fn check_toggle(&mut self, vk_code: i32) -> bool {
        matches!(self.poll(vk_code), Some(HotkeyEvent::Pressed))
    }

    pub fn key_name(vk_code: i32) -> String {
        if vk_code == 0 {
            return "None".to_string();
        }

        match vk_code {
            0x01 => "Left Mouse".to_string(),
            0x02 => "Right Mouse".to_string(),
            0x04 => "Middle Mouse".to_string(),
            0x05 => "X1 Mouse".to_string(),
            0x06 => "X2 Mouse".to_string(),
            0x08 => "Backspace".to_string(),
            0x09 => "Tab".to_string(),
            0x0D => "Enter".to_string(),
            0x10 => "Shift".to_string(),
            0x11 => "Ctrl".to_string(),
            0x12 => "Alt".to_string(),
            0x1B => "Escape".to_string(),
            0x20 => "Space".to_string(),
            0x21 => "Page Up".to_string(),
            0x22 => "Page Down".to_string(),
            0x23 => "End".to_string(),
            0x24 => "Home".to_string(),
            0x25 => "Left Arrow".to_string(),
            0x26 => "Up Arrow".to_string(),
            0x27 => "Right Arrow".to_string(),
            0x28 => "Down Arrow".to_string(),
            0x2D => "Insert".to_string(),
            0x2E => "Delete".to_string(),
            0x30..=0x39 => format!("{}", vk_code - 0x30),
            0x41..=0x5A => format!("{}", vk_code as u8 as char),
            0x60..=0x69 => format!("Numpad {}", vk_code - 0x60),
            0x70..=0x87 => format!("F{}", vk_code - 0x6F),
            _ => format!("Key {:#X}", vk_code),
        }
    }
}
