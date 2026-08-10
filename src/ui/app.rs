use eframe::egui;

use crate::models::state::AppState;
use crate::ui::editor::EditorApp;
use crate::ui::icons::Icons;
use crate::ui::palette::CommandPalette;
use crate::ui::settings::{AppConfig, SettingsApp};
use crate::ui::theme;
use crate::ui::window_framework::FloatingWindow;
use crate::ui::workspace::Workspace;

pub struct XTermApp {
    state: AppState,
    workspaces: Vec<Workspace>,
    active_workspace_idx: usize,
    config: AppConfig,
    last_theme_sync: f64,
    current_font: String,
    close_warning_workspace: Option<usize>,
    sys: sysinfo::System,
    palette: CommandPalette,
}

impl XTermApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Apply acrylic/vibrancy effect to the window
        // Note: window_vibrancy has a z-order bug on macOS with winit 0.30 where it covers the Egui UI.
        // crate::utils::vibrancy::apply_window_vibrancy(cc);

        theme::apply_theme(&cc.egui_ctx, &AppConfig::default());
        theme::apply_font(&cc.egui_ctx, &AppConfig::default().font_family);

        let state = AppState::default();

        let workspaces = vec![Workspace::new("Workspace 1", &cc.egui_ctx)];

        Self {
            state,
            workspaces,
            active_workspace_idx: 0,
            config: AppConfig::default(),
            last_theme_sync: 0.0,
            current_font: "FiraCode".to_owned(),
            close_warning_workspace: None,
            sys: sysinfo::System::new(),
            palette: CommandPalette::new(),
        }
    }

    fn render_top_bar(&mut self, ctx: &egui::Context) {
        let is_mac = std::env::consts::OS == "macos";
        let is_dark = ctx.style().visuals.dark_mode;
        let accent = ctx.style().visuals.selection.bg_fill;

        let panel_frame = egui::Frame::default()
            .fill(ctx.style().visuals.panel_fill)
            .inner_margin(egui::Margin {
                left: 8.0, right: 8.0, top: 6.0, bottom: 0.0,
            })
            .rounding(egui::Rounding {
                nw: if is_mac { 12.0 } else { 0.0 },
                ne: if is_mac { 12.0 } else { 0.0 },
                sw: 0.0, se: 0.0,
            });

        egui::TopBottomPanel::top("top_bar").frame(panel_frame).show(ctx, |ui| {
            // Drag + double-click maximize
            let title_rect = ui.max_rect();
            let title_resp = ui.interact(title_rect, ui.id().with("title_bar"), egui::Sense::click_and_drag());
            if title_resp.double_clicked() {
                let max = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!max));
            } else if title_resp.is_pointer_button_down_on() {
                ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }

            ui.horizontal(|ui| {
                // macOS traffic lights
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

                // === Workspace Tabs ===
                let mut to_remove = None;
                for (idx, workspace) in self.workspaces.iter_mut().enumerate() {
                    let is_active = idx == self.active_workspace_idx;

                    if workspace.is_editing_name {
                        let resp = ui.add(
                            egui::TextEdit::singleline(&mut workspace.name)
                                .desired_width(100.0)
                                .font(egui::FontId::proportional(12.5))
                        );
                        // Exit edit on: Enter, Escape, or clicking elsewhere
                        let should_close = resp.lost_focus()
                            || ui.input(|i| i.key_pressed(egui::Key::Enter))
                            || ui.input(|i| i.key_pressed(egui::Key::Escape));
                        if should_close {
                            workspace.is_editing_name = false;
                        }
                        if !resp.lost_focus() {
                            resp.request_focus();
                        }
                    } else {
                        // Tab with name + close button
                        let text_color = if is_active {
                            ctx.style().visuals.text_color()
                        } else if is_dark {
                            egui::Color32::from_gray(120)
                        } else {
                            egui::Color32::from_gray(130)
                        };

                        let tab_id = ui.id().with(("tab", idx));

                        // Draw tab as a group
                        let tab_resp = ui.horizontal(|ui| {
                            // Tab name button
                            let name_resp = ui.add(
                                egui::Button::new(
                                    egui::RichText::new(&workspace.name).size(12.5).color(text_color)
                                )
                                .fill(egui::Color32::TRANSPARENT)
                                .stroke(egui::Stroke::NONE)
                                .rounding(egui::Rounding { nw: 6.0, ne: 6.0, sw: 0.0, se: 0.0 })
                                .min_size(egui::vec2(0.0, 26.0))
                            );

                            if name_resp.clicked() {
                                self.active_workspace_idx = idx;
                            }
                            if name_resp.double_clicked() {
                                workspace.is_editing_name = true;
                            }

                            // Close button (×) — always visible
                            let close_color = if is_dark {
                                egui::Color32::from_gray(80)
                            } else {
                                egui::Color32::from_gray(150)
                            };
                            let close_resp = ui.add(
                                egui::Button::new(
                                    egui::RichText::new(Icons::CLOSE).size(10.0).color(close_color)
                                )
                                .fill(egui::Color32::TRANSPARENT)
                                .stroke(egui::Stroke::NONE)
                                .rounding(4.0)
                                .min_size(egui::vec2(18.0, 18.0))
                            );
                            if close_resp.clicked() {
                                to_remove = Some(idx);
                            }

                            name_resp
                        });

                        // Active indicator: accent underline
                        if is_active {
                            let r = tab_resp.response.rect;
                            ui.painter().hline(
                                r.left()..=r.right(),
                                r.bottom(),
                                egui::Stroke::new(2.0, accent),
                            );
                        }

                        // Context menu
                        tab_resp.response.context_menu(|ui| {
                            if ui.button(format!("{} Rename", Icons::EDIT)).clicked() {
                                workspace.is_editing_name = true;
                                ui.close_menu();
                            }
                            if ui.button(format!("{} Close", Icons::CLOSE)).clicked() {
                                to_remove = Some(idx);
                                ui.close_menu();
                            }
                        });
                    }
                }

                if let Some(idx) = to_remove {
                    self.workspaces.remove(idx);
                    if self.active_workspace_idx >= self.workspaces.len() && !self.workspaces.is_empty() {
                        self.active_workspace_idx = self.workspaces.len() - 1;
                    }
                }

                // New tab button
                ui.add_space(4.0);
                if ui.add(
                    egui::Button::new(egui::RichText::new("+").size(14.0))
                        .fill(egui::Color32::TRANSPARENT)
                        .rounding(6.0)
                        .min_size(egui::vec2(28.0, 28.0))
                ).on_hover_text("New workspace").clicked() {
                    let n = self.workspaces.len() + 1;
                    self.workspaces.push(Workspace::new(&format!("Workspace {}", n), ctx));
                    self.active_workspace_idx = self.workspaces.len() - 1;
                }

                // Right side buttons
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if !is_mac {
                        // Windows-style controls
                        if ui.add(egui::Button::new("✕").fill(egui::Color32::TRANSPARENT).rounding(4.0).min_size(egui::vec2(32.0, 28.0))).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        if ui.add(egui::Button::new("□").fill(egui::Color32::TRANSPARENT).rounding(4.0).min_size(egui::vec2(32.0, 28.0))).clicked() {
                            let m = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!m));
                        }
                        if ui.add(egui::Button::new("—").fill(egui::Color32::TRANSPARENT).rounding(4.0).min_size(egui::vec2(32.0, 28.0))).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                        }
                        ui.add_space(4.0);
                        ui.separator();
                    }

                    // Settings
                    if ui.add(
                        egui::Button::new(egui::RichText::new(format!("{}", Icons::GEAR)).size(14.0))
                            .rounding(6.0)
                            .min_size(egui::vec2(32.0, 28.0))
                    ).on_hover_text("Settings").clicked() {
                        if let Some(ws) = self.workspaces.get_mut(self.active_workspace_idx) {
                            let win_id = uuid::Uuid::new_v4().to_string();
                            ws.windows.push(FloatingWindow::new(win_id, Box::new(SettingsApp)));
                        }
                    }

                    // Search
                    if ui.add(
                        egui::Button::new(egui::RichText::new(format!("{} Search", Icons::SEARCH)).size(12.0))
                            .rounding(6.0)
                            .min_size(egui::vec2(0.0, 28.0))
                    ).on_hover_text("Cmd+P").clicked() {
                        self.palette.toggle();
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

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

        // Shortcut Cmd+P or Ctrl+P to trigger file search palette
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::P)) {
            self.palette.toggle();
        }

        // Apply theme dynamically in case config changes
        theme::apply_theme(ctx, &self.config);

        if self.config.font_family != self.current_font {
            self.current_font = self.config.font_family.clone();
            theme::apply_font(ctx, &self.current_font);
        }

        self.render_top_bar(ctx);

        if let Some(workspace) = self.workspaces.get_mut(self.active_workspace_idx) {
            workspace.render(ctx, &mut self.config);
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
                        if let Some(ws) = self.workspaces.get_mut(self.active_workspace_idx) {
                            let win_id = uuid::Uuid::new_v4().to_string();
                            ws.windows.push(FloatingWindow::new(win_id, Box::new(crate::ui::settings::SettingsApp)));
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
    }
}
