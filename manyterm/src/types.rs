/// Represents a pressed key.
/// 
/// Borrowed from [`rhysd/tui-textarea`](https://github.com/rhysd/tui-textarea).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    Char(char),
    F(u8),
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    Enter,
    Escape,
    Backspace,
    Delete,
    Tab,
    PageUp,
    PageDown,
    Insert
}

/// Represents the three keyboard modifiers: `Ctrl`, `Alt` and `Shift`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Modifiers {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool
}

impl Modifiers {
    pub fn new(ctrl: bool, shift: bool, alt: bool) -> Modifiers {
        Modifiers { ctrl, shift, alt }
    }

    pub fn none() -> Modifiers {
        Modifiers {
            ctrl: false,
            shift: false,
            alt: false
        }
    }

    pub fn any(&self) -> bool {
        self.ctrl || self.shift || self.alt
    }
}

/// Represents the key pressed and any keyboard
/// modifiers that were also pressed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct KeyInput {
    pub key: Key,
    pub modifiers: Modifiers
}

impl From<Key> for KeyInput {
    fn from(value: Key) -> Self {
        KeyInput {
            key: value,
            modifiers: Modifiers::none()
        }
    }
}

/// Represents the mouse button used in the interaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Middle,
    Right
}

/// Represents the type of mouse input that happened in the terminal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MouseInputKind {
    Press(MouseButton),
    Release(MouseButton),
    Drag(MouseButton),
    Moved,
    ScrollUp,
    ScrollDown,
    ScrollLeft,
    ScrollRight
}

/// Represents a mouse input.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MouseInput {
    pub row: u16,
    pub column: u16,
    pub kind: MouseInputKind,
    pub modifiers: Modifiers
}

/// Represents a terminal size in columns.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Size {
    pub width: u16,
    pub height: u16
}

/// Represents an event that happened in a terminal.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Event {
    Key(KeyInput),
    Mouse(MouseInput),
    Paste(Vec<u8>),
    Resize(Size),
    FocusGained,
    FocusLost
}

impl Event {
    pub fn as_key_input(&self) -> Option<KeyInput> {
        if let Event::Key(k) = *self {
            Some(k)
        }
        else {
            None
        }
    }

    pub fn is_key_input(&self) -> bool {
        matches!(self, Event::Key(_))
    }

    pub fn as_mouse_input(&self) -> Option<MouseInput> {
        if let Event::Mouse(m) = *self {
            Some(m)
        }
        else {
            None
        }
    }

    pub fn is_mouse_input(&self) -> bool {
        matches!(self, Event::Mouse(_))
    }

    pub fn as_paste_event(&self) -> Option<&[u8]> {
        if let Event::Paste(data) = self {
            Some(data)
        }
        else {
            None
        }
    }

    pub fn is_paste_event(&self) -> bool {
        matches!(self, Event::Paste(_))
    }

    pub fn as_resize_event(&self) -> Option<Size> {
        if let Event::Resize(new_size) = *self {
            Some(new_size)
        }
        else {
            None
        }
    }

    pub fn is_resize_event(&self) -> bool {
        matches!(self, Event::Resize(_))
    }

    pub fn is_focus_gained(&self) -> bool {
        matches!(self, Event::FocusGained)
    }

    pub fn is_focus_lost(&self) -> bool {
        matches!(self, Event::FocusLost)
    }
}