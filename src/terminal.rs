use eframe::egui::{self, Event, Key, Modifiers, MouseWheelUnit};
use eframe::epaint::text::{LayoutJob, TextFormat};
use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};
use std::io::{Read, Write};
use std::sync::mpsc::{self, Receiver};
use std::thread;

const INITIAL_ROWS: u16 = 24;
const INITIAL_COLS: u16 = 80;
const SCROLLBACK_ROWS: usize = 2_000;
const DEFAULT_FG: egui::Color32 = egui::Color32::from_rgb(220, 224, 230);
const DEFAULT_BG: egui::Color32 = egui::Color32::TRANSPARENT;
const TERMINAL_BG: egui::Color32 = egui::Color32::from_rgb(15, 17, 21);

pub struct Terminal {
    parser: vt100::Parser,
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    output: Receiver<Vec<u8>>,
    rows: u16,
    cols: u16,
    scroll_fractional_rows: f32,
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
            scroll_fractional_rows: 0.0,
        })
    }

    pub fn process_output(&mut self) {
        let was_following_output = self.parser.screen().scrollback() == 0;
        let mut received_output = false;
        while let Ok(bytes) = self.output.try_recv() {
            self.parser.process(&bytes);
            received_output = true;
        }

        if received_output && was_following_output {
            self.parser.screen_mut().set_scrollback(0);
        }
    }

    pub fn contents(&self, ui: &egui::Ui) -> LayoutJob {
        render_screen(self.parser.screen(), ui)
    }

    pub fn handle_input(&mut self, ctx: &egui::Context, row_height: f32) {
        let events = ctx.input(|input| input.events.clone());
        for event in events {
            if self.handle_scroll(&event, row_height) {
                ctx.request_repaint();
            } else if let Event::Paste(text) = &event {
                self.paste(text);
            } else if let Some(bytes) = encode_event(&event) {
                self.write(&bytes);
            }
        }
    }

    pub fn resize_for_points(&mut self, available: egui::Vec2, ui: &egui::Ui) -> CellMetrics {
        let metrics = cell_metrics(ui);
        let (rows, cols) = grid_size(
            available.x,
            available.y,
            metrics.col_width,
            metrics.row_height,
        );

        if rows == self.rows && cols == self.cols {
            return metrics;
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

        metrics
    }

    fn handle_scroll(&mut self, event: &Event, row_height: f32) -> bool {
        let Event::MouseWheel {
            unit,
            delta,
            modifiers,
            ..
        } = event
        else {
            return false;
        };

        if modifiers.ctrl || delta.y == 0.0 {
            return false;
        }

        let row_delta = match unit {
            MouseWheelUnit::Point => delta.y / row_height.max(1.0),
            MouseWheelUnit::Line => delta.y,
            MouseWheelUnit::Page => delta.y * f32::from(self.rows),
        };
        self.scroll_fractional_rows += row_delta;
        let whole_rows = self.scroll_fractional_rows.trunc() as isize;
        self.scroll_fractional_rows = self.scroll_fractional_rows.fract();

        if whole_rows == 0 {
            return false;
        }

        let current = self.parser.screen().scrollback();
        let requested = if whole_rows.is_positive() {
            current.saturating_add(whole_rows as usize)
        } else {
            current.saturating_sub(whole_rows.unsigned_abs())
        };
        self.parser.screen_mut().set_scrollback(requested);
        self.parser.screen().scrollback() != current
    }

    fn write(&mut self, bytes: &[u8]) {
        self.parser.screen_mut().set_scrollback(0);
        self.scroll_fractional_rows = 0.0;
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

#[derive(Clone, Copy)]
pub struct CellMetrics {
    pub row_height: f32,
    col_width: f32,
}

fn cell_metrics(ui: &egui::Ui) -> CellMetrics {
    let font_id = egui::TextStyle::Monospace.resolve(ui.style());
    let row_height = ui.fonts_mut(|fonts| fonts.row_height(&font_id)).max(1.0);
    let col_width = ui
        .fonts_mut(|fonts| fonts.glyph_width(&font_id, 'W'))
        .max(1.0);
    CellMetrics {
        row_height,
        col_width,
    }
}

fn render_screen(screen: &vt100::Screen, ui: &egui::Ui) -> LayoutJob {
    let (rows, cols) = screen.size();
    let (cursor_row, cursor_col) = screen.cursor_position();
    let cursor_visible = !screen.hide_cursor() && screen.scrollback() == 0;
    let mut last_row = if cursor_visible { cursor_row } else { 0 };

    for row in 0..rows {
        if (0..cols).any(|col| screen.cell(row, col).is_some_and(vt100::Cell::has_contents)) {
            last_row = last_row.max(row);
        }
    }

    let font_id = egui::TextStyle::Monospace.resolve(ui.style());
    let mut job = LayoutJob::default();
    job.wrap.max_width = f32::INFINITY;

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
                append_text(&mut job, "█", cursor_format(font_id.clone()));
            } else if let Some(cell) = screen.cell(row, col)
                && !cell.is_wide_continuation()
            {
                let contents = if cell.has_contents() {
                    cell.contents()
                } else {
                    " "
                };
                append_text(&mut job, contents, cell_format(cell, font_id.clone()));
            }
        }

        if row < last_row {
            append_text(&mut job, "\n", default_format(font_id.clone()));
        }
    }
    job
}

fn append_text(job: &mut LayoutJob, text: &str, format: TextFormat) {
    job.append(text, 0.0, format);
}

fn default_format(font_id: egui::FontId) -> TextFormat {
    TextFormat {
        font_id,
        color: DEFAULT_FG,
        ..Default::default()
    }
}

fn cursor_format(font_id: egui::FontId) -> TextFormat {
    TextFormat {
        font_id,
        color: TERMINAL_BG,
        background: DEFAULT_FG,
        ..Default::default()
    }
}

fn cell_format(cell: &vt100::Cell, font_id: egui::FontId) -> TextFormat {
    let mut foreground = terminal_color(cell.fgcolor(), DEFAULT_FG);
    let mut background = terminal_color(cell.bgcolor(), DEFAULT_BG);

    if cell.bold() && matches!(cell.fgcolor(), vt100::Color::Idx(0..=7)) {
        foreground = terminal_color(
            vt100::Color::Idx(match cell.fgcolor() {
                vt100::Color::Idx(index) => index + 8,
                _ => unreachable!(),
            }),
            DEFAULT_FG,
        );
    }

    if cell.dim() {
        foreground = foreground.gamma_multiply(0.65);
    }

    if cell.inverse() {
        if background == DEFAULT_BG {
            background = foreground;
            foreground = TERMINAL_BG;
        } else {
            std::mem::swap(&mut foreground, &mut background);
        }
    }

    TextFormat {
        font_id,
        color: foreground,
        background,
        italics: cell.italic(),
        underline: if cell.underline() {
            egui::Stroke::new(1.0, foreground)
        } else {
            egui::Stroke::NONE
        },
        ..Default::default()
    }
}

fn terminal_color(color: vt100::Color, default: egui::Color32) -> egui::Color32 {
    match color {
        vt100::Color::Default => default,
        vt100::Color::Rgb(red, green, blue) => egui::Color32::from_rgb(red, green, blue),
        vt100::Color::Idx(index) => indexed_color(index),
    }
}

fn indexed_color(index: u8) -> egui::Color32 {
    const ANSI_16: [(u8, u8, u8); 16] = [
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

    if let Some(&(red, green, blue)) = ANSI_16.get(usize::from(index)) {
        return egui::Color32::from_rgb(red, green, blue);
    }

    if (16..=231).contains(&index) {
        let cube = index - 16;
        let red = cube / 36;
        let green = (cube % 36) / 6;
        let blue = cube % 6;
        return egui::Color32::from_rgb(color_cube(red), color_cube(green), color_cube(blue));
    }

    let gray = 8 + (index.saturating_sub(232) * 10);
    egui::Color32::from_rgb(gray, gray, gray)
}

fn color_cube(value: u8) -> u8 {
    if value == 0 { 0 } else { 55 + value * 40 }
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
    fn maps_indexed_color_cube_and_grayscale() {
        assert_eq!(indexed_color(9), egui::Color32::from_rgb(241, 76, 76));
        assert_eq!(indexed_color(16), egui::Color32::from_rgb(0, 0, 0));
        assert_eq!(indexed_color(231), egui::Color32::from_rgb(255, 255, 255));
        assert_eq!(indexed_color(232), egui::Color32::from_rgb(8, 8, 8));
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
    fn terminal_keeps_scrollback_available() {
        let mut parser = vt100::Parser::new(2, 10, 10);
        parser.process(b"one\ntwo\nthree");
        parser.screen_mut().set_scrollback(1);
        assert_eq!(parser.screen().scrollback(), 1);
        assert!(parser.screen().rows(0, 10).any(|row| row.contains("one")));
    }
}
