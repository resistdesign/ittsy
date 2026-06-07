mod terminal;

use arboard::Clipboard;
use minifb::{Key, KeyRepeat, Window, WindowOptions};
use std::error::Error;
use terminal::Terminal;

const TITLE: &str = "ittsy";
const INITIAL_WIDTH: usize = 640;
const INITIAL_HEIGHT: usize = 360;
const BACKGROUND: u32 = 0x0f1115;
const FOREGROUND: u32 = 0xdce0e6;
const PADDING: usize = 8;
const CELL_WIDTH: usize = 8;
const CELL_HEIGHT: usize = 16;

fn main() -> Result<(), Box<dyn Error>> {
    let mut window = Window::new(
        TITLE,
        INITIAL_WIDTH,
        INITIAL_HEIGHT,
        WindowOptions {
            resize: true,
            ..WindowOptions::default()
        },
    )?;
    window.set_target_fps(15);

    let mut terminal = Terminal::spawn()?;
    let mut clipboard = Clipboard::new().ok();
    let mut buffer = vec![BACKGROUND; INITIAL_WIDTH * INITIAL_HEIGHT];
    let mut size = (INITIAL_WIDTH, INITIAL_HEIGHT);
    let mut dirty = true;

    while window.is_open() && !command_quit(&window) {
        for key in window.get_keys_pressed(KeyRepeat::Yes) {
            if command_paste(&window, key) {
                if let Some(text) = clipboard.as_mut().and_then(|value| value.get_text().ok()) {
                    terminal.paste(&text);
                }
            } else if let Some(bytes) = encode_key(key, modifiers(&window)) {
                terminal.write(&bytes);
            }
        }

        dirty |= terminal.process_output();

        let new_size = window.get_size();
        if new_size != size {
            size = new_size;
            buffer.resize(size.0.saturating_mul(size.1), BACKGROUND);
            dirty = true;
        }

        let (rows, cols) = grid_size(size.0, size.1);
        terminal.resize(rows, cols, size.0 as u16, size.1 as u16);
        if dirty {
            render(&mut buffer, size.0, size.1, terminal.screen());
            window.update_with_buffer(&buffer, size.0, size.1)?;
            dirty = false;
        } else {
            window.update();
        }
    }

    Ok(())
}

#[derive(Clone, Copy, Default)]
struct Modifiers {
    shift: bool,
    ctrl: bool,
    command: bool,
}

fn modifiers(window: &Window) -> Modifiers {
    Modifiers {
        shift: window.is_key_down(Key::LeftShift) || window.is_key_down(Key::RightShift),
        ctrl: window.is_key_down(Key::LeftCtrl) || window.is_key_down(Key::RightCtrl),
        command: window.is_key_down(Key::LeftSuper) || window.is_key_down(Key::RightSuper),
    }
}

fn command_quit(window: &Window) -> bool {
    let command = window.is_key_down(Key::LeftSuper) || window.is_key_down(Key::RightSuper);
    command && window.is_key_down(Key::Q)
}

fn command_paste(window: &Window, key: Key) -> bool {
    let command = window.is_key_down(Key::LeftSuper) || window.is_key_down(Key::RightSuper);
    command && key == Key::V
}

fn grid_size(width: usize, height: usize) -> (u16, u16) {
    let usable_width = width.saturating_sub(PADDING * 2);
    let usable_height = height.saturating_sub(PADDING * 2);
    (
        (usable_height / CELL_HEIGHT).max(2).min(u16::MAX as usize) as u16,
        (usable_width / CELL_WIDTH).max(2).min(u16::MAX as usize) as u16,
    )
}

fn render(buffer: &mut [u32], width: usize, height: usize, screen: &vt100::Screen) {
    buffer.fill(BACKGROUND);
    let (rows, cols) = screen.size();
    let (cursor_row, cursor_col) = screen.cursor_position();
    let cursor_visible = !screen.hide_cursor() && screen.scrollback() == 0;

    for row in 0..rows {
        for col in 0..cols {
            let x = PADDING + usize::from(col) * CELL_WIDTH;
            let y = PADDING + usize::from(row) * CELL_HEIGHT;
            if x + CELL_WIDTH > width || y + CELL_HEIGHT > height {
                continue;
            }

            let cursor = cursor_visible && row == cursor_row && col == cursor_col;
            let (foreground, background) = if cursor {
                (BACKGROUND, FOREGROUND)
            } else {
                (FOREGROUND, BACKGROUND)
            };
            fill_cell(buffer, width, x, y, background);

            if let Some(cell) = screen.cell(row, col)
                && !cell.is_wide_continuation()
            {
                for character in cell.contents().chars() {
                    draw_character(buffer, width, x, y, character, foreground);
                }
            }
        }
    }
}

fn fill_cell(buffer: &mut [u32], width: usize, x: usize, y: usize, color: u32) {
    for pixel_y in y..y + CELL_HEIGHT {
        let start = pixel_y * width + x;
        buffer[start..start + CELL_WIDTH].fill(color);
    }
}

fn draw_character(
    buffer: &mut [u32],
    width: usize,
    x: usize,
    y: usize,
    character: char,
    color: u32,
) {
    use font8x8::{BASIC_FONTS, UnicodeFonts};

    let glyph = BASIC_FONTS
        .get(character)
        .or_else(|| BASIC_FONTS.get('?'))
        .unwrap_or([0; 8]);
    for (glyph_y, bits) in glyph.into_iter().enumerate() {
        for glyph_x in 0..8 {
            if bits & (1 << glyph_x) != 0 {
                let pixel_x = x + glyph_x;
                let pixel_y = y + glyph_y * 2;
                buffer[pixel_y * width + pixel_x] = color;
                buffer[(pixel_y + 1) * width + pixel_x] = color;
            }
        }
    }
}

fn encode_key(key: Key, modifiers: Modifiers) -> Option<Vec<u8>> {
    if modifiers.command {
        return None;
    }

    if modifiers.ctrl
        && let Some(letter) = letter(key)
    {
        return Some(vec![(letter as u8) & 0x1f]);
    }

    let sequence = match key {
        Key::Enter | Key::NumPadEnter => "\r",
        Key::Tab => "\t",
        Key::Backspace => "\x7f",
        Key::Escape => "\x1b",
        Key::Up => "\x1b[A",
        Key::Down => "\x1b[B",
        Key::Right => "\x1b[C",
        Key::Left => "\x1b[D",
        Key::Home => "\x1b[H",
        Key::End => "\x1b[F",
        Key::Insert => "\x1b[2~",
        Key::Delete => "\x1b[3~",
        Key::PageUp => "\x1b[5~",
        Key::PageDown => "\x1b[6~",
        _ => {
            return printable_key(key, modifiers.shift)
                .map(|character| character.to_string().into_bytes());
        }
    };
    Some(sequence.as_bytes().to_vec())
}

fn printable_key(key: Key, shift: bool) -> Option<char> {
    if let Some(letter) = letter(key) {
        return Some(if shift {
            letter.to_ascii_uppercase()
        } else {
            letter
        });
    }

    let value = match (key, shift) {
        (Key::Space, _) => ' ',
        (Key::Key0, false) => '0',
        (Key::Key1, false) => '1',
        (Key::Key2, false) => '2',
        (Key::Key3, false) => '3',
        (Key::Key4, false) => '4',
        (Key::Key5, false) => '5',
        (Key::Key6, false) => '6',
        (Key::Key7, false) => '7',
        (Key::Key8, false) => '8',
        (Key::Key9, false) => '9',
        (Key::Key0, true) => ')',
        (Key::Key1, true) => '!',
        (Key::Key2, true) => '@',
        (Key::Key3, true) => '#',
        (Key::Key4, true) => '$',
        (Key::Key5, true) => '%',
        (Key::Key6, true) => '^',
        (Key::Key7, true) => '&',
        (Key::Key8, true) => '*',
        (Key::Key9, true) => '(',
        (Key::Apostrophe, false) => '\'',
        (Key::Apostrophe, true) => '"',
        (Key::Backquote, false) => '`',
        (Key::Backquote, true) => '~',
        (Key::Backslash, false) => '\\',
        (Key::Backslash, true) => '|',
        (Key::Comma, false) => ',',
        (Key::Comma, true) => '<',
        (Key::Equal, false) => '=',
        (Key::Equal, true) => '+',
        (Key::LeftBracket, false) => '[',
        (Key::LeftBracket, true) => '{',
        (Key::Minus, false) => '-',
        (Key::Minus, true) => '_',
        (Key::Period, false) => '.',
        (Key::Period, true) => '>',
        (Key::RightBracket, false) => ']',
        (Key::RightBracket, true) => '}',
        (Key::Semicolon, false) => ';',
        (Key::Semicolon, true) => ':',
        (Key::Slash, false) => '/',
        (Key::Slash, true) => '?',
        _ => return None,
    };
    Some(value)
}

fn letter(key: Key) -> Option<char> {
    let letter = match key {
        Key::A => 'a',
        Key::B => 'b',
        Key::C => 'c',
        Key::D => 'd',
        Key::E => 'e',
        Key::F => 'f',
        Key::G => 'g',
        Key::H => 'h',
        Key::I => 'i',
        Key::J => 'j',
        Key::K => 'k',
        Key::L => 'l',
        Key::M => 'm',
        Key::N => 'n',
        Key::O => 'o',
        Key::P => 'p',
        Key::Q => 'q',
        Key::R => 'r',
        Key::S => 's',
        Key::T => 't',
        Key::U => 'u',
        Key::V => 'v',
        Key::W => 'w',
        Key::X => 'x',
        Key::Y => 'y',
        Key::Z => 'z',
        _ => return None,
    };
    Some(letter)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculates_grid_with_padding_and_minimums() {
        assert_eq!(grid_size(656, 400), (24, 80));
        assert_eq!(grid_size(1, 1), (2, 2));
    }

    #[test]
    fn encodes_navigation_and_control_keys() {
        assert_eq!(
            encode_key(Key::Up, Modifiers::default()),
            Some(b"\x1b[A".to_vec())
        );
        assert_eq!(
            encode_key(
                Key::C,
                Modifiers {
                    ctrl: true,
                    ..Modifiers::default()
                }
            ),
            Some(vec![3])
        );
    }

    #[test]
    fn encodes_shifted_us_keyboard_characters() {
        assert_eq!(
            encode_key(
                Key::Key1,
                Modifiers {
                    shift: true,
                    ..Modifiers::default()
                }
            ),
            Some(b"!".to_vec())
        );
        assert_eq!(
            encode_key(Key::A, Modifiers::default()),
            Some(b"a".to_vec())
        );
    }
}
