use eframe::egui;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::ui::editor::EditorApp;
use crate::ui::file_viewer::FileViewerApp;
use crate::ui::settings::{AppConfig, SettingsApp};
use crate::ui::sftp_app::SftpApp;
use crate::ui::ssh_manager::SshManagerApp;
use crate::ui::terminal_app::TerminalApp;
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

pub fn get_session_file_path() -> PathBuf {
    let mut path = if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".config")
    } else {
        std::env::temp_dir()
    };
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
        let _ = fs::write(path, json);
    }
}

pub fn load_session(ctx: &egui::Context) -> Option<(Vec<Workspace>, usize, AppConfig)> {
    let path = get_session_file_path();
    if !path.exists() {
        return None;
    }

    let content = fs::read_to_string(path).ok()?;
    let data: SessionData = serde_json::from_str(&content).ok()?;

    if data.workspaces.is_empty() {
        return None;
    }

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
                                if let Ok(ed) = EditorApp::open(path_str) {
                                    Box::new(ed)
                                } else {
                                    Box::new(EditorApp::new_untitled_with_content(path_str, text))
                                }
                            } else {
                                Box::new(EditorApp::new_untitled_with_content(path_str, text))
                            }
                        }
                        "file_viewer" => Box::new(FileViewerApp::new()),
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
