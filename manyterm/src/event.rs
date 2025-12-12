use std::{sync::Mutex, time::Duration};

use crate::{
    sys,
    types::{Key, KeyInput, Modifiers, MouseButton, MouseInput, MouseInputKind, Size},
    vt::{Token, VTStream}
};

pub use crate::types::Event;

fn has_nth_bit(v: u16, n: u16) -> bool {
    let b = 1 << n;

    v & b == b
}

fn get_modifiers_from(kind: u16) -> Modifiers {
    let shift = has_nth_bit(kind, 2);
    let alt = has_nth_bit(kind, 3);
    let ctrl = has_nth_bit(kind, 4);

    Modifiers { ctrl, alt, shift }
}

fn get_key_from_csi(params: &[u16], footer: char) -> Option<Key> {
    use Key::*;

    // Try with XTerm sequences
    let key = match footer {
        'A' => Some(Up),
        'B' => Some(Down),
        'C' => Some(Right),
        'D' => Some(Left),

        'F' => Some(End),
        'H' => Some(Home),
        
        _ => None
    };

    if key.is_some() {
        return key;
    }

    match *params.first()? {
        1 => Some(Home),
        2 => Some(Insert),
        3 => Some(Delete),
        4 => Some(End),
        5 => Some(PageUp),
        6 => Some(PageDown),
        7 => Some(Home),
        8 => Some(End),

        d @ 10..=15 => Some(F(d as u8 - 10)),
        
        d @ 17..=21 => Some(F(d as u8 - 11)),

        d @ 23..=26 => Some(F(d as u8 - 12)),

        28 => Some(F(15)),
        29 => Some(F(16)),

        d @ 31..=34 => Some(F(d as u8 - 14)),
        
        _ => None
    }
}

fn search(input: &[u8], target: &[u8]) -> Option<usize> {
    if input.len() < target.len() {
        return None;
    }

    let end_of_iter = input.len() - target.len();

    for index in 0..end_of_iter {
        let slice = &input[index .. index + target.len()];

        if slice == target {
            return Some(index);
        }
    }

    None
}

pub struct EventStream {
    vt_stream: VTStream
}

impl EventStream {
    pub fn new(input: Vec<u8>) -> EventStream {
        EventStream {
            vt_stream: VTStream::new(input)
        }
    }
}

impl Iterator for EventStream {
    type Item = Event;

    fn next(&mut self) -> Option<Self::Item> {
        match self.vt_stream.next()? {
            // Paste events
            Token::Csi(None, params, Some('~')) if params == [200] => {
                let whole_input = &self.vt_stream.input;
                let offset = self.vt_stream.offset;

                let input = &whole_input[offset..];

                let target = b"\x1b[201~";

                let pasted = if let Some(end_of_paste) = search(input, target) {
                    let slice = &input[..end_of_paste];
                    
                    // Skip to after the closing bracket of the paste.
                    self.vt_stream.offset += slice.len() + target.len();

                    slice
                }
                else {
                    self.vt_stream.offset = whole_input.len();

                    // Couldn't find it so take the entirety of stdin literally.
                    input
                };
                
                Some(Event::Paste(pasted.to_vec()))
            }

            // Resize events
            Token::Csi(None, params, Some('t')) => {
                let width = *params.get(2)?;
                let height = *params.get(1)?;

                Some(Event::Resize(Size { width, height }))
            }
            
            // Parse mouse events
            Token::Csi(Some('<'), params, Some(footer)) => {
                let kind = *params.first()?;
                let column = *params.get(1)?;
                let row = *params.get(2)?;

                // First 2 bits are the mouse button pressed.
                let button = match kind & 0b11 {
                    0 => Some(MouseButton::Left),
                    1 => Some(MouseButton::Middle),
                    2 => Some(MouseButton::Right),
                    _ => None
                };

                let modifiers = get_modifiers_from(kind);

                let is_motion = has_nth_bit(kind, 5);

                let input_kind = if kind >= 64 {
                    // Scroll event, so the context of the bits changed
                    match kind & 0b11 {
                        0 => MouseInputKind::ScrollUp,
                        1 => MouseInputKind::ScrollDown,
                        2 => MouseInputKind::ScrollLeft,
                        3 => MouseInputKind::ScrollRight,

                        _ => unreachable!()
                    }
                }
                else {
                    match button {
                        Some(b) if is_motion => MouseInputKind::Drag(b),
                        Some(b) if footer == 'M' => MouseInputKind::Press(b),
                        Some(b) => MouseInputKind::Release(b),

                        None => MouseInputKind::Moved
                    }
                };

                Some(Event::Mouse(MouseInput {
                    row,
                    column,
                    kind: input_kind,
                    modifiers
                }))
            }
        
            // Gained focus event
            Token::Csi(None, params, Some('I')) if params.is_empty() => Some(Event::FocusGained),

            // Lost focus event
            Token::Csi(None, params, Some('O')) if params.is_empty() => Some(Event::FocusLost),

            // Shift+Tab keypress
            Token::Csi(None, params, Some('Z')) if params.is_empty() => Some(Event::Key(
                KeyInput {
                    key: Key::Tab,
                    modifiers: Modifiers { ctrl: false, shift: true, alt: false }
                }
            )),

            // Custom key sequences
            Token::Csi(None, ref params, Some(footer)) => {
                if let Some(key) = get_key_from_csi(params, footer) {
                    let modifiers = if let Some(&modifier) = params.get(1) {
                        let bitmap = modifier - 1;

                        let shift = has_nth_bit(bitmap, 0);
                        let alt   = has_nth_bit(bitmap, 1);
                        let ctrl  = has_nth_bit(bitmap, 2);

                        Modifiers { ctrl, alt, shift }
                    }
                    else {
                        Modifiers::none()
                    };

                    Some(Event::Key(KeyInput { key, modifiers }))
                }
                else {
                    None
                }
            }

            Token::Esc(ch) => {
                if ch.is_ascii_alphabetic() {
                    let modifiers = Modifiers {
                        ctrl: false,
                        shift: ch.is_uppercase(),
                        alt: true
                    };

                    Some(Event::Key(KeyInput { key: Key::Char(ch.to_lowercase().next().unwrap()), modifiers }))
                }
                else {
                    None
                }
            }

            // Control characters
            Token::Control(ch) => Some({
                let mut modifiers = Modifiers::none();
                
                let key = match ch {
                    '\r' | '\n' => Key::Enter,

                    '\x1b' => Key::Escape,
                    '\t' => Key::Tab,
                    '\u{007F}' | '\u{0008}' => Key::Backspace,

                    c => if c.is_ascii_control() {
                        let fmt = ((c as u8) + 0x60) as char;

                        modifiers.ctrl = true;

                        Key::Char(fmt)
                    }
                    else {
                        modifiers.shift = c.is_ascii_uppercase();

                        Key::Char(c)
                    }
                };

                Event::Key(KeyInput { key, modifiers })
            }),

            Token::Text(ch) => Some({
                let mut modifiers = Modifiers::none();
                
                modifiers.shift = ch.is_ascii_uppercase();

                Event::Key(KeyInput {
                    key: Key::Char(ch),
                    modifiers
                })
            }),

            _ => None
        }
    }
}

static LATEST_EVENT_STREAM: Mutex<Option<EventStream>> = Mutex::new(None);

pub fn read(timeout: Duration) -> Option<Event> {
    sys::read_stdin(timeout)
        .unwrap()
        .filter(|data| !data.is_empty())
        .and_then(|data| {
            let mut lock = LATEST_EVENT_STREAM.lock().unwrap_or_else(|e| {
                let mut lock = e.into_inner();
                
                // Our latest stream got fucked?
                // lol who cares, nuke it
                *lock = None;

                lock
            });

            match lock.as_mut() {
                Some(stream) if !stream.vt_stream.is_done() => {
                    stream.next()
                }

                _ => {
                    let mut new_stream = EventStream::new(data);

                    let next= new_stream.next();

                    *lock = Some(new_stream);

                    next
                }
            }
        }
    )
}