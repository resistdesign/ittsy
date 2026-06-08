use eframe::egui::{
    self, Color32, Event, FontId, Key, Modifiers, MouseWheelUnit, Stroke, TextFormat,
    text::LayoutJob,
};
use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};
use std::ffi::OsString;
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

        let shell = shell_command();
        let mut command = CommandBuilder::new(shell.program);
        for arg in shell.args {
            command.arg(arg);
        }
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

    pub fn contents(&self, font_id: FontId) -> LayoutJob {
        render_screen(self.parser.screen(), font_id)
    }

    pub fn handle_input(&mut self, ctx: &egui::Context, row_height: f32) {
        let (events, modifiers) = ctx.input(|input| (input.events.clone(), input.modifiers));
        let mut consumed_clipboard_control = false;
        for event in events {
            if let Some(byte) = clipboard_control_byte(&event, modifiers) {
                self.scroll_to_bottom();
                self.write(&[byte]);
                consumed_clipboard_control = true;
            } else if let Event::MouseWheel { unit, delta, .. } = event {
                let rows = scroll_rows(unit, delta.y, row_height, self.rows);
                self.scroll(rows);
            } else if let Event::Key {
                key: Key::PageUp,
                pressed: true,
                modifiers: Modifiers { shift: true, .. },
                ..
            } = event
            {
                self.scroll(self.rows.saturating_sub(1) as isize);
            } else if let Event::Key {
                key: Key::PageDown,
                pressed: true,
                modifiers: Modifiers { shift: true, .. },
                ..
            } = event
            {
                self.scroll(-(self.rows.saturating_sub(1) as isize));
            } else if let Event::Paste(text) = &event {
                self.scroll_to_bottom();
                self.paste(text);
            } else if let Some(bytes) = encode_event(&event) {
                self.scroll_to_bottom();
                self.write(&bytes);
            }
        }

        if consumed_clipboard_control {
            ctx.input_mut(|input| {
                input
                    .events
                    .retain(|event| clipboard_control_byte(event, modifiers).is_none());
            });
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

    fn scroll(&mut self, rows: isize) {
        let current = self.parser.screen().scrollback();
        let next = current.saturating_add_signed(rows);
        self.parser.screen_mut().set_scrollback(next);
    }

    fn scroll_to_bottom(&mut self) {
        self.parser.screen_mut().set_scrollback(0);
    }
}

fn render_screen(screen: &vt100::Screen, font_id: FontId) -> LayoutJob {
    let (rows, cols) = screen.size();
    let (cursor_row, cursor_col) = screen.cursor_position();
    let cursor_visible = !screen.hide_cursor() && screen.scrollback() == 0;
    let mut last_row = if cursor_visible { cursor_row } else { 0 };

    for row in 0..rows {
        if (0..cols).any(|col| screen.cell(row, col).is_some_and(vt100::Cell::has_contents)) {
            last_row = last_row.max(row);
        }
    }

    let mut output = LayoutJob::default();
    output.wrap.max_width = f32::INFINITY;
    for row in 0..=last_row {
        let mut last_col = if cursor_visible && row == cursor_row {
            cursor_col
        } else {
            0
        };
        for col in 0..cols {
            if screen.cell(row, col).is_some_and(cell_is_visible) {
                last_col = last_col.max(col);
            }
        }

        for col in 0..=last_col {
            if cursor_visible && row == cursor_row && col == cursor_col {
                output.append(
                    " ",
                    0.0,
                    TextFormat {
                        background: DEFAULT_FOREGROUND,
                        ..terminal_text_format(font_id.clone(), None)
                    },
                );
            } else if let Some(cell) = screen.cell(row, col)
                && !cell.is_wide_continuation()
            {
                let contents = if cell.has_contents() {
                    cell.contents()
                } else {
                    " "
                };
                output.append(
                    contents,
                    0.0,
                    terminal_text_format(font_id.clone(), Some(cell)),
                );
            }
        }

        if row < last_row {
            output.append("\n", 0.0, terminal_text_format(font_id.clone(), None));
        }
    }
    output
}

const DEFAULT_FOREGROUND: Color32 = Color32::from_rgb(220, 224, 230);
const DEFAULT_BACKGROUND: Color32 = Color32::from_rgb(15, 17, 21);

fn cell_is_visible(cell: &vt100::Cell) -> bool {
    cell.has_contents() || cell.bgcolor() != vt100::Color::Default
}

fn terminal_text_format(font_id: FontId, cell: Option<&vt100::Cell>) -> TextFormat {
    let Some(cell) = cell else {
        return TextFormat {
            font_id,
            color: DEFAULT_FOREGROUND,
            ..Default::default()
        };
    };

    let mut foreground = terminal_color(cell.fgcolor(), DEFAULT_FOREGROUND, cell.bold());
    let mut background = terminal_color(cell.bgcolor(), DEFAULT_BACKGROUND, false);
    if cell.inverse() {
        std::mem::swap(&mut foreground, &mut background);
    }
    if cell.dim() {
        foreground = foreground.gamma_multiply(0.65);
    }

    TextFormat {
        font_id,
        color: foreground,
        background: if cell.bgcolor() == vt100::Color::Default && !cell.inverse() {
            Color32::TRANSPARENT
        } else {
            background
        },
        italics: cell.italic(),
        underline: if cell.underline() {
            Stroke::new(1.0, foreground)
        } else {
            Stroke::NONE
        },
        ..Default::default()
    }
}

fn terminal_color(color: vt100::Color, default: Color32, bold: bool) -> Color32 {
    match color {
        vt100::Color::Default => default,
        vt100::Color::Rgb(red, green, blue) => Color32::from_rgb(red, green, blue),
        vt100::Color::Idx(index) => {
            indexed_color(if bold && index < 8 { index + 8 } else { index })
        }
    }
}

fn indexed_color(index: u8) -> Color32 {
    const ANSI: [(u8, u8, u8); 16] = [
        (0, 0, 0),
        (205, 49, 49),
        (13, 188, 121),
        (229, 229, 16),
        (36, 114, 200),
        (188, 63, 188),
        (17, 168, 205),
        (229, 229, 229),
        (102, 102, 102),
        (241, 76, 76),
        (35, 209, 139),
        (245, 245, 67),
        (59, 142, 234),
        (214, 112, 214),
        (41, 184, 219),
        (255, 255, 255),
    ];

    if let Some(&(red, green, blue)) = ANSI.get(index as usize) {
        return Color32::from_rgb(red, green, blue);
    }
    if index < 232 {
        const LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];
        let cube = index - 16;
        return Color32::from_rgb(
            LEVELS[(cube / 36) as usize],
            LEVELS[((cube % 36) / 6) as usize],
            LEVELS[(cube % 6) as usize],
        );
    }

    let gray = 8 + (index - 232) * 10;
    Color32::from_gray(gray)
}

fn scroll_rows(unit: MouseWheelUnit, delta: f32, row_height: f32, rows: u16) -> isize {
    let amount = match unit {
        MouseWheelUnit::Point => delta / row_height.max(1.0),
        MouseWheelUnit::Line => delta,
        MouseWheelUnit::Page => delta * rows as f32,
    };
    amount.round() as isize
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
    if is_application_shortcut(modifiers) {
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

#[cfg(target_os = "macos")]
fn clipboard_control_byte(_event: &Event, _modifiers: Modifiers) -> Option<u8> {
    None
}

#[cfg(not(target_os = "macos"))]
fn clipboard_control_byte(event: &Event, modifiers: Modifiers) -> Option<u8> {
    if modifiers.shift {
        return None;
    }

    match event {
        Event::Copy => Some(3),
        Event::Paste(_) => Some(22),
        Event::Cut => Some(24),
        _ => None,
    }
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

struct ShellCommand {
    program: OsString,
    args: Vec<OsString>,
}

#[cfg(unix)]
fn shell_command() -> ShellCommand {
    let program = std::env::var_os("SHELL")
        .filter(|shell| !shell.is_empty())
        .unwrap_or_else(|| {
            if std::path::Path::new("/bin/bash").is_file() {
                OsString::from("/bin/bash")
            } else {
                OsString::from("/bin/sh")
            }
        });

    ShellCommand {
        program,
        args: vec![OsString::from("-l")],
    }
}

#[cfg(windows)]
fn shell_command() -> ShellCommand {
    ShellCommand {
        program: OsString::from("powershell.exe"),
        args: vec![OsString::from("-NoLogo")],
    }
}

#[cfg(target_os = "macos")]
fn is_application_shortcut(modifiers: Modifiers) -> bool {
    modifiers.command
}

#[cfg(not(target_os = "macos"))]
fn is_application_shortcut(_modifiers: Modifiers) -> bool {
    false
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
    #[cfg(target_os = "macos")]
    fn leaves_command_shortcuts_to_the_application_on_macos() {
        assert_eq!(encode_key(Key::C, Modifiers::COMMAND), None);
        assert_eq!(encode_key(Key::V, Modifiers::COMMAND), None);
    }

    #[test]
    #[cfg(not(target_os = "macos"))]
    fn preserves_control_keys_off_macos() {
        assert_eq!(encode_key(Key::C, Modifiers::COMMAND), Some(vec![3]));
        assert_eq!(encode_key(Key::V, Modifiers::COMMAND), Some(vec![22]));
        assert_eq!(
            clipboard_control_byte(&Event::Copy, Modifiers::CTRL),
            Some(3)
        );
        assert_eq!(
            clipboard_control_byte(&Event::Copy, Modifiers::CTRL | Modifiers::SHIFT),
            None
        );
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
        assert_eq!(
            render_screen(parser.screen(), FontId::monospace(13.0)).text,
            "hi "
        );
    }

    #[test]
    fn maps_ansi_and_truecolor_cells() {
        let mut parser = vt100::Parser::new(2, 10, 0);
        parser.process(b"\x1b[31mR\x1b[38;2;1;2;3mT");
        let output = render_screen(parser.screen(), FontId::monospace(13.0));

        assert_eq!(output.text, "RT ");
        assert_eq!(
            output.sections[0].format.color,
            Color32::from_rgb(205, 49, 49)
        );
        assert_eq!(output.sections[1].format.color, Color32::from_rgb(1, 2, 3));
    }

    #[test]
    fn converts_wheel_units_to_scrollback_rows() {
        assert_eq!(scroll_rows(MouseWheelUnit::Line, 3.0, 15.0, 24), 3);
        assert_eq!(scroll_rows(MouseWheelUnit::Point, -30.0, 15.0, 24), -2);
        assert_eq!(scroll_rows(MouseWheelUnit::Page, 1.0, 15.0, 24), 24);
    }
}
