mod terminal;

use eframe::egui;
use terminal::Terminal;

const WINDOW_TITLE: &str = "ittsy";
const INITIAL_WIDTH: f32 = 520.0;
const INITIAL_HEIGHT: f32 = 300.0;
const WINDOW_MARGIN: f32 = 16.0;

fn main() -> eframe::Result {
    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png"))
        .expect("embedded app icon must be valid");
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(WINDOW_TITLE)
            .with_inner_size([INITIAL_WIDTH, INITIAL_HEIGHT])
            .with_min_inner_size([320.0, 180.0])
            .with_resizable(true)
            .with_icon(icon)
            .with_always_on_top(),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };

    eframe::run_native(
        WINDOW_TITLE,
        options,
        Box::new(|cc| Ok(Box::new(Ittsy::new(cc)))),
    )
}

struct Ittsy {
    terminal: Result<Terminal, String>,
    corner: Corner,
    docked: bool,
    always_on_top: bool,
}

impl Ittsy {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        configure_style(&cc.egui_ctx);
        Self {
            terminal: Terminal::spawn(cc.egui_ctx.clone()),
            corner: Corner::BottomRight,
            docked: false,
            always_on_top: true,
        }
    }

    fn handle_window_shortcuts(&mut self, ctx: &egui::Context) {
        let shortcut = egui::Modifiers::COMMAND | egui::Modifiers::ALT;
        if ctx.input_mut(|input| input.consume_key(shortcut, egui::Key::T)) {
            self.always_on_top = !self.always_on_top;
            let level = if self.always_on_top {
                egui::WindowLevel::AlwaysOnTop
            } else {
                egui::WindowLevel::Normal
            };
            ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(level));
        }

        let direction = [
            (egui::Key::ArrowLeft, DockDirection::Left),
            (egui::Key::ArrowRight, DockDirection::Right),
            (egui::Key::ArrowUp, DockDirection::Top),
            (egui::Key::ArrowDown, DockDirection::Bottom),
        ]
        .into_iter()
        .find_map(|(key, direction)| {
            ctx.input_mut(|input| input.consume_key(shortcut, key))
                .then_some(direction)
        });

        if let Some(direction) = direction {
            self.corner = self.corner.with_direction(direction);
            self.dock(ctx);
        }
    }

    fn dock(&mut self, ctx: &egui::Context) {
        let geometry = ctx.input(|input| {
            let viewport = input.viewport();
            Some((viewport.monitor_size?, viewport.outer_rect?.size()))
        });
        let Some((monitor, window)) = geometry else {
            ctx.request_repaint();
            return;
        };

        let position = self.corner.position(monitor, window);
        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(position));
        self.docked = true;
    }
}

impl eframe::App for Ittsy {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.handle_window_shortcuts(ui.ctx());
        if !self.docked {
            self.dock(ui.ctx());
        }

        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(egui::Color32::from_rgb(15, 17, 21))
                    .inner_margin(egui::Margin::same(10)),
            )
            .show_inside(ui, |ui| match &mut self.terminal {
                Ok(terminal) => {
                    terminal.process_output();

                    let size = ui.available_size();
                    terminal.resize_for_points(size, ui);
                    let font_id = egui::TextStyle::Monospace.resolve(ui.style());
                    let row_height = ui.fonts_mut(|fonts| fonts.row_height(&font_id)).max(1.0);
                    terminal.handle_input(ui.ctx(), row_height);
                    let display = terminal.contents(font_id);

                    let output = ui.add(
                        egui::Label::new(display)
                            .selectable(true)
                            .wrap_mode(egui::TextWrapMode::Extend),
                    );

                    if output.clicked() {
                        output.request_focus();
                    }
                }
                Err(error) => {
                    ui.colored_label(
                        egui::Color32::from_rgb(255, 110, 110),
                        format!("ittsy could not start the shell:\n\n{error}"),
                    );
                }
            });
    }
}

fn configure_style(ctx: &egui::Context) {
    let mut style = (*ctx.global_style()).clone();
    style.visuals = egui::Visuals::dark();
    style.spacing.item_spacing = egui::vec2(0.0, 0.0);
    style
        .text_styles
        .insert(egui::TextStyle::Monospace, egui::FontId::monospace(13.0));
    ctx.set_global_style(style);
}

#[derive(Clone, Copy)]
enum Corner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl Corner {
    fn with_direction(self, direction: DockDirection) -> Self {
        match (self, direction) {
            (Self::TopLeft | Self::BottomLeft, DockDirection::Top) => Self::TopLeft,
            (Self::TopRight | Self::BottomRight, DockDirection::Top) => Self::TopRight,
            (Self::TopLeft | Self::BottomLeft, DockDirection::Bottom) => Self::BottomLeft,
            (Self::TopRight | Self::BottomRight, DockDirection::Bottom) => Self::BottomRight,
            (Self::TopLeft | Self::TopRight, DockDirection::Left) => Self::TopLeft,
            (Self::BottomLeft | Self::BottomRight, DockDirection::Left) => Self::BottomLeft,
            (Self::TopLeft | Self::TopRight, DockDirection::Right) => Self::TopRight,
            (Self::BottomLeft | Self::BottomRight, DockDirection::Right) => Self::BottomRight,
        }
    }

    fn position(self, monitor: egui::Vec2, window: egui::Vec2) -> egui::Pos2 {
        let left = WINDOW_MARGIN;
        let right = (monitor.x - window.x - WINDOW_MARGIN).max(WINDOW_MARGIN);
        let top = WINDOW_MARGIN;
        let bottom = (monitor.y - window.y - WINDOW_MARGIN).max(WINDOW_MARGIN);

        match self {
            Self::TopLeft => egui::pos2(left, top),
            Self::TopRight => egui::pos2(right, top),
            Self::BottomLeft => egui::pos2(left, bottom),
            Self::BottomRight => egui::pos2(right, bottom),
        }
    }
}

#[derive(Clone, Copy)]
enum DockDirection {
    Left,
    Right,
    Top,
    Bottom,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moves_between_all_docking_corners() {
        assert!(matches!(
            Corner::BottomRight.with_direction(DockDirection::Left),
            Corner::BottomLeft
        ));
        assert!(matches!(
            Corner::BottomLeft.with_direction(DockDirection::Top),
            Corner::TopLeft
        ));
    }

    #[test]
    fn calculates_corner_position_with_margin() {
        assert_eq!(
            Corner::BottomRight.position(egui::vec2(1440.0, 900.0), egui::vec2(520.0, 322.0)),
            egui::pos2(904.0, 562.0)
        );
    }
}
