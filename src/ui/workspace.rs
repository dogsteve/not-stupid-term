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
}

impl Workspace {
    pub fn new(name: &str, ctx: &egui::Context) -> Self {
        let win_id = Uuid::new_v4().to_string();
        let default_terminal = Box::new(TerminalApp::new_local("zsh", ctx));
        let default_window = FloatingWindow::new(win_id, default_terminal);

        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            windows: vec![default_window],
            is_editing_name: false,
        }
    }

    pub fn render(&mut self, ctx: &egui::Context, config: &mut crate::ui::settings::AppConfig) {
        let is_mac = std::env::consts::OS == "macos";
        let window_rounding = if is_mac { 12.0 } else { 8.0 };

        let frame = egui::Frame::default()
            .fill(ctx.style().visuals.window_fill())
            .rounding(egui::Rounding {
                nw: 0.0,
                ne: 0.0,
                sw: window_rounding,
                se: window_rounding,
            });

        let mut actions = Vec::new();

        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Single unified dropdown button with real icons
                let btn_text = format!("{} Spawn Window...", Icons::ADD);
                ui.menu_button(btn_text, |ui| {
                    if ui.button(format!("{} Local Terminal", Icons::TERMINAL)).clicked() {
                        let win_id = Uuid::new_v4().to_string();
                        let count = self.windows.len();
                        let title = if count == 0 { "zsh".to_string() } else { format!("zsh ({})", count) };
                        let app = Box::new(TerminalApp::new_local(title, ctx));
                        self.windows.push(FloatingWindow::new(win_id, app));
                        ui.close_menu();
                    }

                    if ui.button(format!("{} File Viewer", Icons::FOLDER)).clicked() {
                        let win_id = Uuid::new_v4().to_string();
                        let app = Box::new(FileViewerApp::new());
                        self.windows.push(FloatingWindow::new(win_id, app));
                        ui.close_menu();
                    }

                    if ui.button(format!("{} SSH & SFTP Manager", Icons::SERVER)).clicked() {
                        let win_id = Uuid::new_v4().to_string();
                        let app = Box::new(SshManagerApp::new());
                        self.windows.push(FloatingWindow::new(win_id, app));
                        ui.close_menu();
                    }

                    if ui.button(format!("{} SFTP Remote Browser", Icons::SERVER)).clicked() {
                        let win_id = Uuid::new_v4().to_string();
                        let app = Box::new(SftpApp::new());
                        self.windows.push(FloatingWindow::new(win_id, app));
                        ui.close_menu();
                    }

                    if ui.button(format!("{} Settings", Icons::GEAR)).clicked() {
                        let win_id = Uuid::new_v4().to_string();
                        let app = Box::new(SettingsApp);
                        self.windows.push(FloatingWindow::new(win_id, app));
                        ui.close_menu();
                    }
                });
            });
        });

        // Render all generic floating windows
        for window in self.windows.iter_mut() {
            if let Some(act) = window.render(ctx, config) {
                actions.push(act);
            }
        }

        // Remove closed windows
        self.windows.retain(|w| w.is_open);

        // Process window actions (SSH connections, SFTP sessions, or opening files)
        for act in actions {
            match act {
                WindowAction::ConnectSsh(cmd) => {
                    let win_id = Uuid::new_v4().to_string();
                    let app = Box::new(TerminalApp::new_ssh("SSH Session", cmd, ctx));
                    self.windows.push(FloatingWindow::new(win_id, app));
                }
                WindowAction::OpenSftp(host) => {
                    let win_id = Uuid::new_v4().to_string();
                    let app = Box::new(SftpApp::with_host(host));
                    self.windows.push(FloatingWindow::new(win_id, app));
                }
                WindowAction::OpenFile(path) => {
                    if let Ok(editor) = EditorApp::open(&path) {
                        let win_id = Uuid::new_v4().to_string();
                        let app = Box::new(editor);
                        self.windows.push(FloatingWindow::new(win_id, app));
                    }
                }
            }
        }
    }
}
