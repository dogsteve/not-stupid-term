use eframe::egui;
use std::fs;
use crate::ui::icons::Icons;
use crate::ui::formatter::{detect_kind, format_content, FileKind};
use crate::ui::window_framework::{WindowAction, WindowApp};

use regex::RegexBuilder;

#[derive(Clone, Debug, Default)]
pub struct FindReplaceState {
    pub is_open: bool,
    pub show_replace: bool,
    pub find_query: String,
    pub replace_query: String,
    pub match_case: bool,
    pub match_whole_word: bool,
    pub use_regex: bool,
    pub current_match_idx: usize,
    pub matches: Vec<(usize, usize)>,
    pub error_msg: Option<String>,
}

impl FindReplaceState {
    pub fn build_regex(&self) -> Result<regex::Regex, regex::Error> {
        let pattern = if self.use_regex {
            if self.match_whole_word {
                format!(r"\b(?:{})\b", self.find_query)
            } else {
                self.find_query.clone()
            }
        } else if self.match_whole_word {
            format!(r"\b{}\b", regex::escape(&self.find_query))
        } else {
            regex::escape(&self.find_query)
        };

        RegexBuilder::new(&pattern)
            .case_insensitive(!self.match_case)
            .build()
    }

    pub fn update_matches(&mut self, content: &str) {
        self.matches.clear();
        self.error_msg = None;

        if self.find_query.is_empty() {
            self.current_match_idx = 0;
            return;
        }

        match self.build_regex() {
            Ok(re) => {
                for mat in re.find_iter(content) {
                    self.matches.push((mat.start(), mat.end()));
                }
            }
            Err(e) => {
                self.error_msg = Some(e.to_string());
            }
        }

        if self.matches.is_empty() {
            self.current_match_idx = 0;
        } else if self.current_match_idx >= self.matches.len() {
            self.current_match_idx = self.matches.len() - 1;
        }
    }

    pub fn next_match(&mut self) {
        if !self.matches.is_empty() {
            self.current_match_idx = (self.current_match_idx + 1) % self.matches.len();
        }
    }

    pub fn prev_match(&mut self) {
        if !self.matches.is_empty() {
            self.current_match_idx = if self.current_match_idx == 0 {
                self.matches.len() - 1
            } else {
                self.current_match_idx - 1
            };
        }
    }

    pub fn replace_current(&mut self, content: &mut String) {
        if self.matches.is_empty() || self.current_match_idx >= self.matches.len() {
            return;
        }

        let (start, end) = self.matches[self.current_match_idx];
        if start <= content.len() && end <= content.len() && content.is_char_boundary(start) && content.is_char_boundary(end) {
            if self.use_regex {
                if let Ok(re) = self.build_regex() {
                    let replaced = re.replace(&content[start..end], self.replace_query.as_str()).to_string();
                    content.replace_range(start..end, &replaced);
                }
            } else {
                content.replace_range(start..end, &self.replace_query);
            }
        }
        self.update_matches(content);
    }

    pub fn replace_all(&mut self, content: &mut String) {
        if self.find_query.is_empty() {
            return;
        }

        if let Ok(re) = self.build_regex() {
            let new_content = if self.use_regex {
                re.replace_all(content, self.replace_query.as_str()).to_string()
            } else {
                re.replace_all(content, regex::NoExpand(&self.replace_query)).to_string()
            };
            *content = new_content;
            self.update_matches(content);
        }
    }
}

pub struct EditorApp {
    pub path: String,
    pub content: String,
    pub original_content: String,
    pub is_dirty: bool,
    pub preview_mode: bool,
    pub save_status: Option<String>,
    /// Transient error message from the last format attempt.
    pub format_error: Option<String>,
    pub find_state: FindReplaceState,
    /// Set to true when next/prev match is triggered; causes scroll-to-match next frame.
    pub scroll_to_match: bool,
    pub pending_scroll_y: Option<f32>,
    pub focus_find_requested: bool,
    pub focus_replace_requested: bool,
}

impl EditorApp {
    pub fn open(path: &str) -> Result<Self, std::io::Error> {
        let content = fs::read_to_string(path)?;
        let is_md = path.ends_with(".md") || path.ends_with(".markdown");
        Ok(Self {
            path: path.to_string(),
            original_content: content.clone(),
            content,
            is_dirty: false,
            preview_mode: is_md,
            save_status: None,
            format_error: None,
            find_state: FindReplaceState::default(),
            scroll_to_match: false,
            pending_scroll_y: None,
            focus_find_requested: false,
            focus_replace_requested: false,
        })
    }

    pub fn new_untitled() -> Self {
        Self {
            path: "Untitled.txt".to_string(),
            content: String::new(),
            original_content: String::new(),
            is_dirty: false,
            preview_mode: false,
            save_status: None,
            format_error: None,
            find_state: FindReplaceState::default(),
            scroll_to_match: false,
            pending_scroll_y: None,
            focus_find_requested: false,
            focus_replace_requested: false,
        }
    }

    pub fn new_untitled_with_content(path: &str, content: &str) -> Self {
        let p = if path.is_empty() { "Untitled.txt" } else { path };
        let is_md = p.ends_with(".md") || p.ends_with(".markdown");
        Self {
            path: p.to_string(),
            original_content: content.to_string(),
            content: content.to_string(),
            is_dirty: false,
            preview_mode: is_md,
            save_status: None,
            format_error: None,
            find_state: FindReplaceState::default(),
            scroll_to_match: false,
            pending_scroll_y: None,
            focus_find_requested: false,
            focus_replace_requested: false,
        }
    }

    /// Computes target scroll Y to center the current match in the viewport.
    pub fn scroll_to_current_match(&mut self, line_h: f32, viewport_h: f32) {
        if let Some(&(start_byte, _)) = self.find_state.matches.get(self.find_state.current_match_idx) {
            let prefix = if start_byte <= self.content.len() {
                &self.content[..start_byte]
            } else {
                &self.content
            };
            let match_line_idx = prefix.chars().filter(|&c| c == '\n').count();
            let match_y = match_line_idx as f32 * line_h;
            let target_y = (match_y - viewport_h / 2.0).max(0.0);
            self.pending_scroll_y = Some(target_y);
        }
    }

    /// Opens the OS native "Save As" dialog and returns the chosen path (as a String).
    /// Pre-fills the filename from `current_path` and tries to suggest the correct
    /// file extension filter based on the current path extension.
    fn show_save_dialog(current_path: &str) -> Option<String> {
        let stem = std::path::Path::new(current_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Untitled.txt");

        let ext = std::path::Path::new(current_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("txt");

        let filter_name = match ext {
            "json"                         => "JSON Files",
            "xml"                          => "XML Files",
            "html" | "htm"                 => "HTML Files",
            "sql"                          => "SQL Files",
            "rs"                           => "Rust Files",
            "py"                           => "Python Files",
            "js" | "mjs"                   => "JavaScript Files",
            "ts" | "tsx" | "jsx"           => "TypeScript Files",
            "go"                           => "Go Files",
            "java"                         => "Java Files",
            "kt" | "kts"                   => "Kotlin Files",
            "c" | "cpp" | "h" | "hpp"      => "C/C++ Files",
            "sh" | "bash" | "zsh"          => "Shell Scripts",
            "toml"                         => "TOML Files",
            "yaml" | "yml"                 => "YAML Files",
            "css" | "scss"                 => "CSS Files",
            "md" | "markdown"              => "Markdown Files",
            _                              => "All Files",
        };

        rfd::FileDialog::new()
            .set_file_name(stem)
            .add_filter(filter_name, &[ext])
            .add_filter("All Files", &["*"])
            .save_file()
            .map(|p| p.to_string_lossy().to_string())
    }
    pub fn save_current_file(&mut self, undo: &mut crate::ui::undo_manager::UndoManager) -> bool {
        let is_untitled = self.path.trim().is_empty()
            || self.path == "Untitled.txt";

        let target_path: Option<String> = if is_untitled {
            Self::show_save_dialog(&self.path)
        } else {
            Some(self.path.trim().to_string())
        };

        if let Some(path) = target_path {
            self.path = path.clone();
            let prev_content = self.original_content.clone();
            if fs::write(&path, &self.content).is_ok() {
                undo.push(crate::ui::undo_manager::UndoAction::EditorSave {
                    file_path: path.clone(),
                    previous_content: prev_content,
                }, format!("Save {}", path));

                self.original_content = self.content.clone();
                self.is_dirty = false;
                self.save_status = Some("Saved!".to_string());
                self.format_error = None;
                return true;
            } else {
                self.save_status = Some("Error saving".to_string());
            }
        }
        false
    }
}

impl WindowApp for EditorApp {
    fn title(&self) -> String {
        if self.path.is_empty() || self.path == "Untitled.txt" {
            format!("{} Notepad (Untitled)", Icons::NOTE)
        } else {
            let file_name = std::path::Path::new(&self.path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&self.path);
            let icon = Icons::get_file_icon(&self.path);
            format!("{} {}", icon, file_name)
        }
    }

    fn window_type(&self) -> &'static str {
        "editor"
    }

    fn save_state(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "path": self.path,
            "content": self.content,
        }))
    }

    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn as_any_mut(&mut self) -> Option<&mut dyn std::any::Any> {
        Some(self)
    }

    fn render(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        config: &mut crate::ui::settings::AppConfig,
        undo: &mut crate::ui::undo_manager::UndoManager,
    ) -> Option<WindowAction> {
        ui.add_space(4.0);

        // TOP EDITOR TOOLBAR
        ui.horizontal(|ui| {
            let ui_font_size = config.ui_font_size;
            let mono_size = config.mono_font_size;
            let btn_size = (ui_font_size - 1.0).max(10.0);

            let icon = Icons::get_file_icon(&self.path);
            ui.label(Icons::rich(icon, (ui_font_size + 2.0).max(14.0)));

            // Editable path field
            let path_resp = ui.add(
                egui::TextEdit::singleline(&mut self.path)
                    .desired_width(180.0)
                    .font(egui::FontId::monospace(mono_size))
            );
            if path_resp.changed() {
                self.is_dirty = true;
            }

            // Auto-detect file kind and show badge
            let kind = detect_kind(&self.path, &self.content);
            let kind_label = kind.label();
            let badge_color = if kind == FileKind::Unknown {
                egui::Color32::from_rgb(100, 100, 100)
            } else {
                egui::Color32::from_rgb(60, 130, 200)
            };
            ui.add(
                egui::Button::new(egui::RichText::new(kind_label).size((ui_font_size - 2.5).max(9.0)).color(egui::Color32::WHITE))
                    .fill(badge_color)
                    .rounding(4.0)
                    .sense(egui::Sense::hover()),
            ).on_hover_text("Auto-detected file type");

            if self.is_dirty {
                ui.label(egui::RichText::new("• Modified").color(egui::Color32::GOLD).size((ui_font_size - 2.0).max(9.0)));
            }

            if let Some(ref status) = self.save_status.clone() {
                ui.label(egui::RichText::new(status).color(egui::Color32::GREEN).size((ui_font_size - 2.0).max(9.0)));
            }

            if let Some(ref err) = self.format_error.clone() {
                ui.label(egui::RichText::new(err).color(egui::Color32::from_rgb(230, 80, 80)).size((ui_font_size - 2.0).max(9.0)));
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let is_dark = ui.style().visuals.dark_mode;

                // Find & Replace button
                let find_btn = ui.add(
                    egui::Button::new(Icons::label_job(Icons::SEARCH, "Find", btn_size, ui.visuals().text_color()))
                        .rounding(6.0)
                ).on_hover_text("Find & Replace (Ctrl+F / Ctrl+H)");

                if find_btn.clicked() {
                    self.find_state.is_open = !self.find_state.is_open;
                    if self.find_state.is_open {
                        self.find_state.update_matches(&self.content);
                    }
                }

                // Save button – only active when there are unsaved changes
                let save_active = self.is_dirty;
                let save_fill = if save_active {
                    egui::Color32::from_rgb(40, 140, 240)
                } else if is_dark {
                    egui::Color32::from_rgb(35, 55, 80)
                } else {
                    egui::Color32::from_rgb(190, 210, 235)
                };
                let save_text_color = if save_active {
                    egui::Color32::WHITE
                } else if is_dark {
                    egui::Color32::from_rgb(100, 130, 160)
                } else {
                    egui::Color32::from_rgb(130, 160, 190)
                };

                let save_btn = ui.add(
                    egui::Button::new(Icons::label_job(Icons::SAVE, "Save", btn_size, save_text_color))
                    .fill(save_fill)
                    .rounding(6.0)
                ).on_hover_text(if save_active { "Save file (Ctrl+S)" } else { "No changes to save" });

                if save_btn.clicked() && save_active {
                    self.save_current_file(undo);
                }

                // Save As button
                let save_as_fill = if is_dark {
                    egui::Color32::from_rgb(45, 48, 58)
                } else {
                    egui::Color32::from_rgb(220, 225, 235)
                };
                let save_as_text = if is_dark {
                    egui::Color32::from_rgb(170, 185, 205)
                } else {
                    egui::Color32::from_rgb(80, 100, 130)
                };
                if ui.add(
                    egui::Button::new(Icons::label_job(Icons::SAVE, "Save As…", btn_size, save_as_text))
                        .fill(save_as_fill)
                        .rounding(6.0)
                ).on_hover_text("Save file to a new location").clicked() {
                    if let Some(path) = Self::show_save_dialog(&self.path) {
                        self.path = path.clone();
                        let prev_content = self.original_content.clone();
                        if fs::write(&path, &self.content).is_ok() {
                            undo.push(crate::ui::undo_manager::UndoAction::EditorSave {
                                file_path: path.clone(),
                                previous_content: prev_content,
                            }, format!("Save As {}", path));

                            self.original_content = self.content.clone();
                            self.is_dirty = false;
                            self.save_status = Some("Saved As!".to_string());
                            self.format_error = None;
                        } else {
                            self.save_status = Some("Error saving".to_string());
                        }
                    }
                }

                // Format button – shown only when formatter supports this kind
                let kind_for_fmt = detect_kind(&self.path, &self.content);
                if kind_for_fmt.can_format() {
                    let fmt_btn = ui.add(
                        egui::Button::new(Icons::label_job(Icons::WRENCH, "Format", btn_size, ui.visuals().text_color()))
                            .fill(egui::Color32::from_rgb(60, 160, 80))
                            .rounding(6.0)
                    ).on_hover_text(format!("Auto-format as {}", kind_for_fmt.label()));

                    if fmt_btn.clicked() {
                        self.format_error = None;
                        self.save_status = None;
                        let prev_content = self.content.clone();
                        match format_content(&self.content, &kind_for_fmt) {
                            Ok(formatted) => {
                                undo.push(crate::ui::undo_manager::UndoAction::EditorFormat {
                                    window_id: self.path.clone(),
                                    previous_content: prev_content,
                                }, format!("Format {}", kind_for_fmt.label()));
                                self.content = formatted;
                                self.is_dirty = self.content != self.original_content;
                                self.save_status = Some(format!("Formatted as {}!", kind_for_fmt.label()));
                            }
                            Err(e) => {
                                self.format_error = Some(format!("Format error: {}", e));
                            }
                        }
                    }
                }

                // Markdown Preview toggle button
                let is_md = self.path.ends_with(".md") || self.path.ends_with(".markdown");
                if is_md {
                    let job = if self.preview_mode {
                        Icons::label_job(Icons::EDIT, "Edit Code", btn_size, ui.visuals().text_color())
                    } else {
                        Icons::label_job(Icons::EYE, "Preview MD", btn_size, ui.visuals().text_color())
                    };
                    if ui.button(job).clicked() {
                        self.preview_mode = !self.preview_mode;
                    }
                }
            });
        });

        // Contextual Shortcuts for Editor (Ctrl+F / Cmd+F, Ctrl+H / Cmd+H, Ctrl+S / Cmd+S)
        let is_find_shortcut = ui.input(|i| (i.modifiers.command || i.modifiers.ctrl) && i.key_pressed(egui::Key::F));
        let is_replace_shortcut = ui.input(|i| (i.modifiers.command || i.modifiers.ctrl) && i.key_pressed(egui::Key::H));
        let is_save_shortcut = ui.input(|i| (i.modifiers.command || i.modifiers.ctrl) && i.key_pressed(egui::Key::S));

        if is_find_shortcut {
            self.find_state.is_open = true;
            self.find_state.show_replace = false;
            self.find_state.update_matches(&self.content);
            self.focus_find_requested = true;
        }
        if is_replace_shortcut {
            self.find_state.is_open = true;
            self.find_state.show_replace = true;
            self.find_state.update_matches(&self.content);
            self.focus_replace_requested = true;
        }
        if is_save_shortcut && self.is_dirty {
            self.save_current_file(undo);
        }
        if self.find_state.is_open && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.find_state.is_open = false;
        }

        // FIND & REPLACE TOOLBAR PANEL
        if self.find_state.is_open {
            let is_dark = ui.style().visuals.dark_mode;
            let panel_bg = if is_dark {
                egui::Color32::from_rgb(32, 34, 42)
            } else {
                egui::Color32::from_rgb(235, 238, 245)
            };
            let border_color = if is_dark {
                egui::Color32::from_white_alpha(30)
            } else {
                egui::Color32::from_black_alpha(30)
            };

            ui.add_space(4.0);
            egui::Frame::default()
                .fill(panel_bg)
                .stroke(egui::Stroke::new(1.0, border_color))
                .rounding(6.0)
                .inner_margin(egui::Margin::symmetric(8.0, 6.0))
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        // --- Row 1: Find Input + Controls + Navigation + Close ---
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("🔍").size(13.0));

                            let mono_size = config.mono_font_size;
                            let line_h = ui.fonts(|f| f.row_height(&egui::FontId::monospace(mono_size))).max(14.0);
                            let avail_h = ui.available_height().max(200.0);

                            let find_id = ui.id().with("editor_find_input");
                            let find_resp = ui.add(
                                egui::TextEdit::singleline(&mut self.find_state.find_query)
                                    .id(find_id)
                                    .hint_text("Find...")
                                    .desired_width(180.0)
                                    .font(egui::FontId::monospace(mono_size)),
                            );

                            if self.focus_find_requested {
                                find_resp.request_focus();
                                self.focus_find_requested = false;
                            }

                            if find_resp.changed() {
                                self.find_state.update_matches(&self.content);
                                if !self.find_state.matches.is_empty() {
                                    self.scroll_to_current_match(line_h, avail_h);
                                    ui.ctx().request_repaint();
                                }
                            }

                            if find_resp.has_focus() {
                                let shift = ui.input(|i| i.modifiers.shift);
                                let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                                let down = ui.input(|i| i.key_pressed(egui::Key::ArrowDown));
                                let up = ui.input(|i| i.key_pressed(egui::Key::ArrowUp));

                                if (enter && !shift) || down {
                                    self.find_state.next_match();
                                    self.scroll_to_current_match(line_h, avail_h);
                                    ui.ctx().request_repaint();
                                } else if (enter && shift) || up {
                                    self.find_state.prev_match();
                                    self.scroll_to_current_match(line_h, avail_h);
                                    ui.ctx().request_repaint();
                                }
                            }

                            // Match Case toggle
                            let mc_fill = if self.find_state.match_case {
                                ui.visuals().selection.bg_fill
                            } else {
                                egui::Color32::TRANSPARENT
                            };
                            if ui.add(
                                egui::Button::new(
                                    egui::RichText::new("Aa")
                                        .size(11.0)
                                        .color(if self.find_state.match_case { egui::Color32::WHITE } else { ui.visuals().text_color() }),
                                )
                                .fill(mc_fill)
                                .rounding(4.0)
                                .min_size(egui::vec2(22.0, 20.0)),
                            ).on_hover_text("Match Case").clicked() {
                                self.find_state.match_case = !self.find_state.match_case;
                                self.find_state.update_matches(&self.content);
                            }

                            // Whole Word toggle
                            let ww_fill = if self.find_state.match_whole_word {
                                ui.visuals().selection.bg_fill
                            } else {
                                egui::Color32::TRANSPARENT
                            };
                            if ui.add(
                                egui::Button::new(
                                    egui::RichText::new(r"\b")
                                        .size(11.0)
                                        .color(if self.find_state.match_whole_word { egui::Color32::WHITE } else { ui.visuals().text_color() }),
                                )
                                .fill(ww_fill)
                                .rounding(4.0)
                                .min_size(egui::vec2(22.0, 20.0)),
                            ).on_hover_text("Match Whole Word").clicked() {
                                self.find_state.match_whole_word = !self.find_state.match_whole_word;
                                self.find_state.update_matches(&self.content);
                            }

                            // Use Regex toggle
                            let rx_fill = if self.find_state.use_regex {
                                ui.visuals().selection.bg_fill
                            } else {
                                egui::Color32::TRANSPARENT
                            };
                            if ui.add(
                                egui::Button::new(
                                    egui::RichText::new(".*")
                                        .size(11.0)
                                        .color(if self.find_state.use_regex { egui::Color32::WHITE } else { ui.visuals().text_color() }),
                                )
                                .fill(rx_fill)
                                .rounding(4.0)
                                .min_size(egui::vec2(22.0, 20.0)),
                            ).on_hover_text("Use Regular Expression").clicked() {
                                self.find_state.use_regex = !self.find_state.use_regex;
                                self.find_state.update_matches(&self.content);
                            }

                            // Match Counter
                            if let Some(ref err) = self.find_state.error_msg {
                                ui.label(egui::RichText::new(format!("⚠ Regex error: {}", err)).size(11.0).color(egui::Color32::RED));
                            } else if !self.find_state.find_query.is_empty() {
                                let total = self.find_state.matches.len();
                                let count_text = if total == 0 {
                                    "No results".to_string()
                                } else {
                                    format!("{} of {}", self.find_state.current_match_idx + 1, total)
                                };
                                let count_color = if total == 0 {
                                    egui::Color32::GRAY
                                } else {
                                    ui.visuals().text_color()
                                };
                                ui.label(egui::RichText::new(count_text).size(11.0).color(count_color));
                            }

                            // Navigation
                            if ui.add(
                                egui::Button::new(Icons::rich(Icons::CARET_UP, 11.0))
                                    .fill(egui::Color32::TRANSPARENT)
                                    .rounding(4.0)
                                    .min_size(egui::vec2(20.0, 20.0)),
                            ).on_hover_text("Previous Match (Shift+Enter / Up Arrow)").clicked() {
                                self.find_state.prev_match();
                                self.scroll_to_current_match(line_h, avail_h);
                                ui.ctx().request_repaint();
                            }

                            if ui.add(
                                egui::Button::new(Icons::rich(Icons::CARET_DOWN, 11.0))
                                    .fill(egui::Color32::TRANSPARENT)
                                    .rounding(4.0)
                                    .min_size(egui::vec2(20.0, 20.0)),
                            ).on_hover_text("Next Match (Enter / Down Arrow)").clicked() {
                                self.find_state.next_match();
                                self.scroll_to_current_match(line_h, avail_h);
                                ui.ctx().request_repaint();
                            }

                            // Toggle Replace Bar
                            let repl_icon = if self.find_state.show_replace { "▲" } else { "▼" };
                            if ui.add(
                                egui::Button::new(egui::RichText::new(repl_icon).size(11.0))
                                    .fill(egui::Color32::TRANSPARENT)
                                    .rounding(4.0)
                                    .min_size(egui::vec2(20.0, 20.0)),
                            ).on_hover_text("Toggle Replace (Ctrl+H)").clicked() {
                                self.find_state.show_replace = !self.find_state.show_replace;
                            }

                            // Close Find Bar
                            if ui.add(
                                egui::Button::new(Icons::rich(Icons::CLOSE, 11.0))
                                    .fill(egui::Color32::TRANSPARENT)
                                    .rounding(4.0)
                                    .min_size(egui::vec2(20.0, 20.0)),
                            ).on_hover_text("Close (Esc)").clicked() {
                                self.find_state.is_open = false;
                            }
                        });

                        // --- Row 2: Replace Input & Buttons ---
                        if self.find_state.show_replace {
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("⇄").size(13.0));

                                let replace_id = ui.id().with("editor_replace_input");
                                let repl_resp = ui.add(
                                    egui::TextEdit::singleline(&mut self.find_state.replace_query)
                                        .id(replace_id)
                                        .hint_text("Replace...")
                                        .desired_width(180.0)
                                        .font(egui::FontId::monospace(config.mono_font_size)),
                                );

                                if self.focus_replace_requested {
                                    repl_resp.request_focus();
                                    self.focus_replace_requested = false;
                                }

                                let mono_size = config.mono_font_size;
                                let line_h = ui.fonts(|f| f.row_height(&egui::FontId::monospace(mono_size))).max(14.0);
                                let avail_h = ui.available_height().max(200.0);

                                if ui.add(
                                    egui::Button::new(egui::RichText::new("Replace").size(11.0))
                                        .rounding(4.0)
                                        .min_size(egui::vec2(55.0, 20.0)),
                                ).on_hover_text("Replace Current Match").clicked() {
                                    let prev = self.content.clone();
                                    self.find_state.replace_current(&mut self.content);
                                    if self.content != prev {
                                        undo.push(crate::ui::undo_manager::UndoAction::EditorReplace {
                                            window_id: self.path.clone(),
                                            previous_content: prev,
                                        }, "Replace");
                                        self.is_dirty = true;
                                        self.scroll_to_current_match(line_h, avail_h);
                                        ui.ctx().request_repaint();
                                    }
                                }

                                if ui.add(
                                    egui::Button::new(egui::RichText::new("Replace All").size(11.0))
                                        .rounding(4.0)
                                        .min_size(egui::vec2(70.0, 20.0)),
                                ).on_hover_text("Replace All Matches").clicked() {
                                    let prev = self.content.clone();
                                    self.find_state.replace_all(&mut self.content);
                                    if self.content != prev {
                                        undo.push(crate::ui::undo_manager::UndoAction::EditorReplaceAll {
                                            window_id: self.path.clone(),
                                            previous_content: prev,
                                        }, "Replace All");
                                        self.is_dirty = true;
                                        ui.ctx().request_repaint();
                                    }
                                }
                            });
                        }
                    });
                });
        }

        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);

        // MAIN CONTENT AREA – TextEdit with overlaid gutter + edge-scroll support
        let avail = ui.available_size();

        // Edge-scroll / Match scroll
        let editor_scroll_id = ui.id().with("editor_edge_scroll");
        let mut desired_offset = ctx
            .data(|d| d.get_temp::<egui::Vec2>(editor_scroll_id))
            .unwrap_or_default();

        if let Some(target_y) = self.pending_scroll_y.take() {
            desired_offset.y = target_y;
            ctx.data_mut(|d| {
                d.insert_temp(editor_scroll_id, desired_offset);
            });
        }

        let output = egui::ScrollArea::both()
            .auto_shrink([false, false])
            .id_salt(ui.id().with("editor_scroll"))
            .scroll_offset(desired_offset)
            .show(ui, |ui| {
                ui.set_min_size(avail);
                if self.preview_mode {
                    render_markdown(ui, &self.content, config);
                } else {
                    let ext = std::path::Path::new(&self.path)
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("");

                    let mono_size = config.mono_font_size;
                    let show_line_nums = config.show_line_numbers;

                    let line_h = ui.fonts(|f| f.row_height(&egui::FontId::monospace(mono_size))).max(14.0);
                    let initial_line_count = self.content.split('\n').count().max(1);
                    let digits = (initial_line_count as f32).log10().floor() as usize + 1;
                    let gutter_w = if show_line_nums {
                        (digits as f32 * mono_size * 0.62 + 20.0).max(36.0)
                    } else {
                        0.0
                    };

                    let is_dark = ui.visuals().dark_mode;
                    let gutter_bg = if is_dark {
                        egui::Color32::from_gray(28)
                    } else {
                        egui::Color32::from_gray(240)
                    };
                    let num_color = if is_dark {
                        egui::Color32::from_gray(90)
                    } else {
                        egui::Color32::from_gray(150)
                    };
                    let sep_color = if is_dark {
                        egui::Color32::from_gray(55)
                    } else {
                        egui::Color32::from_gray(210)
                    };

                    let mut layouter = |ui: &egui::Ui, string: &str, _wrap_width: f32| {
                        let mut layout_job = crate::ui::highlighter::highlight_code(
                            ui, string, ext, mono_size,
                        );
                        if self.find_state.is_open && !self.find_state.matches.is_empty() {
                            apply_search_highlights(&mut layout_job, &self.find_state);
                        }
                        ui.fonts(|f| f.layout_job(layout_job))
                    };

                    let mut content_changed = false;

                    ui.horizontal_top(|ui| {
                        if show_line_nums {
                            let content_h = (initial_line_count as f32 * line_h).max(avail.y);
                            let (gutter_rect, _) = ui.allocate_exact_size(
                                egui::vec2(gutter_w, content_h),
                                egui::Sense::hover(),
                            );

                            let code_w = ui.available_width().max(40.0);
                            let te_out = egui::TextEdit::multiline(&mut self.content)
                                .layouter(&mut layouter)
                                .desired_width(code_w)
                                .frame(false)
                                .show(ui);

                            if te_out.response.changed() {
                                content_changed = true;
                                ctx.request_repaint();
                            }

                            // ── PAINT GUTTER using actual galley row positions with zero drift ──
                            let total_text_h = te_out.galley.rect.height().max(content_h).max(avail.y);
                            let full_gutter_rect = egui::Rect::from_min_max(
                                gutter_rect.min,
                                egui::pos2(gutter_rect.max.x, gutter_rect.min.y + total_text_h),
                            );

                            let view_clip = ui.clip_rect();
                            let painter = ui.painter().with_clip_rect(view_clip);
                            painter.rect_filled(full_gutter_rect, 0.0, gutter_bg);

                            let x_right = gutter_rect.max.x - 8.0;
                            let mut line_num = 1;
                            let mut is_new_line = true;

                            for row in &te_out.galley.rows {
                                let row_center_y = te_out.galley_pos.y + row.rect.center().y;
                                let row_min_y = te_out.galley_pos.y + row.rect.min.y;
                                let row_max_y = te_out.galley_pos.y + row.rect.max.y;

                                if is_new_line {
                                    if row_max_y >= view_clip.min.y - 20.0 && row_min_y <= view_clip.max.y + 20.0 {
                                        painter.text(
                                            egui::pos2(x_right, row_center_y),
                                            egui::Align2::RIGHT_CENTER,
                                            line_num.to_string(),
                                            egui::FontId::monospace(mono_size),
                                            num_color,
                                        );
                                    }
                                    line_num += 1;
                                }

                                is_new_line = row.ends_with_newline;
                            }

                            if self.content.ends_with('\n') {
                                let last_row_bottom = te_out.galley.rows.last().map_or(0.0, |r| r.rect.max.y);
                                let extra_center_y = te_out.galley_pos.y + last_row_bottom + (line_h / 2.0);
                                if extra_center_y >= view_clip.min.y - 20.0 && extra_center_y <= view_clip.max.y + 20.0 {
                                    painter.text(
                                        egui::pos2(x_right, extra_center_y),
                                        egui::Align2::RIGHT_CENTER,
                                        line_num.to_string(),
                                        egui::FontId::monospace(mono_size),
                                        num_color,
                                    );
                                }
                            }

                            painter.line_segment(
                                [full_gutter_rect.right_top(), full_gutter_rect.right_bottom()],
                                egui::Stroke::new(1.0, sep_color),
                            );
                        } else {
                            let te_out = egui::TextEdit::multiline(&mut self.content)
                                .layouter(&mut layouter)
                                .desired_width(ui.available_width())
                                .frame(false)
                                .show(ui);
                            if te_out.response.changed() {
                                content_changed = true;
                                ctx.request_repaint();
                            }
                        }
                    });

                    if content_changed {
                        self.is_dirty = self.content != self.original_content;
                        if self.find_state.is_open {
                            self.find_state.update_matches(&self.content);
                        }
                    }
                }
            });

        // Apply edge-scroll delta and store for next frame
        let edge_delta =
            compute_edge_scroll_delta_editor(ctx, output.inner_rect);
        ctx.data_mut(|d| {
            d.insert_temp(editor_scroll_id, output.state.offset + edge_delta)
        });
        if edge_delta != egui::Vec2::ZERO {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }

        None
    }
}


/// Returns a scroll delta when the pointer is dragging near the edges of `rect`.
/// Returns Vec2::ZERO when not dragging or not near any edge.
fn compute_edge_scroll_delta_editor(ctx: &egui::Context, rect: egui::Rect) -> egui::Vec2 {
    let is_dragging = ctx.input(|i| i.pointer.primary_down());
    if !is_dragging {
        return egui::Vec2::ZERO;
    }
    let ptr = match ctx.pointer_latest_pos() {
        Some(p) => p,
        None => return egui::Vec2::ZERO,
    };
    if !rect.expand(8.0).contains(ptr) {
        return egui::Vec2::ZERO;
    }

    const ZONE: f32 = 40.0;     // px from edge that triggers scroll
    const MAX_SPEED: f32 = 10.0; // px per frame at the very edge

    let mut delta = egui::Vec2::ZERO;

    let dist_left   = ptr.x - rect.min.x;
    let dist_right  = rect.max.x - ptr.x;
    let dist_top    = ptr.y - rect.min.y;
    let dist_bottom = rect.max.y - ptr.y;

    if dist_left  < ZONE && dist_left  >= 0.0 { delta.x = -MAX_SPEED * (1.0 - dist_left  / ZONE); }
    if dist_right < ZONE && dist_right >= 0.0 { delta.x =  MAX_SPEED * (1.0 - dist_right / ZONE); }
    if dist_top   < ZONE && dist_top   >= 0.0 { delta.y = -MAX_SPEED * (1.0 - dist_top   / ZONE); }
    if dist_bottom < ZONE && dist_bottom >= 0.0 { delta.y = MAX_SPEED * (1.0 - dist_bottom / ZONE); }

    delta
}

fn render_markdown(ui: &mut egui::Ui, md_text: &str, config: &crate::ui::settings::AppConfig) {
    let mut in_code_block = false;
    let mut code_block_buf = String::new();

    for line in md_text.lines() {
        if line.trim_start().starts_with("```") {
            if in_code_block {
                let frame = egui::Frame::default()
                    .fill(egui::Color32::from_black_alpha(100))
                    .rounding(4.0)
                    .inner_margin(8.0);
                frame.show(ui, |ui| {
                    ui.label(egui::RichText::new(&code_block_buf).font(egui::FontId::monospace(config.mono_font_size)));
                });
                code_block_buf.clear();
                in_code_block = false;
            } else {
                in_code_block = true;
            }
            continue;
        }

        if in_code_block {
            code_block_buf.push_str(line);
            code_block_buf.push('\n');
            continue;
        }

        let trimmed = line.trim();

        if trimmed.starts_with("# ") {
            ui.add_space(8.0);
            ui.heading(egui::RichText::new(&trimmed[2..]).size(config.ui_font_size + 5.0).strong());
            ui.add_space(4.0);
        } else if trimmed.starts_with("## ") {
            ui.add_space(6.0);
            ui.heading(egui::RichText::new(&trimmed[3..]).size(config.ui_font_size + 2.0).strong());
            ui.add_space(2.0);
        } else if trimmed.starts_with("### ") {
            ui.add_space(4.0);
            ui.heading(egui::RichText::new(&trimmed[4..]).size(config.ui_font_size).strong());
        } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            ui.horizontal(|ui| {
                ui.label("•");
                render_inline_markdown(ui, &trimmed[2..]);
            });
        } else if trimmed.is_empty() {
            ui.add_space(4.0);
        } else {
            ui.horizontal(|ui| {
                render_inline_markdown(ui, trimmed);
            });
        }
    }
}

fn render_inline_markdown(ui: &mut egui::Ui, text: &str) {
    let parts: Vec<&str> = text.split("**").collect();
    for (i, part) in parts.iter().enumerate() {
        if i % 2 == 1 {
            ui.label(egui::RichText::new(*part).strong());
        } else {
            ui.label(*part);
        }
    }
}

fn apply_search_highlights(job: &mut egui::text::LayoutJob, find_state: &FindReplaceState) {
    if find_state.matches.is_empty() {
        return;
    }

    let active_match = find_state.matches.get(find_state.current_match_idx).copied();
    let old_sections = std::mem::take(&mut job.sections);
    let mut new_sections = Vec::new();

    for section in old_sections {
        let sec_start = section.byte_range.start;
        let sec_end = section.byte_range.end;

        if sec_start >= sec_end {
            continue;
        }

        let mut split_points = vec![sec_start, sec_end];
        for &(m_start, m_end) in &find_state.matches {
            if m_start > sec_start && m_start < sec_end {
                split_points.push(m_start);
            }
            if m_end > sec_start && m_end < sec_end {
                split_points.push(m_end);
            }
        }
        split_points.sort_unstable();
        split_points.dedup();

        for window in split_points.windows(2) {
            let sub_start = window[0];
            let sub_end = window[1];

            let is_active = if let Some((m_start, m_end)) = active_match {
                sub_start >= m_start && sub_end <= m_end
            } else {
                false
            };

            let is_match = if is_active {
                true
            } else {
                find_state.matches.iter().any(|&(m_start, m_end)| sub_start >= m_start && sub_end <= m_end)
            };

            let mut fmt = section.format.clone();
            if is_active {
                fmt.background = egui::Color32::from_rgba_unmultiplied(255, 180, 0, 140);
            } else if is_match {
                fmt.background = egui::Color32::from_rgba_unmultiplied(255, 200, 0, 50);
            }

            new_sections.push(egui::text::LayoutSection {
                leading_space: if sub_start == sec_start { section.leading_space } else { 0.0 },
                byte_range: sub_start..sub_end,
                format: fmt,
            });
        }
    }

    job.sections = new_sections;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_text_search_and_replace_dollar_sign() {
        let mut state = FindReplaceState::default();
        state.find_query = "PRICE".to_string();
        state.replace_query = "$100".to_string();
        let mut content = "The item PRICE is discounted.".to_string();

        state.update_matches(&content);
        assert_eq!(state.matches.len(), 1);

        state.replace_all(&mut content);
        assert_eq!(content, "The item $100 is discounted.");
    }

    #[test]
    fn test_regex_search_and_replace_groups() {
        let mut state = FindReplaceState::default();
        state.use_regex = true;
        state.find_query = r"fn (\w+)".to_string();
        state.replace_query = "fn async_$1".to_string();
        let mut content = "fn process() { fn calculate() {} }".to_string();

        state.update_matches(&content);
        assert_eq!(state.matches.len(), 2);

        state.replace_all(&mut content);
        assert_eq!(content, "fn async_process() { fn async_calculate() {} }");
    }

    #[test]
    fn test_whole_word_matching() {
        let mut state = FindReplaceState::default();
        state.match_whole_word = true;
        state.find_query = "cat".to_string();
        let content = "The cat in the category is a catalog.".to_string();

        state.update_matches(&content);
        assert_eq!(state.matches.len(), 1);
    }

    #[test]
    fn test_utf8_replace_current_safety() {
        let mut state = FindReplaceState::default();
        state.find_query = "thử".to_string();
        state.replace_query = "kiểm tra".to_string();
        let mut content = "Xin chào, thử start app lên test.".to_string();

        state.update_matches(&content);
        assert_eq!(state.matches.len(), 1);

        state.replace_current(&mut content);
        assert_eq!(content, "Xin chào, kiểm tra start app lên test.");
    }

    #[test]
    fn test_line_count_trailing_newline() {
        assert_eq!("".split('\n').count().max(1), 1);
        assert_eq!("line 1".split('\n').count().max(1), 1);
        assert_eq!("line 1\n".split('\n').count().max(1), 2);
        assert_eq!("line 1\n\n".split('\n').count().max(1), 3);
        assert_eq!("line 1\nline 2".split('\n').count().max(1), 2);
        assert_eq!("line 1\nline 2\n".split('\n').count().max(1), 3);
    }

    #[test]
    fn test_search_scroll_to_match_calculation() {
        let mut app = EditorApp::new_untitled();
        app.content = "line 0\nline 1\nline 2\ntarget line\nline 4".to_string();
        app.find_state.find_query = "target".to_string();
        app.find_state.update_matches(&app.content);
        assert_eq!(app.find_state.matches.len(), 1);

        let line_h = 20.0_f32;
        let viewport_h = 100.0_f32;
        app.scroll_to_current_match(line_h, viewport_h);

        // Target is at line index 3 (4th line) -> 3 * 20.0 = 60.0.
        // Centered: (60.0 - 100.0 / 2.0).max(0.0) = 10.0.
        assert_eq!(app.pending_scroll_y, Some(10.0));
    }

    #[test]
    fn test_editor_save_to_temp_file() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join(format!("test_editor_save_{}.txt", uuid::Uuid::new_v4()));
        let test_path = test_file.to_string_lossy().to_string();

        let mut app = EditorApp::new_untitled();
        app.path = test_path.clone();
        app.content = "hello world from editor test".to_string();
        app.is_dirty = true;

        let mut undo = crate::ui::undo_manager::UndoManager::new(50);
        let ok = app.save_current_file(&mut undo);
        assert!(ok);
        assert!(!app.is_dirty);
        assert_eq!(app.save_status, Some("Saved!".to_string()));
        assert!(undo.can_undo());

        let read_back = std::fs::read_to_string(&test_path).unwrap();
        assert_eq!(read_back, "hello world from editor test");

        let _ = std::fs::remove_file(&test_path);
    }
}


