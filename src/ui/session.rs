use eframe::egui;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::ui::editor::EditorApp;
use crate::ui::file_viewer::FileViewerApp;
use crate::ui::settings::{AppConfig, SettingsApp};
use crate::ui::sftp_app::SftpApp;
use crate::ui::ssh_manager::SshManagerApp;
use crate::ui::terminal_app::{NotebookBlock, TerminalApp};
use crate::ui::window_framework::FloatingWindow;
use crate::ui::workspace::Workspace;

#[derive(Serialize, Deserialize)]
pub struct SavedWindow {
    pub id: String,
    pub custom_title: Option<String>,
    pub window_type: String,
    pub state: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize)]
pub struct SavedWorkspace {
    pub id: String,
    pub name: String,
    pub windows: Vec<SavedWindow>,
}

#[derive(Serialize, Deserialize)]
pub struct SessionData {
    pub workspaces: Vec<SavedWorkspace>,
    pub active_workspace_idx: usize,
    pub config: AppConfig,
}

pub fn get_home_dir() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        if !home.is_empty() {
            return PathBuf::from(home);
        }
    }
    if let Ok(profile) = std::env::var("USERPROFILE") {
        if !profile.is_empty() {
            return PathBuf::from(profile);
        }
    }
    if let Ok(appdata) = std::env::var("APPDATA") {
        if !appdata.is_empty() {
            return PathBuf::from(appdata);
        }
    }
    std::env::temp_dir()
}

pub fn get_session_file_path() -> PathBuf {
    let mut path = get_home_dir();
    if !cfg!(target_os = "windows") && path != std::env::temp_dir() {
        path.push(".config");
    }
    path.push("smart-term");
    let _ = fs::create_dir_all(&path);
    path.push("session.json");
    path
}

pub fn save_session(workspaces: &[Workspace], active_workspace_idx: usize, config: &AppConfig) {
    let saved_workspaces: Vec<SavedWorkspace> = workspaces
        .iter()
        .map(|ws| {
            let saved_windows: Vec<SavedWindow> = ws
                .windows
                .iter()
                .map(|win| SavedWindow {
                    id: win.id.clone(),
                    custom_title: win.custom_title.clone(),
                    window_type: win.app.window_type().to_string(),
                    state: win.app.save_state(),
                })
                .collect();

            SavedWorkspace {
                id: ws.id.clone(),
                name: ws.name.clone(),
                windows: saved_windows,
            }
        })
        .collect();

    let data = SessionData {
        workspaces: saved_workspaces,
        active_workspace_idx,
        config: config.clone(),
    };

    if let Ok(json) = serde_json::to_string_pretty(&data) {
        let path = get_session_file_path();
        let tmp_path = path.with_extension("json.tmp");
        if let Err(e) = fs::write(&tmp_path, json) {
            eprintln!("[SESSION ERROR] Failed to write session to {:?}: {:?}", tmp_path, e);
        } else if let Err(e) = fs::rename(&tmp_path, &path) {
            eprintln!("[SESSION ERROR] Failed to rename session file {:?}: {:?}", tmp_path, e);
        }
    }
}

pub fn load_session(ctx: &egui::Context) -> Option<(Vec<Workspace>, usize, AppConfig)> {
    let path = get_session_file_path();
    if !path.exists() {
        eprintln!("[SESSION] No session file found at {:?}", path);
        return None;
    }

    let content = fs::read_to_string(&path).ok()?;
    let data: SessionData = match serde_json::from_str(&content) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[SESSION ERROR] Failed to parse session file {:?}: {:?}", path, e);
            return None;
        }
    };

    if data.workspaces.is_empty() {
        return None;
    }

    eprintln!("[SESSION] Successfully loaded session file {:?} with {} workspaces", path, data.workspaces.len());

    let workspaces: Vec<Workspace> = data
        .workspaces
        .into_iter()
        .map(|sws| {
            let windows: Vec<FloatingWindow> = sws
                .windows
                .into_iter()
                .filter_map(|swin| {
                    let app: Box<dyn crate::ui::window_framework::WindowApp> = match swin.window_type.as_str() {
                        "terminal" => {
                            let title = swin
                                .state
                                .as_ref()
                                .and_then(|s| s.get("title"))
                                .and_then(|t| t.as_str())
                                .unwrap_or("zsh");
                            let mut term = TerminalApp::new_local(title, ctx);
                            if let Some(history) = swin
                                .state
                                .as_ref()
                                .and_then(|s| s.get("command_history"))
                                .and_then(|h| h.as_array())
                            {
                                term.command_history = history
                                    .iter()
                                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                    .collect();
                            }
                            if let Some(blocks_json) = swin
                                .state
                                .as_ref()
                                .and_then(|s| s.get("blocks"))
                                .and_then(|b| b.as_array())
                            {
                                use base64::Engine;
                                for b_val in blocks_json {
                                    let cmd = b_val.get("command").and_then(|c| c.as_str()).unwrap_or("");
                                    let raw_str = b_val.get("raw_output").and_then(|r| r.as_str()).unwrap_or("");
                                    let is_comp = b_val.get("is_complete").and_then(|ic| ic.as_bool()).unwrap_or(true);
                                    let is_clear = b_val.get("is_clear_marker").and_then(|cm| cm.as_bool()).unwrap_or(false);

                                    let raw_bytes = base64::engine::general_purpose::STANDARD
                                        .decode(raw_str)
                                        .unwrap_or_else(|_| raw_str.as_bytes().to_vec());

                                    let mut block = NotebookBlock::new(cmd.to_string());
                                    block.raw_output = raw_bytes;
                                    block.is_complete = is_comp;
                                    block.is_clear_marker = is_clear;
                                    term.blocks.push(block);
                                }
                            }
                            Box::new(term)
                        }
                        "editor" => {
                            let path_str = swin
                                .state
                                .as_ref()
                                .and_then(|s| s.get("path"))
                                .and_then(|p| p.as_str())
                                .unwrap_or("Untitled.txt");
                            let text = swin
                                .state
                                .as_ref()
                                .and_then(|s| s.get("content"))
                                .and_then(|c| c.as_str())
                                .unwrap_or("");

                            if !path_str.is_empty() && std::path::Path::new(path_str).exists() {
                                if let Ok(mut ed) = EditorApp::open(path_str) {
                                    if ed.content != text {
                                        ed.content = text.to_string();
                                        ed.is_dirty = true;
                                    }
                                    Box::new(ed)
                                } else {
                                    let mut ed = EditorApp::new_untitled_with_content(path_str, text);
                                    ed.is_dirty = !text.is_empty();
                                    Box::new(ed)
                                }
                            } else {
                                let mut ed = EditorApp::new_untitled_with_content(path_str, text);
                                ed.is_dirty = !text.is_empty();
                                Box::new(ed)
                            }
                        }
                        "file_viewer" => {
                            let mut viewer = FileViewerApp::new();
                            if let Some(root) = swin
                                .state
                                .as_ref()
                                .and_then(|s| s.get("root_path"))
                                .and_then(|r| r.as_str())
                            {
                                let path = std::path::PathBuf::from(root);
                                if path.exists() {
                                    viewer.root_path = path.clone();
                                    viewer.path_history = vec![path];
                                    viewer.history_idx = 0;
                                }
                            }
                            Box::new(viewer)
                        }
                        "ssh_manager" => Box::new(SshManagerApp::new()),
                        "sftp" => {
                            let host = swin
                                .state
                                .as_ref()
                                .and_then(|s| s.get("host"))
                                .and_then(|h| h.as_str())
                                .map(|s| s.to_string());
                            if let Some(h) = host {
                                Box::new(SftpApp::with_host(h))
                            } else {
                                Box::new(SftpApp::new())
                            }
                        }
                        "settings" => Box::new(SettingsApp),
                        _ => return None,
                    };

                    let mut fw = FloatingWindow::new(swin.id, app);
                    fw.custom_title = swin.custom_title;
                    Some(fw)
                })
                .collect();

            Workspace {
                id: sws.id,
                name: sws.name,
                windows,
                is_editing_name: false,
                closed_windows_stack: Vec::new(),
            }
        })
        .collect();

    if workspaces.is_empty() {
        return None;
    }

    let active_idx = if data.active_workspace_idx < workspaces.len() {
        data.active_workspace_idx
    } else {
        0
    };

    Some((workspaces, active_idx, data.config))
}
