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

                let accent = ui.style().visuals.selection.bg_fill;

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

                        // Dropdown spawn menu
                        ui.menu_button(
                            egui::RichText::new(format!("{} New", Icons::ADD)).size(12.0),
                            |ui| {
                                ui.set_min_width(200.0);

                                let items = [
                                    (Icons::TERMINAL, "Local Terminal"),
                                    (Icons::FOLDER, "File Viewer"),
                                    (Icons::SERVER, "SSH & SFTP Manager"),
                                    (Icons::SERVER, "SFTP Remote Browser"),
                                    (Icons::GEAR, "Settings"),
                                ];

                                for (icon, label) in items {
                                    if ui.add(
                                        egui::Button::new(
                                            egui::RichText::new(format!("{} {}", icon, label)).size(12.0),
                                        )
                                        .min_size(egui::vec2(ui.available_width(), 30.0)),
                                    ).clicked() {
                                        let win_id = Uuid::new_v4().to_string();
                                        match label {
                                            "Local Terminal" => {
                                                let count = self.windows.len();
                                                let title = if count == 0 { "zsh".to_string() } else { format!("zsh ({})", count) };
                                                self.windows.push(FloatingWindow::new(win_id, Box::new(TerminalApp::new_local(title, ctx))));
                                            }
                                            "File Viewer" => {
                                                self.windows.push(FloatingWindow::new(win_id, Box::new(FileViewerApp::new())));
                                            }
                                            "SSH & SFTP Manager" => {
                                                self.windows.push(FloatingWindow::new(win_id, Box::new(SshManagerApp::new())));
                                            }
                                            "SFTP Remote Browser" => {
                                                self.windows.push(FloatingWindow::new(win_id, Box::new(SftpApp::new())));
                                            }
                                            "Settings" => {
                                                self.windows.push(FloatingWindow::new(win_id, Box::new(SettingsApp)));
                                            }
                                            _ => {}
                                        }
                                        ui.close_menu();
                                    }
                                }
                            },
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

        // Remove closed windows
        self.windows.retain(|w| w.is_open);

        // Process window actions
        for act in actions {
            match act {
                WindowAction::ConnectSsh(cmd) => {
                    let win_id = Uuid::new_v4().to_string();
                    self.windows.push(FloatingWindow::new(win_id, Box::new(TerminalApp::new_ssh("SSH Session", cmd, ctx))));
                }
                WindowAction::OpenSftp(host) => {
                    let win_id = Uuid::new_v4().to_string();
                    self.windows.push(FloatingWindow::new(win_id, Box::new(SftpApp::with_host(host))));
                }
                WindowAction::OpenFile(path) => {
                    if let Ok(editor) = EditorApp::open(&path) {
                        let win_id = Uuid::new_v4().to_string();
                        self.windows.push(FloatingWindow::new(win_id, Box::new(editor)));
                    }
                }
            }
        }
    }
}
