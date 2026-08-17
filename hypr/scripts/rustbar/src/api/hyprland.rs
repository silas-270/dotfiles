//! Hyprland & Sway Workspace IPC API

use std::io::{BufRead, BufReader};
use std::os::unix::net::UnixStream;
use std::process::Command;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct WorkspaceState {
    pub active_id: i32,
    pub workspaces: Vec<i32>,
}

impl Default for WorkspaceState {
    fn default() -> Self {
        Self {
            active_id: 1,
            workspaces: vec![1],
        }
    }
}

pub fn get_workspace_state() -> WorkspaceState {
    let mut state = WorkspaceState::default();
    let is_sway = std::env::var("SWAYSOCK").is_ok();

    if is_sway {
        if let Ok(output) = Command::new("swaymsg").args(["-t", "get_workspaces"]).output() {
            if let Ok(json) = serde_json::from_slice::<Value>(&output.stdout) {
                if let Some(arr) = json.as_array() {
                    let mut ids = vec![];
                    for w in arr {
                        if let Some(id) = w.get("num").and_then(|v| v.as_i64()) {
                            ids.push(id as i32);
                            if w.get("focused").and_then(|v| v.as_bool()) == Some(true) {
                                state.active_id = id as i32;
                            }
                        }
                    }
                    ids.sort_unstable();
                    ids.dedup();
                    if !ids.is_empty() {
                        state.workspaces = ids;
                    }
                }
            }
        }
    } else {
        if let Ok(output) = Command::new("hyprctl").args(["-j", "activeworkspace"]).output() {
            if output.status.success() {
                if let Ok(json) = serde_json::from_slice::<Value>(&output.stdout) {
                    if let Some(id) = json.get("id").and_then(|v| v.as_i64()) {
                        state.active_id = id as i32;
                    }
                }
            }
        }
        if let Ok(output) = Command::new("hyprctl").args(["-j", "workspaces"]).output() {
            if output.status.success() {
                if let Ok(json) = serde_json::from_slice::<Value>(&output.stdout) {
                    if let Some(arr) = json.as_array() {
                        let mut ids: Vec<i32> = arr
                            .iter()
                            .filter_map(|w| w.get("id").and_then(|v| v.as_i64()).map(|id| id as i32))
                            .collect();
                        ids.sort_unstable();
                        ids.dedup();
                        if !ids.is_empty() {
                            state.workspaces = ids;
                        }
                    }
                }
            }
        }
    }

    if !state.workspaces.contains(&state.active_id) {
        state.workspaces.push(state.active_id);
        state.workspaces.sort_unstable();
    }
    state
}

pub fn switch_workspace(id: i32) {
    let is_sway = std::env::var("SWAYSOCK").is_ok();
    if is_sway {
        let _ = Command::new("swaymsg")
            .args(["workspace", "number", &id.to_string()])
            .status();
    } else {
        let _ = Command::new("hyprctl")
            .args(["dispatch", &format!("workspace {}", id)])
            .status();
    }
}

pub fn listen_hyprland_events(callback: impl Fn() + Send + 'static) {
    std::thread::spawn(move || {
        let is_sway = std::env::var("SWAYSOCK").is_ok();
        
        if is_sway {
            loop {
                if let Ok(mut child) = Command::new("swaymsg")
                    .args(["-t", "subscribe", "-m", "[\"workspace\"]"])
                    .stdout(std::process::Stdio::piped())
                    .spawn()
                {
                    if let Some(stdout) = child.stdout.take() {
                        let reader = BufReader::new(stdout);
                        for _line in reader.lines() {
                            callback();
                        }
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
        } else {
            let xdg = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/run/user/1000".to_string());
            let signature = match std::env::var("HYPRLAND_INSTANCE_SIGNATURE") {
                Ok(s) => s,
                Err(_) => return,
            };
            let socket_path = format!("{}/hypr/{}/.socket2.sock", xdg, signature);
            loop {
                if let Ok(stream) = UnixStream::connect(&socket_path) {
                    let reader = BufReader::new(stream);
                    for line in reader.lines() {
                        if let Ok(line_str) = line {
                            if line_str.starts_with("workspace>>")
                                || line_str.starts_with("focusedmon>>")
                                || line_str.starts_with("createworkspace>>")
                                || line_str.starts_with("destroyworkspace>>")
                                || line_str.starts_with("moveworkspace>>")
                            {
                                callback();
                            }
                        } else {
                            break;
                        }
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
        }
    });
}
