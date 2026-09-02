use crate::api::lyrics::{self, LyricLine, LyricsResult};
use shell_common::api::media::{self, MediaMetadata, PlaybackStatus};
use crate::render::BarText;
use shell_common::font::FontCache;
use shell_common::theme::ThemeConfig;
use shell_common::paint::stroke_rect;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tiny_skia::{Color, PixmapMut};

const ANIM_DURATION: Duration = Duration::from_millis(220);
const MESSAGE_DURATION: Duration = Duration::from_secs(1);
const TRACK_CHANGE_TITLE_DURATION: Duration = Duration::from_millis(1500);
const POSITION_POLL_INTERVAL: Duration = Duration::from_millis(150);
const LINE_SHIFT_PX: f32 = 22.0;
const NOT_FOUND_MESSAGE: &str = "keine synced lyrics gefunden";

pub enum CenterpieceDisplay {
    Title,
    Message(String, Instant),
    Lyrics,
    /// Shown for TRACK_CHANGE_TITLE_DURATION right after a track change while lyrics mode is
    /// active, before switching (back) into Lyrics for the new track.
    TrackChangeTitle(String, Instant),
}

struct LineAnim {
    from_text: String,
    to_text: String,
    start: Instant,
}

pub struct CenterpieceState {
    display: CenterpieceDisplay,
    current_track: Option<(String, String)>,
    lyrics_cache: HashMap<(String, String), LyricsResult>,
    current_lines: Vec<LyricLine>,
    displayed_line_idx: Option<usize>,
    anim: Option<LineAnim>,
    last_position_check: Instant,
    /// Lyrics result for the new track, fetched eagerly on track-change, applied once
    /// TrackChangeTitle's timer expires.
    pending_lyrics: Option<LyricsResult>,
}

impl Default for CenterpieceState {
    fn default() -> Self {
        Self {
            display: CenterpieceDisplay::Title,
            current_track: None,
            lyrics_cache: HashMap::new(),
            current_lines: Vec::new(),
            displayed_line_idx: None,
            anim: None,
            last_position_check: Instant::now(),
            pending_lyrics: None,
        }
    }
}

fn line_text_at(lines: &[LyricLine], idx: Option<usize>) -> String {
    idx.and_then(|i| lines.get(i)).map(|l| l.text.clone()).unwrap_or_default()
}

/// Returns true when the centerpiece is showing an actual track (not the fallback text) —
/// the only state in which Space is allowed to toggle lyrics mode.
pub fn is_showing_track() -> bool {
    matches!(media::get_status(), PlaybackStatus::Playing | PlaybackStatus::Paused)
}

/// Applies a fetched lyrics result: switches into Lyrics mode (with a slide-in animation from
/// `from_text`) if synced lines were found, otherwise shows the "not found" message.
fn begin_lyrics_display(state: &mut CenterpieceState, from_text: String, result: LyricsResult) {
    match result {
        LyricsResult::Synced(lines) if !lines.is_empty() => {
            let pos = lyrics::get_position_secs().unwrap_or(0.0);
            let idx = lyrics::current_line_index(&lines, pos);
            let to_text = line_text_at(&lines, idx);
            state.current_lines = lines;
            state.displayed_line_idx = idx;
            state.display = CenterpieceDisplay::Lyrics;
            state.anim = Some(LineAnim { from_text, to_text, start: Instant::now() });
            state.last_position_check = Instant::now();
        }
        _ => {
            state.current_lines.clear();
            state.displayed_line_idx = None;
            state.display = CenterpieceDisplay::Message(NOT_FOUND_MESSAGE.to_string(), Instant::now());
        }
    }
}

/// Called on Space press while the centerpiece is focused and a track is playing.
pub fn toggle(state: &mut CenterpieceState) {
    match state.display {
        CenterpieceDisplay::Lyrics => {
            state.display = CenterpieceDisplay::Title;
            state.anim = None;
        }
        CenterpieceDisplay::Message(_, _) | CenterpieceDisplay::TrackChangeTitle(_, _) => {
            // Already mid-transition; ignore extra presses.
        }
        CenterpieceDisplay::Title => {
            let Some(meta) = current_track() else {
                state.display = CenterpieceDisplay::Message(NOT_FOUND_MESSAGE.to_string(), Instant::now());
                return;
            };
            let key = (meta.artist.clone(), meta.title.clone());
            let result = state
                .lyrics_cache
                .entry(key.clone())
                .or_insert_with(|| lyrics::fetch_lyrics(&meta.artist, &meta.title, &meta.album, meta.length_secs as u64))
                .clone();

            state.current_track = Some(key);
            begin_lyrics_display(state, String::new(), result);
        }
    }
}

/// Periodic tick (driven by a fast calloop timer) that advances animations, expires the
/// "not found" message, detects track changes and advances the synced lyric line.
/// Returns true if a redraw is needed.
pub fn tick(state: &mut CenterpieceState) -> bool {
    let mut needs_redraw = false;

    if let CenterpieceDisplay::TrackChangeTitle(text, shown_at) = &state.display {
        if shown_at.elapsed() >= TRACK_CHANGE_TITLE_DURATION {
            let from_text = text.clone();
            let result = state.pending_lyrics.take().unwrap_or(LyricsResult::NotFound);
            begin_lyrics_display(state, from_text, result);
            return true;
        }
        // Still showing the new track's title — nothing else to advance.
        return false;
    }

    if let CenterpieceDisplay::Message(_, shown_at) = &state.display {
        if shown_at.elapsed() >= MESSAGE_DURATION {
            state.display = CenterpieceDisplay::Title;
            return true;
        }
    }

    if matches!(state.display, CenterpieceDisplay::Lyrics) {
        let meta_now = current_track();
        let track_now = meta_now.as_ref().map(|m| (m.artist.clone(), m.title.clone()));
        if track_now != state.current_track {
            if let Some(meta) = meta_now {
                // Track changed while showing lyrics: fetch lyrics for the new track eagerly,
                // show its title briefly, then resume lyrics mode once that fetch is in hand.
                let display_text = get_centerpiece_text().0;
                let key = (meta.artist.clone(), meta.title.clone());
                let result = state
                    .lyrics_cache
                    .entry(key.clone())
                    .or_insert_with(|| lyrics::fetch_lyrics(&meta.artist, &meta.title, &meta.album, meta.length_secs as u64))
                    .clone();
                state.pending_lyrics = Some(result);
                state.current_track = Some(key);
                state.anim = None;
                state.display = CenterpieceDisplay::TrackChangeTitle(display_text, Instant::now());
            } else {
                // Player stopped entirely.
                state.display = CenterpieceDisplay::Title;
                state.anim = None;
                state.current_lines.clear();
                state.displayed_line_idx = None;
            }
            return true;
        }
    }

    if let Some(anim) = &state.anim {
        let progress = anim.start.elapsed().as_secs_f32() / ANIM_DURATION.as_secs_f32();
        if progress >= 1.0 {
            state.anim = None;
        }
        needs_redraw = true;
    }

    if matches!(state.display, CenterpieceDisplay::Lyrics)
        && state.anim.is_none()
        && state.last_position_check.elapsed() >= POSITION_POLL_INTERVAL
    {
        state.last_position_check = Instant::now();
        if let Some(pos) = lyrics::get_position_secs() {
            let new_idx = lyrics::current_line_index(&state.current_lines, pos);
            if new_idx != state.displayed_line_idx && new_idx.is_some() {
                let from_text = line_text_at(&state.current_lines, state.displayed_line_idx);
                let to_text = line_text_at(&state.current_lines, new_idx);
                state.anim = Some(LineAnim { from_text, to_text, start: Instant::now() });
                state.displayed_line_idx = new_idx;
                needs_redraw = true;
            }
        }
    }

    needs_redraw
}

fn with_alpha(color: Color, alpha: f32) -> Color {
    Color::from_rgba(color.red(), color.green(), color.blue(), color.alpha() * alpha).unwrap_or(color)
}

/// Draws a single "[ icon text ]" tag line, vertically clipped to the module box, with the
/// given alpha and vertical pixel offset — used for both the resting line and slide frames.
fn draw_tag_line(
    font_cache: &mut FontCache,
    pixmap: &mut PixmapMut,
    text: &str,
    box_x: f32,
    box_top_y: f32,
    box_w: f32,
    box_h: f32,
    font_size: f32,
    color: Color,
    y_offset: f32,
    alpha: f32,
) {
    if text.is_empty() || alpha <= 0.0 {
        return;
    }
    let text_w = font_cache.measure_calibrated_bracket_tag(text, font_size, false);
    let bracket_w = font_cache.measure_text("[", font_size);
    let text_x = box_x + (box_w - text_w) / 2.0;
    let text_h = font_cache.text_height(font_size);
    let text_y = box_top_y + (box_h - text_h) / 2.0 + y_offset;
    let col = with_alpha(color, alpha);
    let clip_top = box_top_y;
    let clip_bottom = box_top_y + box_h;

    font_cache.draw_text_clipped(pixmap, "[", text_x, text_y, font_size, col, clip_top, clip_bottom);
    let inner_x = text_x + bracket_w + 5.33;
    font_cache.draw_text_clipped(pixmap, text, inner_x, text_y, font_size, col, clip_top, clip_bottom);
    let right_bracket_x = text_x + text_w - bracket_w;
    font_cache.draw_text_clipped(pixmap, "]", right_bracket_x, text_y, font_size, col, clip_top, clip_bottom);
}

pub fn render_centerpiece(
    pixmap: &mut PixmapMut,
    font_cache: &mut FontCache,
    theme: &ThemeConfig,
    state: &CenterpieceState,
    start_x: f32,
    top_y: f32,
    font_size: f32,
) -> f32 {
    let box_h = 32.0;

    match &state.display {
        CenterpieceDisplay::Message(text, _) | CenterpieceDisplay::TrackChangeTitle(text, _) => {
            font_cache.draw_module_box(pixmap, text, start_x, top_y, font_size, theme.accent_color, theme.border, false)
        }
        CenterpieceDisplay::Lyrics => {
            if let Some(anim) = &state.anim {
                let progress = (anim.start.elapsed().as_secs_f32() / ANIM_DURATION.as_secs_f32()).min(1.0);
                let box_w = font_cache.measure_calibrated_bracket_tag(&anim.to_text, font_size, false)
                    + 2.0 * (6.0 + 2.0);
                stroke_rect(pixmap, start_x, top_y, box_w, box_h, theme.border, 2.0);

                draw_tag_line(
                    font_cache, pixmap, &anim.from_text, start_x, top_y, box_w, box_h,
                    font_size, theme.accent_color, -progress * LINE_SHIFT_PX, 1.0 - progress,
                );
                draw_tag_line(
                    font_cache, pixmap, &anim.to_text, start_x, top_y, box_w, box_h,
                    font_size, theme.accent_color, (1.0 - progress) * LINE_SHIFT_PX, progress,
                );
                box_w
            } else {
                let text = line_text_at(&state.current_lines, state.displayed_line_idx);
                font_cache.draw_module_box(pixmap, &text, start_x, top_y, font_size, theme.accent_color, theme.border, false)
            }
        }
        CenterpieceDisplay::Title => {
            let (text, _is_playing) = get_centerpiece_text();
            font_cache.draw_module_box(pixmap, &text, start_x, top_y, font_size, theme.accent_color, theme.border, false)
        }
    }
}

pub fn handle_click() {
    std::thread::spawn(|| {
        let _ = std::process::Command::new("/home/silas270/.config/hypr/scripts/rofi-power-profile-menu.sh").status();
        if let Ok(mut stream) = std::os::unix::net::UnixStream::connect("/home/silas270/dotfiles/hypr/scripts/rustbar/rustbar.sock") {
            use std::io::Write;
            let _ = stream.write_all(b"panel_closed");
        }
    });
}

/// The track the player is currently on, or `None` when nothing is playing or
/// the player reports nothing usable.
///
/// `media::get_metadata()` always returns a struct — it substitutes placeholders
/// rather than failing — so the placeholders are what "no track" looks like here.
fn current_track() -> Option<MediaMetadata> {
    if !matches!(media::get_status(), PlaybackStatus::Playing | PlaybackStatus::Paused) {
        return None;
    }
    let meta = media::get_metadata();
    if meta.title.is_empty() || meta.title == media::UNKNOWN_TITLE {
        return None;
    }
    Some(meta)
}

/// Text shown in the centerpiece when it is displaying the track rather than
/// lyrics: a play/pause glyph followed by "artist - title", truncated to fit.
///
/// This is presentation, not an API query — it lives here rather than in
/// `shell_common::api::media` because only the bar renders it.
pub fn get_centerpiece_text() -> (String, bool) {
    const IDLE_TEXT: &str = "I use Arch btw";
    /// Longest track string rendered before it is elided.
    const MAX_CHARS: usize = 32;

    let status = media::get_status();
    let Some(meta) = current_track() else {
        return (IDLE_TEXT.to_string(), false);
    };

    // A placeholder artist is worse than no artist at all in a one-line display.
    let artist = if meta.artist == media::UNKNOWN_ARTIST { "" } else { meta.artist.as_str() };
    let track = if artist.is_empty() {
        meta.title.clone()
    } else {
        format!("{} - {}", artist, meta.title)
    };

    let truncated = if track.chars().count() > MAX_CHARS {
        format!("{}...", track.chars().take(MAX_CHARS - 2).collect::<String>())
    } else {
        track
    };

    let icon = if status == PlaybackStatus::Paused { "4" } else { "8" };
    (format!("{} {}", icon, truncated), status == PlaybackStatus::Playing)
}
