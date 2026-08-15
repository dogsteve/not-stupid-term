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
            let icon = Icons::get_file_icon(&self.path);
            ui.label(egui::RichText::new(icon).size(15.0));

            // Editable path field
            let path_resp = ui.add(
                egui::TextEdit::singleline(&mut self.path)
                    .desired_width(180.0)
                    .font(egui::FontId::monospace(12.0))
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
                egui::Button::new(egui::RichText::new(kind_label).size(10.5).color(egui::Color32::WHITE))
                    .fill(badge_color)
                    .rounding(4.0)
                    .sense(egui::Sense::hover()),
            ).on_hover_text("Auto-detected file type");

            if self.is_dirty {
                ui.label(egui::RichText::new("• Modified").color(egui::Color32::GOLD).size(11.0));
            }

            if let Some(ref status) = self.save_status.clone() {
                ui.label(egui::RichText::new(status).color(egui::Color32::GREEN).size(11.0));
            }

            if let Some(ref err) = self.format_error.clone() {
                ui.label(egui::RichText::new(err).color(egui::Color32::from_rgb(230, 80, 80)).size(11.0));
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let is_dark = ui.style().visuals.dark_mode;

                // Find & Replace button
                let find_btn = ui.add(
                    egui::Button::new(Icons::label_job(Icons::SEARCH, "Find", 12.0, ui.visuals().text_color()))
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
                    egui::Button::new(Icons::label_job(Icons::SAVE, "Save", 12.0, save_text_color))
                    .fill(save_fill)
                    .rounding(6.0)
                ).on_hover_text(if save_active { "Save file (Ctrl+S)" } else { "No changes to save" });

                if save_btn.clicked() && save_active {
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
                        } else {
                            self.save_status = Some("Error saving".to_string());
                        }
                    }
                }

                // Save As button – always shows the file dialog, theme-aware color
                let save_as_fill = if is_dark {
                    egui::Color32::from_rgb(65, 65, 75)
                } else {
                    egui::Color32::from_rgb(210, 210, 220)
                };
                let save_as_text = if is_dark {
                    egui::Color32::from_rgb(210, 210, 220)
                } else {
                    egui::Color32::from_rgb(40, 40, 50)
                };

                let save_as_btn = ui.add(
                    egui::Button::new(Icons::label_job(Icons::SAVE, "Save As…", 12.0, save_as_text))
                    .fill(save_as_fill)
                    .rounding(6.0)
                ).on_hover_text("Choose where to save this file");

                if save_as_btn.clicked() {
                    if let Some(path) = Self::show_save_dialog(&self.path) {
                        self.path = path.clone();
                        let prev_content = std::fs::read_to_string(&path).unwrap_or_default();
                        if fs::write(&path, &self.content).is_ok() {
                            undo.push(crate::ui::undo_manager::UndoAction::EditorSave {
                                file_path: path.clone(),
                                previous_content: prev_content,
                            }, format!("Save As {}", path));
                            
                            self.original_content = self.content.clone();
                            self.is_dirty = false;
                            self.save_status = Some("Saved!".to_string());
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
                        egui::Button::new(Icons::label_job(Icons::WRENCH, "Format", 12.0, ui.visuals().text_color()))
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

                // Toggle Preview mode for Markdown files
                if self.path.ends_with(".md") || self.path.ends_with(".markdown") {
                    let job = if self.preview_mode {
                        Icons::label_job(Icons::EDIT, "Edit Code", 12.0, ui.visuals().text_color())
                    } else {
                        Icons::label_job(Icons::EYE, "Preview MD", 12.0, ui.visuals().text_color())
                    };
                    if ui.button(job).clicked() {
                        self.preview_mode = !self.preview_mode;
                    }
                }
            });
        });

        // Global Shortcuts for Editor Find & Replace
        if ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::F)) {
            self.find_state.is_open = true;
            self.find_state.show_replace = false;
            self.find_state.update_matches(&self.content);
        }
        if ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::H)) {
            self.find_state.is_open = true;
            self.find_state.show_replace = true;
            self.find_state.update_matches(&self.content);
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

                            let find_id = ui.id().with("editor_find_input");
                            let find_resp = ui.add(
                                egui::TextEdit::singleline(&mut self.find_state.find_query)
                                    .id(find_id)
                                    .hint_text("Find...")
                                    .desired_width(180.0)
                                    .font(egui::FontId::monospace(12.0)),
                            );

                            if find_resp.changed() {
                                self.find_state.update_matches(&self.content);
                            }

                            if find_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                if ui.input(|i| i.modifiers.shift) {
                                    self.find_state.prev_match();
                                } else {
                                    self.find_state.next_match();
                                }
                                self.scroll_to_match = true;
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
                                egui::Button::new(Icons::rich(Icons::CARET_LEFT, 11.0))
                                    .fill(egui::Color32::TRANSPARENT)
                                    .rounding(4.0)
                                    .min_size(egui::vec2(20.0, 20.0)),
                            ).on_hover_text("Previous Match (Shift+Enter)").clicked() {
                                self.find_state.prev_match();
                            }

                            if ui.add(
                                egui::Button::new(Icons::rich(Icons::CARET_RIGHT, 11.0))
                                    .fill(egui::Color32::TRANSPARENT)
                                    .rounding(4.0)
                                    .min_size(egui::vec2(20.0, 20.0)),
                            ).on_hover_text("Next Match (Enter)").clicked() {
                                self.find_state.next_match();
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
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.find_state.replace_query)
                                        .id(replace_id)
                                        .hint_text("Replace...")
                                        .desired_width(180.0)
                                        .font(egui::FontId::monospace(12.0)),
                                );

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
                                    }
                                    self.is_dirty = self.content != self.original_content;
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
                                    }
                                    self.is_dirty = self.content != self.original_content;
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

        // Edge-scroll: same pattern as file viewer.
        // Read stored desired offset → pass to ScrollArea → after show(), add delta → store.
        let editor_scroll_id = ui.id().with("editor_edge_scroll");
        let desired_offset = ctx
            .data(|d| d.get_temp::<egui::Vec2>(editor_scroll_id))
            .unwrap_or_default();

        let output = egui::ScrollArea::both()
            .auto_shrink([false, false])
            .id_salt(ui.id().with("editor_scroll"))
            .scroll_offset(desired_offset)
            .show(ui, |ui| {
                ui.set_min_size(avail);
                if self.preview_mode {
                    render_markdown(ui, &self.content);
                } else {
                    let ext = std::path::Path::new(&self.path)
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("");

                    let mono_size = config.mono_font_size;
                    let show_line_nums = config.show_line_numbers;

                    // ── LAYOUT: gutter on left, TextEdit on right (horizontal_top) ──────
                    // Strategy:
                    //   1. Allocate gutter space (fixed width) in horizontal layout.
                    //   2. Render TextEdit in remaining width with DEFAULT margins.
                    //   3. Use gutter_rect.min.y (== response.rect.min.y, same cursor) + 2.0 offset.
                    //   4. Paint gutter background + numbers on top using the painter.
                    //
                    // TextEdit default margin = Margin::symmetric(4.0, 2.0)
                    //   => galley starts at widget_rect.min.y + 2.0
                    const TE_GALLEY_OFFSET_Y: f32 = 2.0;

                    let line_h = ui.fonts(|f| f.row_height(&egui::FontId::monospace(mono_size)));
                    let line_count = self.content.lines().count().max(1);
                    let digits = (line_count as f32).log10().floor() as usize + 1;
                    let gutter_w = if show_line_nums {
                        digits as f32 * mono_size * 0.62 + 20.0
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

                    // content_changed / galley_pos are communicated out of horizontal_top
                    // so the layouter borrow of self.find_state is already released.
                    let mut content_changed = false;
                    // galley_pos.y is the exact screen-space Y where the galley starts.
                    // Captured inside horizontal_top and used outside for scroll-to-match.
                    let mut galley_pos_y: f32 = 0.0;
                    let mut galley_for_scroll: Option<std::sync::Arc<egui::Galley>> = None;

                    ui.horizontal_top(|ui| {
                        if show_line_nums {
                            // Allocate gutter rect — shares the same min.y as TextEdit
                            let content_h = (line_count as f32 * line_h).max(avail.y);
                            let (gutter_rect, _) = ui.allocate_exact_size(
                                egui::vec2(gutter_w, content_h),
                                egui::Sense::hover(),
                            );

                            // Use TextEdit::show() to get exact galley_pos for alignment
                            let code_w = ui.available_width().max(40.0);
                            let te_out = egui::TextEdit::multiline(&mut self.content)
                                .layouter(&mut layouter)
                                .desired_width(code_w)
                                .frame(false)
                                .show(ui);

                            if te_out.response.changed() {
                                content_changed = true;
                            }
                            galley_pos_y = te_out.galley_pos.y;
                            galley_for_scroll = Some(te_out.galley.clone());

                            // ── PAINT GUTTER using galley_pos.y as the exact anchor ──
                            // gutter_rect.min.y ≈ te_out.galley_pos.y - margin.top (≈2px)
                            // Use galley_pos_y directly for pixel-perfect alignment.
                            let painter = ui.painter().with_clip_rect(gutter_rect);
                            painter.rect_filled(gutter_rect, 0.0, gutter_bg);

                            let x_right = gutter_rect.max.x - 8.0;
                            for n in 1..=line_count {
                                let y_center = te_out.galley_pos.y + (n as f32 - 0.5) * line_h;
                                painter.text(
                                    egui::pos2(x_right, y_center),
                                    egui::Align2::RIGHT_CENTER,
                                    n.to_string(),
                                    egui::FontId::monospace(mono_size),
                                    num_color,
                                );
                            }
                            painter.line_segment(
                                [gutter_rect.right_top(), gutter_rect.right_bottom()],
                                egui::Stroke::new(1.0, sep_color),
                            );
                        } else {
                            // No gutter — TextEdit takes full width
                            let te_out = egui::TextEdit::multiline(&mut self.content)
                                .layouter(&mut layouter)
                                .desired_width(ui.available_width())
                                .frame(false)
                                .show(ui);
                            if te_out.response.changed() {
                                content_changed = true;
                            }
                            galley_pos_y = te_out.galley_pos.y;
                            galley_for_scroll = Some(te_out.galley.clone());
                        }
                    });

                    // Act on content change AFTER horizontal_top exits (layouter borrow released)
                    if content_changed {
                        self.is_dirty = self.content != self.original_content;
                        if self.find_state.is_open {
                            self.find_state.update_matches(&self.content);
                        }
                    }

                    // ── SCROLL-TO-MATCH ──────────────────────────────────────────────────
                    // When next/prev is triggered, compute the match's galley-relative Y,
                    // then write to editor_scroll_id so the ScrollArea jumps there next frame.
                    if self.scroll_to_match {
                        self.scroll_to_match = false;
                        if let (Some(galley), Some((start_byte, _))) = (
                            galley_for_scroll,
                            self.find_state.matches.get(self.find_state.current_match_idx).copied(),
                        ) {
                            // Convert byte offset → char count → CCursor → galley row rect
                            let char_idx = self.content[..start_byte].chars().count();
                            let ccursor = egui::text::CCursor::new(char_idx);
                            let cursor = galley.from_ccursor(ccursor);
                            let cursor_rect = galley.pos_from_cursor(&cursor);
                            // cursor_rect.min.y is galley-relative.
                            // Center the match: target_scroll = cursor_y - viewport_h/2
                            let viewport_h = ui.clip_rect().height();
                            let target_y = (cursor_rect.min.y - viewport_h / 2.0).max(0.0);
                            ctx.data_mut(|d| {
                                d.insert_temp(
                                    editor_scroll_id,
                                    egui::vec2(desired_offset.x, target_y),
                                )
                            });
                            ctx.request_repaint();
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

fn render_markdown(ui: &mut egui::Ui, md_text: &str) {
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
                    ui.label(egui::RichText::new(&code_block_buf).font(egui::FontId::monospace(12.0)));
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
            ui.heading(egui::RichText::new(&trimmed[2..]).size(18.0).strong());
            ui.add_space(4.0);
        } else if trimmed.starts_with("## ") {
            ui.add_space(6.0);
            ui.heading(egui::RichText::new(&trimmed[3..]).size(15.0).strong());
            ui.add_space(2.0);
        } else if trimmed.starts_with("### ") {
            ui.add_space(4.0);
            ui.heading(egui::RichText::new(&trimmed[4..]).size(13.0).strong());
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
}

