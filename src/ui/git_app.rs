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
    pub checked_for_commit: bool,
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
    pub amend_last_commit: bool,
    pub is_unified_view: bool,
    pub diff_rows: Vec<DiffRow>,
    pub diff_hunks: Vec<DiffHunk>,
    pub left_file_content: String,
    pub right_file_content: String,
    pub status_message: String,
    pub new_branch_name: String,
    pub is_creating_branch: bool,
    pub needs_refresh: bool,
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
            amend_last_commit: false,
            is_unified_view: false,
            diff_rows: Vec::new(),
            diff_hunks: Vec::new(),
            left_file_content: String::new(),
            right_file_content: String::new(),
            status_message: "Ready".to_string(),
            new_branch_name: String::new(),
            is_creating_branch: false,
            needs_refresh: false,
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
                            checked_for_commit: true,
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
                            checked_for_commit: false,
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

    pub fn delete_file(&mut self, rel_path: &str) {
        let full_path = Path::new(&self.repo_path).join(rel_path);
        if full_path.exists() {
            if full_path.is_dir() {
                let _ = std::fs::remove_dir_all(&full_path);
            } else {
                let _ = std::fs::remove_file(&full_path);
            }
        }
        let _ = Command::new("git")
            .args(["rm", "-f", rel_path])
            .current_dir(&self.repo_path)
            .status();
        self.status_message = format!("Deleted {}", rel_path);
        self.refresh_all();
    }

    pub fn select_prev_file(&mut self) {
        if self.files.is_empty() {
            return;
        }
        let current_idx = self.files.iter().position(|f| Some(&f.path) == self.selected_file_path.as_ref()).unwrap_or(0);
        let prev_idx = if current_idx == 0 { self.files.len() - 1 } else { current_idx - 1 };
        let path = self.files[prev_idx].path.clone();
        self.selected_file_path = Some(path.clone());
        self.load_diff_for_file(&path);
    }

    pub fn select_next_file(&mut self) {
        if self.files.is_empty() {
            return;
        }
        let current_idx = self.files.iter().position(|f| Some(&f.path) == self.selected_file_path.as_ref()).unwrap_or(0);
        let next_idx = (current_idx + 1) % self.files.len();
        let path = self.files[next_idx].path.clone();
        self.selected_file_path = Some(path.clone());
        self.load_diff_for_file(&path);
    }

    pub fn commit(&mut self) -> bool {
        if self.commit_message.trim().is_empty() {
            self.status_message = "Error: Commit message cannot be empty".to_string();
            return false;
        }

        let mut args = vec!["commit"];
        if self.amend_last_commit {
            args.push("--amend");
        }
        let msg = self.commit_message.trim().to_string();
        args.extend(["-m", &msg]);

        if let Ok(out) = Command::new("git")
            .args(&args)
            .current_dir(&self.repo_path)
            .output()
        {
            if out.status.success() {
                self.status_message = "Committed successfully".to_string();
                self.commit_message.clear();
                self.refresh_all();
                true
            } else {
                let err = String::from_utf8_lossy(&out.stderr);
                self.status_message = format!("Error: {}", err.trim());
                false
            }
        } else {
            self.status_message = "Error executing git commit".to_string();
            false
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

            // Search up to 30 lines ahead for sync point
            'search: for di in 0..30 {
                for dj in 0..30 {
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

            let (target_i, target_j) = if match_found {
                (ahead_i, ahead_j)
            } else {
                (left_lines.len(), right_lines.len())
            };

            let left_diff_count = target_i - i;
            let right_diff_count = target_j - j;

            if left_diff_count == right_diff_count {
                // Direct 1-to-1 line replacements (Modified)
                for k in 0..left_diff_count {
                    rows.push(DiffRow {
                        left_line_num: Some(i + k + 1),
                        left_text: left_lines[i + k].to_string(),
                        right_line_num: Some(j + k + 1),
                        right_text: right_lines[j + k].to_string(),
                        kind: DiffKind::Modified,
                        hunk_idx: Some(hunk_id),
                    });
                }
            } else {
                // Block Deletion on Left with Blank Padding on Right
                for k in 0..left_diff_count {
                    rows.push(DiffRow {
                        left_line_num: Some(i + k + 1),
                        left_text: left_lines[i + k].to_string(),
                        right_line_num: None,
                        right_text: String::new(),
                        kind: DiffKind::Deleted,
                        hunk_idx: Some(hunk_id),
                    });
                }
                // Block Addition on Right with Blank Padding on Left
                for k in 0..right_diff_count {
                    rows.push(DiffRow {
                        left_line_num: None,
                        left_text: String::new(),
                        right_line_num: Some(j + k + 1),
                        right_text: right_lines[j + k].to_string(),
                        kind: DiffKind::Added,
                        hunk_idx: Some(hunk_id),
                    });
                }
            }

            if let Some(h) = current_hunk.as_mut() {
                h.end_row = rows.len() - 1;
                h.left_line_count += left_diff_count;
                h.right_line_count += right_diff_count;
            }

            i = target_i;
            j = target_j;
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
        undo: &mut crate::ui::undo_manager::UndoManager,
    ) -> Option<WindowAction> {
        if self.needs_refresh {
            self.refresh_all();
            self.needs_refresh = false;
        }

        let is_dark = ctx.style().visuals.dark_mode;
        let dark = |r: u8, g: u8, b: u8| egui::Color32::from_rgb(r, g, b);
        let gray = |v: u8| egui::Color32::from_gray(v);

        // IntelliJ IDEA Dark Theme Palette
        let panel_bg  = if is_dark { dark(43, 45, 48) } else { dark(240, 240, 245) };  // #2b2d30
        let diff_bg   = if is_dark { dark(30, 31, 34) } else { dark(255, 255, 255) };  // #1e1f22
        let border_c  = if is_dark { dark(57, 59, 64) } else { gray(210) };             // #393b40
        let commit_bg = if is_dark { dark(30, 31, 34) } else { dark(245, 245, 250) };  // #1e1f22
        let hdr_bg    = if is_dark { dark(37, 38, 42) } else { dark(235, 235, 240) };  // #25262a

        let total_size = ui.available_size();

        // Constrain layout to available space so the window doesn't grow unbounded
        ui.set_max_width(total_size.x);
        ui.set_max_height(total_size.y);

        ui.vertical(|ui| {
            // =========================================================================
            // 1. TOP GLOBAL TOOLBAR (IntelliJ Style Git Toolbar)
            // =========================================================================
            egui::Frame::default()
                .fill(hdr_bg)
                .inner_margin(egui::Margin::symmetric(10.0, 5.0))
                .stroke(egui::Stroke::new(1.0, border_c))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(Icons::rich(Icons::GIT_BRANCH, 14.0));
                        ui.menu_button(
                            egui::RichText::new(format!("Branch: {}", self.current_branch)).strong().size(12.5),
                            |ui| {
                                ui.set_min_width(180.0);
                                ui.label(egui::RichText::new("Switch Branch").weak().size(11.0));
                                ui.separator();
                                let branches = self.branches.clone();
                                for b in branches {
                                    if ui.selectable_label(b == self.current_branch, &b).clicked() {
                                        undo.push(crate::ui::undo_manager::UndoAction::GitSwitchBranch { 
                                            repo_path: self.repo_path.clone(), 
                                            previous_branch: self.current_branch.clone(),
                                        }, format!("Switch branch to {}", b));
                                        self.switch_branch(&b);
                                        ui.close_menu();
                                    }
                                }
                                ui.separator();
                                if ui.button(format!("{} New Branch", Icons::ADD)).clicked() {
                                    self.is_creating_branch = true;
                                    ui.close_menu();
                                }
                            },
                        );
                        if self.is_creating_branch {
                            ui.add_space(4.0);
                            ui.add(egui::TextEdit::singleline(&mut self.new_branch_name)
                                .hint_text("Branch name...").desired_width(100.0));
                            if ui.button(Icons::rich(Icons::CHECK, 12.0)).clicked() {
                                let n = self.new_branch_name.clone();
                                undo.push(crate::ui::undo_manager::UndoAction::GitCreateBranch { 
                                    repo_path: self.repo_path.clone(), 
                                    branch_name: n.clone(),
                                    previous_branch: self.current_branch.clone(),
                                }, format!("Create branch: {}", n));
                                self.create_branch(&n);
                            }
                            if ui.button(Icons::rich(Icons::CLOSE, 12.0)).clicked() {
                                self.is_creating_branch = false;
                            }
                        }
                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(6.0);
                        if ui.button(Icons::job(Icons::REFRESH, "Refresh", 12.0)).clicked()       { self.refresh_all(); }
                        if ui.button(Icons::job(Icons::CARET_DOWN_KEY, "Pull", 12.0)).clicked()    { self.pull(); }
                        if ui.button(Icons::job(Icons::CARET_UP_KEY, "Push", 12.0)).clicked()      { self.push(); }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let c = if self.status_message.starts_with("Error") {
                                egui::Color32::from_rgb(240, 90, 90)
                            } else { ui.visuals().weak_text_color() };
                            ui.label(egui::RichText::new(&self.status_message).size(12.0).color(c));
                        });
                    });
                });

            ui.add_space(2.0);

            // =========================================================================
            // 2. MAIN SPLIT BODY (IntelliJ Commit Sidebar + Diff Pane)
            // =========================================================================
            let body_h = (total_size.y - 42.0).max(100.0);
            let sidebar_w = 280.0_f32;
            let diff_w = (total_size.x - sidebar_w - 10.0).max(200.0);

            ui.horizontal(|ui| {
                // ---------------------------------------------------------------------
                // A) LEFT SIDEBAR (IntelliJ Commit / Changes Panel)
                // ---------------------------------------------------------------------
                egui::Frame::default()
                    .fill(panel_bg)
                    .inner_margin(6.0)
                    .rounding(4.0)
                    .stroke(egui::Stroke::new(1.0, border_c))
                    .show(ui, |ui| {
                        ui.set_width(sidebar_w);
                        ui.set_height(body_h);

                        ui.vertical(|ui| {
                            let staged_n   = self.files.iter().filter(|f|  f.staged).count();
                            let unstaged_n = self.files.iter().filter(|f| !f.staged).count();

                            // Sidebar Header
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("Commit").strong().size(13.0));
                                ui.label(egui::RichText::new(format!("({} files)", self.files.len())).weak().size(11.5));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.small_button("Stage All").clicked() {
                                        undo.push(crate::ui::undo_manager::UndoAction::GitStageAll { 
                                            repo_path: self.repo_path.clone() 
                                        }, "Stage all");
                                        self.stage_all(); 
                                    }
                                    if ui.small_button("Unstage All").clicked() {
                                        undo.push(crate::ui::undo_manager::UndoAction::GitUnstageAll { 
                                            repo_path: self.repo_path.clone() 
                                        }, "Unstage all");
                                        self.unstage_all(); 
                                    }
                                });
                            });
                            ui.separator();

                            // Top File Tree (Fills available space above commit box)
                            let commit_box_h = 150.0_f32;
                            let tree_h = (body_h - commit_box_h - 24.0).max(80.0);

                            ui.allocate_ui_with_layout(
                                egui::vec2(sidebar_w - 12.0, tree_h),
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| {
                                    egui::ScrollArea::vertical()
                                        .id_salt("git_sidebar_file_tree_scroll")
                                        .auto_shrink([false, false])
                                        .max_height(tree_h)
                                        .show(ui, |ui| {
                                            // STAGED CHANGES
                                            ui.collapsing(
                                                egui::RichText::new(format!("Staged Changes ({})", staged_n))
                                                    .strong().size(11.5),
                                                |ui| {
                                                    let items: Vec<GitFileItem> = self.files.iter()
                                                        .filter(|f| f.staged).cloned().collect();
                                                    if items.is_empty() {
                                                        ui.label(egui::RichText::new("No staged files").weak().size(11.0));
                                                    }
                                                    for item in items {
                                                        let sel = self.selected_file_path.as_deref() == Some(&item.path);
                                                        ui.push_id(format!("staged_{}", item.path), |ui| {
                                                            ui.horizontal(|ui| {
                                                                let mut chk = true;
                                                                if ui.checkbox(&mut chk, "").changed() {
                                                                    let p = item.path.clone();
                                                                    undo.push(crate::ui::undo_manager::UndoAction::GitUnstageFile { 
                                                                        repo_path: self.repo_path.clone(), 
                                                                        rel_path: p.clone() 
                                                                    }, format!("Unstage: {}", p));
                                                                    self.unstage_file(&p);
                                                                }
                                                                if ui.selectable_label(sel,
                                                                    format!("{} {}", Icons::get_file_icon(&item.path), item.path)
                                                                ).clicked() {
                                                                    let p = item.path.clone();
                                                                    self.selected_file_path = Some(p.clone());
                                                                    self.load_diff_for_file(&p);
                                                                }
                                                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                                    let p = item.path.clone();
                                                                    if ui.small_button("D").on_hover_text("Delete file").clicked() {
                                                                        let full_path = std::path::Path::new(&self.repo_path).join(&p);
                                                                        let was_dir = full_path.is_dir();
                                                                        let saved = if was_dir { Vec::new() } else { std::fs::read(&full_path).unwrap_or_default() };
                                                                        undo.push(crate::ui::undo_manager::UndoAction::GitDeleteFile { 
                                                                            repo_path: self.repo_path.clone(), 
                                                                            rel_path: p.clone(),
                                                                            saved_content: saved,
                                                                            was_dir,
                                                                        }, format!("Delete: {}", p));
                                                                        self.delete_file(&p);
                                                                    }
                                                                    ui.label(egui::RichText::new(item.status.label())
                                                                        .size(9.0).color(item.status.color()));
                                                                });
                                                            });
                                                        });
                                                    }
                                                },
                                            );

                                            ui.add_space(4.0);

                                            // UNSTAGED / UNVERSIONED
                                            ui.collapsing(
                                                egui::RichText::new(format!("Unstaged & Unversioned ({})", unstaged_n))
                                                    .strong().size(11.5),
                                                |ui| {
                                                    let items: Vec<GitFileItem> = self.files.iter()
                                                        .filter(|f| !f.staged).cloned().collect();
                                                    if items.is_empty() {
                                                        ui.label(egui::RichText::new("No unstaged files").weak().size(11.0));
                                                    }
                                                    for item in items {
                                                        let sel = self.selected_file_path.as_deref() == Some(&item.path);
                                                        ui.push_id(format!("unstaged_{}", item.path), |ui| {
                                                            ui.horizontal(|ui| {
                                                                let mut chk = false;
                                                                if ui.checkbox(&mut chk, "").changed() {
                                                                    let p = item.path.clone();
                                                                    undo.push(crate::ui::undo_manager::UndoAction::GitStageFile { 
                                                                        repo_path: self.repo_path.clone(), 
                                                                        rel_path: p.clone() 
                                                                    }, format!("Stage: {}", p));
                                                                    self.stage_file(&p);
                                                                }
                                                                if ui.selectable_label(sel,
                                                                    format!("{} {}", Icons::get_file_icon(&item.path), item.path)
                                                                ).clicked() {
                                                                    let p = item.path.clone();
                                                                    self.selected_file_path = Some(p.clone());
                                                                    self.load_diff_for_file(&p);
                                                                }
                                                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                                    let p = item.path.clone();
                                                                    if ui.small_button("D").on_hover_text("Delete file").clicked() {
                                                                        let full_path = std::path::Path::new(&self.repo_path).join(&p);
                                                                        let was_dir = full_path.is_dir();
                                                                        let saved = if was_dir { Vec::new() } else { std::fs::read(&full_path).unwrap_or_default() };
                                                                        undo.push(crate::ui::undo_manager::UndoAction::GitDeleteFile { 
                                                                            repo_path: self.repo_path.clone(), 
                                                                            rel_path: p.clone(),
                                                                            saved_content: saved,
                                                                            was_dir,
                                                                        }, format!("Delete: {}", p));
                                                                        self.delete_file(&p);
                                                                    }
                                                                    if ui.small_button("R").on_hover_text("Revert file").clicked() {
                                                                        let full_path = std::path::Path::new(&self.repo_path).join(&p);
                                                                        let saved = std::fs::read(&full_path).unwrap_or_default();
                                                                        undo.push(crate::ui::undo_manager::UndoAction::GitRevertFile { 
                                                                            repo_path: self.repo_path.clone(), 
                                                                            rel_path: p.clone(),
                                                                            saved_content: saved,
                                                                        }, format!("Revert: {}", p));
                                                                        self.revert_file(&p);
                                                                    }
                                                                    ui.label(egui::RichText::new(item.status.label())
                                                                        .size(9.0).color(item.status.color()));
                                                                });
                                                            });
                                                        });
                                                    }
                                                },
                                            );
                                        });
                                },
                            );

                            ui.separator();

                            // Bottom Commit Box (Docked cleanly at the bottom of sidebar)
                            egui::Frame::default()
                                .fill(commit_bg)
                                .inner_margin(6.0)
                                .rounding(4.0)
                                .stroke(egui::Stroke::new(1.0, border_c))
                                .show(ui, |ui| {
                                    ui.set_width(sidebar_w - 16.0);
                                    ui.checkbox(&mut self.amend_last_commit, "Amend commit");
                                    ui.add_space(2.0);
                                    ui.add(
                                        egui::TextEdit::multiline(&mut self.commit_message)
                                            .hint_text("Write commit message...")
                                            .desired_rows(3)
                                            .desired_width(ui.available_width()),
                                    );
                                    ui.add_space(4.0);
                                    ui.horizontal(|ui| {
                                        let bw = (ui.available_width() - 4.0) / 2.0;
                                        if ui.add(
                                            egui::Button::new(egui::RichText::new("Commit")
                                                .strong().color(egui::Color32::WHITE))
                                            .fill(dark(53, 116, 240)).rounding(4.0)
                                            .min_size(egui::vec2(bw, 26.0))
                                        ).clicked() {
                                            let prev_head = std::process::Command::new("git")
                                                .args(["-C", &self.repo_path, "rev-parse", "HEAD"])
                                                .output().ok()
                                                .and_then(|o| if o.status.success() { Some(String::from_utf8_lossy(&o.stdout).trim().to_string()) } else { None });
                                            undo.push(crate::ui::undo_manager::UndoAction::GitCommit { 
                                                repo_path: self.repo_path.clone(), 
                                                was_amend: self.amend_last_commit,
                                                prev_head,
                                            }, "Commit");
                                            self.commit(); 
                                        }
                                        if ui.add(
                                            egui::Button::new(egui::RichText::new("Commit & Push")
                                                .strong().color(egui::Color32::WHITE))
                                            .fill(dark(45, 142, 87)).rounding(4.0)
                                            .min_size(egui::vec2(bw, 26.0))
                                        ).clicked() {
                                            if self.commit() { self.push(); }
                                        }
                                    });
                                });
                        });
                    });

                ui.add_space(4.0);

                // ---------------------------------------------------------------------
                // B) RIGHT PANEL (IntelliJ Side-by-Side Diff View)
                // ---------------------------------------------------------------------
                egui::Frame::default()
                    .fill(diff_bg)
                    .inner_margin(4.0)
                    .rounding(4.0)
                    .stroke(egui::Stroke::new(1.0, border_c))
                    .show(ui, |ui| {
                        ui.set_width(diff_w);
                        ui.set_height(body_h);

                        ui.vertical(|ui| {
                            let selected_path = match self.selected_file_path.clone() {
                                Some(p) => p,
                                None => {
                                    ui.centered_and_justified(|ui| {
                                        ui.label(egui::RichText::new(
                                            "Select a changed file on the left to view diff")
                                            .weak().size(14.0));
                                    });
                                    return;
                                }
                            };

                            let cur_idx = self.files.iter()
                                .position(|f| f.path == selected_path)
                                .map(|i| i + 1).unwrap_or(1);
                            let total = self.files.len().max(1);
                            let (add_n, del_n) = self.diff_rows.iter().fold((0usize, 0usize), |(a, d), r| {
                                match r.kind {
                                    DiffKind::Added    => (a + 1, d),
                                    DiffKind::Deleted  => (a, d + 1),
                                    DiffKind::Modified => (a + 1, d + 1),
                                    _                  => (a, d),
                                }
                            });

                            // Diff File Header Toolbar (IntelliJ Diff Header)
                            egui::Frame::default()
                                .fill(hdr_bg)
                                .inner_margin(egui::Margin::symmetric(8.0, 5.0))
                                .stroke(egui::Stroke::new(1.0, border_c))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        if ui.small_button("\u{25b2}").on_hover_text("Previous file").clicked() { self.select_prev_file(); }
                                        if ui.small_button("\u{25bc}").on_hover_text("Next file").clicked()     { self.select_next_file(); }
                                        ui.label(egui::RichText::new(format!("{}/{}", cur_idx, total)).size(11.0).weak());
                                        ui.add_space(6.0); ui.separator(); ui.add_space(6.0);
                                        ui.label(egui::RichText::new(
                                            format!("{} {}", Icons::get_file_icon(&selected_path), selected_path))
                                            .strong().size(12.5));
                                        ui.label(egui::RichText::new(format!("+{} / -{}", add_n, del_n))
                                            .size(11.0).color(egui::Color32::from_rgb(100, 200, 140)));
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            let p = selected_path.clone();
                                            if ui.button("Revert File").clicked() {
                                                let full_path = std::path::Path::new(&self.repo_path).join(&p);
                                                let saved = std::fs::read(&full_path).unwrap_or_default();
                                                undo.push(crate::ui::undo_manager::UndoAction::GitRevertFile { 
                                                    repo_path: self.repo_path.clone(), 
                                                    rel_path: p.clone(),
                                                    saved_content: saved,
                                                }, format!("Revert: {}", p));
                                                self.revert_file(&p);
                                            }
                                            if ui.button("Stage File").clicked()  {
                                                undo.push(crate::ui::undo_manager::UndoAction::GitStageFile { 
                                                    repo_path: self.repo_path.clone(), 
                                                    rel_path: p.clone() 
                                                }, format!("Stage: {}", p));
                                                self.stage_file(&p);
                                            }
                                            ui.add_space(6.0);
                                            let mode_text = if self.is_unified_view { "Unified View" } else { "Side-by-Side View" };
                                            if ui.selectable_label(self.is_unified_view, mode_text).clicked() {
                                                self.is_unified_view = !self.is_unified_view;
                                            }
                                        });
                                    });
                                });

                            let lnum_w: f32 = 40.0;
                            let gutter_w: f32 = 32.0;
                            let code_w = ((diff_w - gutter_w - lnum_w * 2.0 - 24.0) / 2.0).max(280.0);
                            let total_w = lnum_w * 2.0 + code_w * 2.0 + gutter_w;
                            let row_h = 18.0_f32;
                            let mut hunk_to_apply: Option<usize> = None;

                            // IntelliJ Diff High Contrast Colors & Colors for Gutter
                            let added_bg    = if is_dark { dark(24, 60, 36) }  else { dark(200, 245, 215) };  // #183c24
                            let deleted_bg  = if is_dark { dark(64, 28, 28) }  else { dark(255, 215, 215) };  // #401c1c
                            let modified_bg = if is_dark { dark(26, 52, 85) }  else { dark(215, 230, 255) };  // #1a3455
                            let gutter_bg   = if is_dark { dark(33, 34, 38) }  else { dark(240, 240, 245) };  // #212226

                            let added_txt    = if is_dark { dark(175, 235, 185) } else { dark(20, 80, 40) };
                            let deleted_txt  = if is_dark { dark(240, 160, 160) } else { dark(140, 30, 30) };
                            let modified_txt = if is_dark { dark(175, 205, 255) } else { dark(20, 60, 140) };
                            let normal_txt   = if is_dark { dark(188, 190, 196) } else { dark(30, 30, 30) };

                            if self.is_unified_view {
                                // Unified View Header
                                egui::Frame::default()
                                    .fill(hdr_bg)
                                    .inner_margin(egui::Margin::symmetric(4.0, 3.0))
                                    .stroke(egui::Stroke::new(1.0, border_c))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.spacing_mut().item_spacing.x = 0.0;
                                            ui.allocate_ui(egui::vec2(lnum_w * 2.0 + 8.0, 16.0), |ui| {
                                                ui.label(egui::RichText::new("Line #").weak().size(11.0));
                                            });
                                            ui.label(egui::RichText::new("Unified Changes").strong().size(11.0));
                                        });
                                    });

                                // Unified View ScrollArea
                                egui::ScrollArea::both()
                                    .id_salt("git_diff_unified_scroll")
                                    .auto_shrink([false, false])
                                    .show_rows(ui, row_h, self.diff_rows.len(), |ui, row_range| {
                                        for r_idx in row_range {
                                            let row = &self.diff_rows[r_idx];
                                            let (bg, txt_color, prefix, text) = match row.kind {
                                                DiffKind::Added    => (added_bg,    added_txt,    "+ ", &row.right_text),
                                                DiffKind::Deleted  => (deleted_bg,  deleted_txt,  "- ", &row.left_text),
                                                DiffKind::Modified => (modified_bg, modified_txt, "~ ", if !row.right_text.is_empty() { &row.right_text } else { &row.left_text }),
                                                DiffKind::Conflict => (dark(80, 60, 20), dark(255, 200, 100), "! ", &row.right_text),
                                                DiffKind::Equal    => (egui::Color32::TRANSPARENT, normal_txt, "  ", &row.left_text),
                                            };

                                            ui.push_id(r_idx, |ui| {
                                                egui::Frame::default().fill(bg).show(ui, |ui| {
                                                    ui.set_height(row_h);
                                                    ui.horizontal(|ui| {
                                                        ui.spacing_mut().item_spacing.x = 0.0;
                                                        let ln_l = row.left_line_num.map(|n| format!("{:>4} ", n)).unwrap_or_else(|| "     ".into());
                                                        let ln_r = row.right_line_num.map(|n| format!("{:>4} ", n)).unwrap_or_else(|| "     ".into());

                                                        ui.allocate_ui(egui::vec2(lnum_w, row_h), |ui| {
                                                            ui.label(egui::RichText::new(&ln_l).size(11.0).color(dark(90, 93, 99)).family(egui::FontFamily::Monospace));
                                                        });
                                                        ui.allocate_ui(egui::vec2(lnum_w, row_h), |ui| {
                                                            ui.label(egui::RichText::new(&ln_r).size(11.0).color(dark(90, 93, 99)).family(egui::FontFamily::Monospace));
                                                        });
                                                        ui.add_space(8.0);
                                                        let full_str = format!("{}{}", prefix, text);
                                                        ui.add(egui::Label::new(
                                                            egui::RichText::new(&full_str).size(11.5).color(txt_color).family(egui::FontFamily::Monospace)
                                                        ).truncate());
                                                    });
                                                });
                                            });
                                        }
                                    });
                            } else {
                                // Side-by-Side Column Header (With Center Separator Column)
                                egui::Frame::default()
                                    .fill(hdr_bg)
                                    .inner_margin(egui::Margin::symmetric(0.0, 3.0))
                                    .stroke(egui::Stroke::new(1.0, border_c))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.spacing_mut().item_spacing.x = 0.0;
                                            ui.allocate_ui(egui::vec2(lnum_w + code_w, 16.0), |ui| {
                                                ui.horizontal(|ui| {
                                                    ui.add_space(8.0);
                                                    ui.label(egui::RichText::new("HEAD (Original version)").strong().size(11.0));
                                                });
                                            });
                                            // Center gutter header
                                            let (gutter_rect, _) = ui.allocate_exact_size(egui::vec2(gutter_w, 16.0), egui::Sense::hover());
                                            ui.painter().rect_filled(gutter_rect, 0.0, gutter_bg);
                                            ui.painter().line_segment(
                                                [gutter_rect.left_top(), gutter_rect.left_bottom()],
                                                egui::Stroke::new(1.0, border_c),
                                            );
                                            ui.painter().line_segment(
                                                [gutter_rect.right_top(), gutter_rect.right_bottom()],
                                                egui::Stroke::new(1.0, border_c),
                                            );
                                            ui.allocate_ui(egui::vec2(lnum_w + code_w, 16.0), |ui| {
                                                ui.horizontal(|ui| {
                                                    ui.add_space(8.0);
                                                    ui.label(egui::RichText::new("Working Copy (Current version)").strong().size(11.0));
                                                });
                                            });
                                        });
                                    });

                                // Side-by-Side ScrollArea
                                egui::ScrollArea::both()
                                    .id_salt("git_diff_side_by_side_scroll")
                                    .auto_shrink([false, false])
                                    .show_rows(ui, row_h, self.diff_rows.len(), |ui, row_range| {
                                        ui.set_min_width(total_w);

                                        let left_panel_w = lnum_w + code_w;
                                        let right_panel_w = lnum_w + code_w;
                                        let padding_bg = if is_dark { dark(40, 40, 44) } else { dark(235, 235, 240) };

                                        for r_idx in row_range {
                                            let row = &self.diff_rows[r_idx];

                                            // Determine per-panel colors
                                            let (left_bg, right_bg, left_txt, right_txt) = match row.kind {
                                                DiffKind::Added    => (padding_bg,   added_bg,    dark(90, 93, 99), added_txt),
                                                DiffKind::Deleted  => (deleted_bg,   padding_bg,  deleted_txt,      dark(90, 93, 99)),
                                                DiffKind::Modified => (modified_bg,  modified_bg, modified_txt,     modified_txt),
                                                DiffKind::Conflict => (dark(80,60,20), dark(80,60,20), dark(255,200,100), dark(255,200,100)),
                                                DiffKind::Equal    => (egui::Color32::TRANSPARENT, egui::Color32::TRANSPARENT, normal_txt, normal_txt),
                                            };

                                            ui.push_id(r_idx, |ui| {
                                                // Allocate the full row as one horizontal strip
                                                let (row_rect, _) = ui.allocate_exact_size(
                                                    egui::vec2(total_w, row_h),
                                                    egui::Sense::hover(),
                                                );

                                                let painter = ui.painter_at(row_rect);

                                                // --- Paint backgrounds ---
                                                // Left panel bg
                                                let left_rect = egui::Rect::from_min_size(
                                                    row_rect.min,
                                                    egui::vec2(left_panel_w, row_h),
                                                );
                                                if left_bg != egui::Color32::TRANSPARENT {
                                                    painter.rect_filled(left_rect, 0.0, left_bg);
                                                }

                                                // Center gutter bg
                                                let gutter_rect = egui::Rect::from_min_size(
                                                    egui::pos2(row_rect.min.x + left_panel_w, row_rect.min.y),
                                                    egui::vec2(gutter_w, row_h),
                                                );
                                                painter.rect_filled(gutter_rect, 0.0, gutter_bg);
                                                // Gutter vertical border lines
                                                painter.line_segment(
                                                    [gutter_rect.left_top(), gutter_rect.left_bottom()],
                                                    egui::Stroke::new(1.0, border_c),
                                                );
                                                painter.line_segment(
                                                    [gutter_rect.right_top(), gutter_rect.right_bottom()],
                                                    egui::Stroke::new(1.0, border_c),
                                                );

                                                // Right panel bg
                                                let right_rect = egui::Rect::from_min_size(
                                                    egui::pos2(row_rect.min.x + left_panel_w + gutter_w, row_rect.min.y),
                                                    egui::vec2(right_panel_w, row_h),
                                                );
                                                if right_bg != egui::Color32::TRANSPARENT {
                                                    painter.rect_filled(right_rect, 0.0, right_bg);
                                                }

                                                // --- Paint text using painter (pixel-exact positioning) ---
                                                let font = egui::FontId::monospace(11.0);
                                                let code_font = egui::FontId::monospace(11.5);
                                                let lnum_color = dark(90, 93, 99);
                                                let text_y = row_rect.min.y + (row_h - 11.5) / 2.0;

                                                // 1. Left line number
                                                let ln_l = row.left_line_num
                                                    .map(|n| format!("{:>4} ", n))
                                                    .unwrap_or_else(|| "     ".into());
                                                painter.text(
                                                    egui::pos2(left_rect.min.x + 2.0, text_y),
                                                    egui::Align2::LEFT_TOP,
                                                    &ln_l,
                                                    font.clone(),
                                                    lnum_color,
                                                );

                                                // 2. Left code text (clipped)
                                                let left_code_rect = egui::Rect::from_min_size(
                                                    egui::pos2(left_rect.min.x + lnum_w, row_rect.min.y),
                                                    egui::vec2(code_w, row_h),
                                                );
                                                let left_code_painter = painter.with_clip_rect(left_code_rect);
                                                left_code_painter.text(
                                                    egui::pos2(left_code_rect.min.x + 4.0, text_y),
                                                    egui::Align2::LEFT_TOP,
                                                    &row.left_text,
                                                    code_font.clone(),
                                                    left_txt,
                                                );

                                                // 3. Center gutter - apply button (only at hunk start)
                                                if let Some(h_idx) = row.hunk_idx {
                                                    let is_start = self.diff_hunks.iter()
                                                        .any(|h| h.hunk_idx == h_idx && h.start_row == r_idx);
                                                    if is_start {
                                                        let btn_rect = egui::Rect::from_center_size(
                                                            gutter_rect.center(),
                                                            egui::vec2(gutter_w - 4.0, row_h - 2.0),
                                                        );
                                                        let btn_response = ui.interact(btn_rect, ui.id().with(("gutter_btn", r_idx)), egui::Sense::click());
                                                        if btn_response.hovered() {
                                                            painter.rect_filled(btn_rect, 2.0, dark(60, 63, 70));
                                                        }
                                                        painter.text(
                                                            gutter_rect.center(),
                                                            egui::Align2::CENTER_CENTER,
                                                            ">>",
                                                            egui::FontId::proportional(10.0),
                                                            dark(53, 116, 240),
                                                        );
                                                        if btn_response.clicked() {
                                                            hunk_to_apply = Some(h_idx);
                                                        }
                                                    }
                                                }

                                                // 4. Right line number
                                                let ln_r = row.right_line_num
                                                    .map(|n| format!("{:>4} ", n))
                                                    .unwrap_or_else(|| "     ".into());
                                                painter.text(
                                                    egui::pos2(right_rect.min.x + 2.0, text_y),
                                                    egui::Align2::LEFT_TOP,
                                                    &ln_r,
                                                    font.clone(),
                                                    lnum_color,
                                                );

                                                // 5. Right code text (clipped)
                                                let right_code_rect = egui::Rect::from_min_size(
                                                    egui::pos2(right_rect.min.x + lnum_w, row_rect.min.y),
                                                    egui::vec2(code_w, row_h),
                                                );
                                                let right_code_painter = painter.with_clip_rect(right_code_rect);
                                                right_code_painter.text(
                                                    egui::pos2(right_code_rect.min.x + 4.0, text_y),
                                                    egui::Align2::LEFT_TOP,
                                                    &row.right_text,
                                                    code_font.clone(),
                                                    right_txt,
                                                );
                                            });
                                        }
                                    });
                            }

                            if let Some(h_idx) = hunk_to_apply {
                                if let Some(sel) = &self.selected_file_path {
                                    let full_path = std::path::Path::new(&self.repo_path).join(sel);
                                    let saved = std::fs::read(&full_path).unwrap_or_default();
                                    undo.push(crate::ui::undo_manager::UndoAction::GitRevertHunk { 
                                        repo_path: self.repo_path.clone(), 
                                        rel_path: sel.clone(),
                                        saved_content: saved,
                                    }, format!("Revert hunk in: {}", sel));
                                }
                                self.revert_hunk(h_idx);
                            }
                        });
                    });
            });
        });

        None
    }

    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn as_any_mut(&mut self) -> Option<&mut dyn std::any::Any> {
        Some(self)
    }
}

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

    #[test]
    fn test_compute_side_by_side_diff_empty_files() {
        let left = "";
        let right = "new line 1\nnew line 2";
        let (rows, hunks) = compute_side_by_side_diff(left, right);

        assert_eq!(rows.len(), 2);
        assert_eq!(hunks.len(), 1);
        assert_eq!(rows[0].kind, DiffKind::Added);
        assert_eq!(rows[1].kind, DiffKind::Added);
    }

    #[test]
    fn test_compute_side_by_side_diff_deleted_file() {
        let left = "old line 1\nold line 2";
        let right = "";
        let (rows, hunks) = compute_side_by_side_diff(left, right);

        assert_eq!(rows.len(), 2);
        assert_eq!(hunks.len(), 1);
        assert_eq!(rows[0].kind, DiffKind::Deleted);
        assert_eq!(rows[1].kind, DiffKind::Deleted);
    }

    #[test]
    fn test_padding_rows_alignment() {
        let left = "line 1\nold 1\nold 2\nline 4";
        let right = "line 1\nnew 1\nnew 2\nnew 3\nline 4";
        let (rows, hunks) = compute_side_by_side_diff(left, right);

        assert_eq!(hunks.len(), 1);
        // Deleted lines on left have None for right_line_num
        let deleted_rows: Vec<&DiffRow> = rows.iter().filter(|r| r.kind == DiffKind::Deleted).collect();
        assert_eq!(deleted_rows.len(), 2);
        for d in &deleted_rows {
            assert!(d.left_line_num.is_some());
            assert!(d.right_line_num.is_none());
        }

        // Added lines on right have None for left_line_num
        let added_rows: Vec<&DiffRow> = rows.iter().filter(|r| r.kind == DiffKind::Added).collect();
        assert_eq!(added_rows.len(), 3);
        for a in &added_rows {
            assert!(a.left_line_num.is_none());
            assert!(a.right_line_num.is_some());
        }

        // Equal lines at start and end match line numbers
        assert_eq!(rows[0].kind, DiffKind::Equal);
        assert_eq!(rows[0].left_line_num, Some(1));
        assert_eq!(rows[0].right_line_num, Some(1));

        let last = rows.last().unwrap();
        assert_eq!(last.kind, DiffKind::Equal);
        assert_eq!(last.left_line_num, Some(4));
        assert_eq!(last.right_line_num, Some(5));
    }
}


