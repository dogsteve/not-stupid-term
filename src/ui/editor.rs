use eframe::egui;
use std::fs;
use crate::ui::icons::Icons;
use crate::ui::window_framework::{WindowAction, WindowApp};

pub struct EditorApp {
    pub path: String,
    pub content: String,
    pub original_content: String,
    pub is_dirty: bool,
    pub preview_mode: bool,
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
        })
    }
}

impl WindowApp for EditorApp {
    fn title(&self) -> String {
        let file_name = std::path::Path::new(&self.path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&self.path);
        let icon = Icons::get_file_icon(&self.path);
        format!("{} {}", icon, file_name)
    }

    fn render(
        &mut self,
        ui: &mut egui::Ui,
        _ctx: &egui::Context,
        _config: &mut crate::ui::settings::AppConfig,
    ) -> Option<WindowAction> {
        ui.add_space(4.0);

        // TOP EDITOR TOOLBAR
        ui.horizontal(|ui| {
            let file_name = std::path::Path::new(&self.path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&self.path);

            let icon = Icons::get_file_icon(&self.path);
            ui.heading(egui::RichText::new(format!("{} {}", icon, file_name)).size(15.0).strong());

            if self.is_dirty {
                ui.label(egui::RichText::new("• Modified").color(egui::Color32::GOLD).size(11.0));
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Save button
                let save_btn = ui.add_enabled(
                    self.is_dirty,
                    egui::Button::new(format!("{} Save", Icons::SAVE)).fill(egui::Color32::from_rgb(40, 140, 240)),
                );
                if save_btn.clicked() {
                    if fs::write(&self.path, &self.content).is_ok() {
                        self.original_content = self.content.clone();
                        self.is_dirty = false;
                    }
                }

                // Toggle Preview mode for Markdown files
                if self.path.ends_with(".md") || self.path.ends_with(".markdown") {
                    let text = if self.preview_mode {
                        format!("{} Edit Code", Icons::EDIT)
                    } else {
                        format!("{} Preview MD", Icons::EYE)
                    };
                    if ui.button(text).clicked() {
                        self.preview_mode = !self.preview_mode;
                    }
                }
            });
        });

        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);

        // MAIN CONTENT AREA - PINNED SCROLLBARS TO RIGHT & BOTTOM EDGES WITH AUTO_SHRINK([FALSE, FALSE])
        let avail = ui.available_size();
        egui::Frame::none()
            .fill(ui.style().visuals.window_fill)
            .inner_margin(4.0)
            .show(ui, |ui| {
                egui::ScrollArea::both()
                    .auto_shrink([false, false]) // Pinned flush right & bottom!
                    .id_salt("editor_scroll")
                    .show(ui, |ui| {
                        ui.set_min_size(avail);
                        if self.preview_mode {
                            render_markdown(ui, &self.content);
                        } else {
                            let ext = std::path::Path::new(&self.path)
                                .extension()
                                .and_then(|e| e.to_str())
                                .unwrap_or("");

                            let previous_content = self.content.clone();

                            // Syntax Highlighted Layouter
                            let mut layouter = |ui: &egui::Ui, string: &str, _wrap_width: f32| {
                                let layout_job = crate::ui::highlighter::highlight_code(ui, string, ext);
                                ui.fonts(|f| f.layout_job(layout_job))
                            };

                            let response = ui.add(
                                egui::TextEdit::multiline(&mut self.content)
                                    .layouter(&mut layouter)
                                    .desired_width(ui.available_width())
                                    .frame(false),
                            );

                            if response.changed() || self.content != previous_content {
                                self.is_dirty = self.content != self.original_content;
                            }
                        }
                    });
            });

        None
    }
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
