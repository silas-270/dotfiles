//! Media playback API (MPRIS2 via playerctl), shared by both surfaces.
//!
//! Provides transport controls (play/pause, next, previous, seek) and metadata
//! queries (title, artist, album, duration, position) for any MPRIS2-compatible
//! player (Spotify, Firefox, VLC, mpv, …).
//!
//! Every query shells out to `playerctl`, which costs roughly 12ms per spawn.
//! rustbar's lyrics mode polls for track changes every 33ms, so the queries are
//! memoized behind a short TTL: callers can poll as fast as they like without
//! forking more than a few times a second. The cache is short enough
//! (`CACHE_TTL`) to stay below control-center's 500ms sync interval, so it never
//! makes the panel feel stale.

use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// How long a query result stays fresh. Must remain well under
/// control-center's 500ms sync tick.
const CACHE_TTL: Duration = Duration::from_millis(250);

/// Stand-in title when a player is running but reports neither a title nor a
/// URL to derive one from. Public so callers can tell a real title from a
/// placeholder and choose not to display it.
pub const UNKNOWN_TITLE: &str = "Media Playing";

/// Stand-in artist when a player reports none and has no usable name.
pub const UNKNOWN_ARTIST: &str = "Unknown Artist";

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
#[derive(Debug, Clone, Default)]
pub struct MediaMetadata {
    /// Track title (e.g. "Bohemian Rhapsody").
    pub title: String,
    /// Artist / band name (e.g. "Queen").
    pub artist: String,
    /// Album name (e.g. "A Night at the Opera").
    pub album: String,
    /// Total track length in seconds, or `0.0` if unavailable.
    pub length_secs: f64,
}

/// Everything the UI needs in one struct, fetched in one go.
#[derive(Debug, Clone)]
pub struct MediaState {
    pub status: PlaybackStatus,
    pub metadata: MediaMetadata,
    pub position_secs: f64,
}

// ── Query caches ────────────────────────────────────────────────────────────

static STATUS_CACHE: Mutex<Option<(Instant, PlaybackStatus)>> = Mutex::new(None);
static METADATA_CACHE: Mutex<Option<(Instant, MediaMetadata)>> = Mutex::new(None);

/// Returns the cached value if it is younger than [`CACHE_TTL`], otherwise runs
/// `fetch`, stores the result and returns it.
fn cached<T: Clone>(cache: &Mutex<Option<(Instant, T)>>, fetch: impl FnOnce() -> T) -> T {
    let mut guard = match cache.lock() {
        Ok(g) => g,
        // A panic in another thread poisoned the lock; fall back to fetching
        // directly rather than propagating it into the render path.
        Err(_) => return fetch(),
    };
    if let Some((at, value)) = guard.as_ref() {
        if at.elapsed() < CACHE_TTL {
            return value.clone();
        }
    }
    let value = fetch();
    *guard = Some((Instant::now(), value.clone()));
    value
}

/// Drops both cached queries so the next call re-reads the player.
///
/// Called after a transport command so the UI picks the new state up
/// immediately instead of waiting out the TTL.
pub fn invalidate_cache() {
    if let Ok(mut g) = STATUS_CACHE.lock() {
        *g = None;
    }
    if let Ok(mut g) = METADATA_CACHE.lock() {
        *g = None;
    }
}

// ── Queries ─────────────────────────────────────────────────────────────────

/// Returns the playback status of the active player.
pub fn get_status() -> PlaybackStatus {
    cached(&STATUS_CACHE, fetch_status)
}

fn fetch_status() -> PlaybackStatus {
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
/// All fields fall back to sensible defaults when a player is not running or
/// doesn't expose the requested property. When the player reports no title —
/// common for local files played by mpv or VLC — title and artist are derived
/// from the media URL's filename.
pub fn get_metadata() -> MediaMetadata {
    cached(&METADATA_CACHE, fetch_metadata)
}

fn fetch_metadata() -> MediaMetadata {
    let separator = "|||";
    let fmt = format!(
        "{{{{title}}}}{}{{{{artist}}}}{}{{{{album}}}}{}{{{{mpris:length}}}}{}{{{{url}}}}{}{{{{xesam:url}}}}{}{{{{playerName}}}}",
        separator, separator, separator, separator, separator, separator
    );

    let Ok(output) = Command::new("playerctl").args(["metadata", "-f", &fmt]).output() else {
        return MediaMetadata::default();
    };
    if !output.status.success() {
        return MediaMetadata::default();
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = stdout.trim_end().split(separator).collect();
    if parts.len() < 7 {
        return MediaMetadata::default();
    }

    let mut title = parts[0].trim().to_string();
    let mut artist = parts[1].trim().to_string();
    let album = parts[2].trim().to_string();
    let length_us: f64 = parts[3].trim().parse().unwrap_or(0.0);
    let raw_url = parts[4].trim();
    let xesam_url = parts[5].trim();
    let player_name = parts[6].trim();

    let effective_url = if raw_url.is_empty() { xesam_url } else { raw_url };

    // Local files often carry no title tag, but do carry a URL. Derive the
    // title (and artist, for the common "Artist - Title.ext" convention) from
    // the filename rather than showing nothing.
    if title.is_empty() && !effective_url.is_empty() {
        let decoded_path = url_decode(effective_url);
        let filename = std::path::Path::new(&decoded_path)
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| decoded_path.clone());

        if !filename.is_empty() {
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
        title = UNKNOWN_TITLE.to_string();
    }

    if artist.is_empty() {
        artist = if player_name.is_empty() {
            UNKNOWN_ARTIST.to_string()
        } else {
            let mut chars = player_name.chars();
            match chars.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
            }
        };
    }

    MediaMetadata {
        title,
        artist,
        album,
        length_secs: length_us / 1_000_000.0,
    }
}

/// Decodes a percent-encoded `file://` URL into a path.
///
/// Percent escapes are collected as raw bytes and decoded as UTF-8 at the end;
/// decoding each `%XX` straight to a `char` would mangle any multi-byte
/// sequence, which is exactly what non-ASCII filenames are made of.
fn url_decode(input: &str) -> String {
    let s = input.trim_start_matches("file://");
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(val) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(val);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Returns the current playback position in seconds.
///
/// Deliberately uncached: position advances continuously, and the only caller
/// polls it on its own schedule to drive a seek bar.
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

/// Fetches the full media state. Designed to be called from a background thread
/// and sent to the UI via a channel, like the other API modules.
pub fn get_media_state() -> MediaState {
    let status = get_status();
    if status == PlaybackStatus::None {
        return MediaState {
            status,
            metadata: MediaMetadata {
                title: "No Media Playing".to_string(),
                artist: "None".to_string(),
                ..Default::default()
            },
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

fn transport(args: &'static [&'static str]) {
    std::thread::spawn(move || {
        let _ = Command::new("playerctl").args(args).status();
        // The player's state just changed; don't serve it stale.
        invalidate_cache();
    });
}

pub fn play_pause() {
    transport(&["play-pause"]);
}

pub fn next() {
    transport(&["next"]);
}

pub fn previous() {
    transport(&["previous"]);
}

/// Seek to an absolute position (in seconds).
pub fn seek(seconds: f64) {
    std::thread::spawn(move || {
        let _ = Command::new("playerctl")
            .args(["position", &format!("{:.1}", seconds)])
            .status();
        invalidate_cache();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_decode_plain_ascii() {
        assert_eq!(url_decode("file:///music/Queen%20-%20Bohemian.mp3"), "/music/Queen - Bohemian.mp3");
    }

    /// The old implementation cast each decoded byte straight to `char`, which
    /// turned every multi-byte UTF-8 sequence into mojibake.
    #[test]
    fn url_decode_multibyte_utf8() {
        // "Björk" — the ö is %C3%B6.
        assert_eq!(url_decode("file:///music/Bj%C3%B6rk.flac"), "/music/Björk.flac");
        // Japanese: 音楽 is %E9%9F%B3%E6%A5%BD.
        assert_eq!(url_decode("file:///%E9%9F%B3%E6%A5%BD.mp3"), "/音楽.mp3");
    }

    #[test]
    fn url_decode_leaves_unescaped_text_alone() {
        assert_eq!(url_decode("/plain/path.mp3"), "/plain/path.mp3");
        // A stray % that isn't a valid escape must not eat the following bytes.
        assert_eq!(url_decode("/100%.mp3"), "/100%.mp3");
    }

    /// The reason the cache exists: rustbar's lyrics mode asks for the track
    /// every 33ms. Without memoization that is ~30 `playerctl` spawns a second
    /// at ~12ms each; with it, a full second of ticks costs a handful.
    #[test]
    fn rapid_polling_collapses_to_one_fetch_per_ttl() {
        let cache: Mutex<Option<(Instant, u32)>> = Mutex::new(None);
        let fetches = std::cell::Cell::new(0u32);

        // One second of 33ms ticks.
        for _ in 0..30 {
            cached(&cache, || {
                fetches.set(fetches.get() + 1);
                fetches.get()
            });
        }

        assert_eq!(
            fetches.get(),
            1,
            "30 rapid calls must fetch once, not 30 times (got {})",
            fetches.get()
        );
    }

    #[test]
    fn cache_serves_within_ttl_and_refetches_after() {
        let cache: Mutex<Option<(Instant, u32)>> = Mutex::new(None);
        let first = cached(&cache, || 1);
        let second = cached(&cache, || 2);
        assert_eq!(first, 1);
        assert_eq!(second, 1, "second call within the TTL must reuse the cached value");

        // Age the entry past the TTL.
        if let Ok(mut g) = cache.lock() {
            if let Some((at, _)) = g.as_mut() {
                *at = Instant::now() - CACHE_TTL - Duration::from_millis(1);
            }
        }
        assert_eq!(cached(&cache, || 3), 3, "an expired entry must be refetched");
    }
}
