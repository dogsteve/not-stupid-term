use eframe::egui;
use uuid::Uuid;

use super::editor::EditorApp;
use super::file_viewer::FileViewerApp;
use super::icons::Icons;
use super::settings::SettingsApp;
use super::sftp_app::SftpApp;
use super::ssh_manager::SshManagerApp;
use super::terminal_app::TerminalApp;
use super::window_framework::{FloatingWindow, WindowAction};

pub struct Workspace {
    pub id: String,
    pub name: String,
    pub windows: Vec<FloatingWindow>,
    pub is_editing_name: bool,
    pub closed_windows_stack: Vec<FloatingWindow>,
}

impl Workspace {
    pub fn new(name: &str, ctx: &egui::Context) -> Self {
        let win_id = Uuid::new_v4().to_string();
        let default_terminal = Box::new(TerminalApp::new_local("Terminal", ctx));
        let default_window = FloatingWindow::new(win_id, default_terminal);

        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            windows: vec![default_window],
            is_editing_name: false,
            closed_windows_stack: Vec::new(),
        }
    }

    pub fn push_window(&mut self, mut window: FloatingWindow) {
        window.focus_requested = true;
        self.windows.push(window);
    }

    /// Spawns a new local terminal window and pushes it to the front.
    /// Kept for use by keyboard shortcuts (Ctrl+T).
    pub fn open_new_terminal(&mut self, ctx: &egui::Context) {
        let n = self.windows.len() + 1;
        let title = format!("Terminal {}", n);
        let win_id = uuid::Uuid::new_v4().to_string();
        let term = Box::new(TerminalApp::new_local(title, ctx));
        self.push_window(FloatingWindow::new(win_id, term));
    }

    /// Renders the "New Window" dropdown menu body.
    /// Call this inside a `ui.menu_button(...)` closure to share the spawn menu
    /// between the topbar + button and the workspace footer New button.
    pub fn show_new_window_menu(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.set_min_width(200.0);

        let items = [
            (Icons::TERMINAL, "Local Terminal"),
            (Icons::NOTE,     "New Text File (Notepad)"),
            (Icons::FOLDER,   "File Viewer"),
            (Icons::SERVER,   "SSH & SFTP Manager"),
            (Icons::SERVER,   "SFTP Remote Browser"),
            (Icons::GEAR,     "Settings"),
        ];

        for (icon, label) in items {
            let job = Icons::label_job(icon, label, 12.0, ui.visuals().text_color());
            if ui.add(
                egui::Button::new(job)
                    .min_size(egui::vec2(ui.available_width(), 30.0)),
            ).clicked() {
                let win_id = uuid::Uuid::new_v4().to_string();
                match label {
                    "Local Terminal" => {
                        let count = self.windows.len();
                        let title = if count == 0 { "Terminal".to_string() } else { format!("Terminal ({})", count) };
                        self.push_window(FloatingWindow::new(win_id, Box::new(TerminalApp::new_local(title, ctx))));
                    }
                    "New Text File (Notepad)" => {
                        self.push_window(FloatingWindow::new(win_id, Box::new(EditorApp::new_untitled())));
                    }
                    "File Viewer" => {
                        self.push_window(FloatingWindow::new(win_id, Box::new(FileViewerApp::new())));
                    }
                    "SSH & SFTP Manager" => {
                        self.push_window(FloatingWindow::new(win_id, Box::new(SshManagerApp::new())));
                    }
                    "SFTP Remote Browser" => {
                        self.push_window(FloatingWindow::new(win_id, Box::new(SftpApp::new())));
                    }
                    "Settings" => {
                        if let Some(pos) = self.windows.iter().position(|w| w.app.window_type() == "settings") {
                            let mut settings_win = self.windows.remove(pos);
                            settings_win.focus_requested = true;
                            self.windows.push(settings_win);
                        } else {
                            self.push_window(FloatingWindow::new(win_id, Box::new(SettingsApp)));
                        }
                    }
                    _ => {}
                }
                ui.close_menu();
            }
        }
    }

    pub fn close_active_window(&mut self) -> bool {
        if let Some(mut win) = self.windows.pop() {
            win.is_open = false;
            self.closed_windows_stack.push(win);
            if self.closed_windows_stack.len() > 10 {
                self.closed_windows_stack.remove(0);
            }
            if let Some(top) = self.windows.last_mut() {
                top.focus_requested = true;
            }
            true
        } else {
            false
        }
    }

    pub fn reopen_last_closed_window(&mut self) -> bool {
        if let Some(mut win) = self.closed_windows_stack.pop() {
            win.is_open = true;
            win.focus_requested = true;
            self.windows.push(win);
            true
        } else {
            false
        }
    }

    pub fn cycle_window_focus(&mut self, forward: bool) {
        if self.windows.len() > 1 {
            if forward {
                self.windows.rotate_left(1);
            } else {
                self.windows.rotate_right(1);
            }
            if let Some(top) = self.windows.last_mut() {
                top.focus_requested = true;
            }
        }
    }

    pub fn render(&mut self, ctx: &egui::Context, config: &mut crate::ui::settings::AppConfig) {
        let is_dark = ctx.style().visuals.dark_mode;

        // Fluent workspace background with subtle noise-like texture via layered fills
        let workspace_bg = if is_dark {
            egui::Color32::from_rgb(18, 18, 22)
        } else {
            egui::Color32::from_rgb(243, 243, 246)
        };

        let frame = egui::Frame::default()
            .fill(workspace_bg)
            .rounding(egui::Rounding {
                nw: 0.0,
                ne: 0.0,
                sw: 12.0,
                se: 12.0,
            });

        let mut actions = Vec::new();

        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            ui.add_space(8.0);

            // Floating action buttons — centered pill with proper margin
            ui.horizontal(|ui| {
                ui.add_space(12.0);

                let _accent = ui.style().visuals.selection.bg_fill;

                // "+ New" dropdown button — floating pill style
                let btn_bg = if is_dark {
                    egui::Color32::from_white_alpha(12)
                } else {
                    egui::Color32::from_black_alpha(8)
                };

                let pill_frame = egui::Frame::default()
                    .fill(btn_bg)
                    .rounding(8.0)
                    .inner_margin(egui::Margin::symmetric(4.0, 2.0))
                    .stroke(egui::Stroke::new(1.0, if is_dark {
                        egui::Color32::from_white_alpha(15)
                    } else {
                        egui::Color32::from_black_alpha(12)
                    }));

                pill_frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;

                        // Dropdown spawn menu — delegates to shared show_new_window_menu()
                        ui.menu_button(
                            Icons::label_job(Icons::ADD, "New", 12.0, ui.visuals().text_color()),
                            |ui| { self.show_new_window_menu(ui, ctx); },
                        );

                        // Separator dot
                        let dot_color = if is_dark {
                            egui::Color32::from_gray(50)
                        } else {
                            egui::Color32::from_gray(190)
                        };
                        ui.label(egui::RichText::new("·").size(12.0).color(dot_color));

                        // Window count
                        let count = self.windows.len();
                        let count_color = if is_dark {
                            egui::Color32::from_gray(180)
                        } else {
                            egui::Color32::from_gray(60)
                        };
                        ui.label(
                            egui::RichText::new(format!("{}", count))
                                .size(11.5)
                                .color(count_color),
                        );
                    });
                });
            });

            ui.add_space(2.0);
        });

        // Render floating windows
        for window in self.windows.iter_mut() {
            if let Some(act) = window.render(ctx, config) {
                actions.push(act);
            }
        }

        // Track and remove closed windows for Ctrl+Shift+T restore
        let mut i = 0;
        while i < self.windows.len() {
            if !self.windows[i].is_open {
                let mut closed_win = self.windows.remove(i);
                closed_win.focus_requested = true;
                if self.closed_windows_stack.len() >= 10 {
                    self.closed_windows_stack.remove(0);
                }
                self.closed_windows_stack.push(closed_win);
            } else {
                i += 1;
            }
        }

        // Process window actions
        for act in actions {
            match act {
                WindowAction::ConnectSsh(cmd) => {
                    let win_id = Uuid::new_v4().to_string();
                    self.push_window(FloatingWindow::new(win_id, Box::new(TerminalApp::new_ssh("SSH Session", cmd, ctx))));
                }
                WindowAction::OpenSftp(host) => {
                    let win_id = Uuid::new_v4().to_string();
                    self.push_window(FloatingWindow::new(win_id, Box::new(SftpApp::with_host(host))));
                }
                WindowAction::OpenFile(path) => {
                    if let Ok(editor) = EditorApp::open(&path) {
                        let win_id = Uuid::new_v4().to_string();
                        self.push_window(FloatingWindow::new(win_id, Box::new(editor)));
                    }
                }
            }
        }
    }
}
