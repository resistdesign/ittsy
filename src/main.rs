mod terminal;

use eframe::egui;
use terminal::Terminal;

const WINDOW_TITLE: &str = "ittsy";
const INITIAL_WIDTH: f32 = 620.0;
const INITIAL_HEIGHT: f32 = 360.0;
const DOCKED_WIDTH: f32 = 380.0;
const DOCKED_HEIGHT: f32 = 220.0;
const DOCK_MARGIN: f32 = 12.0;
const ICON_SIZE: u32 = 64;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(WINDOW_TITLE)
            .with_inner_size([INITIAL_WIDTH, INITIAL_HEIGHT])
            .with_min_inner_size([320.0, 180.0])
            .with_icon(app_icon())
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
    always_on_top: bool,
}

impl Ittsy {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        configure_style(&cc.egui_ctx);
        Self {
            terminal: Terminal::spawn(cc.egui_ctx.clone()),
            always_on_top: false,
        }
    }
}

impl eframe::App for Ittsy {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(egui::Color32::from_rgb(15, 17, 21))
                    .inner_margin(egui::Margin::same(10)),
            )
            .show(ctx, |ui| {
                if toolbar(ui, &mut self.always_on_top) {
                    apply_window_level(ctx, self.always_on_top);
                }

                ui.add_space(8.0);

                match &mut self.terminal {
                    Ok(terminal) => {
                        terminal.process_output();

                        let size = ui.available_size();
                        let metrics = terminal.resize_for_points(size, ui);
                        terminal.handle_input(ui.ctx(), metrics.row_height);
                        let output = ui.add(
                            egui::Label::new(terminal.contents(ui))
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
                }
            });
    }
}

fn toolbar(ui: &mut egui::Ui, always_on_top: &mut bool) -> bool {
    let mut changed_window_level = false;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(4.0, 0.0);

        if ui
            .small_button("↖")
            .on_hover_text("Dock top left")
            .clicked()
        {
            dock(ui.ctx(), DockCorner::TopLeft);
        }
        if ui
            .small_button("↗")
            .on_hover_text("Dock top right")
            .clicked()
        {
            dock(ui.ctx(), DockCorner::TopRight);
        }
        if ui
            .small_button("↙")
            .on_hover_text("Dock bottom left")
            .clicked()
        {
            dock(ui.ctx(), DockCorner::BottomLeft);
        }
        if ui
            .small_button("↘")
            .on_hover_text("Dock bottom right")
            .clicked()
        {
            dock(ui.ctx(), DockCorner::BottomRight);
        }

        ui.separator();

        let pin_label = if *always_on_top {
            "📌 on top"
        } else {
            "📌"
        };
        if ui
            .selectable_label(*always_on_top, pin_label)
            .on_hover_text("Keep ittsy above other windows")
            .clicked()
        {
            *always_on_top = !*always_on_top;
            changed_window_level = true;
        }
    });
    changed_window_level
}

#[derive(Clone, Copy)]
enum DockCorner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

fn dock(ctx: &egui::Context, corner: DockCorner) {
    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
        DOCKED_WIDTH,
        DOCKED_HEIGHT,
    )));

    if let Some(position) = dock_position(ctx, corner) {
        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(position));
    }
}

fn dock_position(ctx: &egui::Context, corner: DockCorner) -> Option<egui::Pos2> {
    ctx.input(|input| {
        let viewport = input.viewport();
        let monitor = viewport.monitor_size?;
        let window_size = egui::vec2(DOCKED_WIDTH, DOCKED_HEIGHT);
        Some(match corner {
            DockCorner::TopLeft => egui::pos2(DOCK_MARGIN, DOCK_MARGIN),
            DockCorner::TopRight => egui::pos2(
                (monitor.x - window_size.x - DOCK_MARGIN).max(DOCK_MARGIN),
                DOCK_MARGIN,
            ),
            DockCorner::BottomLeft => egui::pos2(
                DOCK_MARGIN,
                (monitor.y - window_size.y - DOCK_MARGIN).max(DOCK_MARGIN),
            ),
            DockCorner::BottomRight => egui::pos2(
                (monitor.x - window_size.x - DOCK_MARGIN).max(DOCK_MARGIN),
                (monitor.y - window_size.y - DOCK_MARGIN).max(DOCK_MARGIN),
            ),
        })
    })
}

fn apply_window_level(ctx: &egui::Context, always_on_top: bool) {
    let level = if always_on_top {
        egui::WindowLevel::AlwaysOnTop
    } else {
        egui::WindowLevel::Normal
    };
    ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(level));
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

fn app_icon() -> egui::IconData {
    let mut rgba = vec![0; (ICON_SIZE * ICON_SIZE * 4) as usize];
    for y in 0..ICON_SIZE {
        for x in 0..ICON_SIZE {
            let mut color = [0, 0, 0, 0];
            if inside_rounded_rect(x as f32 + 0.5, y as f32 + 0.5, 12.0) {
                color = [0x0b, 0x0d, 0x10, 0xff];
            }
            if near_segment(
                x as f32 + 0.5,
                y as f32 + 0.5,
                (15.0, 19.0),
                (28.0, 32.0),
                3.0,
            ) || near_segment(
                x as f32 + 0.5,
                y as f32 + 0.5,
                (28.0, 32.0),
                (15.0, 45.0),
                3.0,
            ) {
                color = [0xc7, 0xff, 0x4a, 0xff];
            }
            if near_segment(
                x as f32 + 0.5,
                y as f32 + 0.5,
                (32.0, 45.0),
                (49.0, 45.0),
                3.0,
            ) {
                color = [0xf4, 0xf2, 0xea, 0xff];
            }

            let index = ((y * ICON_SIZE + x) * 4) as usize;
            rgba[index..index + 4].copy_from_slice(&color);
        }
    }

    egui::IconData {
        rgba,
        width: ICON_SIZE,
        height: ICON_SIZE,
    }
}

fn inside_rounded_rect(x: f32, y: f32, radius: f32) -> bool {
    let max = ICON_SIZE as f32;
    let inner_x = x.clamp(radius, max - radius);
    let inner_y = y.clamp(radius, max - radius);
    let dx = x - inner_x;
    let dy = y - inner_y;
    dx * dx + dy * dy <= radius * radius
}

fn near_segment(x: f32, y: f32, start: (f32, f32), end: (f32, f32), radius: f32) -> bool {
    let segment_x = end.0 - start.0;
    let segment_y = end.1 - start.1;
    let length_squared = segment_x * segment_x + segment_y * segment_y;
    let projection =
        (((x - start.0) * segment_x + (y - start.1) * segment_y) / length_squared).clamp(0.0, 1.0);
    let closest_x = start.0 + projection * segment_x;
    let closest_y = start.1 + projection * segment_y;
    let dx = x - closest_x;
    let dy = y - closest_y;
    dx * dx + dy * dy <= radius * radius
}
