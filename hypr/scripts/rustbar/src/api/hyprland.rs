//! Hyprland Workspace IPC API

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

/// Fetch current workspace state via `hyprctl -j activeworkspace` & `hyprctl -j workspaces`
pub fn get_workspace_state() -> WorkspaceState {
    let mut state = WorkspaceState::default();

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

    // Ensure active workspace is in the workspace list
    if !state.workspaces.contains(&state.active_id) {
        state.workspaces.push(state.active_id);
        state.workspaces.sort_unstable();
    }

    state
}

/// Switch to a workspace by ID
pub fn switch_workspace(id: i32) {
    let _ = Command::new("hyprctl")
        .args(["dispatch", &format!("workspace {}", id)])
        .status();
}

/// Start Hyprland socket2 event listener thread
pub fn listen_hyprland_events(callback: impl Fn() + Send + 'static) {
    std::thread::spawn(move || {
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
    });
}
