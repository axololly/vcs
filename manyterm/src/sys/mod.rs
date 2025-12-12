mod unix;
mod windows;

use std::{io::Result, sync::atomic::{AtomicBool, Ordering}};

#[cfg(unix)]
pub use unix::*;

#[cfg(windows)]
pub use windows::*;

mod utils;
pub use utils::*;

static INITIATED: AtomicBool = AtomicBool::new(false);

/// This should be at the start of your `fn main()`.
/// 
/// It configures the library so that the specific platform hooks are performed correctly.
/// 
/// By default, this also enables focus events, bracketed paste and reporting `Alt+_` presses as `ESC_`.
/// To not enable these, use [`sys::setup`] instead.
pub fn init() -> Result<()> {
    if INITIATED.load(Ordering::Relaxed) {
        return Ok(());
    }

    INITIATED.store(true, Ordering::Relaxed);
    
    setup()?;

    enable_focus_events();
    enable_bracketed_paste();

    // Enable Alt key-press detection
    write_stdout(b"\x1b[?1036h")?;

    Ok(())
}

#[macro_export]
macro_rules! print {
    ($fmt:literal $(, $($args:tt)*)?) => {{
        let s = format!($fmt $(, $($args)*)?);

        let data: Vec<u8> = s.into_bytes();

        use $crate::sys::write_stdout;

        write_stdout(&data).unwrap();
    }};
}

#[macro_export]
macro_rules! println {
    ($fmt:literal $(, $($args:tt)*)?) => {{
        let s = format!($fmt $(, $($args)*)?);

        let mut data: Vec<u8> = s.into_bytes();

        data.push(b'\r');
        data.push(b'\n');

        use $crate::sys::write_stdout;

        write_stdout(&data).unwrap();
    }};
}