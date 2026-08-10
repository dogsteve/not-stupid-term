use eframe::egui;

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
        [150.0, 100.0]
    }

    fn default_size(&self) -> [f32; 2] {
        [650.0, 420.0]
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

        let mut frame = egui::Frame::window(&ctx.style());
        frame.inner_margin = egui::Margin::same(2.0); // Make inner margins smaller to save space

        egui::Window::new(&self.id)
            .id(egui::Id::new(&self.id))
            .title_bar(false) // Hide default large title bar
            .frame(frame)
            .resizable(true)
            .min_size(min_s)
            .default_size(def_s)
            .show(ctx, |ui| {
                // Custom Compact Title Bar
                ui.horizontal(|ui| {
                    ui.add_space(4.0);
                    
                    if self.is_editing_title {
                        let response = ui.add(egui::TextEdit::singleline(&mut self.temp_title).desired_width(150.0));
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
                        let title_response = ui.selectable_label(false, &display_title);
                        if title_response.double_clicked() {
                            self.is_editing_title = true;
                            self.temp_title = display_title.clone();
                        }
                    }

                    // Push content to the right
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(crate::ui::icons::Icons::CLOSE).clicked() {
                            is_open = false;
                        }
                    });
                });
                
                ui.separator();
                
                // Content
                action = self.app.render(ui, ctx, config);
            });

        self.is_open = is_open;
        action
    }
}
