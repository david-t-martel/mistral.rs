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
pub fn from_winit(event: &winit::event::WindowEvent) -> Option<InputEvent> {
    use winit::event::{ElementState, KeyboardInput, WindowEvent};

    match event {
        WindowEvent::KeyboardInput {
            input:
                KeyboardInput {
                    state: ElementState::Pressed,
                    virtual_keycode,
                    text,
                    modifiers,
                    ..
                },
            ..
        } => {
            let mods = Modifiers {
                control: modifiers.control_key(),
                alt: modifiers.alt_key(),
                shift: modifiers.shift_key(),
            };
            let code = virtual_keycode
                .map(convert_winit_code)
                .or_else(|| text.and_then(|t| t.chars().next()).map(KeyCode::Char))
                .unwrap_or(KeyCode::Unknown);
            Some(InputEvent::Key(KeyEvent::new(code, mods)))
        }
        WindowEvent::Resized(size) => {
            Some(InputEvent::Resize(size.width as u16, size.height as u16))
        }
        _ => None,
    }
}

#[cfg(feature = "gpu")]
fn convert_winit_code(code: winit::event::VirtualKeyCode) -> KeyCode {
    use winit::event::VirtualKeyCode as Vk;

    match code {
        Vk::Return => KeyCode::Enter,
        Vk::Back => KeyCode::Backspace,
        Vk::Tab => KeyCode::Tab,
        Vk::Escape => KeyCode::Esc,
        Vk::Up => KeyCode::Up,
        Vk::Down => KeyCode::Down,
        Vk::Left => KeyCode::Left,
        Vk::Right => KeyCode::Right,
        Vk::Home => KeyCode::Home,
        Vk::End => KeyCode::End,
        Vk::PageUp => KeyCode::PageUp,
        Vk::PageDown => KeyCode::PageDown,
        Vk::Delete => KeyCode::Delete,
        Vk::Insert => KeyCode::Insert,
        Vk::F1 => KeyCode::Function(1),
        Vk::F2 => KeyCode::Function(2),
        Vk::F3 => KeyCode::Function(3),
        Vk::F4 => KeyCode::Function(4),
        Vk::F5 => KeyCode::Function(5),
        Vk::F6 => KeyCode::Function(6),
        Vk::F7 => KeyCode::Function(7),
        Vk::F8 => KeyCode::Function(8),
        Vk::F9 => KeyCode::Function(9),
        Vk::F10 => KeyCode::Function(10),
        Vk::F11 => KeyCode::Function(11),
        Vk::F12 => KeyCode::Function(12),
        Vk::Space => KeyCode::Char(' '),
        _ => KeyCode::Unknown,
    }
}
