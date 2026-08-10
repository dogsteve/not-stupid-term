use eframe::egui;

use crate::core::pty::PtySession;
use crate::ui::alias;
use crate::ui::icons::Icons;
use crate::ui::window_framework::{WindowAction, WindowApp};

/// A single command block in the notebook. Stores raw PTY bytes.
struct NotebookBlock {
    /// The command the user typed (displayed as header).
    command: String,
    /// Raw bytes received from PTY for this command's execution.
    raw_output: Vec<u8>,
    /// Whether CommandEnd has been received.
    is_complete: bool,
    /// If true, this block is a "clear" spacer (renders as viewport-height gap).
    is_clear_marker: bool,
    /// Cached rendered text. Invalidated when raw_output changes or cols change.
    cache: Option<(u16, usize, String)>, // (cols, byte_len, rendered)
}

impl NotebookBlock {
    fn new(command: String) -> Self {
        Self {
            command,
            raw_output: Vec::new(),
            is_complete: false,
            is_clear_marker: false,
            cache: None,
        }
    }

    /// Render raw_output through a temporary vt100 parser and return clean text.
    /// Results are cached until raw_output changes or cols change.
    fn rendered_text(&mut self, cols: u16) -> String {
        // Check cache validity
        if let Some((c, len, ref text)) = self.cache {
            if c == cols && len == self.raw_output.len() {
                return text.clone();
            }
        }

        if self.raw_output.is_empty() {
            self.cache = Some((cols, 0, String::new()));
            return String::new();
        }

        // Count lines to size the temporary parser adequately
        let newline_count = self.raw_output.iter().filter(|&&b| b == b'\n').count();
        let rows = (newline_count as u16 + 20).max(24).min(10000);

        let mut parser = vt100::Parser::new(rows, cols, 0);
        parser.process(&self.raw_output);
        let text = parser.screen().contents();
        let mut trimmed = text.trim_end().to_string();

        // Strip echoed command from output (PTY echoes what we type)
        if !self.command.is_empty() {
            let cmd_trimmed = self.command.trim();

            // Try to strip echo line(s) from the beginning
            let mut lines: Vec<&str> = trimmed.lines().collect();
            let mut stripped = 0;

            // Strip leading lines that match the echoed command
            while !lines.is_empty() {
                let line = lines[0].trim();
                // Match: exact command, or with prompt prefix (❯, >, $, %), or "command\r"
                let clean = line
                    .trim_start_matches(|c: char| "❯>$%#→▶ ".contains(c))
                    .trim();
                if clean == cmd_trimmed
                    || clean.starts_with(&format!("{}\r", cmd_trimmed))
                    || (line.contains(cmd_trimmed) && line.len() < cmd_trimmed.len() + 15)
                {
                    lines.remove(0);
                    stripped += 1;
                    // Only strip up to 2 echo lines max
                    if stripped >= 2 { break; }
                } else {
                    break;
                }
            }

            trimmed = lines.join("\n");
        }

        // Strip trailing prompt remnants (lines that are just whitespace + prompt chars)
        while trimmed.ends_with("❯") || trimmed.ends_with("▶") || trimmed.ends_with(">") {
            if let Some(last_newline) = trimmed.rfind('\n') {
                let last_line = trimmed[last_newline + 1..].trim();
                if last_line.len() < 10 {
                    trimmed.truncate(last_newline);
                    trimmed = trimmed.trim_end().to_string();
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        let trimmed = trimmed.trim_end().to_string();
        self.cache = Some((cols, self.raw_output.len(), trimmed.clone()));
        trimmed
    }
}

pub struct TerminalApp {
    pub title: String,
    pub pty: Option<PtySession>,
    /// All notebook blocks, in chronological order (newest last = at bottom).
    blocks: Vec<NotebookBlock>,
    /// Index of the block currently receiving PTY output, if any.
    active_block: Option<usize>,
    /// The command being typed.
    pub command_input: String,
    /// Session command history for recall.
    pub command_history: Vec<String>,
    /// History navigation index (None = not navigating).
    pub history_nav_idx: Option<usize>,
    /// Suggestion index.
    pub suggestion_idx: Option<usize>,
    /// Whether suggestions popup was dismissed.
    pub is_dismissed: bool,
    /// Last known column count.
    last_cols: u16,
    /// Whether we need to scroll to bottom on next frame.
    scroll_to_bottom: bool,
}

impl TerminalApp {
    pub fn new_local(title: impl Into<String>, ctx: &egui::Context) -> Self {
        let pty = PtySession::new(ctx.clone()).ok();
        Self {
            title: title.into(),
            pty,
            blocks: Vec::new(),
            active_block: None,
            command_input: String::new(),
            command_history: Vec::new(),
            history_nav_idx: None,
            suggestion_idx: None,
            is_dismissed: false,
            last_cols: 120,
            scroll_to_bottom: false,
        }
    }

    pub fn new_ssh(title: impl Into<String>, ssh_cmd: String, ctx: &egui::Context) -> Self {
        let mut pty = PtySession::new(ctx.clone()).ok();
        if let Some(p) = &mut pty {
            p.write(format!("{}\n", ssh_cmd).as_bytes());
        }
        Self {
            title: title.into(),
            pty,
            blocks: Vec::new(),
            active_block: None,
            command_input: ssh_cmd,
            command_history: Vec::new(),
            history_nav_idx: None,
            suggestion_idx: None,
            is_dismissed: false,
            last_cols: 120,
            scroll_to_bottom: false,
        }
    }

    /// Ensure there's an active block to receive output. If not, create a startup block.
    fn ensure_active_block(&mut self) {
        if self.active_block.is_none() {
            self.blocks.push(NotebookBlock::new(String::new()));
            self.active_block = Some(self.blocks.len() - 1);
        }
    }

    /// Force-complete the active block and clear active_block.
    fn complete_active_block(&mut self) {
        if let Some(idx) = self.active_block.take() {
            if let Some(block) = self.blocks.get_mut(idx) {
                block.is_complete = true;
            }
        }
    }
}

impl WindowApp for TerminalApp {
    fn title(&self) -> String {
        format!("{} {}", Icons::TERMINAL, self.title)
    }

    fn render(
        &mut self,
        ui: &mut egui::Ui,
        _ctx: &egui::Context,
        config: &mut crate::ui::settings::AppConfig,
    ) -> Option<WindowAction> {
        if self.pty.is_none() {
            ui.centered_and_justified(|ui| {
                ui.label("Failed to initialize PTY session.");
            });
            return None;
        }

        // === Process PTY events (collect first, then process) ===
        let mut had_output = false;
        let mut cmd_end = false;
        let mut clear_screen = false;
        let mut output_chunks: Vec<Vec<u8>> = Vec::new();
        {
            let pty = self.pty.as_mut().unwrap();
            while let Ok(event) = pty.receiver.try_recv() {
                match event {
                    crate::core::pty::PtyEvent::Output(data) => {
                        output_chunks.push(data);
                        had_output = true;
                    }
                    crate::core::pty::PtyEvent::CommandStart => {}
                    crate::core::pty::PtyEvent::CommandEnd { .. } => {
                        cmd_end = true;
                    }
                    crate::core::pty::PtyEvent::ClearScreen => {
                        clear_screen = true;
                    }
                }
            }
        }
        // Now apply output chunks (pty borrow released)
        if !output_chunks.is_empty() {
            self.ensure_active_block();
            if let Some(idx) = self.active_block {
                if let Some(block) = self.blocks.get_mut(idx) {
                    for chunk in &output_chunks {
                        block.raw_output.extend_from_slice(chunk);
                    }
                }
            }
        }

        if had_output {
            ui.ctx().request_repaint();
        }

        // Handle CommandEnd: mark active block as complete
        if cmd_end {
            self.complete_active_block();
        }

        // Handle ClearScreen: insert a spacer block that pushes content above the fold
        if clear_screen {
            self.complete_active_block();
            // Insert a clear marker block — renders as viewport-height spacer
            let mut marker = NotebookBlock::new(String::new());
            marker.is_clear_marker = true;
            marker.is_complete = true;
            self.blocks.push(marker);
            self.scroll_to_bottom = true;
        }

        // === Sizing ===
        let avail = ui.available_size();
        let char_width = config.font_size * 0.6;
        let cols = (avail.x / char_width).max(40.0) as u16;
        if cols != self.last_cols {
            self.last_cols = cols;
            if let Some(pty) = self.pty.as_mut() {
                pty.resize(50, cols);
            }
        }

        // === Theme colors ===
        let is_dark = _ctx.style().visuals.dark_mode;
        let text_color = if is_dark {
            egui::Color32::from_rgb(210, 210, 210)
        } else {
            egui::Color32::from_rgb(30, 30, 30)
        };
        let cmd_color = if is_dark {
            egui::Color32::from_rgb(130, 220, 130)
        } else {
            egui::Color32::from_rgb(0, 140, 60)
        };
        let dim_color = if is_dark {
            egui::Color32::from_gray(100)
        } else {
            egui::Color32::from_gray(140)
        };
        let separator_color = if is_dark {
            egui::Color32::from_gray(50)
        } else {
            egui::Color32::from_gray(215)
        };
        // Input bar: use the window/panel background with subtle contrast
        let input_bg = if is_dark {
            egui::Color32::from_gray(30)
        } else {
            egui::Color32::from_gray(240)
        };
        let input_border = if is_dark {
            egui::Color32::from_gray(60)
        } else {
            egui::Color32::from_gray(190)
        };

        let font_id = egui::FontId::monospace(config.font_size);

        // === Layout: bottom_up ===
        ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {

            // ====== INPUT BAR ======
            ui.add_space(4.0);
            let input_frame = egui::Frame::default()
                .fill(input_bg)
                .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                .rounding(8.0)
                .stroke(egui::Stroke::new(1.0, input_border));

            input_frame.show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("❯")
                            .color(cmd_color)
                            .strong()
                            .family(egui::FontFamily::Monospace)
                            .size(config.font_size),
                    );

                    let input_id = ui.id().with("notebook_cmd");

                    let mut enter_pressed = false;
                    let mut tab_pressed = false;
                    let mut esc_pressed = false;
                    let mut up_pressed = false;
                    let mut down_pressed = false;

                    if ui.memory(|m| m.has_focus(input_id)) {
                        ui.input(|i| {
                            if i.key_pressed(egui::Key::Enter) { enter_pressed = true; }
                            if i.key_pressed(egui::Key::Tab) { tab_pressed = true; }
                            if i.key_pressed(egui::Key::Escape) { esc_pressed = true; }
                            if i.key_pressed(egui::Key::ArrowUp) { up_pressed = true; }
                            if i.key_pressed(egui::Key::ArrowDown) { down_pressed = true; }
                        });
                    }

                    let prev_input = self.command_input.clone();
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.command_input)
                            .id(input_id)
                            .hint_text("Type command…")
                            .font(egui::FontId::monospace(config.font_size))
                            .desired_width(ui.available_width())
                            .frame(false)
                            .text_color(text_color),
                    );

                    // Auto-focus
                    if !ui.memory(|m| m.has_focus(input_id)) {
                        ui.memory_mut(|m| m.request_focus(input_id));
                    }

                    if self.command_input != prev_input {
                        self.is_dismissed = false;
                        self.suggestion_idx = None;
                        self.history_nav_idx = None;
                    }
                    if esc_pressed {
                        self.is_dismissed = true;
                        self.suggestion_idx = None;
                    }

                    // --- History navigation with Up/Down (when no suggestions) ---
                    let suggestions = if !self.command_input.is_empty() && !self.is_dismissed {
                        alias::get_suggestions(&self.command_input, &self.command_history)
                    } else {
                        Vec::new()
                    };

                    if suggestions.is_empty() {
                        // Navigate command_history with arrows
                        if up_pressed && !self.command_history.is_empty() {
                            let idx = match self.history_nav_idx {
                                Some(i) if i > 0 => i - 1,
                                Some(i) => i,
                                None => self.command_history.len() - 1,
                            };
                            self.history_nav_idx = Some(idx);
                            self.command_input = self.command_history[idx].clone();
                        }
                        if down_pressed {
                            if let Some(i) = self.history_nav_idx {
                                if i + 1 < self.command_history.len() {
                                    self.history_nav_idx = Some(i + 1);
                                    self.command_input = self.command_history[i + 1].clone();
                                } else {
                                    self.history_nav_idx = None;
                                    self.command_input.clear();
                                }
                            }
                        }
                    }

                    // --- Suggestions popup ---
                    if !suggestions.is_empty() {
                        if tab_pressed {
                            let idx = self.suggestion_idx.map(|i| (i + 1) % suggestions.len()).unwrap_or(0);
                            self.suggestion_idx = Some(idx);
                            self.command_input = suggestions[idx].fill_cmd.clone();
                        }
                        if up_pressed {
                            let idx = self.suggestion_idx
                                .map(|i| if i == 0 { suggestions.len() - 1 } else { i - 1 })
                                .unwrap_or(suggestions.len() - 1);
                            self.suggestion_idx = Some(idx);
                        }
                        if down_pressed {
                            let idx = self.suggestion_idx.map(|i| (i + 1) % suggestions.len()).unwrap_or(0);
                            self.suggestion_idx = Some(idx);
                        }

                        egui::popup::popup_below_widget(
                            ui,
                            ui.id().with("cmd_popup"),
                            &response,
                            egui::PopupCloseBehavior::CloseOnClick,
                            |ui| {
                                ui.set_min_width(300.0);
                                for (idx, item) in suggestions.iter().enumerate() {
                                    let is_sel = self.suggestion_idx == Some(idx);
                                    let label_str = if item.is_alias {
                                        format!("{} {} {}", Icons::COMMAND, item.display, item.detail)
                                    } else {
                                        format!("{} {}", Icons::HISTORY, item.display)
                                    };
                                    let item_resp = ui.selectable_label(is_sel, &label_str);
                                    if item_resp.clicked() {
                                        self.command_input = item.fill_cmd.clone();
                                        self.suggestion_idx = None;
                                        ui.memory_mut(|m| m.request_focus(input_id));
                                    }
                                }
                            },
                        );
                        ui.memory_mut(|m| m.open_popup(ui.id().with("cmd_popup")));
                    }

                    // --- Execute on Enter ---
                    if enter_pressed && !self.command_input.is_empty() {
                        // Force-complete any previous active block
                        self.complete_active_block();

                        let user_aliases = alias::load_user_shell_aliases();
                        let final_cmd = if let Some(a) = user_aliases.iter().find(|a| a.name == self.command_input.trim()) {
                            a.target.clone()
                        } else {
                            self.command_input.clone()
                        };

                        // Create new block
                        self.blocks.push(NotebookBlock::new(self.command_input.clone()));
                        self.active_block = Some(self.blocks.len() - 1);

                        // Send to PTY
                        if let Some(pty) = self.pty.as_mut() {
                            pty.write(format!("{}\n", final_cmd).as_bytes());
                        }

                        self.command_history.push(final_cmd);
                        self.command_input.clear();
                        self.suggestion_idx = None;
                        self.history_nav_idx = None;
                        self.is_dismissed = false;
                        self.scroll_to_bottom = true;
                        ui.memory_mut(|m| m.request_focus(input_id));
                    }
                });
            });
            ui.add_space(2.0);

            // ====== NOTEBOOK BLOCKS (scroll area fills remaining space) ======
            egui::ScrollArea::vertical()
                .stick_to_bottom(true)
                .auto_shrink([false, false])
                .id_salt("notebook_scroll")
                .show(ui, |ui| {
                    // Force top-down inside scroll area (parent is bottom_up)
                    ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_min_width(ui.available_width());

                    let block_count = self.blocks.len();
                    let viewport_height = avail.y;
                    for i in 0..block_count {
                        let block = &mut self.blocks[i];
                        let is_active = self.active_block == Some(i);

                        // Render clear markers as viewport-height spacers
                        if block.is_clear_marker {
                            ui.add_space(viewport_height);
                            continue;
                        }

                        // Skip startup blocks with empty command AND empty output
                        let rendered = block.rendered_text(cols);
                        if block.command.is_empty() && rendered.is_empty() {
                            continue;
                        }

                        // === Block container with hover highlight ===
                        let block_id = ui.id().with(("block", i));
                        let block_start = ui.cursor().min;

                        // Command header
                        if !block.command.is_empty() {
                            let mut job = egui::text::LayoutJob::default();
                            job.append("❯ ", 0.0, egui::TextFormat {
                                font_id: font_id.clone(),
                                color: cmd_color,
                                ..Default::default()
                            });
                            job.append(&block.command, 0.0, egui::TextFormat {
                                font_id: font_id.clone(),
                                color: text_color,
                                ..Default::default()
                            });

                            if is_active {
                                job.append("  ⏳", 0.0, egui::TextFormat {
                                    font_id: font_id.clone(),
                                    color: dim_color,
                                    ..Default::default()
                                });
                            }

                            job.wrap = egui::text::TextWrapping {
                                max_width: ui.available_width(),
                                ..Default::default()
                            };
                            ui.label(job);
                            ui.add_space(2.0);
                        }

                        // Output text
                        if !rendered.is_empty() {
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(&rendered)
                                        .family(egui::FontFamily::Monospace)
                                        .size(config.font_size)
                                        .color(text_color),
                                ).wrap(),
                            );
                        }

                        // Calculate block rect and draw hover highlight BEHIND content
                        let block_end = ui.cursor().min;
                        let block_rect = egui::Rect::from_min_max(
                            egui::pos2(ui.min_rect().left(), block_start.y),
                            egui::pos2(ui.min_rect().right(), block_end.y),
                        );
                        let block_resp = ui.interact(block_rect, block_id, egui::Sense::hover());
                        if block_resp.hovered() {
                            let hover_color = if is_dark {
                                egui::Color32::from_white_alpha(8)
                            } else {
                                egui::Color32::from_black_alpha(8)
                            };
                            ui.painter().rect_filled(block_rect.expand2(egui::vec2(4.0, 2.0)), 4.0, hover_color);
                        }

                        // Separator
                        ui.add_space(6.0);
                        let sep_rect = ui.available_rect_before_wrap();
                        ui.painter().hline(
                            sep_rect.x_range(),
                            sep_rect.top(),
                            egui::Stroke::new(0.5, separator_color),
                        );
                        ui.add_space(6.0);
                    }
                    }); // end top_down
                });
        });

        None
    }
}
