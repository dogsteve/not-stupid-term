use eframe::egui;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{channel, Receiver};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::ui::icons::Icons;
use crate::ui::sftp_app::SftpDragPayload;
use crate::ui::window_framework::{WindowAction, WindowApp};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LocalDragPayload {
    pub path: String,
}

struct DirCacheEntry {
    last_updated: Instant,
    entries: Vec<(PathBuf, String, bool)>,
}

static DIR_CACHE: Mutex<Option<HashMap<PathBuf, DirCacheEntry>>> = Mutex::new(None);

pub struct FileViewerApp {
    pub root_path: PathBuf,
    pub path_history: Vec<PathBuf>,
    pub history_idx: usize,
    pub status: String,
    download_rx: Option<Receiver<String>>,
    /// Whether the path bar is in edit mode.
    is_editing_path: bool,
    /// Buffer for the editable path string.
    path_edit_text: String,
}

impl FileViewerApp {
    pub fn new() -> Self {
        let root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self {
            root_path: root.clone(),
            path_history: vec![root],
            history_idx: 0,
            status: "Drag local files to SFTP window, or drop SFTP files here to download"
                .to_string(),
            download_rx: None,
            is_editing_path: false,
            path_edit_text: String::new(),
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

    /// Try to navigate to `path_edit_text`. If it's a directory, navigate into it.
    /// If it's a file, open it. Returns Some(WindowAction::OpenFile) for files.
    fn commit_path_edit(&mut self) -> Option<WindowAction> {
        self.is_editing_path = false;
        let trimmed = self.path_edit_text.trim().to_string();
        if trimmed.is_empty() {
            return None;
        }
        let candidate = PathBuf::from(&trimmed);
        if candidate.is_dir() {
            self.navigate_to(candidate);
            None
        } else if candidate.is_file() {
            Some(WindowAction::OpenFile(trimmed))
        } else {
            self.status = format!("Path not found: {}", trimmed);
            None
        }
    }
}

impl WindowApp for FileViewerApp {
    fn title(&self) -> String {
        format!("{} File Viewer", Icons::FOLDER)
    }

    fn window_type(&self) -> &'static str {
        "file_viewer"
    }

    fn save_state(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "root_path": self.root_path.to_string_lossy(),
        }))
    }

    fn render(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        _config: &mut crate::ui::settings::AppConfig,
    ) -> Option<WindowAction> {
        let mut action = None;

        // Poll for completed background downloads
        if let Some(ref rx) = self.download_rx {
            if let Ok(msg) = rx.try_recv() {
                self.status = msg;
                self.download_rx = None;
            }
        }

        // Check if SFTP file payload was dropped — download asynchronously
        if ui.input(|i| i.pointer.any_released()) {
            if let Some(sftp_payload) =
                ctx.data(|d| d.get_temp::<SftpDragPayload>(egui::Id::new("dnd_sftp")))
            {
                let dest = self.root_path.join(&sftp_payload.file_name);
                let dest_str = dest.to_string_lossy().to_string();
                let file_name = sftp_payload.file_name.clone();

                self.status = format!("Downloading {} from SFTP in background...", file_name);

                let (tx, rx) = channel();
                self.download_rx = Some(rx);

                let ctx_clone = ctx.clone();
                std::thread::spawn(move || {
                    let status = Command::new("scp")
                        .arg("-o").arg("BatchMode=yes")
                        .arg("-o").arg("ConnectTimeout=10")
                        .arg("-P").arg(sftp_payload.port.to_string())
                        .arg("--")
                        .arg(format!("{}:{}", sftp_payload.host, sftp_payload.remote_path))
                        .arg(&dest_str)
                        .status();

                    if let Ok(st) = status {
                        if st.success() {
                            let _ = tx.send(format!("Downloaded {} from SFTP!", file_name));
                        } else {
                            let _ = tx.send(format!("Failed to download {}", file_name));
                        }
                    } else {
                        let _ = tx.send(format!("SCP command failed for {}", file_name));
                    }
                    ctx_clone.request_repaint();
                });

                ctx.data_mut(|d| d.remove_temp::<SftpDragPayload>(egui::Id::new("dnd_sftp")));
            }
        }

        // ── TOP NAVIGATION TOOLBAR ──────────────────────────────────────────
        ui.horizontal(|ui| {
            let is_dark = ui.visuals().dark_mode;
            let icon_color = ui.visuals().text_color();

            // Back
            let can_back = self.history_idx > 0;
            let back_job = Icons::label_job(Icons::BACK, "", 13.0, icon_color);
            if ui
                .add_enabled(
                    can_back,
                    egui::Button::new(back_job)
                        .min_size(egui::vec2(28.0, 24.0))
                        .rounding(6.0),
                )
                .on_hover_text("Back")
                .clicked()
            {
                self.history_idx -= 1;
                self.root_path = self.path_history[self.history_idx].clone();
            }

            // Forward
            let can_fwd = self.history_idx + 1 < self.path_history.len();
            let fwd_job = Icons::label_job(Icons::FORWARD, "", 13.0, icon_color);
            if ui
                .add_enabled(
                    can_fwd,
                    egui::Button::new(fwd_job)
                        .min_size(egui::vec2(28.0, 24.0))
                        .rounding(6.0),
                )
                .on_hover_text("Forward")
                .clicked()
            {
                self.history_idx += 1;
                self.root_path = self.path_history[self.history_idx].clone();
            }

            // Up (parent directory) — use CARET_UP which is the correct icon
            let has_parent = self.root_path.parent().is_some();
            let up_job = Icons::label_job(Icons::CARET_UP, "", 13.0, icon_color);
            if ui
                .add_enabled(
                    has_parent,
                    egui::Button::new(up_job)
                        .min_size(egui::vec2(28.0, 24.0))
                        .rounding(6.0),
                )
                .on_hover_text("Parent directory")
                .clicked()
            {
                if let Some(parent) = self.root_path.parent().map(|p| p.to_path_buf()) {
                    self.navigate_to(parent);
                }
            }

            ui.add_space(4.0);

            // ── EDITABLE PATH BAR ──────────────────────────────────────────
            // Shows the current path as a clickable label. Clicking enters edit mode.
            // Pressing Enter navigates to the typed path (dir → navigate, file → open).
            // Pressing Escape cancels editing.
            let path_str = self.root_path.to_string_lossy().to_string();
            let path_bar_fill = if is_dark {
                egui::Color32::from_white_alpha(10)
            } else {
                egui::Color32::from_black_alpha(8)
            };
            let path_bar_stroke = if is_dark {
                egui::Stroke::new(1.0, egui::Color32::from_white_alpha(25))
            } else {
                egui::Stroke::new(1.0, egui::Color32::from_black_alpha(20))
            };

            let path_edit_id = ui.id().with("path_edit_input");

            egui::Frame::default()
                .fill(path_bar_fill)
                .stroke(path_bar_stroke)
                .rounding(6.0)
                .inner_margin(egui::Margin::symmetric(8.0, 3.0))
                .show(ui, |ui| {
                    let avail_w = ui.available_width().max(120.0);
                    ui.set_max_width(avail_w);

                    if self.is_editing_path {
                        // ── EDIT MODE ──────────────────────────────────────
                        let resp = ui.add(
                            egui::TextEdit::singleline(&mut self.path_edit_text)
                                .id(path_edit_id)
                                .desired_width(avail_w - 4.0)
                                .font(egui::FontId::proportional(12.0))
                                .frame(false),
                        );

                        // Auto-focus the input on the first frame it appears
                        if !ui.memory(|m| m.has_focus(path_edit_id)) {
                            ui.memory_mut(|m| m.request_focus(path_edit_id));
                        }

                        let pressed_enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                        let pressed_esc = ui.input(|i| i.key_pressed(egui::Key::Escape));
                        let lost_focus = resp.lost_focus();

                        if pressed_enter {
                            if let Some(act) = self.commit_path_edit() {
                                action = Some(act);
                            }
                        } else if pressed_esc || (lost_focus && !pressed_enter) {
                            self.is_editing_path = false;
                        }
                    } else {
                        // ── DISPLAY MODE ───────────────────────────────────
                        // Show folder icon + path. Click → enter edit mode.
                        ui.horizontal(|ui| {
                            ui.add(egui::Label::new(
                                Icons::rich(Icons::FOLDER_OPEN, 13.0)
                                    .color(egui::Color32::from_rgb(230, 180, 60)),
                            ));
                            let label_resp = ui.add(
                                egui::Label::new(
                                    egui::RichText::new(&path_str)
                                        .size(12.0)
                                        .color(if is_dark {
                                            egui::Color32::from_gray(170)
                                        } else {
                                            egui::Color32::from_gray(60)
                                        }),
                                )
                                .sense(egui::Sense::click())
                                .truncate(),
                            );
                            if label_resp.clicked() {
                                self.path_edit_text = path_str;
                                self.is_editing_path = true;
                            }
                            label_resp.on_hover_text("Click to edit path");
                        });
                    }
                });
        });

        ui.add_space(2.0);
        ui.label(egui::RichText::new(&self.status).size(11.0).weak());
        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);

        // ── FILE TREE WITH EDGE-SCROLL WHEN DRAGGING ──────────────────────
        let mut navigate_req = None;

        // Correct edge-scroll pattern for egui ScrollArea:
        //   1. Read the "desired offset" we stored last frame.
        //   2. Start ScrollArea with that offset (scroll_offset()).
        //   3. After .show(), read output.state.offset — the actual current position.
        //   4. Add the edge-scroll delta to get next frame's desired offset.
        //   5. Store for next frame.
        // This gives a 1-frame latency (imperceptible) without fighting egui's internal state.
        let scroll_id = egui::Id::new("file_viewer_scroll_offset");
        let desired_offset = ctx
            .data(|d| d.get_temp::<egui::Vec2>(scroll_id))
            .unwrap_or_default();

        let output = egui::ScrollArea::both()
            .auto_shrink([false, false])
            .id_salt("file_viewer_scroll")
            .scroll_offset(desired_offset)
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

        // Compute edge-scroll delta based on the ACTUAL visible rect from this frame
        let edge_delta = compute_edge_scroll_delta(ctx, output.inner_rect);
        let next_offset = output.state.offset + edge_delta;
        ctx.data_mut(|d| d.insert_temp(scroll_id, next_offset));
        if edge_delta != egui::Vec2::ZERO {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }

        if let Some(new_p) = navigate_req {
            self.navigate_to(new_p);
        }

        action
    }
}

/// Returns a scroll delta Vec2 when the pointer is dragging near the edges of `rect`.
/// Returns ZERO when not dragging or not near any edge.
fn compute_edge_scroll_delta(ctx: &egui::Context, rect: egui::Rect) -> egui::Vec2 {
    let is_dragging = ctx.input(|i| i.pointer.primary_down());
    if !is_dragging {
        return egui::Vec2::ZERO;
    }
    let ptr = match ctx.pointer_latest_pos() {
        Some(p) => p,
        None => return egui::Vec2::ZERO,
    };
    if !rect.expand(8.0).contains(ptr) {
        return egui::Vec2::ZERO;
    }

    const ZONE: f32 = 40.0;  // pixels from edge that trigger scrolling
    const MAX_SPEED: f32 = 12.0; // pixels per frame at the very edge

    let mut delta = egui::Vec2::ZERO;

    // Horizontal
    let dist_left  = ptr.x - rect.min.x;
    let dist_right = rect.max.x - ptr.x;
    if dist_left < ZONE && dist_left >= 0.0 {
        delta.x = -MAX_SPEED * (1.0 - dist_left / ZONE);
    } else if dist_right < ZONE && dist_right >= 0.0 {
        delta.x =  MAX_SPEED * (1.0 - dist_right / ZONE);
    }

    // Vertical
    let dist_top    = ptr.y - rect.min.y;
    let dist_bottom = rect.max.y - ptr.y;
    if dist_top < ZONE && dist_top >= 0.0 {
        delta.y = -MAX_SPEED * (1.0 - dist_top / ZONE);
    } else if dist_bottom < ZONE && dist_bottom >= 0.0 {
        delta.y =  MAX_SPEED * (1.0 - dist_bottom / ZONE);
    }

    delta
}

fn get_cached_dir_entries(dir_path: &Path) -> Vec<(PathBuf, String, bool)> {
    if let Ok(guard) = DIR_CACHE.lock() {
        if let Some(ref cache) = *guard {
            if let Some(entry) = cache.get(dir_path) {
                if entry.last_updated.elapsed() < Duration::from_secs(2) {
                    return entry.entries.clone();
                }
            }
        }
    }

    let mut list = Vec::new();
    if let Ok(read) = fs::read_dir(dir_path) {
        let mut entries: Vec<_> = read.filter_map(|e| e.ok()).collect();
        entries.sort_by_key(|e| (!e.path().is_dir(), e.file_name()));
        for entry in entries {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let is_dir = path.is_dir();
            list.push((path, name, is_dir));
        }
    }

    if let Ok(mut guard) = DIR_CACHE.lock() {
        let map = guard.get_or_insert_with(HashMap::new);
        // Evict stale entries to prevent unbounded memory growth
        map.retain(|_, entry| entry.last_updated.elapsed() < Duration::from_secs(10));
        map.insert(
            dir_path.to_path_buf(),
            DirCacheEntry {
                last_updated: Instant::now(),
                entries: list.clone(),
            },
        );
    }

    list
}

fn render_dir_tree(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    dir_path: &Path,
    depth: usize,
) -> Option<(Option<PathBuf>, Option<String>)> {
    if depth > 30 {
        return None;
    }

    let mut nav_target = None;
    let mut selected_file = None;

    let entries = get_cached_dir_entries(dir_path);

    for (path, name, is_dir) in entries {
        if name.starts_with('.') || name == "target" || name == "node_modules" {
            continue;
        }

        if is_dir {
            let job = Icons::label_job(Icons::FOLDER, &name, 12.0, ui.visuals().text_color());
            let header = egui::CollapsingHeader::new(job).id_salt(&path);

            let res = header.show(ui, |ui| render_dir_tree(ui, ctx, &path, depth + 1));

            if let Some(Some((sub_nav, sub_sel))) = res.body_returned {
                if sub_nav.is_some() {
                    nav_target = sub_nav;
                }
                if sub_sel.is_some() {
                    selected_file = sub_sel;
                }
            }

            if res.header_response.double_clicked() {
                nav_target = Some(path);
            }
        } else {
            let icon = Icons::get_file_icon(&name);
            let job = Icons::label_job(icon, &name, 12.0, ui.visuals().text_color());
            let path_str = path.to_string_lossy().to_string();

            let btn = egui::Button::new(job)
                .fill(egui::Color32::TRANSPARENT)
                .frame(false);

            let response = ui.add(btn);

            if response.drag_started() {
                ctx.data_mut(|d| {
                    d.insert_temp(
                        egui::Id::new("dnd_local"),
                        LocalDragPayload { path: path_str.clone() },
                    )
                });
            }

            if response.clicked() {
                selected_file = Some(path_str);
            }
        }
    }

    Some((nav_target, selected_file))
}
