use std::time::Instant;

/// Maximum default undo stack size
pub const DEFAULT_UNDO_STACK_SIZE: usize = 50;

#[derive(Clone, Debug)]
pub enum UndoAction {
    // Git Operations
    GitStageFile { repo_path: String, rel_path: String },
    GitUnstageFile { repo_path: String, rel_path: String },
    GitStageAll { repo_path: String },
    GitUnstageAll { repo_path: String },
    GitRevertFile { repo_path: String, rel_path: String, saved_content: Vec<u8> },
    GitDeleteFile { repo_path: String, rel_path: String, saved_content: Vec<u8>, was_dir: bool },
    GitRevertHunk { repo_path: String, rel_path: String, saved_content: Vec<u8> },
    GitCommit { repo_path: String, was_amend: bool, prev_head: Option<String> },
    GitSwitchBranch { repo_path: String, previous_branch: String },
    GitCreateBranch { repo_path: String, branch_name: String, previous_branch: String },

    // Editor Operations
    EditorSave { file_path: String, previous_content: String },
    EditorFormat { window_id: String, previous_content: String },
    EditorReplace { window_id: String, previous_content: String },
    EditorReplaceAll { window_id: String, previous_content: String },

    // Settings
    SettingsChange { previous_config: Box<crate::ui::settings::AppConfig> },

    // Workspace
    WorkspaceRename { ws_index: usize, previous_name: String },
    // WorkspaceClose { ws_index: usize, workspace: Box<crate::ui::workspace::Workspace> },
}

#[derive(Clone, Debug)]
pub struct UndoEntry {
    pub action: UndoAction,
    pub description: String,
    pub timestamp: Instant,
}

pub struct UndoManager {
    undo_stack: Vec<UndoEntry>,
    redo_stack: Vec<UndoEntry>,
    max_stack_size: usize,
    /// Toast message to display, with timestamp when it was set
    pub toast: Option<(String, Instant)>,
}

impl UndoManager {
    pub fn new(max_stack_size: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_stack_size,
            toast: None,
        }
    }

    pub fn push(&mut self, action: UndoAction, description: impl Into<String>) {
        self.undo_stack.push(UndoEntry {
            action,
            description: description.into(),
            timestamp: Instant::now(),
        });
        self.redo_stack.clear();
        while self.undo_stack.len() > self.max_stack_size {
            self.undo_stack.remove(0);
        }
    }

    pub fn undo(&mut self) -> Option<UndoEntry> {
        if let Some(entry) = self.undo_stack.pop() {
            self.redo_stack.push(entry.clone());
            Some(entry)
        } else {
            None
        }
    }

    pub fn redo(&mut self) -> Option<UndoEntry> {
        if let Some(entry) = self.redo_stack.pop() {
            self.undo_stack.push(entry.clone());
            Some(entry)
        } else {
            None
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn last_undo_description(&self) -> Option<&str> {
        self.undo_stack.last().map(|e| e.description.as_str())
    }

    pub fn last_redo_description(&self) -> Option<&str> {
        self.redo_stack.last().map(|e| e.description.as_str())
    }

    pub fn set_max_stack_size(&mut self, size: usize) {
        self.max_stack_size = size;
        while self.undo_stack.len() > self.max_stack_size {
            self.undo_stack.remove(0);
        }
    }

    pub fn show_toast(&mut self, msg: impl Into<String>) {
        self.toast = Some((msg.into(), Instant::now()));
    }

    pub fn take_expired_toast(&mut self, duration_secs: f32) -> bool {
        if let Some((_, timestamp)) = &self.toast {
            if timestamp.elapsed().as_secs_f32() > duration_secs {
                self.toast = None;
                false
            } else {
                true
            }
        } else {
            false
        }
    }
}

impl Default for UndoManager {
    fn default() -> Self {
        Self::new(DEFAULT_UNDO_STACK_SIZE)
    }
}

/// Execute an undo action and return the reverse action for redo.
/// Returns (reverse_action, description) or None if execution failed.
pub fn execute_undo_action(action: &UndoAction) -> Option<(UndoAction, String)> {
    match action {
        UndoAction::GitStageFile { repo_path, rel_path } => {
            let output = std::process::Command::new("git")
                .args(["-C", repo_path, "restore", "--staged", rel_path])
                .output().ok()?;
            if output.status.success() {
                Some((UndoAction::GitUnstageFile { repo_path: repo_path.clone(), rel_path: rel_path.clone() },
                      format!("Unstage file: {}", rel_path)))
            } else { None }
        },
        UndoAction::GitUnstageFile { repo_path, rel_path } => {
            let output = std::process::Command::new("git")
                .args(["-C", repo_path, "add", rel_path])
                .output().ok()?;
            if output.status.success() {
                Some((UndoAction::GitStageFile { repo_path: repo_path.clone(), rel_path: rel_path.clone() },
                      format!("Stage file: {}", rel_path)))
            } else { None }
        },
        UndoAction::GitStageAll { repo_path } => {
            let output = std::process::Command::new("git")
                .args(["-C", repo_path, "restore", "--staged", "."])
                .output().ok()?;
            if output.status.success() {
                Some((UndoAction::GitUnstageAll { repo_path: repo_path.clone() },
                      "Unstage all files".to_string()))
            } else { None }
        },
        UndoAction::GitUnstageAll { repo_path } => {
            let output = std::process::Command::new("git")
                .args(["-C", repo_path, "add", "-A"])
                .output().ok()?;
            if output.status.success() {
                Some((UndoAction::GitStageAll { repo_path: repo_path.clone() },
                      "Stage all files".to_string()))
            } else { None }
        },
        UndoAction::GitRevertFile { repo_path, rel_path, saved_content } => {
            let full_path = std::path::Path::new(repo_path).join(rel_path);
            let current_content = std::fs::read(&full_path).unwrap_or_default();
            std::fs::write(&full_path, saved_content).ok()?;
            Some((UndoAction::GitRevertFile { repo_path: repo_path.clone(), rel_path: rel_path.clone(), saved_content: current_content },
                  format!("Revert file: {}", rel_path)))
        },
        UndoAction::GitDeleteFile { repo_path, rel_path, saved_content, was_dir: _ } => {
            let full_path = std::path::Path::new(repo_path).join(rel_path);
            if let Some(parent) = full_path.parent() {
                std::fs::create_dir_all(parent).ok()?;
            }
            std::fs::write(&full_path, saved_content).ok()?;
            let _ = std::process::Command::new("git")
                .args(["-C", repo_path, "add", rel_path])
                .output();
            Some((UndoAction::GitDeleteFile { repo_path: repo_path.clone(), rel_path: rel_path.clone(), saved_content: Vec::new(), was_dir: false },
                  format!("Restore file: {}", rel_path)))
        },
        UndoAction::GitRevertHunk { repo_path, rel_path, saved_content } => {
            let full_path = std::path::Path::new(repo_path).join(rel_path);
            let current_content = std::fs::read(&full_path).unwrap_or_default();
            std::fs::write(&full_path, saved_content).ok()?;
            Some((UndoAction::GitRevertHunk { repo_path: repo_path.clone(), rel_path: rel_path.clone(), saved_content: current_content },
                  format!("Revert hunk in: {}", rel_path)))
        },
        UndoAction::GitCommit { repo_path, was_amend: _, prev_head: _ } => {
            let output = std::process::Command::new("git")
                .args(["-C", repo_path, "reset", "--soft", "HEAD~1"])
                .output().ok()?;
            if output.status.success() {
                Some((UndoAction::GitCommit { repo_path: repo_path.clone(), was_amend: false, prev_head: None },
                      "Undo commit".to_string()))
            } else { None }
        },
        UndoAction::GitSwitchBranch { repo_path, previous_branch } => {
            let current_branch_output = std::process::Command::new("git")
                .args(["-C", repo_path, "branch", "--show-current"])
                .output().ok()?;
            let current_branch = String::from_utf8_lossy(&current_branch_output.stdout).trim().to_string();
            
            let output = std::process::Command::new("git")
                .args(["-C", repo_path, "checkout", previous_branch])
                .output().ok()?;
            if output.status.success() {
                Some((UndoAction::GitSwitchBranch { repo_path: repo_path.clone(), previous_branch: current_branch },
                      format!("Switch branch to {}", previous_branch)))
            } else { None }
        },
        UndoAction::GitCreateBranch { repo_path, branch_name, previous_branch } => {
            let _ = std::process::Command::new("git")
                .args(["-C", repo_path, "checkout", previous_branch])
                .output();
            let output = std::process::Command::new("git")
                .args(["-C", repo_path, "branch", "-d", branch_name])
                .output().ok()?;
            if output.status.success() {
                Some((UndoAction::GitCreateBranch { repo_path: repo_path.clone(), branch_name: branch_name.clone(), previous_branch: previous_branch.clone() },
                      format!("Undo branch creation: {}", branch_name)))
            } else { None }
        },
        UndoAction::EditorSave { file_path, previous_content } => {
            let current_content = std::fs::read_to_string(file_path).unwrap_or_default();
            std::fs::write(file_path, previous_content).ok()?;
            Some((UndoAction::EditorSave { file_path: file_path.clone(), previous_content: current_content },
                  format!("Undo save: {}", file_path)))
        },
        UndoAction::EditorFormat { window_id, previous_content } => {
            Some((UndoAction::EditorFormat { window_id: window_id.clone(), previous_content: previous_content.clone() },
                  "Undo format".to_string()))
        },
        UndoAction::EditorReplace { window_id, previous_content } => {
            Some((UndoAction::EditorReplace { window_id: window_id.clone(), previous_content: previous_content.clone() },
                  "Undo replace".to_string()))
        },
        UndoAction::EditorReplaceAll { window_id, previous_content } => {
            Some((UndoAction::EditorReplaceAll { window_id: window_id.clone(), previous_content: previous_content.clone() },
                  "Undo replace all".to_string()))
        },
        UndoAction::SettingsChange { previous_config } => {
            Some((UndoAction::SettingsChange { previous_config: previous_config.clone() },
                  "Undo settings change".to_string()))
        },
        UndoAction::WorkspaceRename { ws_index, previous_name } => {
            Some((UndoAction::WorkspaceRename { ws_index: *ws_index, previous_name: previous_name.clone() },
                  "Undo workspace rename".to_string()))
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_push_and_undo() {
        let mut um = UndoManager::new(10);
        um.push(UndoAction::WorkspaceRename { ws_index: 0, previous_name: "test".to_string() }, "rename");
        assert!(um.can_undo());
        let undone = um.undo().unwrap();
        assert_eq!(undone.description, "rename");
        assert!(!um.can_undo());
        assert!(um.can_redo());
    }
    
    #[test]
    fn test_redo() {
        let mut um = UndoManager::new(10);
        um.push(UndoAction::WorkspaceRename { ws_index: 0, previous_name: "test".to_string() }, "rename");
        um.undo();
        let redone = um.redo().unwrap();
        assert_eq!(redone.description, "rename");
        assert!(um.can_undo());
        assert!(!um.can_redo());
    }
    
    #[test]
    fn test_redo_cleared_on_new_push() {
        let mut um = UndoManager::new(10);
        um.push(UndoAction::WorkspaceRename { ws_index: 0, previous_name: "test".to_string() }, "rename1");
        um.undo();
        assert!(um.can_redo());
        um.push(UndoAction::WorkspaceRename { ws_index: 0, previous_name: "test2".to_string() }, "rename2");
        assert!(!um.can_redo());
    }
    
    #[test]
    fn test_stack_overflow_trims_oldest() {
        let mut um = UndoManager::new(2);
        um.push(UndoAction::WorkspaceRename { ws_index: 0, previous_name: "1".to_string() }, "1");
        um.push(UndoAction::WorkspaceRename { ws_index: 0, previous_name: "2".to_string() }, "2");
        um.push(UndoAction::WorkspaceRename { ws_index: 0, previous_name: "3".to_string() }, "3");
        assert_eq!(um.undo_stack.len(), 2);
        assert_eq!(um.undo_stack[0].description, "2");
        assert_eq!(um.undo_stack[1].description, "3");
    }
    
    #[test]
    fn test_can_undo_redo() {
        let mut um = UndoManager::new(10);
        assert!(!um.can_undo());
        assert!(!um.can_redo());
        um.push(UndoAction::WorkspaceRename { ws_index: 0, previous_name: "1".to_string() }, "1");
        assert!(um.can_undo());
        assert!(!um.can_redo());
    }
    
    #[test]
    fn test_toast() {
        let mut um = UndoManager::new(10);
        um.show_toast("test");
        assert!(um.take_expired_toast(10.0));
        std::thread::sleep(Duration::from_millis(50));
        assert!(!um.take_expired_toast(0.01));
        assert!(um.toast.is_none());
    }
}
