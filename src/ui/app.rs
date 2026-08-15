use eframe::egui;

use crate::models::state::AppState;
use crate::ui::editor::EditorApp;
use crate::ui::icons::Icons;
use crate::ui::palette::CommandPalette;
use crate::ui::session;
use crate::ui::settings::{AppConfig, SettingsApp};
use crate::ui::theme;
use crate::ui::terminal_app::TerminalApp;
use crate::ui::undo_manager::UndoManager;
use crate::ui::window_framework::FloatingWindow;
use crate::ui::workspace::Workspace;

pub struct XTermApp {
    _state: AppState,
    workspaces: Vec<Workspace>,
    undo_manager: UndoManager,
    active_workspace_idx: usize,
    config: AppConfig,
    _last_theme_sync: f64,
    current_ui_font: String,
    current_mono_font: String,
    _close_warning_workspace: Option<usize>,
    _sys: sysinfo::System,
    palette: CommandPalette,
    /// Set to true when user has confirmed quitting despite running commands.
    close_confirmed: bool,
    /// Set to true when the OS close event fires and running commands exist.
    close_warning_visible: bool,
    last_saved_time: std::time::Instant,
    last_applied_theme: Option<(crate::ui::settings::AppTheme, f32, f32)>,
    last_applied_fonts: Option<(String, String, f32, f32)>,
}

impl XTermApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Try restoring saved session from ~/.config/smart-term/session.json
        let (workspaces, active_idx, config) = session::load_session(&cc.egui_ctx)
            .unwrap_or_else(|| {
                (vec![Workspace::new("Workspace 1", &cc.egui_ctx)], 0, AppConfig::default())
            });

        theme::apply_theme(&cc.egui_ctx, &config);
        theme::apply_font(&cc.egui_ctx, &config.ui_font_family, &config.mono_font_family);

        let ui_font = config.ui_font_family.clone();
        let mono_font = config.mono_font_family.clone();

        let theme_key = (config.theme.clone(), config.blur_level, config.window_rounding);
        let font_key = (ui_font.clone(), mono_font.clone(), config.ui_font_size, config.mono_font_size);
        let undo_stack_size = config.undo_stack_size;

        Self {
            _state: AppState::default(),
            workspaces,
            undo_manager: UndoManager::new(undo_stack_size),
            active_workspace_idx: active_idx,
            config,
            _last_theme_sync: 0.0,
            current_ui_font: ui_font,
            current_mono_font: mono_font,
            _close_warning_workspace: None,
            _sys: sysinfo::System::new(),
            palette: CommandPalette::new(),
            close_confirmed: false,
            close_warning_visible: false,
            last_saved_time: std::time::Instant::now(),
            last_applied_theme: Some(theme_key),
            last_applied_fonts: Some(font_key),
        }
    }

    fn open_or_focus_settings(&mut self) {
        if let Some(ws) = self.workspaces.get_mut(self.active_workspace_idx) {
            if let Some(pos) = ws.windows.iter().position(|w| w.app.window_type() == "settings") {
                let mut settings_win = ws.windows.remove(pos);
                settings_win.focus_requested = true;
                ws.windows.push(settings_win);
            } else {
                let win_id = uuid::Uuid::new_v4().to_string();
                ws.windows.push(FloatingWindow::new(win_id, Box::new(SettingsApp)));
            }
        }
    }

    fn open_or_focus_git_manager(&mut self) {
        if let Some(ws) = self.workspaces.get_mut(self.active_workspace_idx) {
            if let Some(pos) = ws.windows.iter().position(|w| w.app.window_type() == "git_manager") {
                let mut git_win = ws.windows.remove(pos);
                git_win.focus_requested = true;
                ws.windows.push(git_win);
            } else {
                let win_id = uuid::Uuid::new_v4().to_string();
                ws.windows.push(FloatingWindow::new(win_id, Box::new(crate::ui::git_app::GitApp::new())));
            }
        }
    }

    fn apply_undo_action(&mut self, action: &crate::ui::undo_manager::UndoAction) {
        use crate::ui::undo_manager::UndoAction;
        match action {
            UndoAction::EditorSave { file_path, previous_content } => {
                for ws in &mut self.workspaces {
                    for win in &mut ws.windows {
                        if win.app.window_type() == "editor" {
                            if let Some(editor) = win.app.as_any_mut().and_then(|a| a.downcast_mut::<EditorApp>()) {
                                if editor.path == *file_path {
                                    editor.content = previous_content.clone();
                                    editor.original_content = previous_content.clone();
                                    editor.is_dirty = false;
                                    editor.save_status = Some("Undid save".to_string());
                                }
                            }
                        }
                    }
                }
            }
            UndoAction::EditorFormat { window_id, previous_content }
            | UndoAction::EditorReplace { window_id, previous_content }
            | UndoAction::EditorReplaceAll { window_id, previous_content } => {
                for ws in &mut self.workspaces {
                    for win in &mut ws.windows {
                        if win.app.window_type() == "editor" {
                            if let Some(editor) = win.app.as_any_mut().and_then(|a| a.downcast_mut::<EditorApp>()) {
                                if editor.path == *window_id {
                                    editor.content = previous_content.clone();
                                    editor.is_dirty = editor.content != editor.original_content;
                                }
                            }
                        }
                    }
                }
            }
            UndoAction::SettingsChange { previous_config } => {
                self.config = *previous_config.clone();
            }
            UndoAction::WorkspaceRename { ws_index, previous_name } => {
                if let Some(ws) = self.workspaces.get_mut(*ws_index) {
                    ws.name = previous_name.clone();
                }
            }
            UndoAction::GitStageFile { .. }
            | UndoAction::GitUnstageFile { .. }
            | UndoAction::GitStageAll { .. }
            | UndoAction::GitUnstageAll { .. }
            | UndoAction::GitRevertFile { .. }
            | UndoAction::GitDeleteFile { .. }
            | UndoAction::GitRevertHunk { .. }
            | UndoAction::GitCommit { .. }
            | UndoAction::GitSwitchBranch { .. }
            | UndoAction::GitCreateBranch { .. } => {
                // Refresh all open GitApp windows after undo
                for ws in &mut self.workspaces {
                    for win in &mut ws.windows {
                        if win.app.window_type() == "git_manager" {
                            if let Some(git_app) = win.app.as_any_mut()
                                .and_then(|a| a.downcast_mut::<crate::ui::git_app::GitApp>())
                            {
                                git_app.needs_refresh = true;
                            }
                        }
                    }
                }
            }
        }
    }

    fn handle_frameless_resize(&self, ctx: &egui::Context) {
        let is_maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
        if is_maximized {
            return;
        }

        let rect = ctx.screen_rect();
        if let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) {
            let margin = 6.0;
            let on_left = pos.x <= rect.left() + margin;
            let on_right = pos.x >= rect.right() - margin;
            let on_top = pos.y <= rect.top() + margin;
            let on_bottom = pos.y >= rect.bottom() - margin;

            if on_left || on_right || on_top || on_bottom {
                let dir = match (on_left, on_right, on_top, on_bottom) {
                    (true, _, true, _) => Some(egui::viewport::ResizeDirection::NorthWest),
                    (_, true, true, _) => Some(egui::viewport::ResizeDirection::NorthEast),
                    (true, _, _, true) => Some(egui::viewport::ResizeDirection::SouthWest),
                    (_, true, _, true) => Some(egui::viewport::ResizeDirection::SouthEast),
                    (true, _, _, _) => Some(egui::viewport::ResizeDirection::West),
                    (_, true, _, _) => Some(egui::viewport::ResizeDirection::East),
                    (_, _, true, _) => Some(egui::viewport::ResizeDirection::North),
                    (_, _, _, true) => Some(egui::viewport::ResizeDirection::South),
                    _ => None,
                };

                if let Some(direction) = dir {
                    if ctx.input(|i| i.pointer.primary_down()) {
                        ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(direction));
                    }
                }
            }
        }
    }

    fn render_top_bar(&mut self, ctx: &egui::Context) {
        let is_mac = cfg!(target_os = "macos");
        let is_dark = ctx.style().visuals.dark_mode;

        let topbar_fill = if is_dark {
            egui::Color32::from_rgb(22, 24, 32)
        } else {
            egui::Color32::from_rgb(240, 242, 246)
        };

        let panel_frame = egui::Frame::default()
            .fill(topbar_fill)
            .inner_margin(egui::Margin {
                left: 10.0,
                right: 10.0,
                top: 0.0,
                bottom: 0.0,
            })
            .rounding(egui::Rounding {
                nw: if is_mac { 12.0 } else { 0.0 },
                ne: if is_mac { 12.0 } else { 0.0 },
                sw: 0.0,
                se: 0.0,
            });

        egui::TopBottomPanel::top("top_bar")
            .frame(panel_frame)
            .exact_height(36.0)
            .show(ctx, |ui| {
                // TopBar Background Drag to Move Window
                let top_bar_rect = ui.available_rect_before_wrap();
                let top_bar_resp = ui.interact(
                    top_bar_rect,
                    ui.id().with("top_bar_drag_bg"),
                    egui::Sense::click_and_drag(),
                );

                if top_bar_resp.double_clicked() {
                    let is_maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
                } else if top_bar_resp.drag_started_by(egui::PointerButton::Primary) {
                    let is_maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                    if !is_maximized {
                        ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                    }
                }

                ui.horizontal_centered(|ui| {
                    // 1. macOS traffic lights (Left side)
                    if is_mac {
                        ui.add_space(4.0);
                        for (color, action) in [
                            (egui::Color32::from_rgb(255, 95, 86), "close"),
                            (egui::Color32::from_rgb(255, 189, 46), "minimize"),
                            (egui::Color32::from_rgb(39, 201, 63), "maximize"),
                        ] {
                            let (rect, resp) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::click());
                            if ui.is_rect_visible(rect) {
                                ui.painter().circle_filled(rect.center(), 6.0, color);
                            }
                            if resp.clicked() {
                                match action {
                                    "close" => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                                    "minimize" => ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true)),
                                    _ => {
                                        let m = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                                        ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!m));
                                    }
                                }
                            }
                            ui.add_space(4.0);
                        }
                        ui.add_space(8.0);
                    }

                    // Calculate space reserved for right-side buttons (Settings, Search, Min, Max, Close)
                    let right_buttons_w = if is_mac { 120.0 } else { 220.0 };
                    let tabs_avail_w = (ui.available_width() - right_buttons_w).max(80.0);

                    // 2. Workspace Tabs (Căn CHÍNH GIỮA DỌC/NGANG với 2 phím mũi tên cuộn < và >)
                    let scroll_id = ui.id().with("workspace_tabs_scroll");

                    // Left arrow (<)
                    if ui.add(
                        egui::Button::new(Icons::rich(Icons::CARET_LEFT, 11.0))
                            .fill(egui::Color32::TRANSPARENT)
                            .stroke(egui::Stroke::NONE)
                            .rounding(4.0)
                            .min_size(egui::vec2(16.0, 20.0)),
                    ).on_hover_text("Scroll tabs left").clicked() {
                        let cur = ctx.data(|d| d.get_temp::<f32>(scroll_id)).unwrap_or(0.0);
                        ctx.data_mut(|d| d.insert_temp(scroll_id, (cur - 120.0).max(0.0)));
                    }

                    let tabs_area_w = (tabs_avail_w - 40.0).max(40.0);
                    let tab_offset = ctx.data(|d| d.get_temp::<f32>(scroll_id)).unwrap_or(0.0);

                    ui.allocate_ui_with_layout(
                        egui::vec2(tabs_area_w, 24.0),
                        egui::Layout::left_to_right(egui::Align::Center).with_main_align(egui::Align::Center),
                        |ui| {
                            ui.set_max_height(24.0);
                            egui::ScrollArea::horizontal()
                                .id_salt(scroll_id)
                                .max_height(24.0)
                                .scroll_offset(egui::vec2(tab_offset, 0.0))
                                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                                .show(ui, |ui| {
                                    ui.horizontal_centered(|ui| {
                                        ui.spacing_mut().item_spacing.x = 4.0;
                                        let mut to_remove = None;
                                        for (idx, workspace) in self.workspaces.iter_mut().enumerate() {
                                            let is_active = idx == self.active_workspace_idx;

                                            // Firefox Card Tab Styling (Consistent size for active and inactive)
                                            let fill_color = if is_active {
                                                if is_dark {
                                                    egui::Color32::from_rgb(42, 46, 60)
                                                } else {
                                                    egui::Color32::from_rgb(255, 255, 255)
                                                }
                                            } else {
                                                egui::Color32::TRANSPARENT
                                            };

                                            let stroke = if is_active {
                                                if is_dark {
                                                    egui::Stroke::new(1.0, egui::Color32::from_white_alpha(45))
                                                } else {
                                                    egui::Stroke::new(1.0, egui::Color32::from_black_alpha(35))
                                                }
                                            } else {
                                                egui::Stroke::new(1.0, egui::Color32::TRANSPARENT)
                                            };

                                            let tab_frame = egui::Frame::default()
                                                .fill(fill_color)
                                                .stroke(stroke)
                                                .rounding(6.0)
                                                .inner_margin(egui::Margin {
                                                    left: 8.0,
                                                    right: 6.0,
                                                    top: 2.0,
                                                    bottom: 2.0,
                                                });

                                            tab_frame.show(ui, |ui| {
                                                if workspace.is_editing_name {
                                                    let edit_id = ui.id().with(("ws_rename", idx));
                                                    let resp = ui.add(
                                                        egui::TextEdit::singleline(&mut workspace.name)
                                                            .id(edit_id)
                                                            .desired_width(90.0)
                                                            .font(egui::FontId::proportional(12.0)),
                                                    );

                                                    if !ui.memory(|m| m.has_focus(edit_id)) {
                                                        ui.memory_mut(|m| m.request_focus(edit_id));
                                                    }

                                                    let lost_focus = resp.lost_focus();
                                                    let enter_esc = ui.input(|i| {
                                                        i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Escape)
                                                    });
                                                    let clicked_outside = ui.input(|i| i.pointer.any_pressed()) && !resp.hovered();

                                                    if lost_focus || enter_esc || clicked_outside {
                                                        workspace.is_editing_name = false;
                                                    }
                                                } else {
                                                    let text_color = if is_active {
                                                        ctx.style().visuals.text_color()
                                                    } else if is_dark {
                                                        egui::Color32::from_gray(140)
                                                    } else {
                                                        egui::Color32::from_gray(120)
                                                    };

                                                    let tab_resp = ui.horizontal_centered(|ui| {
                                                        let name_text = if is_active {
                                                            egui::RichText::new(&workspace.name).size(12.0).color(text_color).strong()
                                                        } else {
                                                            egui::RichText::new(&workspace.name).size(12.0).color(text_color)
                                                        };

                                                        let name_resp = ui.add(
                                                            egui::Button::new(name_text)
                                                                .fill(egui::Color32::TRANSPARENT)
                                                                .stroke(egui::Stroke::NONE)
                                                                .frame(false)
                                                                .rounding(4.0)
                                                                .min_size(egui::vec2(0.0, 16.0)),
                                                        );

                                                        if name_resp.clicked() {
                                                            self.active_workspace_idx = idx;
                                                        }
                                                        if name_resp.double_clicked() {
                                                            workspace.is_editing_name = true;
                                                        }

                                                        let close_color = if is_dark {
                                                            egui::Color32::from_gray(110)
                                                        } else {
                                                            egui::Color32::from_gray(140)
                                                        };
                                                        let close_resp = ui.add(
                                                            egui::Button::new(
                                                                egui::RichText::new(Icons::CLOSE).size(9.5).color(close_color),
                                                            )
                                                            .fill(egui::Color32::TRANSPARENT)
                                                            .stroke(egui::Stroke::NONE)
                                                            .frame(false)
                                                            .rounding(4.0)
                                                            .min_size(egui::vec2(16.0, 16.0)),
                                                        );
                                                        if close_resp.clicked() {
                                                            to_remove = Some(idx);
                                                        }

                                                        name_resp
                                                    });

                                                    tab_resp.response.context_menu(|ui| {
                                                        if ui.button(Icons::label_job(Icons::EDIT, "Rename", 12.0, ui.visuals().text_color())).clicked() {
                                                            workspace.is_editing_name = true;
                                                            ui.close_menu();
                                                        }
                                                        if ui.button(Icons::label_job(Icons::CLOSE, "Close", 12.0, ui.visuals().text_color())).clicked() {
                                                            to_remove = Some(idx);
                                                            ui.close_menu();
                                                        }
                                                    });
                                                }
                                            });
                                        }

                                        if let Some(idx) = to_remove {
                                            self.workspaces.remove(idx);
                                            if self.workspaces.is_empty() {
                                                self.workspaces.push(Workspace::new("Workspace 1", ctx));
                                                self.active_workspace_idx = 0;
                                            } else if self.active_workspace_idx >= self.workspaces.len() {
                                                self.active_workspace_idx = self.workspaces.len() - 1;
                                            }
                                        }

                                        // New tab button (+)
                                        ui.add_space(2.0);
                                        if ui.add(
                                            egui::Button::new(egui::RichText::new("+").size(13.0))
                                                .fill(egui::Color32::TRANSPARENT)
                                                .rounding(4.0)
                                                .min_size(egui::vec2(22.0, 22.0)),
                                        ).on_hover_text("New workspace").clicked() {
                                            let n = self.workspaces.len() + 1;
                                            self.workspaces.push(Workspace::new(&format!("Workspace {}", n), ctx));
                                            self.active_workspace_idx = self.workspaces.len() - 1;
                                        }
                                    });
                                });
                        },
                    );

                    // Right arrow (>)
                    if ui.add(
                        egui::Button::new(Icons::rich(Icons::CARET_RIGHT, 11.0))
                            .fill(egui::Color32::TRANSPARENT)
                            .stroke(egui::Stroke::NONE)
                            .rounding(4.0)
                            .min_size(egui::vec2(16.0, 20.0)),
                    ).on_hover_text("Scroll tabs right").clicked() {
                        let cur = ctx.data(|d| d.get_temp::<f32>(scroll_id)).unwrap_or(0.0);
                        ctx.data_mut(|d| d.insert_temp(scroll_id, cur + 120.0));
                    }

                    // 3. Right-side buttons (Settings, Search, Min, Max, Close)
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(4.0);

                        if !is_mac {
                            let is_maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));

                            if ui.add(
                                egui::Button::new(egui::RichText::new(Icons::CLOSE).size(12.0))
                                    .fill(egui::Color32::TRANSPARENT)
                                    .rounding(4.0)
                                    .min_size(egui::vec2(28.0, 24.0)),
                            ).on_hover_text("Close").clicked() {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            }

                            if ui.add(
                                egui::Button::new(egui::RichText::new(Icons::APP_WINDOW).size(12.0))
                                    .fill(egui::Color32::TRANSPARENT)
                                    .rounding(4.0)
                                    .min_size(egui::vec2(28.0, 24.0)),
                            ).on_hover_text(if is_maximized { "Restore" } else { "Maximize" }).clicked() {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
                            }

                            if ui.add(
                                egui::Button::new(egui::RichText::new(Icons::MINUS).size(12.0))
                                    .fill(egui::Color32::TRANSPARENT)
                                    .rounding(4.0)
                                    .min_size(egui::vec2(28.0, 24.0)),
                            ).on_hover_text("Minimize").clicked() {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                            }
                            ui.add_space(4.0);
                            ui.separator();
                        }

                        let btn_border = if is_dark {
                            egui::Color32::from_white_alpha(30)
                        } else {
                            egui::Color32::from_black_alpha(25)
                        };
                        let btn_fill = if is_dark {
                            egui::Color32::from_white_alpha(15)
                        } else {
                            egui::Color32::from_black_alpha(10)
                        };

                        // Active Windows List Selector Dropdown
                        let win_count = self.workspaces.get(self.active_workspace_idx).map_or(0, |ws| ws.windows.len());
                        let win_btn_job = Icons::label_job(Icons::APP_WINDOW, &format!("Windows ({})", win_count), 11.5, ui.visuals().text_color());

                        ui.menu_button(win_btn_job, |ui| {
                            ui.set_min_width(220.0);
                            ui.label(egui::RichText::new("Active Windows in Workspace").weak().size(11.0));
                            ui.separator();

                            let mut focus_win_idx = None;
                            let mut close_win_idx = None;

                            if let Some(ws) = self.workspaces.get(self.active_workspace_idx) {
                                if ws.windows.is_empty() {
                                    ui.label(egui::RichText::new("No active windows").weak().size(12.0));
                                } else {
                                    for (idx, win) in ws.windows.iter().enumerate() {
                                        let is_last = (idx == ws.windows.len() - 1);
                                        let title = win.custom_title.clone().unwrap_or_else(|| win.app.title());

                                        ui.horizontal(|ui| {
                                            let prefix = if is_last { "● " } else { "  " };
                                            if ui.selectable_label(is_last, format!("{}{}", prefix, title)).clicked() {
                                                focus_win_idx = Some(idx);
                                                ui.close_menu();
                                            }

                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                if ui.button(Icons::rich(Icons::CLOSE, 10.0)).clicked() {
                                                    close_win_idx = Some(idx);
                                                    ui.close_menu();
                                                }
                                            });
                                        });
                                    }
                                }
                            }

                            if let Some(idx) = focus_win_idx {
                                if let Some(ws) = self.workspaces.get_mut(self.active_workspace_idx) {
                                    if idx < ws.windows.len() {
                                        let mut target = ws.windows.remove(idx);
                                        target.focus_requested = true;
                                        ws.windows.push(target);
                                    }
                                }
                            } else if let Some(idx) = close_win_idx {
                                if let Some(ws) = self.workspaces.get_mut(self.active_workspace_idx) {
                                    if idx < ws.windows.len() {
                                        ws.windows[idx].is_open = false;
                                    }
                                }
                            }
                        });

                        ui.add_space(4.0);

                        if ui.add(
                            egui::Button::new(Icons::rich(Icons::GEAR, 13.0))
                                .fill(btn_fill)
                                .stroke(egui::Stroke::new(1.0, btn_border))
                                .rounding(6.0)
                                .min_size(egui::vec2(30.0, 24.0)),
                        ).on_hover_text("Settings (Cmd+, / Ctrl+,)").clicked() {
                            self.open_or_focus_settings();
                        }

                        // New window menu button — styled to match Search/Settings buttons.
                        // We temporarily override the inactive widget style so menu_button
                        // renders with the same fill/border/rounding as other topbar buttons.
                        ui.add_space(4.0);
                        if let Some(ws) = self.workspaces.get_mut(self.active_workspace_idx) {
                            let saved_fill   = ui.visuals().widgets.inactive.weak_bg_fill;
                            let saved_stroke = ui.visuals().widgets.inactive.bg_stroke;
                            let saved_round  = ui.visuals().widgets.inactive.rounding;
                            ui.visuals_mut().widgets.inactive.weak_bg_fill = btn_fill;
                            ui.visuals_mut().widgets.inactive.bg_stroke    = egui::Stroke::new(1.0, btn_border);
                            ui.visuals_mut().widgets.inactive.rounding      = egui::Rounding::same(6.0);

                            let new_label = Icons::label_job(Icons::ADD, "New", 11.5, ui.visuals().text_color());
                            ui.menu_button(new_label, |ui| {
                                ws.show_new_window_menu(ui, ctx);
                            });

                            // Restore original visuals so other widgets are unaffected.
                            ui.visuals_mut().widgets.inactive.weak_bg_fill = saved_fill;
                            ui.visuals_mut().widgets.inactive.bg_stroke    = saved_stroke;
                            ui.visuals_mut().widgets.inactive.rounding      = saved_round;
                        }

                        {
                            let mut job = egui::text::LayoutJob::default();
                            job.append(
                                Icons::SEARCH,
                                0.0,
                                egui::text::TextFormat {
                                    font_id: egui::FontId::new(
                                        11.5,
                                        egui::FontFamily::Name("phosphor".into()),
                                    ),
                                    color: egui::Color32::PLACEHOLDER,
                                    ..Default::default()
                                },
                            );
                            job.append(
                                " Search",
                                0.0,
                                egui::text::TextFormat {
                                    font_id: egui::FontId::proportional(11.5),
                                    color: egui::Color32::PLACEHOLDER,
                                    ..Default::default()
                                },
                            );
                            if ui.add(
                                egui::Button::new(job)
                                    .fill(btn_fill)
                                    .stroke(egui::Stroke::new(1.0, btn_border))
                                    .rounding(6.0)
                                    .min_size(egui::vec2(68.0, 24.0)),
                            ).on_hover_text("Search (Cmd+P / Ctrl+P)").clicked() {
                                self.palette.toggle();
                            }
                        }
                    });
                });
            });
    }
}

impl eframe::App for XTermApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Color32::TRANSPARENT.to_normalized_gamma_f32()
    }

    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        session::save_session(&self.workspaces, self.active_workspace_idx, &self.config);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle window edge drag resizing for frameless window
        self.handle_frameless_resize(ctx);

        // Global shortcuts and window frame updates

        // ── Intercept OS close/quit event ────────────────────────────────────
        // close_requested() is true on the SAME frame the OS sends the close.
        // We must send CancelClose on that frame, AND keep sending it every
        // subsequent frame while the dialog is visible, otherwise the OS may
        // close the window between frames.
        let close_requested = ctx.input(|i| i.viewport().close_requested());
        if close_requested {
            session::save_session(&self.workspaces, self.active_workspace_idx, &self.config);
        }

        if self.close_warning_visible {
            // Keep blocking close while dialog is shown
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        } else if close_requested && !self.close_confirmed {
            let has_running = self.workspaces.iter().any(|ws| {
                ws.windows.iter().any(|w| w.app.is_running())
            });
            if has_running {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.close_warning_visible = true;
            }
            // No running commands → let OS close normally (no CancelClose)
        }

        if self.close_confirmed {
            session::save_session(&self.workspaces, self.active_workspace_idx, &self.config);
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        // ── Close-warning modal ───────────────────────────────────────────────
        if self.close_warning_visible {
            egui::Window::new("⚠  Commands Still Running")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.set_min_width(340.0);
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new(
                            "One or more terminal sessions have commands still running.\n\
                             Closing now will forcefully kill those processes."
                        )
                        .size(13.0)
                    );
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        if ui.add(
                            egui::Button::new(
                                egui::RichText::new("  Kill & Quit  ")
                                    .color(egui::Color32::WHITE)
                            )
                            .fill(egui::Color32::from_rgb(210, 50, 50))
                            .rounding(6.0)
                        ).clicked() {
                            for ws in &mut self.workspaces {
                                for w in &mut ws.windows {
                                    w.app.kill_all_processes();
                                }
                            }
                            self.close_confirmed = true;
                            self.close_warning_visible = false;
                        }

                        ui.add_space(8.0);

                        if ui.add(
                            egui::Button::new("  Cancel  ")
                                .rounding(6.0)
                        ).clicked() {
                            self.close_warning_visible = false;
                        }
                    });
                    ui.add_space(8.0);
                });
            return; // don't render the rest of the app while modal is visible
        }

        // ── Global Chrome-Style Keyboard Shortcuts ───────────────────────────
        if ctx.input(|i| !i.raw.events.is_empty()) {
            let shortcuts = &self.config.shortcuts;

            if match_shortcut(ctx, "Cmd+Shift+Z") || match_shortcut(ctx, "Ctrl+Shift+Z") {
                if self.undo_manager.can_redo() {
                    if let Some(entry) = self.undo_manager.redo() {
                        if let Some((rev_action, desc)) = crate::ui::undo_manager::execute_undo_action(&entry.action) {
                            self.undo_manager.show_toast(format!("Redo: {}", desc));
                            self.apply_undo_action(&rev_action);
                        }
                    }
                }
            } else if match_shortcut(ctx, "Cmd+Z") || match_shortcut(ctx, "Ctrl+Z") {
                if self.undo_manager.can_undo() {
                    if let Some(entry) = self.undo_manager.undo() {
                        if let Some((rev_action, desc)) = crate::ui::undo_manager::execute_undo_action(&entry.action) {
                            self.undo_manager.show_toast(format!("Undo: {}", desc));
                            self.apply_undo_action(&rev_action);
                        }
                    }
                }
            } else if match_shortcut(ctx, &shortcuts.new_terminal) {
                let win_id = uuid::Uuid::new_v4().to_string();
                if let Some(ws) = self.workspaces.get_mut(self.active_workspace_idx) {
                    let count = ws.windows.len();
                    let title = if count == 0 { "Terminal".to_string() } else { format!("Terminal ({})", count) };
                    ws.push_window(FloatingWindow::new(win_id, Box::new(TerminalApp::new_local(title, ctx))));
                }
            } else if match_shortcut(ctx, &shortcuts.close_window) {
                if let Some(ws) = self.workspaces.get_mut(self.active_workspace_idx) {
                    ws.close_active_window();
                }
            } else if match_shortcut(ctx, &shortcuts.reopen_window) {
                if let Some(ws) = self.workspaces.get_mut(self.active_workspace_idx) {
                    ws.reopen_last_closed_window();
                }
            } else if match_shortcut(ctx, &shortcuts.next_window) {
                if let Some(ws) = self.workspaces.get_mut(self.active_workspace_idx) {
                    ws.cycle_window_focus(true);
                }
            } else if match_shortcut(ctx, &shortcuts.prev_window) {
                if let Some(ws) = self.workspaces.get_mut(self.active_workspace_idx) {
                    ws.cycle_window_focus(false);
                }
            } else if match_shortcut(ctx, &shortcuts.next_workspace) {
                if !self.workspaces.is_empty() {
                    self.active_workspace_idx = (self.active_workspace_idx + 1) % self.workspaces.len();
                }
            } else if match_shortcut(ctx, &shortcuts.prev_workspace) {
                if !self.workspaces.is_empty() {
                    self.active_workspace_idx = if self.active_workspace_idx == 0 {
                        self.workspaces.len() - 1
                    } else {
                        self.active_workspace_idx - 1
                    };
                }
            } else if match_shortcut(ctx, &shortcuts.open_settings) {
                self.open_or_focus_settings();
            } else if match_shortcut(ctx, "Cmd+Shift+G") || match_shortcut(ctx, "Ctrl+Shift+G") {
                self.open_or_focus_git_manager();
            } else if match_shortcut(ctx, &shortcuts.command_palette) || match_shortcut(ctx, &shortcuts.find) {
                self.palette.toggle();
            } else if match_shortcut(ctx, &shortcuts.jump_workspace_1) && !self.workspaces.is_empty() {
                self.active_workspace_idx = 0;
            } else if match_shortcut(ctx, &shortcuts.jump_workspace_2) && self.workspaces.len() > 1 {
                self.active_workspace_idx = 1;
            } else if match_shortcut(ctx, &shortcuts.jump_workspace_3) && self.workspaces.len() > 2 {
                self.active_workspace_idx = 2;
            } else if match_shortcut(ctx, &shortcuts.jump_workspace_4) && self.workspaces.len() > 3 {
                self.active_workspace_idx = 3;
            } else if match_shortcut(ctx, &shortcuts.jump_workspace_5) && self.workspaces.len() > 4 {
                self.active_workspace_idx = 4;
            } else if match_shortcut(ctx, &shortcuts.jump_workspace_6) && self.workspaces.len() > 5 {
                self.active_workspace_idx = 5;
            } else if match_shortcut(ctx, &shortcuts.jump_workspace_7) && self.workspaces.len() > 6 {
                self.active_workspace_idx = 6;
            } else if match_shortcut(ctx, &shortcuts.jump_workspace_8) && self.workspaces.len() > 7 {
                self.active_workspace_idx = 7;
            } else if match_shortcut(ctx, &shortcuts.jump_workspace_9) && self.workspaces.len() > 8 {
                self.active_workspace_idx = 8;
            }
        }

        // Apply theme dynamically only when configuration changes.
        // Compare by reference (&str) to avoid cloning String fields every frame.
        let theme_changed = self.last_applied_theme.as_ref().map_or(true, |(t, blur, rnd)| {
            t != &self.config.theme
                || *blur != self.config.blur_level
                || *rnd != self.config.window_rounding
        });
        if theme_changed {
            theme::apply_theme(ctx, &self.config);
            self.last_applied_theme = Some((self.config.theme.clone(), self.config.blur_level, self.config.window_rounding));
        }

        let font_changed = self.last_applied_fonts.as_ref().map_or(true, |(ui_f, mono_f, ui_sz, mono_sz)| {
            ui_f.as_str() != self.config.ui_font_family.as_str()
                || mono_f.as_str() != self.config.mono_font_family.as_str()
                || *ui_sz != self.config.ui_font_size
                || *mono_sz != self.config.mono_font_size
        });
        if font_changed {
            self.current_ui_font = self.config.ui_font_family.clone();
            self.current_mono_font = self.config.mono_font_family.clone();
            theme::apply_font(ctx, &self.current_ui_font, &self.current_mono_font);
            theme::apply_theme(ctx, &self.config);
            self.last_applied_fonts = Some((
                self.config.ui_font_family.clone(),
                self.config.mono_font_family.clone(),
                self.config.ui_font_size,
                self.config.mono_font_size,
            ));
        }

        // Ensure at least 1 workspace exists and active index is always valid
        if self.workspaces.is_empty() {
            self.workspaces.push(Workspace::new("Workspace 1", ctx));
            self.active_workspace_idx = 0;
        } else if self.active_workspace_idx >= self.workspaces.len() {
            self.active_workspace_idx = self.workspaces.len() - 1;
        }

        self.render_top_bar(ctx);

        if let Some(workspace) = self.workspaces.get_mut(self.active_workspace_idx) {
            workspace.render(ctx, &mut self.config, &mut self.undo_manager);
        }

        // Render command palette
        if let Some(action) = self.palette.render(ctx) {
            match action {
                crate::ui::palette::PaletteAction::OpenFile(path) => {
                    if let Ok(editor) = EditorApp::open(&path) {
                        if let Some(ws) = self.workspaces.get_mut(self.active_workspace_idx) {
                            let win_id = uuid::Uuid::new_v4().to_string();
                            let app = Box::new(editor);
                            ws.windows.push(FloatingWindow::new(win_id, app));
                        }
                    }
                }
                crate::ui::palette::PaletteAction::Command(cmd) => {
                    if cmd == "Settings" {
                        self.open_or_focus_settings();
                    } else if cmd == "Git Manager" {
                        self.open_or_focus_git_manager();
                    } else if cmd == "Search" {
                        self.palette.toggle();
                    } else if cmd == "New Notepad" {
                        if let Some(ws) = self.workspaces.get_mut(self.active_workspace_idx) {
                            let win_id = uuid::Uuid::new_v4().to_string();
                            ws.windows.push(FloatingWindow::new(win_id, Box::new(EditorApp::new_untitled())));
                        }
                    } else if cmd == "Local Terminal" {
                        if let Some(ws) = self.workspaces.get_mut(self.active_workspace_idx) {
                            let win_id = uuid::Uuid::new_v4().to_string();
                            let term = crate::ui::terminal_app::TerminalApp::new_local(win_id.clone(), ctx);
                            ws.windows.push(FloatingWindow::new(win_id, Box::new(term)));
                        }
                    } else if cmd == "SSH Manager" {
                        if let Some(ws) = self.workspaces.get_mut(self.active_workspace_idx) {
                            let win_id = uuid::Uuid::new_v4().to_string();
                            ws.windows.push(FloatingWindow::new(win_id, Box::new(crate::ui::ssh_manager::SshManagerApp::new())));
                        }
                    } else if cmd == "New Workspace" {
                        let new_idx = self.workspaces.len() + 1;
                        self.workspaces.push(Workspace::new(&format!("Workspace {}", new_idx), ctx));
                        self.active_workspace_idx = self.workspaces.len() - 1;
                    }
                }
            }
        }

        // ── Undo/Redo Toast Notification ─────────────────────────────────────
        if self.undo_manager.take_expired_toast(2.5) {
            if let Some((msg, _)) = &self.undo_manager.toast {
                let toast_msg = msg.clone();
                egui::Area::new(egui::Id::new("undo_toast"))
                    .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -40.0])
                    .interactable(false)
                    .order(egui::Order::Foreground)
                    .show(ctx, |ui| {
                        egui::Frame::default()
                            .fill(egui::Color32::from_rgba_unmultiplied(30, 30, 30, 230))
                            .rounding(8.0)
                            .inner_margin(egui::Margin::symmetric(16.0, 10.0))
                            .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(60)))
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(&toast_msg)
                                        .color(egui::Color32::from_gray(220))
                                        .size(13.0),
                                );
                            });
                    });
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            }
        }

        // Auto-save session state periodically (every 5 seconds) instead of every frame
        if self.last_saved_time.elapsed() >= std::time::Duration::from_secs(5) {
            session::save_session(&self.workspaces, self.active_workspace_idx, &self.config);
            self.last_saved_time = std::time::Instant::now();
        }
    }
}

fn match_shortcut(ctx: &egui::Context, shortcut: &str) -> bool {
    if shortcut.is_empty() {
        return false;
    }
    ctx.input(|i| {
        let parts: Vec<&str> = shortcut.split('+').collect();
        if parts.is_empty() {
            return false;
        }

        let target_key = *parts.last().unwrap();
        let has_ctrl = parts.iter().any(|p| *p == "Ctrl" || *p == "Cmd");
        let has_shift = parts.iter().any(|p| *p == "Shift");
        let has_alt = parts.iter().any(|p| *p == "Alt");

        let mod_ctrl = i.modifiers.ctrl || i.modifiers.command;
        let mod_shift = i.modifiers.shift;
        let mod_alt = i.modifiers.alt;

        if has_ctrl && !mod_ctrl {
            return false;
        }
        if has_shift && !mod_shift {
            return false;
        }
        if has_alt && !mod_alt {
            return false;
        }

        let key = match target_key {
            "T" | "t" => egui::Key::T,
            "W" | "w" => egui::Key::W,
            "P" | "p" => egui::Key::P,
            "F" | "f" => egui::Key::F,
            "G" | "g" => egui::Key::G,
            "Z" | "z" => egui::Key::Z,
            "Tab" => egui::Key::Tab,
            "Right" => egui::Key::ArrowRight,
            "Left" => egui::Key::ArrowLeft,
            "," => egui::Key::Comma,
            "1" => egui::Key::Num1,
            "2" => egui::Key::Num2,
            "3" => egui::Key::Num3,
            "4" => egui::Key::Num4,
            "5" => egui::Key::Num5,
            "6" => egui::Key::Num6,
            "7" => egui::Key::Num7,
            "8" => egui::Key::Num8,
            "9" => egui::Key::Num9,
            _ => return false,
        };

        i.key_pressed(key)
    })
}
