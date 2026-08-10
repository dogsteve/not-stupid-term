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

        // Load Fira Code font
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "FiraCode".to_owned(),
            egui::FontData::from_static(include_bytes!(
                "../../assets/fonts/FiraCode-Regular.ttf"
            )),
        );
        fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap().insert(0, "FiraCode".to_owned());
        fonts.families.get_mut(&egui::FontFamily::Monospace).unwrap().insert(0, "FiraCode".to_owned());
        cc.egui_ctx.set_fonts(fonts);

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
        let window_rounding = if is_mac { 12.0 } else { 8.0 };

        let panel_frame = egui::Frame::default()
            .fill(egui::Color32::TRANSPARENT)
            .inner_margin(egui::Margin {
                left: 8.0,
                right: 8.0,
                top: 8.0,
                bottom: 0.0,
            })
            .rounding(egui::Rounding {
                nw: window_rounding,
                ne: window_rounding,
                sw: 0.0,
                se: 0.0,
            })
            .fill(ctx.style().visuals.panel_fill);

        egui::TopBottomPanel::top("top_bar").frame(panel_frame).show(ctx, |ui| {
            // Top Bar Double-Click for Fullscreen/Maximize & Single Click Dragging
            let title_bar_rect = ui.max_rect();
            let title_bar_response =
                ui.interact(title_bar_rect, ui.id().with("main_title_bar"), egui::Sense::click_and_drag());

            if title_bar_response.double_clicked() {
                let is_maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
            } else if title_bar_response.is_pointer_button_down_on() {
                ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }

            ui.horizontal(|ui| {
                let is_mac = std::env::consts::OS == "macos";

                if is_mac {
                    // macOS style window controls (Left)
                    ui.add_space(4.0);

                    // Close (Red)
                    let (rect, response) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::click());
                    if ui.is_rect_visible(rect) {
                        let color = egui::Color32::from_rgb(255, 95, 86);
                        ui.painter().circle_filled(rect.center(), 6.0, color);
                    }
                    if response.clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }

                    ui.add_space(4.0);

                    // Minimize (Yellow)
                    let (rect, response) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::click());
                    if ui.is_rect_visible(rect) {
                        let color = egui::Color32::from_rgb(255, 189, 46);
                        ui.painter().circle_filled(rect.center(), 6.0, color);
                    }
                    if response.clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                    }

                    ui.add_space(4.0);

                    // Maximize / Fullscreen (Green) - Double click / Click to toggle Maximize
                    let (rect, response) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::click());
                    if ui.is_rect_visible(rect) {
                        let color = egui::Color32::from_rgb(39, 201, 63);
                        ui.painter().circle_filled(rect.center(), 6.0, color);
                    }
                    if response.clicked() {
                        let is_maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
                    }

                    ui.add_space(16.0);
                } else {
                    ui.add_space(8.0);
                }

                let mut to_remove = None;
                for (idx, workspace) in self.workspaces.iter_mut().enumerate() {
                    let is_active = idx == self.active_workspace_idx;

                    if workspace.is_editing_name {
                        let response =
                            ui.add(egui::TextEdit::singleline(&mut workspace.name).desired_width(100.0));
                        if response.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            workspace.is_editing_name = false;
                        }
                        response.request_focus();
                    } else {
                        let mut is_close_clicked = false;

                        let tab_color = if is_active {
                            ctx.style().visuals.window_fill()
                        } else {
                            ctx.style().visuals.faint_bg_color
                        };

                        let tab_frame = egui::Frame::default()
                            .fill(tab_color)
                            .rounding(egui::Rounding {
                                nw: 12.0,
                                ne: 12.0,
                                sw: 0.0,
                                se: 0.0,
                            })
                            .inner_margin(egui::Margin {
                                left: 16.0,
                                right: 8.0,
                                top: 8.0,
                                bottom: 6.0,
                            });

                        let mut close_button_rect = egui::Rect::NOTHING;

                        let tab_response = tab_frame
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(&workspace.name);

                                    ui.add_space(4.0);
                                    let btn_resp =
                                        ui.add(egui::Button::new(Icons::CLOSE).fill(egui::Color32::TRANSPARENT).frame(false));
                                    close_button_rect = btn_resp.rect;
                                    if btn_resp.clicked() {
                                        is_close_clicked = true;
                                    }
                                });
                            })
                            .response;

                        let interact_response =
                            ui.interact(tab_response.rect, ui.id().with(idx), egui::Sense::click());
                        if interact_response.clicked() {
                            if let Some(pos) = interact_response.interact_pointer_pos() {
                                if close_button_rect.expand(4.0).contains(pos) {
                                    is_close_clicked = true;
                                } else {
                                    self.active_workspace_idx = idx;
                                }
                            } else {
                                self.active_workspace_idx = idx;
                            }
                        }

                        if interact_response.double_clicked() {
                            workspace.is_editing_name = true;
                        }

                        interact_response.context_menu(|ui| {
                            if ui.button(format!("{} Rename", Icons::EDIT)).clicked() {
                                workspace.is_editing_name = true;
                                ui.close_menu();
                            }
                        });

                        if is_close_clicked {
                            to_remove = Some(idx);
                        }
                    }
                }

                if let Some(idx) = to_remove {
                    self.workspaces.remove(idx);
                    if self.active_workspace_idx >= self.workspaces.len() && !self.workspaces.is_empty() {
                        self.active_workspace_idx = self.workspaces.len() - 1;
                    }
                }

                ui.add_space(8.0);
                if ui.button(format!("{} New", Icons::ADD)).clicked() {
                    let new_idx = self.workspaces.len() + 1;
                    self.workspaces.push(Workspace::new(&format!("Workspace {}", new_idx), ctx));
                    self.active_workspace_idx = self.workspaces.len() - 1;
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let is_mac = std::env::consts::OS == "macos";
                    if !is_mac {
                        ui.add_space(4.0);
                        if ui.add(egui::Button::new(Icons::CLOSE).fill(egui::Color32::TRANSPARENT).frame(false)).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        if ui.add(egui::Button::new(Icons::ADD).fill(egui::Color32::TRANSPARENT).frame(false)).clicked() {
                            let is_maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
                        }
                        if ui.add(egui::Button::new("—").fill(egui::Color32::TRANSPARENT).frame(false)).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                        }
                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(8.0);
                    }

                    // Settings Button -> Spawn SettingsApp floating window
                    if ui.button(format!("{} Settings", Icons::GEAR)).clicked() {
                        if let Some(ws) = self.workspaces.get_mut(self.active_workspace_idx) {
                            let win_id = uuid::Uuid::new_v4().to_string();
                            let app = Box::new(SettingsApp);
                            ws.windows.push(FloatingWindow::new(win_id, app));
                        }
                    }

                    ui.add_space(4.0);

                    // Command Palette Search Button
                    if ui.button(format!("{} Search (Cmd+P)", Icons::SEARCH)).clicked() {
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
