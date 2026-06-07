mod terminal;

use eframe::egui;
use terminal::Terminal;

const WINDOW_TITLE: &str = "ittsy";
const INITIAL_WIDTH: f32 = 620.0;
const INITIAL_HEIGHT: f32 = 360.0;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(WINDOW_TITLE)
            .with_inner_size([INITIAL_WIDTH, INITIAL_HEIGHT])
            .with_min_inner_size([320.0, 180.0])
            .with_resizable(true),
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
    display: String,
}

impl Ittsy {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        configure_style(&cc.egui_ctx);
        Self {
            terminal: Terminal::spawn(cc.egui_ctx.clone()),
            display: String::new(),
        }
    }
}

impl eframe::App for Ittsy {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(egui::Color32::from_rgb(15, 17, 21))
                    .inner_margin(egui::Margin::same(10)),
            )
            .show_inside(ui, |ui| match &mut self.terminal {
                Ok(terminal) => {
                    terminal.process_output();
                    terminal.handle_input(ui.ctx());

                    let size = ui.available_size();
                    terminal.resize_for_points(size, ui);
                    self.display = terminal.contents();

                    let output = ui.add(
                        egui::Label::new(
                            egui::RichText::new(&self.display)
                                .monospace()
                                .color(egui::Color32::from_rgb(220, 224, 230)),
                        )
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
                        format!("ittsy could not start bash:\n\n{error}"),
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
