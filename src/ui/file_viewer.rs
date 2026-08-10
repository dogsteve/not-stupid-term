use eframe::egui;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::ui::icons::Icons;
use crate::ui::sftp_app::SftpDragPayload;
use crate::ui::window_framework::{WindowAction, WindowApp};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LocalDragPayload {
    pub path: String,
}

pub struct FileViewerApp {
    pub root_path: PathBuf,
    pub path_history: Vec<PathBuf>,
    pub history_idx: usize,
    pub status: String,
}

impl FileViewerApp {
    pub fn new() -> Self {
        let root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self {
            root_path: root.clone(),
            path_history: vec![root],
            history_idx: 0,
            status: "Drag local files to SFTP window, or drop SFTP files here to download".to_string(),
        }
    }

    pub fn navigate_to(&mut self, new_path: PathBuf) {
        if self.root_path == new_path {
            return;
        }
        self.path_history.truncate(self.history_idx + 1);
        self.path_history.push(new_path.clone());
        self.history_idx = self.path_history.len() - 1;
        self.root_path = new_path;
    }
}

impl WindowApp for FileViewerApp {
    fn title(&self) -> String {
        format!("{} File Viewer", Icons::FOLDER)
    }

    fn window_type(&self) -> &'static str {
        "file_viewer"
    }

    fn render(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        _config: &mut crate::ui::settings::AppConfig,
    ) -> Option<WindowAction> {
        let mut action = None;

        // Check if SFTP file payload was dropped onto FileViewer -> Download from remote server!
        if ui.input(|i| i.pointer.any_released()) {
            if let Some(sftp_payload) = ctx.data(|d| d.get_temp::<SftpDragPayload>(egui::Id::new("dnd_sftp"))) {
                let dest = self.root_path.join(&sftp_payload.file_name);
                let dest_str = dest.to_string_lossy().to_string();

                let status = Command::new("scp")
                    .arg("-P")
                    .arg(sftp_payload.port.to_string())
                    .arg(format!("{}:{}", sftp_payload.host, sftp_payload.remote_path))
                    .arg(&dest_str)
                    .status();

                if let Ok(st) = status {
                    if st.success() {
                        self.status = format!("Downloaded {} from SFTP!", sftp_payload.file_name);
                    } else {
                        self.status = format!("Failed to download {}", sftp_payload.file_name);
                    }
                }
                ctx.data_mut(|d| d.remove_temp::<SftpDragPayload>(egui::Id::new("dnd_sftp")));
            }
        }

        // TOP NAVIGATION TOOLBAR WITH RICH GRAPHICAL ICONS
        ui.horizontal(|ui| {
            let can_back = self.history_idx > 0;
            if ui.add_enabled(can_back, egui::Button::new(format!("{} Back", Icons::BACK))).clicked() {
                self.history_idx -= 1;
                self.root_path = self.path_history[self.history_idx].clone();
            }

            let can_fwd = self.history_idx + 1 < self.path_history.len();
            if ui.add_enabled(can_fwd, egui::Button::new(format!("{} Fwd", Icons::FORWARD))).clicked() {
                self.history_idx += 1;
                self.root_path = self.path_history[self.history_idx].clone();
            }

            let has_parent = self.root_path.parent().is_some();
            if ui.add_enabled(has_parent, egui::Button::new(format!("{} Up", Icons::REFRESH))).clicked() {
                if let Some(parent) = self.root_path.parent().map(|p| p.to_path_buf()) {
                    self.navigate_to(parent);
                }
            }

            ui.add_space(4.0);
            ui.label(egui::RichText::new(Icons::FOLDER_OPEN).size(15.0));
            ui.label(egui::RichText::new(self.root_path.to_string_lossy()).weak().size(12.0));
        });

        ui.add_space(2.0);
        ui.label(egui::RichText::new(&self.status).size(11.0).weak());
        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);

        // UNLIMITED DEEP NESTED FILE TREE WITH PINNED SCROLLBARS
        let mut navigate_req = None;
        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .id_salt("file_viewer_scroll")
            .show(ui, |ui| {
                if let Some((nav, sel)) = render_dir_tree(ui, ctx, &self.root_path, 0) {
                    if let Some(n) = nav {
                        navigate_req = Some(n);
                    }
                    if let Some(s) = sel {
                        action = Some(WindowAction::OpenFile(s));
                    }
                }
            });

        if let Some(new_p) = navigate_req {
            self.navigate_to(new_p);
        }

        action
    }
}

fn render_dir_tree(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    dir_path: &Path,
    depth: usize,
) -> Option<(Option<PathBuf>, Option<String>)> {
    // Increased max depth to 30 levels for unlimited deep folder browsing!
    if depth > 30 {
        return None;
    }

    let mut nav_target = None;
    let mut selected_file = None;

    let entries = match fs::read_dir(dir_path) {
        Ok(read) => {
            let mut list: Vec<_> = read.filter_map(|e| e.ok()).collect();
            list.sort_by_key(|e| (!e.path().is_dir(), e.file_name()));
            list
        }
        Err(_) => return None,
    };

    for entry in entries {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if name.starts_with('.') || name == "target" || name == "node_modules" {
            continue;
        }

        if path.is_dir() {
            let header_text = format!("{} {}", Icons::FOLDER, name);
            let header = egui::CollapsingHeader::new(header_text).id_salt(&path);

            let res = header.show(ui, |ui| {
                render_dir_tree(ui, ctx, &path, depth + 1)
            });

            if let Some(Some((sub_nav, sub_sel))) = res.body_returned {
                if sub_nav.is_some() { nav_target = sub_nav; }
                if sub_sel.is_some() { selected_file = sub_sel; }
            }

            if res.header_response.double_clicked() {
                nav_target = Some(path);
            }
        } else {
            let icon = Icons::get_file_icon(&name);
            let label_text = format!("{} {}", icon, name);
            let path_str = path.to_string_lossy().to_string();

            let btn = egui::Button::new(egui::RichText::new(label_text).size(13.0))
                .fill(egui::Color32::TRANSPARENT)
                .frame(false);

            let response = ui.add(btn);

            if response.drag_started() {
                ctx.data_mut(|d| d.insert_temp(egui::Id::new("dnd_local"), LocalDragPayload { path: path_str.clone() }));
            }

            if response.clicked() {
                selected_file = Some(path_str);
            }
        }
    }

    Some((nav_target, selected_file))
}
