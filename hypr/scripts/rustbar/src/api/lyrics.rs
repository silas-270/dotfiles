//! Synced lyrics fetching via lrclib.net

use std::process::Command;

#[derive(Debug, Clone)]
pub struct LyricLine {
    pub time: f64,
    pub text: String,
}

#[derive(Debug, Clone)]
pub enum LyricsResult {
    Synced(Vec<LyricLine>),
    NotFound,
}

/// Parses an LRC-format synced lyrics blob ("[mm:ss.xx]text" per line) into LyricLine entries.
fn parse_lrc(lrc: &str) -> Vec<LyricLine> {
    let mut lines = Vec::new();
    for raw_line in lrc.lines() {
        let raw_line = raw_line.trim();
        if !raw_line.starts_with('[') {
            continue;
        }
        let Some(close_idx) = raw_line.find(']') else { continue };
        let tag = &raw_line[1..close_idx];
        let text = raw_line[close_idx + 1..].trim().to_string();

        // tag format: mm:ss.xx
        let Some((mm, rest)) = tag.split_once(':') else { continue };
        let Ok(minutes) = mm.parse::<f64>() else { continue };
        let Ok(seconds) = rest.parse::<f64>() else { continue };

        if text.is_empty() {
            continue;
        }

        lines.push(LyricLine {
            time: minutes * 60.0 + seconds,
            text,
        });
    }
    lines.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap_or(std::cmp::Ordering::Equal));
    lines
}

pub fn fetch_lyrics(artist: &str, title: &str, album: &str, duration_secs: u64) -> LyricsResult {
    if artist.is_empty() || title.is_empty() {
        return LyricsResult::NotFound;
    }

    let url = format!(
        "https://lrclib.net/api/get?track_name={}&artist_name={}&album_name={}&duration={}",
        urlencode(title),
        urlencode(artist),
        urlencode(album),
        duration_secs
    );

    let output = match Command::new("curl").args(["-s", "-m", "5", &url]).output() {
        Ok(o) if o.status.success() => o,
        _ => return LyricsResult::NotFound,
    };

    let body = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(_) => return LyricsResult::NotFound,
    };

    match json.get("syncedLyrics").and_then(|v| v.as_str()) {
        Some(lrc) if !lrc.trim().is_empty() => {
            let lines = parse_lrc(lrc);
            if lines.is_empty() {
                LyricsResult::NotFound
            } else {
                LyricsResult::Synced(lines)
            }
        }
        _ => LyricsResult::NotFound,
    }
}

fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

pub fn get_position_secs() -> Option<f64> {
    let output = Command::new("playerctl").arg("position").output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout).trim().parse::<f64>().ok()
}

/// Returns the index of the last line whose timestamp is <= pos.
pub fn current_line_index(lines: &[LyricLine], pos: f64) -> Option<usize> {
    if lines.is_empty() {
        return None;
    }
    let mut idx = None;
    for (i, line) in lines.iter().enumerate() {
        if line.time <= pos {
            idx = Some(i);
        } else {
            break;
        }
    }
    idx.or(Some(0))
}
