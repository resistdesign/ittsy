use eframe::egui::{self, Event, Key, Modifiers};
use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};
use std::io::{Read, Write};
use std::sync::mpsc::{self, Receiver};
use std::thread;

const INITIAL_ROWS: u16 = 24;
const INITIAL_COLS: u16 = 80;
const SCROLLBACK_ROWS: usize = 2_000;

pub struct Terminal {
    parser: vt100::Parser,
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    output: Receiver<Vec<u8>>,
    rows: u16,
    cols: u16,
}

impl Terminal {
    pub fn spawn(repaint: egui::Context) -> Result<Self, String> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: INITIAL_ROWS,
                cols: INITIAL_COLS,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|error| error.to_string())?;

        let mut command = CommandBuilder::new("/bin/bash");
        command.arg("--login");
        command.env("TERM", "xterm-256color");
        command.env("COLORTERM", "truecolor");
        command.env("TERM_PROGRAM", "ittsy");

        pair.slave
            .spawn_command(command)
            .map_err(|error| error.to_string())?;
        drop(pair.slave);

        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|error| error.to_string())?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|error| error.to_string())?;
        let (sender, output) = mpsc::channel();

        thread::Builder::new()
            .name("ittsy-pty-reader".into())
            .spawn(move || {
                let mut buffer = [0_u8; 8 * 1024];
                while let Ok(read) = reader.read(&mut buffer) {
                    if read == 0 || sender.send(buffer[..read].to_vec()).is_err() {
                        break;
                    }
                    repaint.request_repaint();
                }
            })
            .map_err(|error| error.to_string())?;

        Ok(Self {
            parser: vt100::Parser::new(INITIAL_ROWS, INITIAL_COLS, SCROLLBACK_ROWS),
            master: pair.master,
            writer,
            output,
            rows: INITIAL_ROWS,
            cols: INITIAL_COLS,
        })
    }

    pub fn process_output(&mut self) {
        while let Ok(bytes) = self.output.try_recv() {
            self.parser.process(&bytes);
        }
    }

    pub fn contents(&self) -> String {
        render_screen(self.parser.screen())
    }

    pub fn handle_input(&mut self, ctx: &egui::Context) {
        let events = ctx.input(|input| input.events.clone());
        for event in events {
            if let Event::Paste(text) = &event {
                self.paste(text);
            } else if let Some(bytes) = encode_event(&event) {
                self.write(&bytes);
            }
        }
    }

    pub fn resize_for_points(&mut self, available: egui::Vec2, ui: &egui::Ui) {
        let font_id = egui::TextStyle::Monospace.resolve(ui.style());
        let row_height = ui.fonts_mut(|fonts| fonts.row_height(&font_id)).max(1.0);
        let col_width = ui
            .fonts_mut(|fonts| fonts.glyph_width(&font_id, 'W'))
            .max(1.0);
        let (rows, cols) = grid_size(available.x, available.y, col_width, row_height);

        if rows == self.rows && cols == self.cols {
            return;
        }

        if self
            .master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: available.x.max(0.0) as u16,
                pixel_height: available.y.max(0.0) as u16,
            })
            .is_ok()
        {
            self.parser.screen_mut().set_size(rows, cols);
            self.rows = rows;
            self.cols = cols;
        }
    }

    fn write(&mut self, bytes: &[u8]) {
        let _ = self.writer.write_all(bytes);
        let _ = self.writer.flush();
    }

    fn paste(&mut self, text: &str) {
        if self.parser.screen().bracketed_paste() {
            self.write(b"\x1b[200~");
            self.write(text.as_bytes());
            self.write(b"\x1b[201~");
        } else {
            self.write(text.as_bytes());
        }
    }
}

fn render_screen(screen: &vt100::Screen) -> String {
    let (rows, cols) = screen.size();
    let (cursor_row, cursor_col) = screen.cursor_position();
    let cursor_visible = !screen.hide_cursor() && screen.scrollback() == 0;
    let mut last_row = if cursor_visible { cursor_row } else { 0 };

    for row in 0..rows {
        if (0..cols).any(|col| screen.cell(row, col).is_some_and(vt100::Cell::has_contents)) {
            last_row = last_row.max(row);
        }
    }

    let mut output = String::new();
    for row in 0..=last_row {
        let mut last_col = if cursor_visible && row == cursor_row {
            cursor_col
        } else {
            0
        };
        for col in 0..cols {
            if screen.cell(row, col).is_some_and(vt100::Cell::has_contents) {
                last_col = last_col.max(col);
            }
        }

        for col in 0..=last_col {
            if cursor_visible && row == cursor_row && col == cursor_col {
                output.push('█');
            } else if let Some(cell) = screen.cell(row, col)
                && !cell.is_wide_continuation()
            {
                if cell.has_contents() {
                    output.push_str(cell.contents());
                } else {
                    output.push(' ');
                }
            }
        }

        if row < last_row {
            output.push('\n');
        }
    }
    output
}

fn grid_size(width: f32, height: f32, col_width: f32, row_height: f32) -> (u16, u16) {
    let rows = (height / row_height).floor().clamp(2.0, u16::MAX as f32) as u16;
    let cols = (width / col_width).floor().clamp(2.0, u16::MAX as f32) as u16;
    (rows, cols)
}

fn encode_event(event: &Event) -> Option<Vec<u8>> {
    match event {
        Event::Text(text) => Some(text.as_bytes().to_vec()),
        Event::Key {
            key,
            pressed: true,
            modifiers,
            ..
        } => encode_key(*key, *modifiers),
        _ => None,
    }
}

fn encode_key(key: Key, modifiers: Modifiers) -> Option<Vec<u8>> {
    if modifiers.command {
        return None;
    }

    if modifiers.ctrl
        && let Some(byte) = control_byte(key)
    {
        return Some(vec![byte]);
    }

    let sequence = match key {
        Key::Enter => "\r",
        Key::Tab => "\t",
        Key::Backspace => "\x7f",
        Key::Escape => "\x1b",
        Key::ArrowUp => "\x1b[A",
        Key::ArrowDown => "\x1b[B",
        Key::ArrowRight => "\x1b[C",
        Key::ArrowLeft => "\x1b[D",
        Key::Home => "\x1b[H",
        Key::End => "\x1b[F",
        Key::Insert => "\x1b[2~",
        Key::Delete => "\x1b[3~",
        Key::PageUp => "\x1b[5~",
        Key::PageDown => "\x1b[6~",
        _ => return None,
    };
    Some(sequence.as_bytes().to_vec())
}

fn control_byte(key: Key) -> Option<u8> {
    let letter = match key {
        Key::A => b'a',
        Key::B => b'b',
        Key::C => b'c',
        Key::D => b'd',
        Key::E => b'e',
        Key::F => b'f',
        Key::G => b'g',
        Key::H => b'h',
        Key::I => b'i',
        Key::J => b'j',
        Key::K => b'k',
        Key::L => b'l',
        Key::M => b'm',
        Key::N => b'n',
        Key::O => b'o',
        Key::P => b'p',
        Key::Q => b'q',
        Key::R => b'r',
        Key::S => b's',
        Key::T => b't',
        Key::U => b'u',
        Key::V => b'v',
        Key::W => b'w',
        Key::X => b'x',
        Key::Y => b'y',
        Key::Z => b'z',
        _ => return None,
    };
    Some(letter & 0x1f)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculates_terminal_grid_with_minimums() {
        assert_eq!(grid_size(640.0, 360.0, 8.0, 15.0), (24, 80));
        assert_eq!(grid_size(1.0, 1.0, 8.0, 15.0), (2, 2));
    }

    #[test]
    fn encodes_navigation_and_control_keys() {
        assert_eq!(
            encode_key(Key::ArrowUp, Modifiers::NONE),
            Some(b"\x1b[A".to_vec())
        );
        assert_eq!(encode_key(Key::C, Modifiers::CTRL), Some(vec![3]));
    }

    #[test]
    fn leaves_command_shortcuts_to_macos() {
        assert_eq!(encode_key(Key::C, Modifiers::COMMAND), None);
        assert_eq!(encode_key(Key::V, Modifiers::COMMAND), None);
    }

    #[test]
    fn renders_cursor_after_terminal_contents() {
        let mut parser = vt100::Parser::new(2, 10, 0);
        parser.process(b"\x1b[?25hhi");
        assert_eq!(
            (
                parser.screen().hide_cursor(),
                parser.screen().scrollback(),
                parser.screen().cursor_position()
            ),
            (false, 0, (0, 2))
        );
        assert_eq!(render_screen(parser.screen()), "hi█");
    }
}
