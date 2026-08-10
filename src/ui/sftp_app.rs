use eframe::egui;
use std::process::Command;

use crate::ui::file_viewer::LocalDragPayload;
use crate::ui::icons::Icons;
use crate::ui::window_framework::{WindowAction, WindowApp};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SftpFileItem {
    pub name: String,
    pub is_dir: bool,
    pub size: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SftpDragPayload {
    pub host: String,
    pub port: u16,
    pub remote_path: String,
    pub file_name: String,
}

pub struct SftpApp {
    pub host: String,
    pub remote_dir: String,
    pub port: u16,
    pub auth_type: String,
    pub secret: String,
    pub items: Vec<SftpFileItem>,
    pub status: String,
    pub is_loading: bool,
    pub show_wizard: bool,
}

impl SftpApp {
    pub fn new() -> Self {
        Self {
            host: "root@127.0.0.1".to_string(),
            remote_dir: "~".to_string(),
            port: 22,
            auth_type: "Password".to_string(),
            secret: String::new(),
            items: Vec::new(),
            status: "Select or enter host and click Connect".to_string(),
            is_loading: false,
            show_wizard: true,
        }
    }

    pub fn with_host(host: impl Into<String>) -> Self {
        let mut app = Self::new();
        app.host = host.into();
        app.refresh_remote_dir();
        app
    }

    pub fn refresh_remote_dir(&mut self) {
        self.is_loading = true;
        self.items.clear();

        let cmd_str = format!("ls -la {}", self.remote_dir);
        let output = Command::new("ssh")
            .arg("-o")
            .arg("ConnectTimeout=5")
            .arg("-p")
            .arg(self.port.to_string())
            .arg(&self.host)
            .arg(cmd_str)
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                for line in stdout.lines() {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 9 {
                        let is_dir = parts[0].starts_with('d');
                        let size = parts[4].to_string();
                        let name = parts[8..].join(" ");
                        if name != "." && name != ".." {
                            self.items.push(SftpFileItem { name, is_dir, size });
                        }
                    }
                }
                self.status = format!("Loaded {} remote items", self.items.len());
                self.show_wizard = false;
            }
            Ok(out) => {
                self.status = format!("Error: {}", String::from_utf8_lossy(&out.stderr));
            }
            Err(e) => {
                self.status = format!("Connection failed: {}", e);
            }
        }
        self.is_loading = false;
    }

    pub fn download_and_edit_file(&mut self, remote_file: &str) -> Option<WindowAction> {
        let remote_path = format!("{}/{}", self.remote_dir, remote_file);
        let tmp_path = format!("/tmp/sftp_{}", remote_file);

        let status = Command::new("scp")
            .arg("-P")
            .arg(self.port.to_string())
            .arg(format!("{}:{}", self.host, remote_path))
            .arg(&tmp_path)
            .status();

        if let Ok(st) = status {
            if st.success() {
                return Some(WindowAction::OpenFile(tmp_path));
            }
        }
        self.status = format!("Failed to download {}", remote_file);
        None
    }

    pub fn upload_local_file(&mut self, local_path: &str) {
        let status = Command::new("scp")
            .arg("-P")
            .arg(self.port.to_string())
            .arg(local_path)
            .arg(format!("{}:{}", self.host, self.remote_dir))
            .status();

        if let Ok(st) = status {
            if st.success() {
                self.status = format!("Uploaded {} successfully!", local_path);
                self.refresh_remote_dir();
                return;
            }
        }
        self.status = format!("Failed to upload {}", local_path);
    }
}

impl WindowApp for SftpApp {
    fn title(&self) -> String {
        format!("{} SFTP Remote Browser", Icons::SERVER)
    }

    fn render(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        _config: &mut crate::ui::settings::AppConfig,
    ) -> Option<WindowAction> {
        let mut action = None;

        // Check if Local File was dropped onto SFTP window -> Upload to remote server!
        if ui.input(|i| i.pointer.any_released()) {
            if let Some(local_payload) = ctx.data(|d| d.get_temp::<LocalDragPayload>(egui::Id::new("dnd_local"))) {
                self.upload_local_file(&local_payload.path);
                ctx.data_mut(|d| d.remove_temp::<LocalDragPayload>(egui::Id::new("dnd_local")));
            }
        }

        // SFTP CONNECTION WIZARD STEP
        if self.show_wizard {
            ui.heading(egui::RichText::new("SFTP Connection Wizard").size(16.0).strong());
            ui.add_space(8.0);

            let card = egui::Frame::default()
                .fill(ui.style().visuals.faint_bg_color)
                .rounding(8.0)
                .inner_margin(12.0);

            card.show(ui, |ui| {
                egui::Grid::new("sftp_wizard_grid")
                    .num_columns(2)
                    .spacing([16.0, 10.0])
                    .show(ui, |ui| {
                        ui.label("Remote Host (user@ip):");
                        ui.text_edit_singleline(&mut self.host);
                        ui.end_row();

                        ui.label("Port:");
                        let mut port_str = self.port.to_string();
                        if ui.text_edit_singleline(&mut port_str).changed() {
                            if let Ok(p) = port_str.parse() {
                                self.port = p;
                            }
                        }
                        ui.end_row();

                        ui.label("Initial Directory:");
                        ui.text_edit_singleline(&mut self.remote_dir);
                        ui.end_row();

                        ui.label("Auth Method:");
                        egui::ComboBox::from_id_salt("sftp_auth_type")
                            .selected_text(&self.auth_type)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.auth_type, "Password".to_string(), "Password");
                                ui.selectable_value(&mut self.auth_type, "SSH Key".to_string(), "SSH Key Path");
                            });
                        ui.end_row();

                        ui.label(if self.auth_type == "Password" { "Password:" } else { "Key Path:" });
                        ui.add(egui::TextEdit::singleline(&mut self.secret).password(self.auth_type == "Password"));
                        ui.end_row();
                    });
            });

            ui.add_space(12.0);

            ui.horizontal(|ui| {
                let btn = egui::Button::new(
                    egui::RichText::new("Connect & Browse SFTP")
                        .size(13.0)
                        .color(egui::Color32::WHITE)
                        .strong(),
                )
                .fill(egui::Color32::from_rgb(34, 160, 100))
                .min_size([180.0, 36.0].into())
                .rounding(6.0);

                if ui.add(btn).clicked() {
                    self.refresh_remote_dir();
                }
            });

            ui.add_space(8.0);
            ui.label(egui::RichText::new(&self.status).size(11.0).weak());
            return None;
        }

        // ACTIVE SFTP TOP BAR
        ui.horizontal(|ui| {
            ui.label(Icons::SERVER);
            ui.label(egui::RichText::new(&self.host).strong());
            ui.label("Path:");
            ui.add(egui::TextEdit::singleline(&mut self.remote_dir).desired_width(140.0));

            if ui.button(format!("{} Refresh", Icons::REFRESH)).clicked() {
                self.refresh_remote_dir();
            }

            if ui.button(format!("{} Connection Wizard", Icons::GEAR)).clicked() {
                self.show_wizard = true;
            }
        });

        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);

        // DRAG AND DROP FILE UPLOAD AREA
        let drop_frame = egui::Frame::default()
            .fill(ui.style().visuals.faint_bg_color)
            .rounding(6.0)
            .stroke(egui::Stroke::new(1.0, ui.style().visuals.widgets.noninteractive.bg_stroke.color))
            .inner_margin(8.0);

        drop_frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(format!("{} Drag & Drop files between FileViewer and SFTP 2-way", Icons::ADD)).size(12.0).weak());
            });
        });

        // Handle dropped files from OS Finder
        ui.input(|i| {
            for dropped in &i.raw.dropped_files {
                if let Some(path) = &dropped.path {
                    let p_str = path.to_string_lossy().to_string();
                    self.upload_local_file(&p_str);
                }
            }
        });

        ui.add_space(4.0);
        ui.label(egui::RichText::new(&self.status).size(11.0).weak());
        ui.add_space(4.0);

        // REMOTE FILE LIST BROWSER WITH PINNED SCROLLBARS TO RIGHT & BOTTOM EDGES
        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .id_salt("sftp_scroll")
            .show(ui, |ui| {
                let items = self.items.clone();

                for item in items {
                    ui.horizontal(|ui| {
                        if item.is_dir {
                            ui.label(Icons::FOLDER);
                            let btn = egui::Button::new(egui::RichText::new(&item.name).strong())
                                .fill(egui::Color32::TRANSPARENT)
                                .frame(false);
                            if ui.add(btn).clicked() {
                                self.remote_dir = format!("{}/{}", self.remote_dir, item.name);
                                self.refresh_remote_dir();
                            }
                        } else {
                            let icon = Icons::get_file_icon(&item.name);
                            ui.label(icon);

                            let btn = egui::Button::new(egui::RichText::new(&item.name))
                                .fill(egui::Color32::TRANSPARENT)
                                .frame(false);

                            let response = ui.add(btn);

                            if response.drag_started() {
                                let remote_full_path = format!("{}/{}", self.remote_dir, item.name);
                                ctx.data_mut(|d| {
                                    d.insert_temp(
                                        egui::Id::new("dnd_sftp"),
                                        SftpDragPayload {
                                            host: self.host.clone(),
                                            port: self.port,
                                            remote_path: remote_full_path,
                                            file_name: item.name.clone(),
                                        },
                                    );
                                });
                            }

                            if response.clicked() {
                                if let Some(act) = self.download_and_edit_file(&item.name) {
                                    action = Some(act);
                                }
                            }

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(egui::RichText::new(&item.size).weak().size(11.0));
                            });
                        }
                    });
                }
            });

        action
    }
}
