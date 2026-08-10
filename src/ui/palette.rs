use eframe::egui;
use walkdir::WalkDir;

#[derive(Clone)]
pub enum PaletteAction {
    OpenFile(String),
    Command(String),
}

#[derive(Clone)]
pub struct PaletteItem {
    pub label: String,
    pub icon: &'static str,
    pub action: PaletteAction,
}

pub struct CommandPalette {
    pub is_open: bool,
    pub query: String,
    pub results: Vec<PaletteItem>,
    pub selected_idx: usize,
}

impl CommandPalette {
    pub fn new() -> Self {
        Self {
            is_open: false,
            query: String::new(),
            results: Vec::new(),
            selected_idx: 0,
        }
    }

    pub fn toggle(&mut self) {
        self.is_open = !self.is_open;
        if self.is_open {
            self.query.clear();
            self.update_results();
        }
    }

    pub fn update_results(&mut self) {
        self.results.clear();
        let q = self.query.to_lowercase();
        let mut count = 0;

        // Add built-in commands first
        let commands = [
            ("Settings", "⚙️", PaletteAction::Command("Settings".to_string())),
            ("New Workspace", "➕", PaletteAction::Command("New Workspace".to_string())),
            ("New Text File (Notepad)", "📝", PaletteAction::Command("New Notepad".to_string())),
            ("Local Terminal", "💻", PaletteAction::Command("Local Terminal".to_string())),
            ("SSH & SFTP Manager", "🌐", PaletteAction::Command("SSH Manager".to_string())),
            ("Search Commands", "🔍", PaletteAction::Command("Search".to_string())),
        ];

        for (label, icon, action) in commands {
            if q.is_empty() || label.to_lowercase().contains(&q) {
                self.results.push(PaletteItem {
                    label: label.to_string(),
                    icon,
                    action,
                });
                count += 1;
            }
        }

        // Add files
        for entry in walkdir::WalkDir::new(".").max_depth(4).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                let path_str = entry.path().to_string_lossy().to_string();
                if path_str.contains("/.") || path_str.contains("/target/") {
                    continue; // Skip hidden and target build files
                }

                if q.is_empty() || path_str.to_lowercase().contains(&q) {
                    let icon = if path_str.ends_with(".md") { "📝" } else { "📄" };
                    self.results.push(PaletteItem {
                        label: path_str.clone(),
                        icon,
                        action: PaletteAction::OpenFile(path_str),
                    });
                    count += 1;
                    if count >= 20 {
                        break;
                    }
                }
            }
        }
        self.selected_idx = 0;
    }

    /// Renders the Command Palette overlay. Returns `Some(file_path)` if user selects a file.
    pub fn render(&mut self, ctx: &egui::Context) -> Option<PaletteAction> {
        if !self.is_open {
            return None;
        }

        let mut selected_action = None;
        let mut close_requested = false;

        let frame = egui::Frame::window(&ctx.style())
            .fill(ctx.style().visuals.window_fill)
            .rounding(14.0)
            .stroke(egui::Stroke::new(1.0, ctx.style().visuals.widgets.noninteractive.bg_stroke.color))
            .inner_margin(16.0)
            .shadow(egui::epaint::Shadow {
                offset: [0.0, 15.0].into(),
                blur: 30.0,
                spread: 0.0,
                color: egui::Color32::from_black_alpha(180),
            });

        egui::Window::new("Command Palette")
            .title_bar(false)
            .frame(frame)
            .resizable(false)
            .collapsible(false)
            .min_size([550.0, 320.0])
            .anchor(egui::Align2::CENTER_CENTER, [0.0, -80.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("🔍").size(18.0));
                    let input = ui.add(
                        egui::TextEdit::singleline(&mut self.query)
                            .hint_text("Type file name to search...")
                            .font(egui::FontId::proportional(16.0))
                            .desired_width(f32::INFINITY),
                    );

                    input.request_focus();

                    if input.changed() {
                        self.update_results();
                    }

                    if input.has_focus() {
                        if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                            if !self.results.is_empty() {
                                self.selected_idx = (self.selected_idx + 1) % self.results.len();
                            }
                        }
                        if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                            if !self.results.is_empty() {
                                self.selected_idx = self.selected_idx.checked_sub(1).unwrap_or(self.results.len() - 1);
                            }
                        }
                        if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            if let Some(res) = self.results.get(self.selected_idx) {
                                selected_action = Some(res.action.clone());
                                close_requested = true;
                            }
                        }
                        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                            close_requested = true;
                        }
                    }
                });

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(8.0);

                egui::ScrollArea::vertical().id_salt("palette_results").show(ui, |ui| {
                    if self.results.is_empty() {
                        ui.label(egui::RichText::new("No files found.").color(egui::Color32::GRAY));
                    } else {
                        for (idx, item) in self.results.iter().enumerate() {
                            let is_sel = idx == self.selected_idx;
                            let btn_fill = if is_sel {
                                ctx.style().visuals.selection.bg_fill
                            } else {
                                egui::Color32::TRANSPARENT
                            };

                            let text_color = if is_sel {
                                egui::Color32::WHITE
                            } else {
                                ctx.style().visuals.text_color()
                            };

                            let label_text = format!("{}  {}", item.icon, item.label);

                            let (rect, response) = ui.allocate_exact_size(
                                egui::vec2(ui.available_width(), 32.0),
                                egui::Sense::click(),
                            );
                            
                            if ui.is_rect_visible(rect) {
                                if is_sel {
                                    ui.painter().rect_filled(rect, 4.0, btn_fill);
                                }
                                ui.painter().text(
                                    rect.min + egui::vec2(12.0, 16.0),
                                    egui::Align2::LEFT_CENTER,
                                    label_text,
                                    egui::FontId::proportional(14.0),
                                    text_color,
                                );
                            }

                            if response.clicked() {
                                selected_action = Some(item.action.clone());
                                close_requested = true;
                            }
                        }
                    }
                });
            });

        if close_requested {
            self.is_open = false;
        }

        selected_action
    }
}
