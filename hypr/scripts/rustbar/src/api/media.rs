//! Media API module for centerpiece display

use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackStatus {
    Playing,
    Paused,
    Stopped,
    None,
}

pub fn get_status() -> PlaybackStatus {
    if let Ok(output) = Command::new("playerctl").arg("status").output() {
        if output.status.success() {
            return match String::from_utf8_lossy(&output.stdout).trim() {
                "Playing" => PlaybackStatus::Playing,
                "Paused" => PlaybackStatus::Paused,
                "Stopped" => PlaybackStatus::Stopped,
                _ => PlaybackStatus::None,
            };
        }
    }
    PlaybackStatus::None
}

pub fn get_centerpiece_text() -> (String, bool) {
    let status = get_status();
    if status == PlaybackStatus::Playing || status == PlaybackStatus::Paused {
        let fmt = "{{artist}}|||{{title}}";
        if let Ok(output) = Command::new("playerctl").args(["metadata", "-f", fmt]).output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let parts: Vec<&str> = stdout.trim().split("|||").collect();
                let artist = parts.get(0).unwrap_or(&"").trim();
                let title = parts.get(1).unwrap_or(&"").trim();

                let track = if !artist.is_empty() && !title.is_empty() {
                    format!("{} - {}", artist, title)
                } else if !title.is_empty() {
                    title.to_string()
                } else {
                    String::new()
                };

                if !track.is_empty() {
                    let truncated = if track.chars().count() > 32 {
                        format!("{}...", track.chars().take(30).collect::<String>())
                    } else {
                        track
                    };

                    let icon = if status == PlaybackStatus::Paused { "󰏤" } else { "󰎈" };
                    return (format!("{} {}", icon, truncated), status == PlaybackStatus::Playing);
                }
            }
        }
    }

    ("I use Arch btw".to_string(), false)
}
