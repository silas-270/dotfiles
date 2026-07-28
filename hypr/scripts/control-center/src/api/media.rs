//! Media playback API module (MPRIS2 via playerctl)
//!
//! Provides transport controls (play/pause, next, previous, seek) and
//! metadata queries (title, artist, album, artwork, duration, position)
//! for any MPRIS2-compatible player (Spotify, Firefox, VLC, mpv, …).

use std::process::Command;

// ── Data types ──────────────────────────────────────────────────────────────

/// Current playback state of the active media player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackStatus {
    Playing,
    Paused,
    Stopped,
    /// No MPRIS2 player is running at all.
    None,
}

/// Metadata for the currently playing track.
#[derive(Debug, Clone)]
pub struct MediaMetadata {
    /// Track title (e.g. "Bohemian Rhapsody").
    pub title: String,
    /// Artist / band name (e.g. "Queen").
    pub artist: String,
    /// Album name (e.g. "A Night at the Opera").
    pub album: String,
    /// Cover art URL.  May be a `file://` path or `https://` URL,
    /// or empty if the player doesn't provide artwork.
    pub art_url: String,
    /// Total track length in seconds, or `0.0` if unavailable.
    pub length_secs: f64,
}

impl Default for MediaMetadata {
    fn default() -> Self {
        Self {
            title: String::new(),
            artist: String::new(),
            album: String::new(),
            art_url: String::new(),
            length_secs: 0.0,
        }
    }
}

// ── Queries (synchronous – meant to be called from background threads) ──────

/// Returns `true` if at least one MPRIS2 player is running.
pub fn has_player() -> bool {
    Command::new("playerctl")
        .arg("status")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Returns the playback status of the active player.
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

/// Returns metadata for the current track.
///
/// All fields fall back to sensible defaults when a player is not running
/// or doesn't expose the requested property.
pub fn get_metadata() -> MediaMetadata {
    let separator = "|||";
    let fmt = format!(
        "{{{{title}}}}{}{{{{artist}}}}{}{{{{album}}}}{}{{{{mpris:artUrl}}}}{}{{{{mpris:length}}}}{}{{{{url}}}}{}{{{{xesam:url}}}}{}{{{{playerName}}}}",
        separator, separator, separator, separator, separator, separator, separator
    );

    if let Ok(output) = Command::new("playerctl")
        .args(["metadata", "-f", &fmt])
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let parts: Vec<&str> = stdout.trim_end().split("|||").collect();
            if parts.len() >= 8 {
                let mut title = parts[0].trim().to_string();
                let mut artist = parts[1].trim().to_string();
                let album = parts[2].trim().to_string();
                let art_url = parts[3].trim().to_string();
                let length_us: f64 = parts[4].parse().unwrap_or(0.0);
                let raw_url = parts[5].trim();
                let xesam_url = parts[6].trim();
                let player_name = parts[7].trim();

                let effective_url = if !raw_url.is_empty() {
                    raw_url
                } else {
                    xesam_url
                };

                // If title is empty, derive title (and optionally artist) from the media URL / filename
                if title.is_empty() && !effective_url.is_empty() {
                    let decoded_path = url_decode(effective_url);
                    let filename = std::path::Path::new(&decoded_path)
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| decoded_path.clone());

                    if !filename.is_empty() {
                        // Try splitting "Artist - Title" if present
                        if let Some((a, t)) = filename.split_once(" - ") {
                            if artist.is_empty() {
                                artist = a.trim().to_string();
                            }
                            title = t.trim().to_string();
                        } else {
                            title = filename;
                        }
                    }
                }

                if title.is_empty() {
                    title = "Media Playing".to_string();
                }

                if artist.is_empty() {
                    if !player_name.is_empty() {
                        let mut chars = player_name.chars();
                        artist = match chars.next() {
                            None => String::new(),
                            Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
                        };
                    } else {
                        artist = "Unknown Artist".to_string();
                    }
                }

                return MediaMetadata {
                    title,
                    artist,
                    album,
                    art_url,
                    length_secs: length_us / 1_000_000.0,
                };
            }
        }
    }
    MediaMetadata::default()
}

/// Helper function to decode simple percent-encoded URLs (e.g. %20 -> space).
fn url_decode(input: &str) -> String {
    let s = input.trim_start_matches("file://");
    let mut result = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(val) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                result.push(val as char);
                i += 3;
                continue;
            }
        }
        result.push(bytes[i] as char);
        i += 1;
    }
    result
}

/// Returns the current playback position in seconds.
pub fn get_position() -> f64 {
    if let Ok(output) = Command::new("playerctl").arg("position").output() {
        if output.status.success() {
            if let Ok(pos) = String::from_utf8_lossy(&output.stdout).trim().parse::<f64>() {
                return pos;
            }
        }
    }
    0.0
}

// ── Combined query (single call for the sync loop) ──────────────────────────

/// Everything the UI needs in one struct, fetched in one go.
#[derive(Debug, Clone)]
pub struct MediaState {
    pub status: PlaybackStatus,
    pub metadata: MediaMetadata,
    pub position_secs: f64,
}

/// Fetches the full media state.  Designed to be called from a background
/// thread and sent to the UI via a channel, just like the other API modules.
pub fn get_media_state() -> MediaState {
    let status = get_status();
    if status == PlaybackStatus::None {
        return MediaState {
            status,
            metadata: MediaMetadata::default(),
            position_secs: 0.0,
        };
    }
    MediaState {
        status,
        metadata: get_metadata(),
        position_secs: get_position(),
    }
}

// ── Transport controls (fire-and-forget on a background thread) ─────────────

pub fn play_pause() {
    std::thread::spawn(|| {
        let _ = Command::new("playerctl").arg("play-pause").status();
    });
}

pub fn next() {
    std::thread::spawn(|| {
        let _ = Command::new("playerctl").arg("next").status();
    });
}

pub fn previous() {
    std::thread::spawn(|| {
        let _ = Command::new("playerctl").arg("previous").status();
    });
}

/// Seek to an absolute position (in seconds).
pub fn seek(seconds: f64) {
    std::thread::spawn(move || {
        let _ = Command::new("playerctl")
            .args(["position", &format!("{:.1}", seconds)])
            .status();
    });
}
