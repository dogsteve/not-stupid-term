use eframe::egui;
use crate::ui::icons::Icons;

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

/// Built-in commands with proper Phosphor icons (PUA codepoints).
const BUILTIN_COMMANDS: &[(&str, &str, &str)] = &[
    ("Settings",                Icons::GEAR,       "Settings"),
    ("Git Manager",             Icons::GIT_BRANCH, "Git Manager"),
    ("New Workspace",           Icons::ADD,        "New Workspace"),
    ("New Text File (Notepad)", Icons::NOTE,       "New Notepad"),
    ("Local Terminal",          Icons::TERMINAL,   "Local Terminal"),
    ("SSH & SFTP Manager",      Icons::SERVER,     "SSH Manager"),
    ("Search Commands",         Icons::SEARCH,     "Search"),
];

pub struct CommandPalette {
    pub is_open: bool,
    pub query: String,
    pub results: Vec<PaletteItem>,
    pub selected_idx: usize,
    pub just_opened: bool,
    pub file_cache: Vec<String>,
    /// 0.0 = fully hidden, 1.0 = fully visible — drives fade+slide animation.
    open_anim: f32,
    pub last_cache_time: Option<std::time::Instant>,
}

impl CommandPalette {
    pub fn new() -> Self {
        Self {
            is_open: false,
            query: String::new(),
            results: Vec::new(),
            selected_idx: 0,
            just_opened: false,
            file_cache: Vec::new(),
            open_anim: 0.0,
            last_cache_time: None,
        }
    }

    pub fn refresh_file_cache(&mut self) {
        if let Some(t) = self.last_cache_time {
            if t.elapsed().as_secs() < 10 && !self.file_cache.is_empty() {
                return;
            }
        }
        self.last_cache_time = Some(std::time::Instant::now());
        self.file_cache.clear();
        for entry in walkdir::WalkDir::new(".").max_depth(6).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                let path_str = entry.path().to_string_lossy().to_string();
                let clean_path = path_str.trim_start_matches("./");
                if clean_path.starts_with('.')
                    || clean_path.contains("/.")
                    || clean_path.starts_with("target/")
                    || clean_path.contains("/target/")
                    || clean_path.starts_with("node_modules/")
                    || clean_path.contains("/node_modules/")
                    || clean_path.starts_with("build/")
                    || clean_path.contains("/build/")
                {
                    continue;
                }
                self.file_cache.push(clean_path.to_string());
            }
        }
    }

    pub fn toggle(&mut self) {
        self.is_open = !self.is_open;
        if self.is_open {
            self.query.clear();
            self.open_anim = 0.0;
            self.refresh_file_cache();
            self.update_results();
            self.just_opened = true;
        }
    }

    pub fn update_results(&mut self) {
        self.results.clear();
        let q = self.query.to_lowercase();
        let mut count = 0;
        const MAX_RESULTS: usize = 40;

        for &(label, icon, cmd) in BUILTIN_COMMANDS {
            if q.is_empty() || label.to_lowercase().contains(&q) {
                self.results.push(PaletteItem {
                    label: label.to_string(),
                    icon,
                    action: PaletteAction::Command(cmd.to_string()),
                });
                count += 1;
            }
        }

        if count < MAX_RESULTS {
            for path in &self.file_cache {
                if q.is_empty() || path.to_lowercase().contains(&q) {
                    let icon = crate::ui::icons::Icons::get_file_icon(path);
                    self.results.push(PaletteItem {
                        label: path.clone(),
                        icon,
                        action: PaletteAction::OpenFile(path.clone()),
                    });
                    count += 1;
                    if count >= MAX_RESULTS {
                        break;
                    }
                }
            }
        }
        self.selected_idx = 0;
    }

    /// Renders the Command Palette overlay with fade+slide animation.
    /// Returns `Some(PaletteAction)` when user selects an item.
    pub fn render(&mut self, ctx: &egui::Context) -> Option<PaletteAction> {
        if !self.is_open && self.open_anim <= 0.001 {
            return None;
        }

        // Lerp open_anim toward 1.0 when open, 0.0 when closing.
        let target: f32 = if self.is_open { 1.0 } else { 0.0 };
        let dt = ctx.input(|i| i.stable_dt).min(0.05);
        self.open_anim += (target - self.open_anim) * 12.0 * dt;
        self.open_anim = self.open_anim.clamp(0.0, 1.0);
        if self.open_anim > 0.001 && self.open_anim < 0.999 {
            ctx.request_repaint();
        }

        let alpha = self.open_anim;
        // Slides up from 20px below its resting position when opening.
        let slide_offset = (1.0 - alpha) * 20.0;

        let mut selected_action = None;
        let mut close_requested = false;

        let bg_alpha = (alpha * 255.0) as u8;
        let stroke_alpha = (alpha * 80.0) as u8;
        let text_alpha = (alpha * 255.0) as u8;

        let base_bg = ctx.style().visuals.window_fill;
        let frame_fill = egui::Color32::from_rgba_premultiplied(
            (base_bg.r() as f32 * alpha) as u8,
            (base_bg.g() as f32 * alpha) as u8,
            (base_bg.b() as f32 * alpha) as u8,
            bg_alpha,
        );

        let frame = egui::Frame::window(&ctx.style())
            .fill(frame_fill)
            .rounding(14.0)
            .stroke(egui::Stroke::new(
                1.0_f32,
                egui::Color32::from_rgba_premultiplied(120, 120, 140, stroke_alpha),
            ))
            .inner_margin(16.0)
            .shadow(egui::epaint::Shadow {
                offset: [0.0, 15.0].into(),
                blur: 30.0,
                spread: 0.0,
                color: egui::Color32::from_black_alpha((180.0 * alpha) as u8),
            });

        let text_col = {
            let c = ctx.style().visuals.text_color();
            egui::Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), text_alpha)
        };

        let ui_font_size = ctx.style().text_styles.get(&egui::TextStyle::Body).map(|f| f.size).unwrap_or(13.0);

        egui::Window::new("Command Palette")
            .title_bar(false)
            .frame(frame)
            .resizable(false)
            .collapsible(false)
            .min_size([550.0, 320.0])
            .anchor(egui::Align2::CENTER_CENTER, [0.0, -80.0 + slide_offset])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(Icons::rich(Icons::SEARCH, (ui_font_size + 4.0).max(16.0)).color(text_col));
                    let input = ui.add(
                        egui::TextEdit::singleline(&mut self.query)
                            .hint_text("Type command or file name...")
                            .font(egui::FontId::proportional(ui_font_size + 3.0))
                            .desired_width(f32::INFINITY)
                            .text_color(text_col),
                    );

                    if self.just_opened {
                        input.request_focus();
                        self.just_opened = false;
                    }
                    if input.changed() {
                        self.update_results();
                    }

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
                });

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(8.0);

                egui::ScrollArea::vertical().id_salt("palette_results").show(ui, |ui| {
                    if self.results.is_empty() {
                        ui.label(
                            egui::RichText::new("No files or commands found.")
                                .color(egui::Color32::from_rgba_unmultiplied(160, 160, 160, text_alpha)),
                        );
                    } else {
                        for (idx, item) in self.results.iter().enumerate() {
                            let is_sel = idx == self.selected_idx;
                            let sel_fill = ctx.style().visuals.selection.bg_fill;
                            let btn_fill = if is_sel {
                                egui::Color32::from_rgba_unmultiplied(
                                    sel_fill.r(), sel_fill.g(), sel_fill.b(), text_alpha,
                                )
                            } else {
                                egui::Color32::TRANSPARENT
                            };
                            let label_color = if is_sel {
                                egui::Color32::from_rgba_unmultiplied(255, 255, 255, text_alpha)
                            } else {
                                text_col
                            };

                            // All palette icons are Phosphor PUA — always use label_job.
                            let job = Icons::label_job(item.icon, &item.label, ui_font_size, label_color);
                            let button = egui::Button::new(job)
                                .fill(btn_fill)
                                .rounding(6.0)
                                .min_size(egui::vec2(ui.available_width(), 32.0));
                            let response = ui.add(button);
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
