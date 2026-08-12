use eframe::egui;
use std::process::Command;
use std::path::Path;

use super::icons::Icons;
use super::window_framework::{WindowAction, WindowApp};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileGitStatus {
    Modified,
    Added,
    Deleted,
    Untracked,
    Renamed,
    Conflict,
}

impl FileGitStatus {
    pub fn label(&self) -> &'static str {
        match self {
            FileGitStatus::Modified => "MODIFIED",
            FileGitStatus::Added => "ADDED",
            FileGitStatus::Deleted => "DELETED",
            FileGitStatus::Untracked => "UNTRACKED",
            FileGitStatus::Renamed => "RENAMED",
            FileGitStatus::Conflict => "CONFLICT",
        }
    }

    pub fn color(&self) -> egui::Color32 {
        match self {
            FileGitStatus::Modified => egui::Color32::from_rgb(100, 180, 255),
            FileGitStatus::Added => egui::Color32::from_rgb(100, 220, 140),
            FileGitStatus::Deleted => egui::Color32::from_rgb(240, 100, 100),
            FileGitStatus::Untracked => egui::Color32::from_rgb(180, 180, 180),
            FileGitStatus::Renamed => egui::Color32::from_rgb(220, 180, 80),
            FileGitStatus::Conflict => egui::Color32::from_rgb(255, 140, 40),
        }
    }
}

#[derive(Debug, Clone)]
pub struct GitFileItem {
    pub path: String,
    pub status: FileGitStatus,
    pub staged: bool,
    pub is_selected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffKind {
    Equal,
    Added,
    Deleted,
    Modified,
    Conflict,
}

#[derive(Debug, Clone)]
pub struct DiffRow {
    pub left_line_num: Option<usize>,
    pub left_text: String,
    pub right_line_num: Option<usize>,
    pub right_text: String,
    pub kind: DiffKind,
    pub hunk_idx: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct DiffHunk {
    pub hunk_idx: usize,
    pub start_row: usize,
    pub end_row: usize,
    pub kind: DiffKind,
    pub left_line_start: usize,
    pub left_line_count: usize,
    pub right_line_start: usize,
    pub right_line_count: usize,
}

pub struct GitApp {
    pub repo_path: String,
    pub current_branch: String,
    pub branches: Vec<String>,
    pub files: Vec<GitFileItem>,
    pub selected_file_path: Option<String>,
    pub commit_message: String,
    pub is_unified_view: bool,
    pub diff_rows: Vec<DiffRow>,
    pub diff_hunks: Vec<DiffHunk>,
    pub left_file_content: String,
    pub right_file_content: String,
    pub status_message: String,
    pub new_branch_name: String,
    pub is_creating_branch: bool,
}

impl GitApp {
    pub fn new() -> Self {
        let repo_path = std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".to_string());

        let mut app = Self {
            repo_path,
            current_branch: "main".to_string(),
            branches: Vec::new(),
            files: Vec::new(),
            selected_file_path: None,
            commit_message: String::new(),
            is_unified_view: false,
            diff_rows: Vec::new(),
            diff_hunks: Vec::new(),
            left_file_content: String::new(),
            right_file_content: String::new(),
            status_message: "Ready".to_string(),
            new_branch_name: String::new(),
            is_creating_branch: false,
        };

        app.refresh_all();
        app
    }

    pub fn refresh_all(&mut self) {
        self.refresh_branches();
        self.refresh_status();
        if let Some(path) = self.selected_file_path.clone() {
            self.load_diff_for_file(&path);
        } else if let Some(first) = self.files.first() {
            let path = first.path.clone();
            self.selected_file_path = Some(path.clone());
            self.load_diff_for_file(&path);
        } else {
            self.diff_rows.clear();
            self.diff_hunks.clear();
        }
    }

    pub fn refresh_branches(&mut self) {
        if let Ok(out) = Command::new("git")
            .args(["branch", "-a"])
            .current_dir(&self.repo_path)
            .output()
        {
            if out.status.success() {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let mut branches = Vec::new();
                for line in stdout.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with('*') {
                        let name = trimmed.trim_start_matches('*').trim().to_string();
                        self.current_branch = name.clone();
                        branches.push(name);
                    } else if !trimmed.is_empty() && !trimmed.contains("->") {
                        branches.push(trimmed.to_string());
                    }
                }
                branches.dedup();
                self.branches = branches;
            }
        }
    }

    pub fn refresh_status(&mut self) {
        let mut items = Vec::new();

        if let Ok(out) = Command::new("git")
            .args(["status", "--porcelain=v1"])
            .current_dir(&self.repo_path)
            .output()
        {
            if out.status.success() {
                let stdout = String::from_utf8_lossy(&out.stdout);
                for line in stdout.lines() {
                    if line.len() < 3 {
                        continue;
                    }
                    let index_status = line.as_bytes()[0] as char;
                    let work_status = line.as_bytes()[1] as char;
                    let path = line[3..].trim().trim_matches('"').to_string();

                    // Staged item
                    if index_status != ' ' && index_status != '?' {
                        let st = match index_status {
                            'A' => FileGitStatus::Added,
                            'M' => FileGitStatus::Modified,
                            'D' => FileGitStatus::Deleted,
                            'R' => FileGitStatus::Renamed,
                            'U' => FileGitStatus::Conflict,
                            _ => FileGitStatus::Modified,
                        };
                        items.push(GitFileItem {
                            path: path.clone(),
                            status: st,
                            staged: true,
                            is_selected: self.selected_file_path.as_deref() == Some(&path),
                        });
                    }

                    // Unstaged item
                    if work_status != ' ' {
                        let st = match work_status {
                            '?' => FileGitStatus::Untracked,
                            'M' => FileGitStatus::Modified,
                            'D' => FileGitStatus::Deleted,
                            'U' => FileGitStatus::Conflict,
                            _ => FileGitStatus::Modified,
                        };
                        items.push(GitFileItem {
                            path: path.clone(),
                            status: st,
                            staged: false,
                            is_selected: self.selected_file_path.as_deref() == Some(&path),
                        });
                    }
                }
            }
        }

        self.files = items;
    }

    pub fn load_diff_for_file(&mut self, rel_path: &str) {
        let full_path = Path::new(&self.repo_path).join(rel_path);

        // Fetch HEAD content
        let head_content = if let Ok(out) = Command::new("git")
            .args(["show", &format!("HEAD:{}", rel_path)])
            .current_dir(&self.repo_path)
            .output()
        {
            if out.status.success() {
                String::from_utf8_lossy(&out.stdout).to_string()
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        // Fetch Working copy content
        let work_content = if full_path.exists() {
            std::fs::read_to_string(&full_path).unwrap_or_default()
        } else {
            String::new()
        };

        self.left_file_content = head_content;
        self.right_file_content = work_content;

        let (rows, hunks) = compute_side_by_side_diff(&self.left_file_content, &self.right_file_content);
        self.diff_rows = rows;
        self.diff_hunks = hunks;
    }

    pub fn stage_file(&mut self, rel_path: &str) {
        let _ = Command::new("git")
            .args(["add", rel_path])
            .current_dir(&self.repo_path)
            .status();
        self.status_message = format!("Staged {}", rel_path);
        self.refresh_all();
    }

    pub fn unstage_file(&mut self, rel_path: &str) {
        let _ = Command::new("git")
            .args(["restore", "--staged", rel_path])
            .current_dir(&self.repo_path)
            .status();
        self.status_message = format!("Unstaged {}", rel_path);
        self.refresh_all();
    }

    pub fn stage_all(&mut self) {
        let _ = Command::new("git")
            .args(["add", "-A"])
            .current_dir(&self.repo_path)
            .status();
        self.status_message = "Staged all changes".to_string();
        self.refresh_all();
    }

    pub fn unstage_all(&mut self) {
        let _ = Command::new("git")
            .args(["restore", "--staged", "."])
            .current_dir(&self.repo_path)
            .status();
        self.status_message = "Unstaged all changes".to_string();
        self.refresh_all();
    }

    pub fn revert_file(&mut self, rel_path: &str) {
        let full_path = Path::new(&self.repo_path).join(rel_path);
        if let Ok(out) = Command::new("git")
            .args(["status", "--porcelain", rel_path])
            .current_dir(&self.repo_path)
            .output()
        {
            let status_str = String::from_utf8_lossy(&out.stdout);
            if status_str.starts_with("??") {
                let _ = std::fs::remove_file(full_path);
            } else {
                let _ = Command::new("git")
                    .args(["checkout", "HEAD", "--", rel_path])
                    .current_dir(&self.repo_path)
                    .status();
            }
        }
        self.status_message = format!("Reverted {}", rel_path);
        self.refresh_all();
    }

    pub fn revert_hunk(&mut self, hunk_idx: usize) {
        let rel_path = match &self.selected_file_path {
            Some(p) => p.clone(),
            None => return,
        };
        let hunk = match self.diff_hunks.iter().find(|h| h.hunk_idx == hunk_idx) {
            Some(h) => h.clone(),
            None => return,
        };

        // Revert hunk by updating working copy lines
        let left_lines: Vec<&str> = self.left_file_content.lines().collect();
        let mut right_lines: Vec<String> = self.right_file_content.lines().map(|s| s.to_string()).collect();

        let l_start = hunk.left_line_start;
        let l_count = hunk.left_line_count;
        let r_start = hunk.right_line_start;
        let r_count = hunk.right_line_count;

        let replacement: Vec<String> = left_lines
            .iter()
            .skip(l_start)
            .take(l_count)
            .map(|s| s.to_string())
            .collect();

        if r_start <= right_lines.len() {
            let drain_end = (r_start + r_count).min(right_lines.len());
            right_lines.splice(r_start..drain_end, replacement);
        }

        let new_content = if right_lines.is_empty() {
            String::new()
        } else {
            right_lines.join("\n") + "\n"
        };

        let full_path = Path::new(&self.repo_path).join(&rel_path);
        if std::fs::write(&full_path, new_content).is_ok() {
            self.status_message = format!("Applied/reverted hunk #{}", hunk_idx + 1);
            self.load_diff_for_file(&rel_path);
        }
    }

    pub fn commit(&mut self) -> bool {
        if self.commit_message.trim().is_empty() {
            self.status_message = "Error: Commit message cannot be empty".to_string();
            return false;
        }

        let out = Command::new("git")
            .args(["commit", "-m", self.commit_message.trim()])
            .current_dir(&self.repo_path)
            .output();

        match out {
            Ok(res) if res.status.success() => {
                self.status_message = "Committed successfully!".to_string();
                self.commit_message.clear();
                self.refresh_all();
                true
            }
            Ok(res) => {
                let err = String::from_utf8_lossy(&res.stderr);
                self.status_message = format!("Commit failed: {}", err.trim());
                false
            }
            Err(e) => {
                self.status_message = format!("Commit error: {}", e);
                false
            }
        }
    }

    pub fn push(&mut self) {
        let out = Command::new("git")
            .args(["push"])
            .current_dir(&self.repo_path)
            .output();

        match out {
            Ok(res) if res.status.success() => {
                self.status_message = "Pushed to remote!".to_string();
                self.refresh_all();
            }
            Ok(res) => {
                let err = String::from_utf8_lossy(&res.stderr);
                self.status_message = format!("Push failed: {}", err.trim());
            }
            Err(e) => {
                self.status_message = format!("Push error: {}", e);
            }
        }
    }

    pub fn pull(&mut self) {
        let out = Command::new("git")
            .args(["pull"])
            .current_dir(&self.repo_path)
            .output();

        match out {
            Ok(res) if res.status.success() => {
                self.status_message = "Pulled latest changes!".to_string();
                self.refresh_all();
            }
            Ok(res) => {
                let err = String::from_utf8_lossy(&res.stderr);
                self.status_message = format!("Pull failed: {}", err.trim());
            }
            Err(e) => {
                self.status_message = format!("Pull error: {}", e);
            }
        }
    }

    pub fn switch_branch(&mut self, branch_name: &str) {
        let clean_branch = branch_name.trim_start_matches("remotes/origin/").trim();
        let out = Command::new("git")
            .args(["checkout", clean_branch])
            .current_dir(&self.repo_path)
            .output();

        match out {
            Ok(res) if res.status.success() => {
                self.status_message = format!("Switched to branch {}", clean_branch);
                self.refresh_all();
            }
            Ok(res) => {
                let err = String::from_utf8_lossy(&res.stderr);
                self.status_message = format!("Checkout failed: {}", err.trim());
            }
            Err(e) => {
                self.status_message = format!("Checkout error: {}", e);
            }
        }
    }

    pub fn create_branch(&mut self, name: &str) {
        if name.trim().is_empty() {
            return;
        }
        let out = Command::new("git")
            .args(["checkout", "-b", name.trim()])
            .current_dir(&self.repo_path)
            .output();

        match out {
            Ok(res) if res.status.success() => {
                self.status_message = format!("Created & switched to branch {}", name.trim());
                self.new_branch_name.clear();
                self.is_creating_branch = false;
                self.refresh_all();
            }
            Ok(res) => {
                let err = String::from_utf8_lossy(&res.stderr);
                self.status_message = format!("Branch creation failed: {}", err.trim());
            }
            Err(e) => {
                self.status_message = format!("Branch creation error: {}", e);
            }
        }
    }
}

/// Computes an aligned side-by-side line diff between left (HEAD) and right (Working) texts.
pub fn compute_side_by_side_diff(left_str: &str, right_str: &str) -> (Vec<DiffRow>, Vec<DiffHunk>) {
    let left_lines: Vec<&str> = left_str.lines().collect();
    let right_lines: Vec<&str> = right_str.lines().collect();

    let mut rows = Vec::new();
    let mut hunks = Vec::new();

    let mut i = 0;
    let mut j = 0;
    let mut current_hunk: Option<DiffHunk> = None;
    let mut hunk_counter = 0;

    while i < left_lines.len() || j < right_lines.len() {
        if i < left_lines.len() && j < right_lines.len() && left_lines[i] == right_lines[j] {
            // Close any active hunk
            if let Some(h) = current_hunk.take() {
                hunks.push(h);
            }

            rows.push(DiffRow {
                left_line_num: Some(i + 1),
                left_text: left_lines[i].to_string(),
                right_line_num: Some(j + 1),
                right_text: right_lines[j].to_string(),
                kind: DiffKind::Equal,
                hunk_idx: None,
            });
            i += 1;
            j += 1;
        } else {
            // Look ahead for matching lines
            let mut match_found = false;
            let mut ahead_i = i;
            let mut ahead_j = j;

            // Search up to 15 lines ahead for sync point
            'search: for di in 0..15 {
                for dj in 0..15 {
                    if di == 0 && dj == 0 {
                        continue;
                    }
                    if i + di < left_lines.len() && j + dj < right_lines.len() && left_lines[i + di] == right_lines[j + dj] {
                        ahead_i = i + di;
                        ahead_j = j + dj;
                        match_found = true;
                        break 'search;
                    }
                }
            }

            let hunk_id = current_hunk.as_ref().map(|h| h.hunk_idx).unwrap_or_else(|| {
                let id = hunk_counter;
                hunk_counter += 1;
                id
            });

            if current_hunk.is_none() {
                current_hunk = Some(DiffHunk {
                    hunk_idx: hunk_id,
                    start_row: rows.len(),
                    end_row: rows.len(),
                    kind: DiffKind::Modified,
                    left_line_start: i,
                    left_line_count: 0,
                    right_line_start: j,
                    right_line_count: 0,
                });
            }

            if match_found {
                let left_diff_count = ahead_i - i;
                let right_diff_count = ahead_j - j;

                let max_diff = left_diff_count.max(right_diff_count);
                for k in 0..max_diff {
                    let l_line = if k < left_diff_count { Some(i + k + 1) } else { None };
                    let l_txt = if k < left_diff_count { left_lines[i + k].to_string() } else { String::new() };

                    let r_line = if k < right_diff_count { Some(j + k + 1) } else { None };
                    let r_txt = if k < right_diff_count { right_lines[j + k].to_string() } else { String::new() };

                    let kind = if l_line.is_some() && r_line.is_some() {
                        DiffKind::Modified
                    } else if l_line.is_some() {
                        DiffKind::Deleted
                    } else {
                        DiffKind::Added
                    };

                    rows.push(DiffRow {
                        left_line_num: l_line,
                        left_text: l_txt,
                        right_line_num: r_line,
                        right_text: r_txt,
                        kind,
                        hunk_idx: Some(hunk_id),
                    });
                }

                if let Some(h) = current_hunk.as_mut() {
                    h.end_row = rows.len() - 1;
                    h.left_line_count += left_diff_count;
                    h.right_line_count += right_diff_count;
                }

                i = ahead_i;
                j = ahead_j;
            } else {
                // Drain remaining lines to end of file
                let l_line = if i < left_lines.len() { Some(i + 1) } else { None };
                let l_txt = if i < left_lines.len() { left_lines[i].to_string() } else { String::new() };

                let r_line = if j < right_lines.len() { Some(j + 1) } else { None };
                let r_txt = if j < right_lines.len() { right_lines[j].to_string() } else { String::new() };

                let kind = if l_line.is_some() && r_line.is_some() {
                    DiffKind::Modified
                } else if l_line.is_some() {
                    DiffKind::Deleted
                } else {
                    DiffKind::Added
                };

                rows.push(DiffRow {
                    left_line_num: l_line,
                    left_text: l_txt,
                    right_line_num: r_line,
                    right_text: r_txt,
                    kind,
                    hunk_idx: Some(hunk_id),
                });

                if let Some(h) = current_hunk.as_mut() {
                    h.end_row = rows.len() - 1;
                    if l_line.is_some() { h.left_line_count += 1; }
                    if r_line.is_some() { h.right_line_count += 1; }
                }

                if i < left_lines.len() { i += 1; }
                if j < right_lines.len() { j += 1; }
            }
        }
    }

    if let Some(h) = current_hunk.take() {
        hunks.push(h);
    }

    (rows, hunks)
}

impl WindowApp for GitApp {
    fn title(&self) -> String {
        format!("{} Git Manager ({})", Icons::GIT_BRANCH, self.current_branch)
    }

    fn default_size(&self) -> [f32; 2] {
        [1080.0, 700.0]
    }

    fn min_size(&self) -> [f32; 2] {
        [750.0, 500.0]
    }

    fn window_type(&self) -> &'static str {
        "git_manager"
    }

    fn render(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        _config: &mut crate::ui::settings::AppConfig,
    ) -> Option<WindowAction> {
        let is_dark = ctx.style().visuals.dark_mode;

        let bg_color = if is_dark {
            egui::Color32::from_rgb(22, 22, 26)
        } else {
            egui::Color32::from_rgb(248, 248, 250)
        };

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(bg_color))
            .show_inside(ui, |ui| {
                // ===============================================
                // TOP TOOLBAR
                // ===============================================
                let top_frame = egui::Frame::default()
                    .fill(if is_dark { egui::Color32::from_rgb(30, 30, 36) } else { egui::Color32::from_rgb(235, 235, 240) })
                    .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                    .stroke(egui::Stroke::new(1.0, if is_dark { egui::Color32::from_gray(45) } else { egui::Color32::from_gray(210) }));

                top_frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        // Branch Selector
                        ui.label(Icons::rich(Icons::GIT_BRANCH, 14.0));
                        ui.menu_button(
                            egui::RichText::new(&self.current_branch).strong().size(13.0),
                            |ui| {
                                ui.set_min_width(200.0);
                                ui.label(egui::RichText::new("Switch Branch").weak().size(11.0));
                                ui.separator();

                                let branches = self.branches.clone();
                                for b in branches {
                                    let is_active = b == self.current_branch;
                                    if ui.selectable_label(is_active, &b).clicked() {
                                        self.switch_branch(&b);
                                        ui.close_menu();
                                    }
                                }

                                ui.separator();
                                if ui.button(format!("{} New Branch...", Icons::ADD)).clicked() {
                                    self.is_creating_branch = true;
                                    ui.close_menu();
                                }
                            },
                        );

                        if self.is_creating_branch {
                            ui.add_space(8.0);
                            ui.add(
                                egui::TextEdit::singleline(&mut self.new_branch_name)
                                    .hint_text("Branch name...")
                                    .desired_width(120.0),
                            );
                            if ui.button(Icons::rich(Icons::CHECK, 12.0)).clicked() {
                                let name = self.new_branch_name.clone();
                                self.create_branch(&name);
                            }
                            if ui.button(Icons::rich(Icons::CLOSE, 12.0)).clicked() {
                                self.is_creating_branch = false;
                            }
                        }

                        ui.add_space(16.0);
                        ui.separator();
                        ui.add_space(8.0);

                        // Actions
                        if ui.button(format!("{} Refresh", Icons::REFRESH)).clicked() {
                            self.refresh_all();
                        }
                        if ui.button(format!("{} Pull", Icons::CARET_DOWN_KEY)).clicked() {
                            self.pull();
                        }
                        if ui.button(format!("{} Push", Icons::CARET_UP_KEY)).clicked() {
                            self.push();
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let msg_color = if self.status_message.starts_with("Error") {
                                egui::Color32::from_rgb(240, 90, 90)
                            } else {
                                ui.visuals().weak_text_color()
                            };
                            ui.label(egui::RichText::new(&self.status_message).size(12.0).color(msg_color));
                        });
                    });
                });

                ui.add_space(2.0);

                // ===============================================
                // MAIN BODY: SPLIT LEFT (CHANGES) AND RIGHT (DIFF)
                // ===============================================
                ui.columns(2, |columns| {
                    // -------------------------------------------
                    // LEFT PANEL: STAGED & UNSTAGED CHANGES
                    // -------------------------------------------
                    let left_ui = &mut columns[0];
                    left_ui.set_max_width(300.0);

                    let panel_frame = egui::Frame::default()
                        .fill(if is_dark { egui::Color32::from_rgb(26, 26, 32) } else { egui::Color32::from_rgb(240, 240, 245) })
                        .inner_margin(8.0)
                        .rounding(6.0);

                    panel_frame.show(left_ui, |ui| {
                        let staged_count = self.files.iter().filter(|f| f.staged).count();
                        let unstaged_count = self.files.iter().filter(|f| !f.staged).count();

                        // Header & Stage All / Unstage All
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Local Changes").strong().size(13.0));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("Stage All").clicked() {
                                    self.stage_all();
                                }
                                if ui.button("Unstage All").clicked() {
                                    self.unstage_all();
                                }
                            });
                        });
                        ui.separator();

                        egui::ScrollArea::vertical()
                            .max_height(360.0)
                            .show(ui, |ui| {
                                // STAGED CHANGES
                                ui.collapsing(
                                    egui::RichText::new(format!("Staged Changes ({})", staged_count))
                                        .strong()
                                        .size(12.0),
                                    |ui| {
                                        let staged_files: Vec<GitFileItem> = self.files.iter().filter(|f| f.staged).cloned().collect();
                                        if staged_files.is_empty() {
                                            ui.label(egui::RichText::new("No staged files").weak().size(11.0));
                                        } else {
                                            for item in staged_files {
                                                let is_sel = self.selected_file_path.as_deref() == Some(&item.path);
                                                ui.horizontal(|ui| {
                                                    if ui.button("-").on_hover_text("Unstage file").clicked() {
                                                        let path = item.path.clone();
                                                        self.unstage_file(&path);
                                                    }
                                                    let resp = ui.selectable_label(
                                                        is_sel,
                                                        format!("{} {}", Icons::get_file_icon(&item.path), item.path),
                                                    );
                                                    if resp.clicked() {
                                                        let path = item.path.clone();
                                                        self.selected_file_path = Some(path.clone());
                                                        self.load_diff_for_file(&path);
                                                    }
                                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                        ui.label(
                                                            egui::RichText::new(item.status.label())
                                                                .size(9.0)
                                                                .color(item.status.color()),
                                                        );
                                                    });
                                                });
                                            }
                                        }
                                    },
                                );

                                ui.add_space(4.0);

                                // UNSTAGED CHANGES
                                ui.collapsing(
                                    egui::RichText::new(format!("Unstaged Changes ({})", unstaged_count))
                                        .strong()
                                        .size(12.0),
                                    |ui| {
                                        let unstaged_files: Vec<GitFileItem> = self.files.iter().filter(|f| !f.staged).cloned().collect();
                                        if unstaged_files.is_empty() {
                                            ui.label(egui::RichText::new("No modified files").weak().size(11.0));
                                        } else {
                                            for item in unstaged_files {
                                                let is_sel = self.selected_file_path.as_deref() == Some(&item.path);
                                                ui.horizontal(|ui| {
                                                    if ui.button("+").on_hover_text("Stage file").clicked() {
                                                        let path = item.path.clone();
                                                        self.stage_file(&path);
                                                    }
                                                    if ui.button(Icons::rich(Icons::REFRESH, 10.0)).on_hover_text("Revert changes").clicked() {
                                                        let path = item.path.clone();
                                                        self.revert_file(&path);
                                                    }
                                                    let resp = ui.selectable_label(
                                                        is_sel,
                                                        format!("{} {}", Icons::get_file_icon(&item.path), item.path),
                                                    );
                                                    if resp.clicked() {
                                                        let path = item.path.clone();
                                                        self.selected_file_path = Some(path.clone());
                                                        self.load_diff_for_file(&path);
                                                    }
                                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                        ui.label(
                                                            egui::RichText::new(item.status.label())
                                                                .size(9.0)
                                                                .color(item.status.color()),
                                                        );
                                                    });
                                                });
                                            }
                                        }
                                    },
                                );
                            });

                        ui.separator();
                        ui.add_space(4.0);

                        // COMMIT BOX
                        ui.label(egui::RichText::new("Commit Message").strong().size(12.0));
                        ui.add(
                            egui::TextEdit::multiline(&mut self.commit_message)
                                .hint_text("Write commit message...")
                                .desired_rows(3)
                                .desired_width(ui.available_width()),
                        );
                        ui.add_space(4.0);

                        ui.horizontal(|ui| {
                            if ui.add(
                                egui::Button::new(
                                    egui::RichText::new(format!("{} Commit", Icons::GIT_COMMIT))
                                        .strong()
                                        .color(egui::Color32::WHITE),
                                )
                                .fill(egui::Color32::from_rgb(45, 125, 220))
                                .rounding(4.0)
                                .min_size(egui::vec2(90.0, 28.0)),
                            ).clicked() {
                                self.commit();
                            }

                            if ui.add(
                                egui::Button::new(
                                    egui::RichText::new("Commit & Push")
                                        .strong()
                                        .color(egui::Color32::WHITE),
                                )
                                .fill(egui::Color32::from_rgb(35, 150, 100))
                                .rounding(4.0)
                                .min_size(egui::vec2(110.0, 28.0)),
                            ).clicked() {
                                if self.commit() {
                                    self.push();
                                }
                            }
                        });
                    });

                    // -------------------------------------------
                    // RIGHT PANEL: INTELLIJ SIDE-BY-SIDE DIFF VIEWER
                    // -------------------------------------------
                    let right_ui = &mut columns[1];

                    let diff_frame = egui::Frame::default()
                        .fill(if is_dark { egui::Color32::from_rgb(18, 18, 22) } else { egui::Color32::from_rgb(255, 255, 255) })
                        .inner_margin(8.0)
                        .rounding(6.0)
                        .stroke(egui::Stroke::new(1.0, if is_dark { egui::Color32::from_gray(40) } else { egui::Color32::from_gray(210) }));

                    diff_frame.show(right_ui, |ui| {
                        let selected_path = match &self.selected_file_path {
                            Some(p) => p.clone(),
                            None => {
                                ui.centered_and_justified(|ui| {
                                    ui.label(
                                        egui::RichText::new("Select a changed file on the left to view diff")
                                            .weak()
                                            .size(14.0),
                                    );
                                });
                                return;
                            }
                        };

                        // DIFF TOOLBAR HEADER
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(format!("{} {}", Icons::get_file_icon(&selected_path), selected_path))
                                    .strong()
                                    .size(13.0),
                            );

                            let (add_count, del_count) = self.diff_rows.iter().fold((0, 0), |(a, d), r| match r.kind {
                                DiffKind::Added => (a + 1, d),
                                DiffKind::Deleted => (a, d + 1),
                                DiffKind::Modified => (a + 1, d + 1),
                                _ => (a, d),
                            });

                            ui.label(
                                egui::RichText::new(format!("+{} / -{} lines", add_count, del_count))
                                    .size(11.0)
                                    .color(egui::Color32::from_rgb(100, 200, 140)),
                            );

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let path = selected_path.clone();
                                if ui.button("Revert File").clicked() {
                                    self.revert_file(&path);
                                }
                                if ui.button("Stage File").clicked() {
                                    self.stage_file(&path);
                                }
                            });
                        });
                        ui.separator();

                        // SIDE-BY-SIDE DUAL PANE HEADER
                        ui.horizontal(|ui| {
                            let half_w = (ui.available_width() - 40.0) / 2.0;
                            ui.allocate_ui(egui::vec2(half_w, 20.0), |ui| {
                                ui.label(egui::RichText::new("HEAD (Original)").weak().strong().size(11.5));
                            });
                            ui.allocate_ui(egui::vec2(36.0, 20.0), |ui| {
                                ui.centered_and_justified(|ui| {
                                    ui.label(egui::RichText::new("Action").weak().size(10.0));
                                });
                            });
                            ui.allocate_ui(egui::vec2(half_w, 20.0), |ui| {
                                ui.label(egui::RichText::new("Working Copy (Modified)").weak().strong().size(11.5));
                            });
                        });
                        ui.separator();

                        // SIDE-BY-SIDE DIFF BODY WITH GUTTER CONTROLS
                        let mut hunk_to_revert: Option<usize> = None;

                        egui::ScrollArea::both().show(ui, |ui| {
                            let total_w = ui.available_width();
                            let col_w = (total_w - 46.0) / 2.0;

                            for (r_idx, row) in self.diff_rows.iter().enumerate() {
                                let (bg_color, _text_color) = match row.kind {
                                    DiffKind::Added => (
                                        if is_dark { egui::Color32::from_rgba_unmultiplied(40, 140, 70, 50) } else { egui::Color32::from_rgba_unmultiplied(180, 240, 190, 80) },
                                        egui::Color32::from_rgb(100, 220, 140),
                                    ),
                                    DiffKind::Deleted => (
                                        if is_dark { egui::Color32::from_rgba_unmultiplied(180, 50, 50, 50) } else { egui::Color32::from_rgba_unmultiplied(255, 200, 200, 80) },
                                        egui::Color32::from_rgb(240, 100, 100),
                                    ),
                                    DiffKind::Modified => (
                                        if is_dark { egui::Color32::from_rgba_unmultiplied(50, 90, 160, 50) } else { egui::Color32::from_rgba_unmultiplied(200, 220, 255, 80) },
                                        egui::Color32::from_rgb(100, 180, 255),
                                    ),
                                    DiffKind::Conflict => (
                                        if is_dark { egui::Color32::from_rgba_unmultiplied(180, 120, 30, 60) } else { egui::Color32::from_rgba_unmultiplied(255, 230, 180, 90) },
                                        egui::Color32::from_rgb(255, 180, 60),
                                    ),
                                    DiffKind::Equal => (egui::Color32::TRANSPARENT, ui.visuals().text_color()),
                                };

                                let row_frame = egui::Frame::none().fill(bg_color);

                                row_frame.show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        // LEFT PANE (Original)
                                        ui.allocate_ui(egui::vec2(col_w, 18.0), |ui| {
                                            ui.horizontal(|ui| {
                                                let line_str = row.left_line_num.map(|n| n.to_string()).unwrap_or_default();
                                                ui.label(
                                                    egui::RichText::new(format!("{:>4}", line_str))
                                                        .size(11.0)
                                                        .weak()
                                                        .family(egui::FontFamily::Monospace),
                                                );
                                                ui.add_space(4.0);
                                                ui.label(
                                                    egui::RichText::new(&row.left_text)
                                                        .size(11.5)
                                                        .family(egui::FontFamily::Monospace),
                                                );
                                            });
                                        });

                                        // CENTER GUTTER (IntelliJ Move / Apply / Revert Actions)
                                        ui.allocate_ui(egui::vec2(40.0, 18.0), |ui| {
                                            ui.centered_and_justified(|ui| {
                                                if let Some(h_idx) = row.hunk_idx {
                                                    // Display action button once per hunk (on first line of hunk)
                                                    let is_hunk_start = self.diff_hunks.iter().any(|h| h.hunk_idx == h_idx && h.start_row == r_idx);
                                                    if is_hunk_start {
                                                        let apply_btn = ui.add(
                                                            egui::Button::new(
                                                                egui::RichText::new("≫")
                                                                    .strong()
                                                                    .size(11.0)
                                                                    .color(egui::Color32::from_rgb(100, 200, 255)),
                                                            )
                                                            .fill(egui::Color32::TRANSPARENT)
                                                            .min_size(egui::vec2(18.0, 16.0)),
                                                        ).on_hover_text("Accept / Apply hunk from left to right (IntelliJ Move)");

                                                        if apply_btn.clicked() {
                                                            hunk_to_revert = Some(h_idx);
                                                        }
                                                    }
                                                }
                                            });
                                        });

                                        // RIGHT PANE (Modified)
                                        ui.allocate_ui(egui::vec2(col_w, 18.0), |ui| {
                                            ui.horizontal(|ui| {
                                                let line_str = row.right_line_num.map(|n| n.to_string()).unwrap_or_default();
                                                ui.label(
                                                    egui::RichText::new(format!("{:>4}", line_str))
                                                        .size(11.0)
                                                        .weak()
                                                        .family(egui::FontFamily::Monospace),
                                                );
                                                ui.add_space(4.0);
                                                ui.label(
                                                    egui::RichText::new(&row.right_text)
                                                        .size(11.5)
                                                        .family(egui::FontFamily::Monospace),
                                                );
                                            });
                                        });
                                    });
                                });
                            }
                        });

                        if let Some(h_idx) = hunk_to_revert {
                            self.revert_hunk(h_idx);
                        }
                    });
                });
            });

        None
    }
}

// Extra phosphor icon helpers
impl Icons {
    pub const CARET_DOWN_KEY: &'static str = "\u{e136}";
    pub const CARET_UP_KEY: &'static str = "\u{e140}";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_side_by_side_diff_equal() {
        let left = "line 1\nline 2\nline 3";
        let right = "line 1\nline 2\nline 3";
        let (rows, hunks) = compute_side_by_side_diff(left, right);

        assert_eq!(rows.len(), 3);
        assert!(hunks.is_empty());
        assert_eq!(rows[0].kind, DiffKind::Equal);
    }

    #[test]
    fn test_compute_side_by_side_diff_added_modified() {
        let left = "line 1\nline 2\nline 3";
        let right = "line 1\nline 2 modified\nline 3\nline 4 added";
        let (rows, hunks) = compute_side_by_side_diff(left, right);

        assert_eq!(rows.len(), 4);
        assert_eq!(hunks.len(), 2);
        assert_eq!(rows[1].kind, DiffKind::Modified);
        assert_eq!(rows[3].kind, DiffKind::Added);
    }
}
