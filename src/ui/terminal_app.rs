use eframe::egui;
use egui::text_edit::TextEditState;

use crate::core::pty::PtySession;
use crate::ui::completion::{self, FrecencyStore, SuggestionSource};
use crate::ui::icons::Icons;
use crate::ui::window_framework::{WindowAction, WindowApp};

/// A single command block in the notebook. Stores raw PTY bytes and pre-rendered lines.
pub(crate) struct NotebookBlock {
    /// The command the user typed (displayed as header).
    pub(crate) command: String,
    /// Raw bytes received from PTY for this command's execution.
    pub(crate) raw_output: Vec<u8>,
    /// Whether CommandEnd has been received.
    pub(crate) is_complete: bool,
    /// If true, this block is a "clear" spacer (renders as viewport-height gap).
    pub(crate) is_clear_marker: bool,
    /// Pre-rendered LayoutJob lines for fast 0ms rendering in egui.
    cached_lines: Vec<egui::text::LayoutJob>,
    /// Pre-combined multiline LayoutJob — shared via Arc to avoid per-frame clone.
    cached_combined_job: Option<std::sync::Arc<egui::text::LayoutJob>>,
    /// Pre-built header job ("❯ command") — stable between frames.
    cached_cmd_job: Option<egui::text::LayoutJob>,
    /// Cached (cols, generation) to invalidate caches on new output.
    cache_key: Option<(u16, u32)>,
    /// Monotonic counter incremented each time raw_output grows.
    output_generation: u32,
}

impl NotebookBlock {
    pub(crate) fn new(command: String) -> Self {
        Self {
            command,
            raw_output: Vec::new(),
            is_complete: false,
            is_clear_marker: false,
            cached_lines: Vec::new(),
            cached_combined_job: None,
            cached_cmd_job: None,
            cache_key: None,
            output_generation: 0,
        }
    }

    /// Render raw_output into pre-built LayoutJob line jobs.
    /// Results are cached until raw_output changes (tracked via generation) or cols change.
    fn rendered_lines(&mut self, cols: u16, font_id: &egui::FontId, text_color: egui::Color32) -> &[egui::text::LayoutJob] {
        if let Some((c, generation)) = self.cache_key {
            if c == cols && generation == self.output_generation {
                return &self.cached_lines;
            }
        }

        if self.raw_output.is_empty() {
            self.cached_lines.clear();
            self.cache_key = Some((cols, self.output_generation));
            return &self.cached_lines;
        }

        // Performance Optimization: slice raw_output to last 64KB for ultra-fast incremental parsing
        let bytes_to_parse = if self.raw_output.len() > 65536 {
            let start = self.raw_output.len() - 65536;
            let aligned = self.raw_output[start..]
                .iter()
                .position(|&b| b == b'\n')
                .map(|p| start + p + 1)
                .unwrap_or(start);
            &self.raw_output[aligned..]
        } else {
            &self.raw_output[..]
        };

        // Count lines to size the temporary parser adequately
        let newline_count = bytes_to_parse.iter().filter(|&&b| b == b'\n').count();
        let rows = (newline_count as u16 + 20).max(24).min(2000);

        let mut parser = vt100::Parser::new(rows, cols, 0);
        parser.process(bytes_to_parse);
        let screen = parser.screen();
        let (_screen_rows, screen_cols) = screen.size();
        let text = screen.contents();
        let mut lines_str: Vec<&str> = text.lines().map(|l| l.trim_end()).collect();
        while lines_str.last().map_or(false, |l| l.is_empty()) {
            lines_str.pop();
        }

        // Strip echoed command from top lines
        let mut start_row = 0;
        if !self.command.is_empty() && !lines_str.is_empty() {
            let cmd_trimmed = self.command.trim();
            for r in 0..lines_str.len().min(2) {
                let clean = lines_str[r]
                    .trim_start_matches(|c: char| "❯>$%#→▶ ".contains(c))
                    .trim();
                if clean == cmd_trimmed
                    || clean.starts_with(&format!("{}\r", cmd_trimmed))
                    || (clean.contains(cmd_trimmed) && clean.len() < cmd_trimmed.len() + 15)
                {
                    start_row = r + 1;
                } else {
                    break;
                }
            }
        }

        let mut new_lines = Vec::new();

        for r in start_row..lines_str.len() {
            let line_str = lines_str[r];
            if line_str.is_empty() {
                let mut job = egui::text::LayoutJob::default();
                job.append(" ", 0.0, egui::TextFormat {
                    font_id: font_id.clone(),
                    color: text_color,
                    ..Default::default()
                });
                new_lines.push(job);
                continue;
            }

            let mut job = egui::text::LayoutJob::default();
            job.wrap = egui::text::TextWrapping {
                max_width: f32::INFINITY,
                ..Default::default()
            };

            let mut current_str = String::new();
            let mut current_fmt: Option<egui::TextFormat> = None;

            for c in 0..screen_cols {
                let cell = screen.cell(r as u16, c);
                let (ch, cell_fg, cell_bg, _bold, underline) = if let Some(cell) = cell {
                    let contents = cell.contents();
                    let ch = if contents.is_empty() { " " } else { contents };
                    (ch, cell.fgcolor(), cell.bgcolor(), cell.bold(), cell.underline())
                } else {
                    (" ", vt100::Color::Default, vt100::Color::Default, false, false)
                };

                let fg = vt_color_to_color32(cell_fg, text_color);
                let bg = vt_color_to_color32(cell_bg, egui::Color32::TRANSPARENT);
                let background = if bg == egui::Color32::TRANSPARENT { egui::Color32::TRANSPARENT } else { bg };

                let fmt = egui::TextFormat {
                    font_id: font_id.clone(),
                    color: fg,
                    background,
                    line_height: Some(font_id.size * 1.05),
                    underline: if underline { egui::Stroke::new(1.0, fg) } else { egui::Stroke::NONE },
                    ..Default::default()
                };

                if Some(&fmt) == current_fmt.as_ref() {
                    current_str.push_str(ch);
                } else {
                    if let Some(prev) = current_fmt.take() {
                        job.append(&current_str, 0.0, prev);
                        current_str.clear();
                    }
                    current_fmt = Some(fmt);
                    current_str.push_str(ch);
                }
            }

            if let Some(fmt) = current_fmt {
                job.append(&current_str, 0.0, fmt);
            }

            new_lines.push(job);
        }

        self.cached_lines = new_lines;
        self.cache_key = Some((cols, self.output_generation));
        &self.cached_lines
    }

    fn rendered_combined_job(&mut self, cols: u16, font_id: &egui::FontId, text_color: egui::Color32) -> Option<std::sync::Arc<egui::text::LayoutJob>> {
        if let Some((c, generation)) = self.cache_key {
            if c == cols && generation == self.output_generation && self.cached_combined_job.is_some() {
                return self.cached_combined_job.clone(); // Arc clone — O(1), no data copy
            }
        }

        let lines_to_draw = self.rendered_lines(cols, font_id, text_color);
        if lines_to_draw.is_empty() {
            self.cached_combined_job = None;
            return None;
        }

        let mut combined_job = egui::text::LayoutJob::default();
        for (idx, line_job) in lines_to_draw.iter().enumerate() {
            if idx > 0 {
                let start = combined_job.text.len();
                combined_job.text.push('\n');
                combined_job.sections.push(egui::text::LayoutSection {
                    leading_space: 0.0,
                    byte_range: start..combined_job.text.len(),
                    format: egui::TextFormat {
                        font_id: font_id.clone(),
                        color: text_color,
                        line_height: Some(font_id.size * 1.05),
                        ..Default::default()
                    },
                });
            }
            let offset = combined_job.text.len();
            combined_job.text.push_str(&line_job.text);
            for sec in &line_job.sections {
                let mut fmt = sec.format.clone();
                fmt.line_height = Some(font_id.size * 1.05);
                combined_job.sections.push(egui::text::LayoutSection {
                    leading_space: sec.leading_space,
                    byte_range: (sec.byte_range.start + offset)..(sec.byte_range.end + offset),
                    format: fmt,
                });
            }
        }

        self.cached_combined_job = Some(std::sync::Arc::new(combined_job));
        self.cached_combined_job.clone()
    }

    /// Returns the cached command-header LayoutJob, building it once per command.
    fn cmd_header_job(
        &mut self,
        font_id: &egui::FontId,
        cmd_color: egui::Color32,
        text_color: egui::Color32,
        dim_color: egui::Color32,
        is_active: bool,
    ) -> egui::text::LayoutJob {
        // Rebuild if not cached yet or if active status changed (spinner shown/hidden).
        if let Some(ref job) = self.cached_cmd_job {
            if !is_active {
                return job.clone(); // Cheap — job is small (one command line)
            }
        }
        let mut job = egui::text::LayoutJob::default();
        job.append("❯ ", 0.0, egui::TextFormat {
            font_id: font_id.clone(),
            color: cmd_color,
            line_height: Some(font_id.size * 1.05),
            ..Default::default()
        });
        job.append(&self.command, 0.0, egui::TextFormat {
            font_id: font_id.clone(),
            color: text_color,
            line_height: Some(font_id.size * 1.05),
            ..Default::default()
        });
        if is_active {
            job.append("  ⏳", 0.0, egui::TextFormat {
                font_id: font_id.clone(),
                color: dim_color,
                ..Default::default()
            });
        } else {
            // Only cache the non-active (final) version — active version changes.
            self.cached_cmd_job = Some(job.clone());
        }
        job
    }

    /// Appends raw PTY bytes and bumps the generation counter to invalidate caches.
    pub(crate) fn extend_raw_output(&mut self, data: &[u8]) {
        self.raw_output.extend_from_slice(data);
        self.output_generation = self.output_generation.wrapping_add(1);
    }
}

pub struct TerminalApp {
    pub title: String,
    pub pty: Option<PtySession>,
    /// All notebook blocks, in chronological order (newest last = at bottom).
    pub(crate) blocks: Vec<NotebookBlock>,
    /// Index of the block currently receiving PTY output, if any.
    active_block: Option<usize>,
    /// The raw text currently shown in the input box (may be a suggestion preview).
    pub command_input: String,
    /// What the user actually typed – used as the stable query for suggestions.
    /// Arrow navigation updates `command_input` (preview) but NOT this field.
    typed_input: String,
    /// Session command history for recall.
    pub command_history: Vec<String>,
    /// History navigation index (None = not navigating).
    pub history_nav_idx: Option<usize>,
    /// Suggestion index.
    pub suggestion_idx: Option<usize>,
    /// Whether suggestions popup was dismissed.
    pub is_dismissed: bool,
    /// True while a command has been sent and its CommandEnd has not yet fired.
    pub is_running: bool,
    /// The command line currently executing, if any. Used for dynamic window title.
    pub running_command: Option<String>,
    /// True when a running application requests Alternate Screen Buffer (TUI apps like nano, vim, htop).
    pub is_tui_mode: bool,
    /// Frecency tracker – boosts recently-used commands in suggestions.
    frecency: FrecencyStore,
    /// Last known column count.
    last_cols: u16,
    /// Last known row count.
    last_rows: u16,
    /// Whether we need to scroll to bottom on next frame.
    scroll_to_bottom: bool,
    /// Persistent vt100 parser for live raw terminal screen rendering.
    raw_screen: vt100::Parser,
    /// Monotonic counter — incremented every time raw_screen receives new data.
    /// When the counter is the same as tui_cache_generation, tui_cached_jobs is valid.
    tui_screen_generation: u64,
    /// Cached per-row LayoutJobs for TUI mode. Rebuilt only when tui_screen_generation changes.
    tui_cached_jobs: Vec<egui::text::LayoutJob>,
    /// The generation at which tui_cached_jobs was last built.
    tui_cache_generation: u64,
    pub focus_input_requested: bool,
    /// Set to true when command_input is programmatically replaced (suggestion/history).
    /// Causes cursor to be moved to end-of-text after TextEdit renders next frame.
    need_cursor_reset: bool,
    /// Cached suggestions — recomputed only when `typed_input` changes, not every frame.
    cached_suggestions: Vec<completion::SuggestionItem>,
    /// The `typed_input` value at the time `cached_suggestions` was built.
    cached_suggestions_for: String,
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
            typed_input: String::new(),
            command_history: Vec::new(),
            history_nav_idx: None,
            suggestion_idx: None,
            need_cursor_reset: false,
            is_dismissed: false,
            is_running: false,
            running_command: None,
            is_tui_mode: false,
            frecency: FrecencyStore::default(),
            last_cols: 120,
            last_rows: 24,
            scroll_to_bottom: false,
            raw_screen: vt100::Parser::new(24, 120, 0),
            tui_screen_generation: 0,
            tui_cached_jobs: Vec::new(),
            tui_cache_generation: u64::MAX, // force first build
            focus_input_requested: true,
            cached_suggestions: Vec::new(),
            cached_suggestions_for: String::new(),
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
            command_input: ssh_cmd.clone(),
            typed_input: ssh_cmd,
            command_history: Vec::new(),
            history_nav_idx: None,
            suggestion_idx: None,
            need_cursor_reset: false,
            is_dismissed: false,
            is_running: false,
            running_command: None,
            is_tui_mode: false,
            frecency: FrecencyStore::default(),
            last_cols: 120,
            last_rows: 24,
            scroll_to_bottom: false,
            raw_screen: vt100::Parser::new(24, 120, 0),
            tui_screen_generation: 0,
            tui_cached_jobs: Vec::new(),
            tui_cache_generation: u64::MAX,
            focus_input_requested: true,
            cached_suggestions: Vec::new(),
            cached_suggestions_for: String::new(),
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

    /// Returns true when a command is actively running (set on Enter, cleared on CommandEnd).
    pub fn has_running_command(&self) -> bool {
        self.is_running
    }

    /// Send SIGKILL to the PTY process and all its children.
    pub fn kill_all(&self) {
        if let Some(pty) = &self.pty {
            pty.kill_process();
        }
    }
}

fn ansi_256_to_color32(idx: u8) -> egui::Color32 {
    match idx {
        0 => egui::Color32::from_rgb(0, 0, 0),
        1 => egui::Color32::from_rgb(205, 0, 0),
        2 => egui::Color32::from_rgb(0, 205, 0),
        3 => egui::Color32::from_rgb(205, 205, 0),
        4 => egui::Color32::from_rgb(0, 0, 238),
        5 => egui::Color32::from_rgb(205, 0, 205),
        6 => egui::Color32::from_rgb(0, 205, 205),
        7 => egui::Color32::from_rgb(229, 229, 229),
        8 => egui::Color32::from_rgb(127, 127, 127),
        9 => egui::Color32::from_rgb(255, 0, 0),
        10 => egui::Color32::from_rgb(0, 255, 0),
        11 => egui::Color32::from_rgb(255, 255, 0),
        12 => egui::Color32::from_rgb(92, 92, 255),
        13 => egui::Color32::from_rgb(255, 0, 255),
        14 => egui::Color32::from_rgb(0, 255, 255),
        15 => egui::Color32::from_rgb(255, 255, 255),
        16..=231 => {
            let i = idx - 16;
            let r_idx = (i / 36) as usize;
            let g_idx = ((i % 36) / 6) as usize;
            let b_idx = (i % 6) as usize;
            let steps = [0, 95, 135, 175, 215, 255];
            egui::Color32::from_rgb(steps[r_idx], steps[g_idx], steps[b_idx])
        }
        232..=255 => {
            let v = 8 + (idx - 232) * 10;
            egui::Color32::from_rgb(v, v, v)
        }
    }
}

fn vt_color_to_color32(color: vt100::Color, default_color: egui::Color32) -> egui::Color32 {
    match color {
        vt100::Color::Default => default_color,
        vt100::Color::Idx(idx) => ansi_256_to_color32(idx),
        vt100::Color::Rgb(r, g, b) => egui::Color32::from_rgb(r, g, b),
    }
}

fn key_to_ctrl_byte(key: egui::Key) -> Option<u8> {
    match key {
        egui::Key::A => Some(0x01),
        egui::Key::B => Some(0x02),
        egui::Key::C => Some(0x03),
        egui::Key::D => Some(0x04),
        egui::Key::E => Some(0x05),
        egui::Key::F => Some(0x06),
        egui::Key::G => Some(0x07),
        egui::Key::H => Some(0x08),
        egui::Key::I => Some(0x09),
        egui::Key::J => Some(0x0A),
        egui::Key::K => Some(0x0B),
        egui::Key::L => Some(0x0C),
        egui::Key::M => Some(0x0D),
        egui::Key::N => Some(0x0E),
        egui::Key::O => Some(0x0F),
        egui::Key::P => Some(0x10),
        egui::Key::Q => Some(0x11),
        egui::Key::R => Some(0x12),
        egui::Key::S => Some(0x13),
        egui::Key::T => Some(0x14),
        egui::Key::U => Some(0x15),
        egui::Key::V => Some(0x16),
        egui::Key::W => Some(0x17),
        egui::Key::X => Some(0x18),
        egui::Key::Y => Some(0x19),
        egui::Key::Z => Some(0x1A),
        egui::Key::OpenBracket => Some(0x1B),
        egui::Key::Backslash => Some(0x1C),
        egui::Key::CloseBracket => Some(0x1D),
        _ => None,
    }
}

fn special_key_to_bytes(key: egui::Key) -> Option<&'static [u8]> {
    match key {
        egui::Key::ArrowUp => Some(b"\x1b[A"),
        egui::Key::ArrowDown => Some(b"\x1b[B"),
        egui::Key::ArrowRight => Some(b"\x1b[C"),
        egui::Key::ArrowLeft => Some(b"\x1b[D"),
        egui::Key::Home => Some(b"\x1b[H"),
        egui::Key::End => Some(b"\x1b[F"),
        egui::Key::PageUp => Some(b"\x1b[5~"),
        egui::Key::PageDown => Some(b"\x1b[6~"),
        egui::Key::Insert => Some(b"\x1b[2~"),
        egui::Key::Delete => Some(b"\x1b[3~"),
        egui::Key::Enter => Some(b"\r"),
        egui::Key::Backspace => Some(b"\x7f"),
        egui::Key::Tab => Some(b"\t"),
        egui::Key::Escape => Some(b"\x1b"),
        egui::Key::F1 => Some(b"\x1bOP"),
        egui::Key::F2 => Some(b"\x1bOQ"),
        egui::Key::F3 => Some(b"\x1bOR"),
        egui::Key::F4 => Some(b"\x1bOS"),
        egui::Key::F5 => Some(b"\x1b[15~"),
        egui::Key::F6 => Some(b"\x1b[17~"),
        egui::Key::F7 => Some(b"\x1b[18~"),
        egui::Key::F8 => Some(b"\x1b[19~"),
        egui::Key::F9 => Some(b"\x1b[20~"),
        egui::Key::F10 => Some(b"\x1b[21~"),
        egui::Key::F11 => Some(b"\x1b[23~"),
        egui::Key::F12 => Some(b"\x1b[24~"),
        _ => None,
    }
}

impl WindowApp for TerminalApp {
    fn title(&self) -> String {
        if self.is_running {
            if let Some(cmd) = &self.running_command {
                let trimmed = cmd.lines().next().unwrap_or("").trim();
                if !trimmed.is_empty() {
                    return format!("{} {}", Icons::TERMINAL, trimmed);
                }
            }
        }
        format!("{} {}", Icons::TERMINAL, self.title)
    }

    fn window_type(&self) -> &'static str {
        "terminal"
    }

    fn save_state(&self) -> Option<serde_json::Value> {
        use base64::Engine;
        let blocks_json: Vec<serde_json::Value> = self
            .blocks
            .iter()
            .map(|b| {
                serde_json::json!({
                    "command": b.command,
                    "raw_output": base64::engine::general_purpose::STANDARD.encode(&b.raw_output),
                    "is_complete": b.is_complete,
                    "is_clear_marker": b.is_clear_marker,
                })
            })
            .collect();

        Some(serde_json::json!({
            "title": self.title,
            "command_history": self.command_history,
            "blocks": blocks_json,
        }))
    }

    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn as_any_mut(&mut self) -> Option<&mut dyn std::any::Any> {
        Some(self)
    }

    /// Send Ctrl+C (ETX byte) to the PTY. Called from global ctx in app.rs or keyboard listeners.
    fn interrupt(&mut self) {
        if let Some(pty) = self.pty.as_mut() {
            pty.send_interrupt();
        }
        self.complete_active_block();
        self.is_running = false;
        self.running_command = None;
        self.is_tui_mode = false;
        self.command_input.clear();
        self.typed_input.clear();
        self.suggestion_idx = None;
        self.history_nav_idx = None;
        self.is_dismissed = false;
    }

    fn is_running(&self) -> bool {
        self.is_running
    }

    fn kill_all_processes(&mut self) {
        if let Some(pty) = &self.pty {
            pty.kill_process();
        }
        self.is_running = false;
        self.running_command = None;
        self.is_tui_mode = false;
    }

    fn on_focus(&mut self, _ctx: &egui::Context) {
        self.focus_input_requested = true;
    }

    fn render(
        &mut self,
        ui: &mut egui::Ui,
        _ctx: &egui::Context,
        config: &mut crate::ui::settings::AppConfig,
        _undo: &mut crate::ui::undo_manager::UndoManager,
    ) -> Option<WindowAction> {
        // Prevent content from pushing the window horizontally to avoid infinite resize loops
        let initial_avail_x = ui.available_width();
        ui.set_max_width(initial_avail_x);

        if self.pty.is_none() {
            ui.centered_and_justified(|ui| {
                ui.label("Failed to initialize PTY session.");
            });
            return None;
        }

        // === Global Keyboard Event Check (Ctrl+C) ===
        // Use targeted key check instead of cloning all events every frame.
        let global_ctrl_c = _ctx.input(|i| {
            let is_ctrl = i.modifiers.ctrl || (cfg!(target_os = "macos") && i.modifiers.command);
            is_ctrl && i.key_pressed(egui::Key::C)
        });
        if global_ctrl_c {
            self.interrupt();
        }

        // === Process PTY events ===
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
                    crate::core::pty::PtyEvent::CommandStart => {
                        self.is_running = true;
                    }
                    crate::core::pty::PtyEvent::CommandEnd { exit_code: _ } => {
                        cmd_end = true;
                        self.is_tui_mode = false;
                    }
                    crate::core::pty::PtyEvent::ClearScreen => {
                        clear_screen = true;
                    }
                    crate::core::pty::PtyEvent::EnterAltScreen => {
                        self.is_tui_mode = true;
                    }
                    crate::core::pty::PtyEvent::LeaveAltScreen => {
                        self.is_tui_mode = false;
                    }
                }
            }
        }

        // Apply output chunks
        if !output_chunks.is_empty() {
            self.ensure_active_block();
            if let Some(idx) = self.active_block {
                if let Some(block) = self.blocks.get_mut(idx) {
                    for chunk in &output_chunks {
                        block.extend_raw_output(chunk);
                        self.raw_screen.process(chunk);
                        self.tui_screen_generation = self.tui_screen_generation.wrapping_add(1);
                    }
                }
            }
        }

        if had_output {
            if self.is_running {
                self.scroll_to_bottom = true;
            }
            // Cap repaints at ~60fps (16ms) to avoid burning CPU on rapid PTY output.
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(16));
        }

        let font_id = egui::FontId::monospace(config.mono_font_size);

        // === Sizing ===
        let avail = ui.available_size();
        let char_width = ui.fonts(|f| f.glyph_width(&font_id, 'M')).max(1.0);
        let line_height = ui.fonts(|f| f.row_height(&font_id)).max(1.0);
        let calculated_cols = ((avail.x - 8.0) / char_width).floor().max(40.0) as u16;
        let calculated_rows = ((avail.y - 8.0) / line_height).floor().max(10.0) as u16;

        if self.is_running {
            if (calculated_cols as i32 - self.last_cols as i32).abs() >= 2
                || (calculated_rows as i32 - self.last_rows as i32).abs() >= 2
            {
                self.last_cols = calculated_cols;
                self.last_rows = calculated_rows;
                if let Some(pty) = self.pty.as_mut() {
                    pty.resize(calculated_rows, calculated_cols);
                }
                self.raw_screen = vt100::Parser::new(calculated_rows, calculated_cols, 0);
            }
        } else {
            if self.last_cols == 0 || calculated_cols != self.last_cols || calculated_rows != self.last_rows {
                self.last_cols = calculated_cols;
                self.last_rows = calculated_rows;
                if let Some(pty) = self.pty.as_mut() {
                    pty.resize(calculated_rows, calculated_cols);
                }
                self.raw_screen = vt100::Parser::new(calculated_rows, calculated_cols, 0);
            }
        }

        let cols = self.last_cols;

        // Handle CommandEnd: mark active block as complete, clear running flag and reset raw_screen
        if cmd_end {
            self.complete_active_block();
            self.is_running = false;
            self.running_command = None;
            self.is_tui_mode = false;
            self.raw_screen = vt100::Parser::new(self.last_rows.max(10), self.last_cols.max(40), 0);
        }

        // Handle ClearScreen: insert a clear spacer block, reset raw_screen parser, and scroll to bottom
        if clear_screen {
            self.complete_active_block();
            let mut marker = NotebookBlock::new(String::new());
            marker.is_clear_marker = true;
            marker.is_complete = true;
            self.blocks.push(marker);
            self.raw_screen = vt100::Parser::new(self.last_rows.max(10), self.last_cols.max(40), 0);
            self.scroll_to_bottom = true;
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

        if self.is_running && self.is_tui_mode {
            // ==========================================
            // FULL RAW TUI MODE (AltScreen Active, e.g. nano/vim/htop)
            // ==========================================
            let raw_id = ui.id().with("full_raw_terminal");

            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .id_salt("tui_raw_scroll")
                .show(ui, |ui| {
                    let raw_rect = ui.available_rect_before_wrap();
                    let response = ui.interact(raw_rect, raw_id, egui::Sense::click_and_drag());
                    if response.clicked() {
                        response.request_focus();
                    }
                    if self.focus_input_requested {
                        ui.memory_mut(|m| m.request_focus(raw_id));
                        self.focus_input_requested = false;
                    }

                    // Keyboard input forwarding — process events in-place to avoid Vec clone.
                    let mut bytes_to_send = Vec::new();
                    let has_focus = ui.memory(|m| m.has_focus(raw_id));
                    if has_focus {
                        ui.input(|i| {
                            let is_ctrl_active = i.modifiers.ctrl || (cfg!(target_os = "macos") && i.modifiers.command);
                            for event in &i.events {
                                match event {
                                    egui::Event::Key { key, pressed: true, modifiers, .. } => {
                                        let is_ctrl = modifiers.ctrl || (cfg!(target_os = "macos") && modifiers.command);
                                        if is_ctrl {
                                            if let Some(ctrl_b) = key_to_ctrl_byte(*key) {
                                                bytes_to_send.push(ctrl_b);
                                            }
                                        } else if let Some(seq) = special_key_to_bytes(*key) {
                                            bytes_to_send.extend_from_slice(seq);
                                        }
                                    }
                                    egui::Event::Text(t) => {
                                        // Only process text if Ctrl is NOT active to prevent duplicate bytes for Ctrl shortcuts
                                        if !is_ctrl_active {
                                            for b in t.bytes() {
                                                if b >= 0x20 || b == b'\t' || b == b'\n' || b == b'\r' {
                                                    bytes_to_send.push(b);
                                                }
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        });
                    }
                    if !bytes_to_send.is_empty() {
                        if let Some(pty) = self.pty.as_mut() {
                            let _ = pty.write(&bytes_to_send);
                        }
                    }

                    // Screen rendering — cached per tui_screen_generation.
                    // Only rebuild LayoutJobs when PTY sent new data (generation changed).
                    // On a static screen (e.g. user is reading output) cost = 0 allocs.
                    let default_bg = if is_dark {
                        egui::Color32::from_rgb(18, 18, 18)
                    } else {
                        egui::Color32::from_rgb(250, 250, 250)
                    };
                    let default_fg = text_color;

                    // Cursor position changes the render even if screen bytes are the same,
                    // so we bake cursor into the generation check via a combined key stored
                    // in tui_cache_generation. We encode cursor as a sentinel in the upper
                    // bits: generation XOR (row<<20 | col<<8 | cursor_visible_bit).
                    let screen = self.raw_screen.screen();
                    let (screen_rows, screen_cols) = screen.size();
                    let cursor_pos = if !screen.hide_cursor() {
                        Some(screen.cursor_position())
                    } else {
                        None
                    };
                    let cursor_key: u64 = match cursor_pos {
                        Some((r, c)) => ((r as u64) << 20) | ((c as u64) << 8) | 1,
                        None => 0,
                    };
                    let effective_generation = self.tui_screen_generation ^ cursor_key;

                    if effective_generation != self.tui_cache_generation {
                        // Rebuild cache — this only runs when screen content actually changed.
                        self.tui_cached_jobs.clear();
                        // Pre-allocate to exact row count to avoid repeated Vec growth.
                        self.tui_cached_jobs.reserve(screen_rows as usize);

                        for r in 0..screen_rows {
                            let mut job = egui::text::LayoutJob::default();
                            job.wrap = egui::text::TextWrapping {
                                max_width: f32::INFINITY,
                                ..Default::default()
                            };

                            let mut current_str = String::with_capacity(screen_cols as usize);
                            let mut current_format: Option<egui::TextFormat> = None;

                            for c in 0..screen_cols {
                                let is_cursor = cursor_pos == Some((r, c));
                                let cell = screen.cell(r, c);
                                let (ch, cell_fg, cell_bg, _bold, underline) = if let Some(cell) = cell {
                                    let contents = cell.contents();
                                    let ch = if contents.is_empty() { " " } else { contents };
                                    (ch, cell.fgcolor(), cell.bgcolor(), cell.bold(), cell.underline())
                                } else {
                                    (" ", vt100::Color::Default, vt100::Color::Default, false, false)
                                };

                                let mut fg = vt_color_to_color32(cell_fg, default_fg);
                                let mut bg = vt_color_to_color32(cell_bg, default_bg);

                                if is_cursor {
                                    fg = default_bg;
                                    bg = default_fg;
                                }

                                let background = if bg == default_bg && !is_cursor {
                                    egui::Color32::TRANSPARENT
                                } else {
                                    bg
                                };

                                let fmt = egui::TextFormat {
                                    font_id: font_id.clone(),
                                    color: fg,
                                    background,
                                    line_height: Some(font_id.size * 1.05),
                                    underline: if underline { egui::Stroke::new(1.0, fg) } else { egui::Stroke::NONE },
                                    ..Default::default()
                                };

                                if Some(&fmt) == current_format.as_ref() {
                                    current_str.push_str(ch);
                                } else {
                                    if let Some(prev_fmt) = current_format.take() {
                                        job.append(&current_str, 0.0, prev_fmt);
                                        current_str.clear();
                                    }
                                    current_format = Some(fmt);
                                    current_str.push_str(ch);
                                }
                            }

                            if let Some(fmt) = current_format {
                                job.append(&current_str, 0.0, fmt);
                            }

                            self.tui_cached_jobs.push(job);
                        }
                        self.tui_cache_generation = effective_generation;
                    }

                    egui::Frame::none()
                        .fill(default_bg)
                        .inner_margin(2.0)
                        .show(ui, |ui| {
                            ui.set_clip_rect(ui.available_rect_before_wrap());

                            // Zero out item spacing to prevent vertical gaps between rows
                            ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

                            for job in &self.tui_cached_jobs {
                                ui.label(job.clone());
                            }
                        });
                });

            return None;
        }

        // ==========================================
        // NORMAL NOTEBOOK MODE (Streaming logs & normal commands)
        // ==========================================
        let avail_for_layout = ui.available_size();
        ui.allocate_ui_with_layout(avail_for_layout, egui::Layout::bottom_up(egui::Align::Min), |ui| {

            // ====== SMART INPUT BAR ======
            ui.add_space(4.0);
            let input_frame = egui::Frame::default()
                .fill(input_bg)
                .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                .rounding(8.0)
                .stroke(egui::Stroke::new(1.0, input_border));

            // Calculate the exact pixel height the input frame needs.
            // line_height ≈ mono_font_size * 1.5 (egui's default row height ratio).
            // When running a command, the stop button is ~26px tall; use that as minimum.
            // .lines() does NOT count a trailing '\n' as a new line in Rust.
            // After Shift+Enter pushes '\n', we must account for the extra visual row
            // explicitly, otherwise input_h is under-allocated by one line causing
            // the TextEdit to overflow and trigger a layout feedback loop.
            let trailing_newline = if self.command_input.ends_with('\n') { 1 } else { 0 };
            let line_count = (self.command_input.lines().count() + trailing_newline).max(1).min(10);
            let mono_h = config.mono_font_size * 1.45;
            let v_margin = 16.0; // 8.0 top + 8.0 bottom inner margin
            let stop_btn_h = 26.0 + v_margin; // minimum height when stop button shown
            let text_input_h = line_count as f32 * mono_h + v_margin;
            let input_h = if self.is_running {
                stop_btn_h.max(text_input_h)
            } else {
                text_input_h
            };

            // Allocate exactly the needed height so bottom_up does not grant
            // this frame the entire remaining window height.
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), input_h),
                egui::Layout::left_to_right(egui::Align::TOP),
                |ui| {
                    input_frame.show(ui, |ui| {
                        ui.horizontal_top(|ui| {
                if self.is_running {
                        let stop_btn = ui.add(
                            egui::Button::new(
                                egui::RichText::new("⏹ Stop (Ctrl+C)")
                                    .color(egui::Color32::WHITE)
                                    .size(11.5)
                            )
                            .fill(egui::Color32::from_rgb(210, 50, 50))
                            .rounding(6.0)
                        );
                        if stop_btn.clicked() {
                            self.interrupt();
                        }
                    } else {
                        ui.label(
                            egui::RichText::new("❯")
                                .color(cmd_color)
                                .strong()
                                .family(egui::FontFamily::Monospace)
                                .size(config.mono_font_size),
                        );
                    }

                    let input_id = ui.id().with("notebook_cmd");

                    let mut enter_pressed = false;
                    let mut tab_pressed = false;
                    let mut esc_pressed = false;
                    let mut up_pressed = false;
                    let mut down_pressed = false;
                    let mut ctrl_c_pressed = false;

                    // Ctrl+C is always checked globally (not gated by focus) so it
                    // works even when the TextEdit is not focused during command execution.
                    ui.input(|i| {
                        let is_ctrl = i.modifiers.ctrl || (cfg!(target_os = "macos") && i.modifiers.command);
                        if is_ctrl && i.key_pressed(egui::Key::C) {
                            ctrl_c_pressed = true;
                        }
                    });

                    if ui.memory(|m| m.has_focus(input_id)) {
                        ui.input(|i| {
                            if i.key_pressed(egui::Key::Enter) {
                                if !i.modifiers.shift && !i.modifiers.alt {
                                    enter_pressed = true;
                                }
                            }
                            if i.key_pressed(egui::Key::Tab) { tab_pressed = true; }
                            if i.key_pressed(egui::Key::Escape) { esc_pressed = true; }
                            if i.key_pressed(egui::Key::ArrowUp) { up_pressed = true; }
                            if i.key_pressed(egui::Key::ArrowDown) { down_pressed = true; }
                        });
                    }

                    if ctrl_c_pressed {
                        self.interrupt();
                    }

                    let prev_input = self.command_input.clone();
                    let hint = if self.is_running { "Type input to process…" } else { "Type command… (Shift+Enter ↵ for multiline)" };
                    // desired_rows matches actual line count (capped at 10) so
                    // pasting multiline code expands the TextEdit box correctly.
                    let response = ui.add(
                        egui::TextEdit::multiline(&mut self.command_input)
                            .id(input_id)
                            .hint_text(hint)
                            .font(egui::FontId::monospace(config.mono_font_size))
                            .desired_rows(line_count)
                            .desired_width(ui.available_width())
                            .frame(false)
                            .text_color(text_color),
                    );

                    // Auto-focus when window is created or takes focus
                    if self.focus_input_requested {
                        ui.memory_mut(|m| m.request_focus(input_id));
                        self.focus_input_requested = false;
                    }

                    // When command_input was replaced by a suggestion or history entry,
                    // reset the TextEdit cursor to end-of-text so egui doesn't use the
                    // stale cursor position that would cause garbled/broken input.
                    if self.need_cursor_reset {
                        self.need_cursor_reset = false;
                        let char_count = self.command_input.chars().count();
                        let mut state = TextEditState::load(ui.ctx(), input_id)
                            .unwrap_or_default();
                        let ccursor = egui::text::CCursor::new(char_count);
                        state.cursor.set_char_range(Some(egui::text::CCursorRange::one(ccursor)));
                        state.store(ui.ctx(), input_id);
                    }

                    // Sync typed_input whenever the user actually edits the text box.
                    if self.command_input != prev_input {
                        self.typed_input = self.command_input.clone();
                        self.is_dismissed = false;
                        self.suggestion_idx = None;
                        self.history_nav_idx = None;
                        // Invalidate suggestion cache so it rebuilds on next access below.
                        self.cached_suggestions_for.clear();
                    }

                    if esc_pressed {
                        if self.suggestion_idx.is_some() {
                            self.command_input = self.typed_input.clone();
                            self.suggestion_idx = None;
                        } else {
                            self.is_dismissed = true;
                        }
                    }

                    // --- Compute suggestions from typed_input (cached per typed_input value) ---
                    let want_suggestions = !self.typed_input.is_empty() && !self.is_dismissed && !self.is_running;
                    if !want_suggestions {
                        self.cached_suggestions.clear();
                        self.cached_suggestions_for.clear();
                    } else if self.cached_suggestions_for != self.typed_input {
                        // Only recompute when input actually changed — avoids per-frame disk IO.
                        self.cached_suggestions = completion::get_suggestions(
                            &self.typed_input,
                            &self.command_history,
                            Some(&self.frecency),
                        );
                        self.cached_suggestions_for = self.typed_input.clone();
                    }
                    let suggestions = &self.cached_suggestions;

                    if suggestions.is_empty() && !self.is_running {
                        if up_pressed && !self.command_history.is_empty() {
                            let idx = match self.history_nav_idx {
                                Some(i) if i > 0 => i - 1,
                                Some(i) => i,
                                None => self.command_history.len() - 1,
                            };
                            self.history_nav_idx = Some(idx);
                            self.command_input = self.command_history[idx].clone();
                            self.typed_input = self.command_input.clone();
                            self.need_cursor_reset = true;
                        }
                        if down_pressed {
                            if let Some(i) = self.history_nav_idx {
                                if i + 1 < self.command_history.len() {
                                    self.history_nav_idx = Some(i + 1);
                                    self.command_input = self.command_history[i + 1].clone();
                                    self.typed_input = self.command_input.clone();
                                    self.need_cursor_reset = true;
                                } else {
                                    self.history_nav_idx = None;
                                    self.command_input = self.typed_input.clone();
                                }
                            }
                        }
                    }

                    // --- Suggestions popup ---
                    if !suggestions.is_empty() {
                        if tab_pressed {
                            let idx = self.suggestion_idx
                                .map(|i| (i + 1) % suggestions.len())
                                .unwrap_or(0);
                            self.suggestion_idx = Some(idx);
                            self.command_input = suggestions[idx].fill_cmd.clone();
                            self.typed_input = self.command_input.clone();
                            self.is_dismissed = true;
                            self.need_cursor_reset = true;
                        }

                        if up_pressed {
                            let idx = self.suggestion_idx
                                .map(|i| if i == 0 { suggestions.len() - 1 } else { i - 1 })
                                .unwrap_or(suggestions.len() - 1);
                            self.suggestion_idx = Some(idx);
                            self.command_input = suggestions[idx].fill_cmd.clone();
                            self.need_cursor_reset = true;
                        }
                        if down_pressed {
                            let new_idx = self.suggestion_idx
                                .map(|i| i + 1)
                                .unwrap_or(0);
                            if new_idx < suggestions.len() {
                                self.suggestion_idx = Some(new_idx);
                                self.command_input = suggestions[new_idx].fill_cmd.clone();
                                self.need_cursor_reset = true;
                            } else {
                                self.suggestion_idx = None;
                                self.command_input = self.typed_input.clone();
                            }
                        }

                        egui::popup::popup_below_widget(
                            ui,
                            ui.id().with("cmd_popup"),
                            &response,
                            egui::PopupCloseBehavior::CloseOnClick,
                            |ui| {
                                ui.set_min_width(420.0);
                                ui.set_max_width(600.0);
                                let is_dark = ui.style().visuals.dark_mode;

                                for (idx, item) in suggestions.iter().enumerate() {
                                    let is_sel = self.suggestion_idx == Some(idx);

                                    let (icon, tag_color) = match &item.source {
                                        SuggestionSource::SubCommand => (
                                            Icons::COMMAND,
                                            egui::Color32::from_rgb(100, 180, 255),
                                        ),
                                        SuggestionSource::Alias => (
                                            Icons::GEAR,
                                            egui::Color32::from_rgb(255, 180, 80),
                                        ),
                                        SuggestionSource::Frecency => (
                                            Icons::STAR,
                                            egui::Color32::from_rgb(255, 220, 60),
                                        ),
                                        SuggestionSource::History => (
                                            Icons::HISTORY,
                                            egui::Color32::from_rgb(160, 160, 170),
                                        ),
                                        SuggestionSource::Path => (
                                            Icons::FOLDER,
                                            egui::Color32::from_rgb(100, 220, 140),
                                        ),
                                        SuggestionSource::Snippet => (
                                            Icons::NOTE,
                                            egui::Color32::from_rgb(200, 130, 255),
                                        ),
                                    };

                                    let bg = if is_sel {
                                        if is_dark {
                                            egui::Color32::from_rgb(50, 90, 150)
                                        } else {
                                            egui::Color32::from_rgb(200, 220, 255)
                                        }
                                    } else {
                                        egui::Color32::TRANSPARENT
                                    };

                                    let row = egui::Frame::none()
                                        .fill(bg)
                                        .rounding(4.0)
                                        .inner_margin(egui::Margin::symmetric(6.0, 2.0));

                                    let row_resp = row.show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                Icons::rich(icon, (config.ui_font_size - 1.0).max(10.0))
                                                    .color(tag_color)
                                            );
                                            ui.label(
                                                egui::RichText::new(&item.display)
                                                    .font(egui::FontId::monospace(config.mono_font_size))
                                            );
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                ui.label(
                                                    egui::RichText::new(&item.detail)
                                                        .size((config.ui_font_size - 3.0).max(9.0))
                                                        .color(tag_color)
                                                );
                                            });
                                        });
                                    });

                                    if row_resp.response.interact(egui::Sense::click()).clicked() {
                                        self.command_input = item.fill_cmd.clone();
                                        self.typed_input = item.fill_cmd.clone();
                                        self.suggestion_idx = None;
                                        self.is_dismissed = true;
                                        self.need_cursor_reset = true;
                                        ui.memory_mut(|m| m.request_focus(input_id));
                                    }
                                }
                            },
                        );
                        ui.memory_mut(|m| m.open_popup(ui.id().with("cmd_popup")));
                    }

                    // --- Execute on Enter ---
                    if enter_pressed {
                        self.focus_input_requested = true;
                        ui.ctx().request_repaint();

                        while self.command_input.ends_with('\n') {
                            self.command_input.pop();
                        }

                        if !self.command_input.is_empty() {
                            if self.is_running {
                                // Forward input directly to running process (e.g. interactive tail/logs responses)
                                let to_send = format!("{}\n", self.command_input);
                                if let Some(pty) = self.pty.as_mut() {
                                    let _ = pty.write(to_send.as_bytes());
                                }
                                self.command_input.clear();
                                self.typed_input.clear();
                            } else {
                                self.complete_active_block();

                                let user_aliases = completion::load_user_shell_aliases();
                                let final_cmd = if let Some(a) = user_aliases.iter().find(|a| a.name == self.command_input.trim()) {
                                    a.target.clone()
                                } else {
                                    self.command_input.clone()
                                };

                                self.frecency.record(&final_cmd);

                                self.blocks.push(NotebookBlock::new(self.command_input.clone()));
                                self.active_block = Some(self.blocks.len() - 1);
                                let cmd_display = self.command_input.trim().to_string();
                                self.running_command = Some(if cmd_display.is_empty() { final_cmd.clone() } else { cmd_display });
                                self.is_running = true;
                                self.raw_screen = vt100::Parser::new(self.last_rows.max(10), self.last_cols.max(40), 0);

                                if let Some(pty) = self.pty.as_mut() {
                                    pty.write(format!("{}\n", final_cmd).as_bytes());
                                }

                                self.command_history.push(final_cmd);
                                self.command_input.clear();
                                self.typed_input.clear();
                                self.suggestion_idx = None;
                                self.history_nav_idx = None;
                                self.is_dismissed = false;
                                self.scroll_to_bottom = true;
                            }
                        }
                    }
                });
            });
            });
            ui.add_space(2.0);

            // ====== NOTEBOOK BLOCKS ======
            egui::ScrollArea::both()
                .stick_to_bottom(true)
                .auto_shrink([false, true])
                .id_salt("notebook_scroll")
                .show(ui, |ui| {
                    ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                        ui.spacing_mut().interact_size.y = 0.0;
                        let avail_w = ui.available_width();
                        ui.set_max_width(avail_w);

                        let block_count = self.blocks.len();
                        let viewport_height = avail.y;
                        for i in 0..block_count {
                            let block = &mut self.blocks[i];
                            let is_active = self.active_block == Some(i);

                            if block.is_clear_marker {
                                ui.add_space(viewport_height);
                                continue;
                            }

                            // rendered_combined_job returns Arc<LayoutJob> \u2014 O(1) ref bump, no copy.
                            let combined = block.rendered_combined_job(cols, &font_id, text_color);
                            let has_cmd = !block.command.is_empty();
                            if !has_cmd && combined.is_none() {
                                continue;
                            }

                            let block_id = ui.id().with(("block", i));
                            let block_start = ui.cursor().min;

                            if has_cmd {
                                // cmd_header_job() returns cached LayoutJob, rebuilt only on state change.
                                let mut job = block.cmd_header_job(&font_id, cmd_color, text_color, dim_color, is_active);
                                job.wrap = egui::text::TextWrapping {
                                    max_width: avail.x.max(100.0) - 20.0,
                                    ..Default::default()
                                };
                                ui.label(job);
                                ui.add_space(1.0);
                            }

                            if let Some(arc_job) = combined {
                                // Clone the Arc (O(1)) and set wrap on the local copy.
                                let mut combined_job = (*arc_job).clone();
                                combined_job.wrap = egui::text::TextWrapping {
                                    max_width: avail.x.max(100.0) - 20.0,
                                    break_anywhere: true,
                                    ..Default::default()
                                };
                                ui.label(combined_job);
                            }

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

                            ui.add_space(6.0);
                            let sep_rect = ui.available_rect_before_wrap();
                            ui.painter().hline(
                                sep_rect.x_range(),
                                sep_rect.top(),
                                egui::Stroke::new(0.5, separator_color),
                            );
                            ui.add_space(6.0);
                        }
                    });
                });
        });

        None
    }
}

