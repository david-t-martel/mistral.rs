#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub control: bool,
    pub alt: bool,
    pub shift: bool,
}

impl Modifiers {
    pub const NONE: Self = Self {
        control: false,
        alt: false,
        shift: false,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCode {
    Char(char),
    Enter,
    Backspace,
    Tab,
    Esc,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    Delete,
    Insert,
    Function(u8),
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyEvent {
    pub code: KeyCode,
    pub modifiers: Modifiers,
}

impl KeyEvent {
    pub fn new(code: KeyCode, modifiers: Modifiers) -> Self {
        Self { code, modifiers }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputEvent {
    Tick,
    Key(KeyEvent),
    Resize(u16, u16),
}

#[cfg(feature = "terminal")]
pub fn from_crossterm(event: crossterm::event::Event) -> Option<InputEvent> {
    use crossterm::event::Event;

    match event {
        Event::Key(key) => {
            let mods = convert_crossterm_modifiers(key.modifiers);
            let code = convert_crossterm_code(key.code);
            Some(InputEvent::Key(KeyEvent::new(code, mods)))
        }
        Event::Resize(width, height) => Some(InputEvent::Resize(width, height)),
        _ => None,
    }
}

#[cfg(feature = "terminal")]
fn convert_crossterm_modifiers(modifiers: crossterm::event::KeyModifiers) -> Modifiers {
    use crossterm::event::KeyModifiers as KM;
    Modifiers {
        control: modifiers.contains(KM::CONTROL),
        alt: modifiers.contains(KM::ALT),
        shift: modifiers.contains(KM::SHIFT),
    }
}

#[cfg(feature = "terminal")]
fn convert_crossterm_code(code: crossterm::event::KeyCode) -> KeyCode {
    use crossterm::event::KeyCode as C;

    match code {
        C::Backspace => KeyCode::Backspace,
        C::Enter => KeyCode::Enter,
        C::Left => KeyCode::Left,
        C::Right => KeyCode::Right,
        C::Up => KeyCode::Up,
        C::Down => KeyCode::Down,
        C::Home => KeyCode::Home,
        C::End => KeyCode::End,
        C::PageUp => KeyCode::PageUp,
        C::PageDown => KeyCode::PageDown,
        C::Tab => KeyCode::Tab,
        C::Delete => KeyCode::Delete,
        C::Insert => KeyCode::Insert,
        C::Esc => KeyCode::Esc,
        C::F(number) => KeyCode::Function(number as u8),
        C::Char(c) => KeyCode::Char(c),
        _ => KeyCode::Unknown,
    }
}

#[cfg(feature = "gpu")]
pub fn from_winit(event: &winit::event::WindowEvent, modifiers: Modifiers) -> Option<InputEvent> {
    use winit::event::{ElementState, WindowEvent};

    match event {
        WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
            let code = convert_winit_key(&event.logical_key, event.text.as_deref());
            Some(InputEvent::Key(KeyEvent::new(code, modifiers)))
        }
        WindowEvent::Resized(size) => {
            Some(InputEvent::Resize(size.width as u16, size.height as u16))
        }
        _ => None,
    }
}

#[cfg(feature = "gpu")]
fn convert_winit_key(key: &winit::keyboard::Key, text: Option<&str>) -> KeyCode {
    use winit::keyboard::{Key, NamedKey};

    match key {
        Key::Character(value) => value
            .chars()
            .next()
            .or_else(|| text.and_then(|t| t.chars().next()))
            .map(KeyCode::Char)
            .unwrap_or(KeyCode::Unknown),
        Key::Named(named) => match named {
            NamedKey::Enter => KeyCode::Enter,
            NamedKey::Tab => KeyCode::Tab,
            NamedKey::Space => KeyCode::Char(' '),
            NamedKey::ArrowUp => KeyCode::Up,
            NamedKey::ArrowDown => KeyCode::Down,
            NamedKey::ArrowLeft => KeyCode::Left,
            NamedKey::ArrowRight => KeyCode::Right,
            NamedKey::Home => KeyCode::Home,
            NamedKey::End => KeyCode::End,
            NamedKey::PageUp => KeyCode::PageUp,
            NamedKey::PageDown => KeyCode::PageDown,
            NamedKey::Backspace => KeyCode::Backspace,
            NamedKey::Delete => KeyCode::Delete,
            NamedKey::Insert => KeyCode::Insert,
            NamedKey::Escape => KeyCode::Esc,
            NamedKey::F1 => KeyCode::Function(1),
            NamedKey::F2 => KeyCode::Function(2),
            NamedKey::F3 => KeyCode::Function(3),
            NamedKey::F4 => KeyCode::Function(4),
            NamedKey::F5 => KeyCode::Function(5),
            NamedKey::F6 => KeyCode::Function(6),
            NamedKey::F7 => KeyCode::Function(7),
            NamedKey::F8 => KeyCode::Function(8),
            NamedKey::F9 => KeyCode::Function(9),
            NamedKey::F10 => KeyCode::Function(10),
            NamedKey::F11 => KeyCode::Function(11),
            NamedKey::F12 => KeyCode::Function(12),
            _ => text
                .and_then(|t| t.chars().next())
                .map(KeyCode::Char)
                .unwrap_or(KeyCode::Unknown),
        },
        _ => text
            .and_then(|t| t.chars().next())
            .map(KeyCode::Char)
            .unwrap_or(KeyCode::Unknown),
    }
}
