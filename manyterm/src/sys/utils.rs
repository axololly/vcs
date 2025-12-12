use crate::print;

/// Enter the alternate screen.
/// 
/// This preserves the original buffer and switches to another one.
pub fn enter_alternate_screen() {
    print!("\x1b[?1049h\x1b[H")
}

/// Leave the alternate screen.
/// 
/// This disposes of the current buffer and switches back to the original.
pub fn leave_alternate_screen() {
    print!("\x1b[?1049l")
}

/// Enable the ability to tell apart pasted input from typed input.
/// 
/// This gives you access to [`Event::Paste`] events.
pub fn enable_bracketed_paste() {
    print!("\x1b[?2004h")
}

/// Enable the ability to tell apart pasted input from typed input.
/// 
/// This removes access to [`Event::Paste`] events.
pub fn disable_bracketed_paste() {
    print!("\x1b[?2004l")
}

/// Enable dispatching events when the terminal goes into and out of focus.
/// 
/// This gives you access to [`Event::FocusGained`] and [`Event::FocusLost`] events.
pub fn enable_focus_events() {
    print!("\x1b[?2004h")
}

/// Enable dispatching events when the terminal goes into and out of focus.
/// 
/// This removes access to [`Event::FocusGained`] and [`Event::FocusLost`] events.
pub fn disable_focus_events() {
    print!("\x1b[?2004l")
}

const MOUSE_CODES: [u16; 6] = [1000, 1001, 1002, 1003, 1006, 1015];

/// Get events about the mouse during execution.
/// 
/// This gives you access to [`Event::Mouse`] events.
pub fn enable_mouse_input() {
    print!("{}", MOUSE_CODES
        .map(|d| format!("\x1b[?{d}h"))
        .join("")
    )
}

/// Stop getting events about the mouse during execution.
/// 
/// This removes access to [`Event::Mouse`] events.
pub fn disable_mouse_input() {
    print!("{}", MOUSE_CODES
        .map(|d| format!("\x1b[?{d}l"))
        .join("")
    )
}