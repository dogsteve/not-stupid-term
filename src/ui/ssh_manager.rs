use eframe::egui;
use crate::ui::icons::Icons;
use crate::ui::window_framework::{WindowAction, WindowApp};

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct SshHost {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_type: String, // "Password" or "Key"
    pub secret: String,
}

impl Default for SshHost {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: "New Server".to_string(),
            host: "127.0.0.1".to_string(),
            port: 22,
            username: "root".to_string(),
            auth_type: "Password".to_string(),
            secret: "".to_string(),
        }
    }
}

pub struct SshManagerApp {
    pub hosts: Vec<SshHost>,
    pub selected_id: Option<String>,
    pub filter: String,
}

impl SshManagerApp {
    pub fn new() -> Self {
        let default_host = SshHost {
            id: "demo-1".to_string(),
            name: "Production VPS".to_string(),
            host: "192.168.1.100".to_string(),
            port: 22,
            username: "ubuntu".to_string(),
            auth_type: "Password".to_string(),
            secret: "".to_string(),
        };

        Self {
            hosts: vec![default_host],
            selected_id: Some("demo-1".to_string()),
            filter: String::new(),
        }
    }
}

impl WindowApp for SshManagerApp {
    fn title(&self) -> String {
        format!("{} SSH & SFTP Manager", Icons::SERVER)
    }

    fn window_type(&self) -> &'static str {
        "ssh_manager"
    }

    fn render(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        _config: &mut crate::ui::settings::AppConfig,
    ) -> Option<WindowAction> {
        let mut action = None;

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .id_salt("ssh_manager_scroll")
            .show(ui, |ui| {
                ui.add_space(6.0);

                ui.horizontal(|ui| {
                    // LEFT SIDEBAR - Host List
                    ui.vertical(|ui| {
                        ui.set_width(170.0);

                        ui.horizontal(|ui| {
                            ui.heading(egui::RichText::new("SSH Hosts").size(15.0).strong());
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button(format!("{} New", crate::ui::icons::Icons::ADD)).clicked() {
                                    let new_h = SshHost::default();
                                    let new_id = new_h.id.clone();
                                    self.hosts.push(new_h);
                                    self.selected_id = Some(new_id);
                                }
                            });
                        });

                        ui.add_space(6.0);
                        ui.add(egui::TextEdit::singleline(&mut self.filter).hint_text("Search..."));
                        ui.add_space(6.0);

                        egui::ScrollArea::vertical().id_salt("ssh_host_list").show(ui, |ui| {
                            let mut to_delete = None;

                            for h in &self.hosts {
                                if !self.filter.is_empty()
                                    && !h.name.to_lowercase().contains(&self.filter.to_lowercase())
                                    && !h.host.contains(&self.filter)
                                {
                                    continue;
                                }

                                let is_sel = self.selected_id.as_deref() == Some(&h.id);
                                let item_bg = if is_sel {
                                    ctx.style().visuals.selection.bg_fill
                                } else {
                                    ctx.style().visuals.faint_bg_color
                                };

                                let item_frame = egui::Frame::default()
                                    .fill(item_bg)
                                    .rounding(6.0)
                                    .inner_margin(8.0);

                                item_frame.show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.vertical(|ui| {
                                            ui.label(egui::RichText::new(&h.name).strong().size(12.0));
                                            ui.label(
                                                egui::RichText::new(format!("{}@{}", h.username, h.host))
                                                    .size(10.0)
                                                    .color(egui::Color32::GRAY),
                                            );
                                        });

                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            if ui.small_button(crate::ui::icons::Icons::CLOSE).clicked() {
                                                to_delete = Some(h.id.clone());
                                            }
                                        });
                                    });
                                });

                                let rect = ui.min_rect();
                                if ui.interact(rect, ui.id().with(&h.id), egui::Sense::click()).clicked() {
                                    self.selected_id = Some(h.id.clone());
                                }

                                ui.add_space(4.0);
                            }

                            if let Some(del_id) = to_delete {
                                self.hosts.retain(|h| h.id != del_id);
                                if self.selected_id.as_deref() == Some(&del_id) {
                                    self.selected_id = self.hosts.first().map(|h| h.id.clone());
                                }
                            }
                        });
                    });

                    ui.separator();
                    ui.add_space(8.0);

                    // RIGHT PANEL - Host Configuration & Shared SSH/SFTP Launch Buttons
                    ui.vertical(|ui| {
                        if let Some(sel_id) = self.selected_id.clone() {
                            if let Some(host) = self.hosts.iter_mut().find(|h| h.id == sel_id) {
                                ui.heading(egui::RichText::new(&host.name).size(16.0).strong());
                                ui.add_space(10.0);

                                let card = egui::Frame::default()
                                    .fill(ctx.style().visuals.faint_bg_color)
                                    .rounding(8.0)
                                    .inner_margin(12.0);

                                card.show(ui, |ui| {
                                    egui::Grid::new("ssh_form_grid")
                                        .num_columns(2)
                                        .spacing([16.0, 12.0])
                                        .show(ui, |ui| {
                                            ui.label("Display Name:");
                                            ui.text_edit_singleline(&mut host.name);
                                            ui.end_row();

                                            ui.label("Host Address:");
                                            ui.text_edit_singleline(&mut host.host);
                                            ui.end_row();

                                            ui.label("Port:");
                                            let mut port_str = host.port.to_string();
                                            if ui.text_edit_singleline(&mut port_str).changed() {
                                                if let Ok(p) = port_str.parse() {
                                                    host.port = p;
                                                }
                                            }
                                            ui.end_row();

                                            ui.label("Username:");
                                            ui.text_edit_singleline(&mut host.username);
                                            ui.end_row();

                                            ui.label("Auth Type:");
                                            egui::ComboBox::from_id_salt("ssh_auth_type")
                                                .selected_text(&host.auth_type)
                                                .show_ui(ui, |ui| {
                                                    ui.selectable_value(&mut host.auth_type, "Password".to_string(), "Password");
                                                    ui.selectable_value(&mut host.auth_type, "SSH Key".to_string(), "SSH Key Path");
                                                });
                                            ui.end_row();

                                            ui.label(if host.auth_type == "Password" { "Password:" } else { "Key Path:" });
                                            ui.add(egui::TextEdit::singleline(&mut host.secret).password(host.auth_type == "Password"));
                                            ui.end_row();
                                        });
                                });

                                ui.add_space(14.0);

                                ui.horizontal(|ui| {
                                    let btn_ssh = egui::Button::new(
                                        egui::RichText::new("⚡ Connect SSH")
                                            .size(13.0)
                                            .color(egui::Color32::WHITE)
                                            .strong(),
                                    )
                                    .fill(egui::Color32::from_rgb(40, 140, 240))
                                    .min_size([130.0, 36.0].into())
                                    .rounding(6.0);

                                    if ui.add(btn_ssh).clicked() {
                                        let cmd = format!("ssh {}@{} -p {}", host.username, host.host, host.port);
                                        action = Some(WindowAction::ConnectSsh(cmd));
                                    }

                                    ui.add_space(8.0);

                                    let btn_sftp = egui::Button::new(
                                        egui::RichText::new("📁 Connect SFTP")
                                            .size(13.0)
                                            .color(egui::Color32::WHITE)
                                            .strong(),
                                    )
                                    .fill(egui::Color32::from_rgb(34, 160, 100))
                                    .min_size([130.0, 36.0].into())
                                    .rounding(6.0);

                                    if ui.add(btn_sftp).clicked() {
                                        let host_str = format!("{}@{}", host.username, host.host);
                                        action = Some(WindowAction::OpenSftp(host_str));
                                    }
                                });
                            }
                        } else {
                            ui.centered_and_justified(|ui| {
                                ui.label("Select or create an SSH host.");
                            });
                        }
                    });
                });
            });

        action
    }
}
