use eframe::egui;
use super::icons::Icons;

pub enum WindowAction {
    ConnectSsh(String),
    OpenSftp(String),
    OpenFile(String),
}

/// Generic trait for any application running inside a floating window
pub trait WindowApp {
    fn title(&self) -> String;
    fn render(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        config: &mut crate::ui::settings::AppConfig,
    ) -> Option<WindowAction>;

    fn min_size(&self) -> [f32; 2] {
        [200.0, 150.0]
    }

    fn default_size(&self) -> [f32; 2] {
        [700.0, 480.0]
    }

    fn window_type(&self) -> &'static str {
        "unknown"
    }

    fn save_state(&self) -> Option<serde_json::Value> {
        None
    }
}

pub struct FloatingWindow {
    pub id: String,
    pub is_open: bool,
    pub app: Box<dyn WindowApp>,
    pub custom_title: Option<String>,
    pub is_editing_title: bool,
    pub temp_title: String,
}

impl FloatingWindow {
    pub fn new(id: impl Into<String>, app: Box<dyn WindowApp>) -> Self {
        Self {
            id: id.into(),
            is_open: true,
            app,
            custom_title: None,
            is_editing_title: false,
            temp_title: String::new(),
        }
    }

    pub fn render(
        &mut self,
        ctx: &egui::Context,
        config: &mut crate::ui::settings::AppConfig,
    ) -> Option<WindowAction> {
        let mut is_open = self.is_open;
        let mut action = None;

        let display_title = self.custom_title.clone().unwrap_or_else(|| self.app.title());
        let min_s = self.app.min_size();
        let def_s = self.app.default_size();

        let is_dark = ctx.style().visuals.dark_mode;
        let panel_fill = ctx.style().visuals.panel_fill;
        let window_fill = ctx.style().visuals.window_fill();

        // Use the actual theme colors for consistency
        let title_bar_bg = if is_dark {
            // Slightly lighter than window background
            egui::Color32::from_rgb(
                window_fill.r().saturating_add(8),
                window_fill.g().saturating_add(8),
                window_fill.b().saturating_add(8),
            )
        } else {
            // Slightly darker than window background
            egui::Color32::from_rgb(
                window_fill.r().saturating_sub(8),
                window_fill.g().saturating_sub(8),
                window_fill.b().saturating_sub(8),
            )
        };

        let border_color = if is_dark {
            egui::Color32::from_gray(50)
        } else {
            egui::Color32::from_gray(195)
        };

        let frame = egui::Frame::default()
            .fill(window_fill)
            .rounding(10.0)
            .inner_margin(0.0)
            .stroke(egui::Stroke::new(1.0, border_color))
            .shadow(egui::epaint::Shadow {
                offset: [0.0, 4.0].into(),
                blur: 20.0,
                spread: 0.0,
                color: egui::Color32::from_black_alpha(if is_dark { 140 } else { 50 }),
            });

        egui::Window::new(&self.id)
            .id(egui::Id::new(&self.id))
            .title_bar(false)
            .frame(frame)
            .resizable(true)
            .min_size(min_s)
            .default_size(def_s)
            .show(ctx, |ui| {
                // === Title Bar ===
                let title_frame = egui::Frame::default()
                    .fill(title_bar_bg)
                    .inner_margin(egui::Margin {
                        left: 12.0,
                        right: 8.0,
                        top: 6.0,
                        bottom: 6.0,
                    })
                    .rounding(egui::Rounding {
                        nw: 10.0,
                        ne: 10.0,
                        sw: 0.0,
                        se: 0.0,
                    });

                title_frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if self.is_editing_title {
                            let response = ui.add(
                                egui::TextEdit::singleline(&mut self.temp_title)
                                    .desired_width(180.0)
                                    .font(egui::FontId::proportional(12.0)),
                            );
                            if response.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                self.is_editing_title = false;
                                if !self.temp_title.is_empty() {
                                    self.custom_title = Some(self.temp_title.clone());
                                } else {
                                    self.custom_title = None;
                                }
                            }
                            response.request_focus();
                        } else {
                            let title_color = if is_dark {
                                egui::Color32::from_gray(180)
                            } else {
                                egui::Color32::from_gray(70)
                            };
                            let title_response = ui.add(
                                egui::Label::new(
                                    egui::RichText::new(&display_title)
                                        .size(12.0)
                                        .color(title_color),
                                )
                                .sense(egui::Sense::click()),
                            );
                            if title_response.double_clicked() {
                                self.is_editing_title = true;
                                self.temp_title = display_title.clone();
                            }
                        }

                        // Close button on the right
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let close_color = if is_dark {
                                egui::Color32::from_gray(100)
                            } else {
                                egui::Color32::from_gray(150)
                            };
                            let close_btn = ui.add(
                                egui::Button::new(
                                    egui::RichText::new(Icons::CLOSE).size(11.0).color(close_color),
                                )
                                .fill(egui::Color32::TRANSPARENT)
                                .rounding(4.0)
                                .min_size(egui::vec2(20.0, 20.0)),
                            );
                            if close_btn.clicked() {
                                is_open = false;
                            }
                        });
                    });
                });

                // === Content Area ===
                let content_frame = egui::Frame::default()
                    .inner_margin(egui::Margin::same(4.0));

                content_frame.show(ui, |ui| {
                    action = self.app.render(ui, ctx, config);
                });
            });

        self.is_open = is_open;
        action
    }
}
