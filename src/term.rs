//! Terminal queries and setup.

use std::io;

const DEFAULT_WIDTH: u32 = 80;

/// Width in columns of whichever standard stream is attached to a terminal
/// (so `imcat x.png | less -R` still sizes to the terminal), otherwise 80.
pub fn width() -> u32 {
    use terminal_size::{Width, terminal_size_of};
    [
        terminal_size_of(io::stdout()),
        terminal_size_of(io::stderr()),
        terminal_size_of(io::stdin()),
    ]
    .into_iter()
    .flatten()
    .map(|(Width(w), _)| u32::from(w))
    .find(|&w| w > 0)
    .unwrap_or(DEFAULT_WIDTH)
}

/// Windows consoles need virtual terminal processing turned on to interpret ANSI escapes.
#[cfg(windows)]
pub fn enable_ansi_escapes() {
    use windows_sys::Win32::System::Console::{
        ENABLE_VIRTUAL_TERMINAL_PROCESSING, GetConsoleMode, GetStdHandle, STD_OUTPUT_HANDLE,
        SetConsoleMode,
    };
    // SAFETY: plain Win32 calls on the process's own stdout handle; failures are harmless.
    unsafe {
        let handle = GetStdHandle(STD_OUTPUT_HANDLE);
        let mut mode = 0;
        if GetConsoleMode(handle, &mut mode) != 0 {
            SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
        }
    }
}

#[cfg(not(windows))]
pub fn enable_ansi_escapes() {}
