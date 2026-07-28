use crate::grid;
use crate::render;
use crate::api::media::{MediaState, PlaybackStatus};
use super::text::FontCache;

#[derive(Clone, Copy, Debug, PartialEq)]
enum MediaButton {
    Prev,
    PlayPause,
    Next,
}

pub struct MediaPlayer {
    pub rect: (i32, i32, i32, i32),
    pub visible: bool,
    title: String,
    artist: String,
    art_url: String,
    art_pixmap: Option<tiny_skia::Pixmap>,
    fallback_icon: Option<tiny_skia::Pixmap>,
    position_secs: f64,
    duration_secs: f64,
    status: PlaybackStatus,
    play_icon: Option<tiny_skia::Pixmap>,
    pause_icon: Option<tiny_skia::Pixmap>,
    prev_icon: Option<tiny_skia::Pixmap>,
    next_icon: Option<tiny_skia::Pixmap>,
    play_pause_handler: Option<Box<dyn Fn()>>,
    next_handler: Option<Box<dyn Fn()>>,
    prev_handler: Option<Box<dyn Fn()>>,
    hovered_btn: Option<MediaButton>,
    pressed_btn: Option<MediaButton>,
}

impl MediaPlayer {
    pub fn new() -> Self {
        let assets = env!("CARGO_MANIFEST_DIR");
        let play_icon = super::svg_utils::load_svg_colored(
            std::path::Path::new(&format!("{}/assets/play.svg", assets)), "#5A3A14", 36);
        let pause_icon = super::svg_utils::load_svg_colored(
            std::path::Path::new(&format!("{}/assets/pause.svg", assets)), "#5A3A14", 36);
        let prev_icon = super::svg_utils::load_svg_colored(
            std::path::Path::new(&format!("{}/assets/skip-back.svg", assets)), "#5A3A14", 36);
        let next_icon = super::svg_utils::load_svg_colored(
            std::path::Path::new(&format!("{}/assets/skip-forward.svg", assets)), "#5A3A14", 36);
        let fallback_icon = super::svg_utils::load_svg_colored(
            std::path::Path::new(&format!("{}/assets/music.svg", assets)), "#7A5020", 48);

        Self {
            rect: (0, 0, 0, 0),
            visible: false,
            title: "No Media".to_string(),
            artist: "Unknown Artist".to_string(),
            art_url: String::new(),
            art_pixmap: None,
            fallback_icon,
            position_secs: 0.0,
            duration_secs: 0.0,
            status: PlaybackStatus::None,
            play_icon,
            pause_icon,
            prev_icon,
            next_icon,
            play_pause_handler: None,
            next_handler: None,
            prev_handler: None,
            hovered_btn: None,
            pressed_btn: None,
        }
    }

    pub fn set_rect(&mut self, x: i32, y: i32, w: i32, h: i32) {
        self.rect = (x, y, w, h);
    }

    pub fn update_from_state(&mut self, state: &MediaState) {
        self.title = if state.metadata.title.is_empty() {
            "No Media".to_string()
        } else {
            state.metadata.title.clone()
        };
        self.artist = if state.metadata.artist.is_empty() {
            "Unknown Artist".to_string()
        } else {
            state.metadata.artist.clone()
        };
        self.position_secs = state.position_secs;
        self.duration_secs = state.metadata.length_secs;
        self.status = state.status.clone();

        // Load album art if URL changed
        if state.metadata.art_url != self.art_url {
            self.art_url = state.metadata.art_url.clone();
            self.art_pixmap = None;
            if self.art_url.starts_with("file://") {
                let path = self.art_url.trim_start_matches("file://");
                self.art_pixmap = super::svg_utils::load_image_scaled(path, 48, 48);
            }
        }

        self.visible = self.status == PlaybackStatus::Playing
            || self.status == PlaybackStatus::Paused;
    }

    pub fn connect_play_pause<F: Fn() + 'static>(&mut self, f: F) {
        self.play_pause_handler = Some(Box::new(f));
    }
    pub fn connect_next<F: Fn() + 'static>(&mut self, f: F) {
        self.next_handler = Some(Box::new(f));
    }
    pub fn connect_previous<F: Fn() + 'static>(&mut self, f: F) {
        self.prev_handler = Some(Box::new(f));
    }

    pub fn contains(&self, px: f64, py: f64) -> bool {
        if !self.visible { return false; }
        let (x, y, w, h) = self.rect;
        px >= x as f64 && px < (x + w) as f64 && py >= y as f64 && py < (y + h) as f64
    }

    fn btn_rects(&self) -> [(f32, f32, f32, f32); 3] {
        let (rx, ry, rw, rh) = self.rect;
        let pad = 12.0;
        let btn_size = 28.0;
        let gap = 2.0;
        let transport_w = btn_size * 3.0 + gap * 2.0;
        let transport_x = rx as f32 + rw as f32 - pad - transport_w;
        let upper_h = rh as f32 - pad * 2.0 - 20.0 - 6.0;
        let transport_y = ry as f32 + pad + (upper_h - btn_size) / 2.0;

        [
            (transport_x, transport_y, btn_size, btn_size),
            (transport_x + btn_size + gap, transport_y, btn_size, btn_size),
            (transport_x + (btn_size + gap) * 2.0, transport_y, btn_size, btn_size),
        ]
    }

    fn hit_test_btn(&self, px: f64, py: f64) -> Option<MediaButton> {
        let rects = self.btn_rects();
        for (i, (bx, by, bw, bh)) in rects.iter().enumerate() {
            if px >= *bx as f64 && px < (*bx + *bw) as f64
                && py >= *by as f64 && py < (*by + *bh) as f64
            {
                return match i {
                    0 => Some(MediaButton::Prev),
                    1 => Some(MediaButton::PlayPause),
                    2 => Some(MediaButton::Next),
                    _ => None,
                };
            }
        }
        None
    }

    pub fn on_pointer_motion(&mut self, px: f64, py: f64) -> bool {
        let new_hover = self.hit_test_btn(px, py);
        if new_hover != self.hovered_btn {
            self.hovered_btn = new_hover;
            return true;
        }
        false
    }

    pub fn on_pointer_leave(&mut self) -> bool {
        let changed = self.hovered_btn.is_some() || self.pressed_btn.is_some();
        self.hovered_btn = None;
        self.pressed_btn = None;
        changed
    }

    pub fn on_pointer_press(&mut self, px: f64, py: f64) -> bool {
        self.pressed_btn = self.hit_test_btn(px, py);
        self.pressed_btn.is_some()
    }

    pub fn on_pointer_release(&mut self) -> bool {
        let btn = self.pressed_btn.take();
        match btn {
            Some(MediaButton::Prev) => {
                if let Some(ref cb) = self.prev_handler { cb(); }
            }
            Some(MediaButton::PlayPause) => {
                if let Some(ref cb) = self.play_pause_handler { cb(); }
            }
            Some(MediaButton::Next) => {
                if let Some(ref cb) = self.next_handler { cb(); }
            }
            None => return false,
        }
        true
    }

    pub fn draw(&self, pixmap: &mut tiny_skia::PixmapMut, fonts: &FontCache) {
        if !self.visible { return; }
        let (rx, ry, rw, rh) = self.rect;
        let (x, y, w, h) = (rx as f32, ry as f32, rw as f32, rh as f32);
        if w <= 0.0 || h <= 0.0 { return; }

        let pad = 12.0_f32;

        // Background card
        let bg = tiny_skia::Color::from_rgba(0.77, 0.48, 0.06, 0.08).unwrap();
        render::fill_rounded_rect(pixmap, x, y, w, h, grid::WIDGET_RADIUS, bg);

        let lower_h = 20.0_f32;
        let gap = 6.0_f32;
        let upper_h = h - pad * 2.0 - lower_h - gap;

        // ── Album art ──────────────────────────────────────────────────────
        let art_size = 48.0_f32;
        let art_x = x + pad;
        let art_y = y + pad + (upper_h - art_size) / 2.0;

        // Art background
        let art_bg = tiny_skia::Color::from_rgba(0.77, 0.48, 0.06, 0.12).unwrap();
        render::fill_rounded_rect(pixmap, art_x, art_y, art_size, art_size, 8.0, art_bg);

        if let Some(ref art) = self.art_pixmap {
            render::draw_pixmap_scaled(pixmap, art.as_ref(), art_x, art_y, art_size, art_size);
        } else if let Some(ref fallback) = self.fallback_icon {
            let fsz = 24.0;
            let fx = art_x + (art_size - fsz) / 2.0;
            let fy = art_y + (art_size - fsz) / 2.0;
            render::draw_pixmap_scaled(pixmap, fallback.as_ref(), fx, fy, fsz, fsz);
        }

        // ── Title & artist ─────────────────────────────────────────────────
        let text_x = art_x + art_size + 12.0;
        let title_y = y + pad + (upper_h / 2.0) - 14.0;
        let artist_y = title_y + 16.0;

        let title_color = tiny_skia::Color::from_rgba(0.227, 0.141, 0.031, 1.0).unwrap();
        let artist_color = tiny_skia::Color::from_rgba(0.353, 0.227, 0.078, 0.8).unwrap();

        // Truncate long text to fit available width
        let btn_rects = self.btn_rects();
        let max_text_w = btn_rects[0].0 - text_x - 4.0;
        let display_title = truncate_to_width(fonts, &self.title, 13.0, true, max_text_w);
        let display_artist = truncate_to_width(fonts, &self.artist, 11.0, false, max_text_w);

        fonts.draw_text(pixmap, &display_title, text_x, title_y, 13.0, true, title_color);
        fonts.draw_text(pixmap, &display_artist, text_x, artist_y, 11.0, false, artist_color);

        // ── Transport buttons ──────────────────────────────────────────────
        let icons = [self.prev_icon.as_ref(), {
            if self.status == PlaybackStatus::Playing {
                self.pause_icon.as_ref()
            } else {
                self.play_icon.as_ref()
            }
        }, self.next_icon.as_ref()];
        let btns = [MediaButton::Prev, MediaButton::PlayPause, MediaButton::Next];

        for (i, (bx, by, bw, bh)) in btn_rects.iter().enumerate() {
            // Hover/press highlight
            let this_btn = btns[i];
            if self.pressed_btn == Some(this_btn) {
                let hl = tiny_skia::Color::from_rgba(0.77, 0.48, 0.06, 0.15).unwrap();
                render::fill_rounded_rect(pixmap, *bx, *by, *bw, *bh, *bw / 2.0, hl);
            } else if self.hovered_btn == Some(this_btn) {
                let hl = tiny_skia::Color::from_rgba(0.77, 0.48, 0.06, 0.10).unwrap();
                render::fill_rounded_rect(pixmap, *bx, *by, *bw, *bh, *bw / 2.0, hl);
            }

            if let Some(icon) = icons[i] {
                let isz = 18.0_f32;
                let ix = bx + (bw - isz) / 2.0;
                let iy = by + (bh - isz) / 2.0;
                render::draw_pixmap_scaled(pixmap, icon.as_ref(), ix, iy, isz, isz);
            }
        }

        // ── Progress bar ───────────────────────────────────────────────────
        let prog_y = y + pad + upper_h + gap;
        let prog_w = w - pad * 2.0;
        let prog_h = 6.0_f32;
        let prog_x = x + pad;

        let trough_bg = tiny_skia::Color::from_rgba(0.478, 0.314, 0.125, 0.15).unwrap();
        render::fill_rounded_rect(pixmap, prog_x, prog_y, prog_w, prog_h, 3.0, trough_bg);

        if self.duration_secs > 0.0 {
            let frac = (self.position_secs / self.duration_secs).clamp(0.0, 1.0) as f32;
            let fill_w = prog_w * frac;
            if fill_w > 0.0 {
                let fill_color = tiny_skia::Color::from_rgba(0.769, 0.478, 0.063, 1.0).unwrap();
                render::fill_rounded_rect(pixmap, prog_x, prog_y, fill_w, prog_h, 3.0, fill_color);
            }
        }

        // ── Time labels ────────────────────────────────────────────────────
        let time_y = prog_y + prog_h + 2.0;
        let time_color = tiny_skia::Color::from_rgba(0.353, 0.227, 0.078, 0.5).unwrap();
        fonts.draw_text(pixmap, &Self::format_time(self.position_secs), prog_x, time_y, 9.0, false, time_color);

        let dur_str = Self::format_time(self.duration_secs);
        let dur_w = fonts.measure_text(&dur_str, 9.0, false);
        fonts.draw_text(pixmap, &dur_str, prog_x + prog_w - dur_w, time_y, 9.0, false, time_color);
    }

    fn format_time(seconds: f64) -> String {
        if seconds < 0.0 || seconds.is_nan() {
            return "0:00".to_string();
        }
        let total = seconds.round() as u64;
        format!("{}:{:02}", total / 60, total % 60)
    }
}

impl Default for MediaPlayer {
    fn default() -> Self {
        Self::new()
    }
}

fn truncate_to_width(fonts: &FontCache, text: &str, size: f32, bold: bool, max_w: f32) -> String {
    if max_w <= 0.0 { return String::new(); }
    let full_w = fonts.measure_text(text, size, bold);
    if full_w <= max_w {
        return text.to_string();
    }
    let ellipsis = "…";
    let ew = fonts.measure_text(ellipsis, size, bold);
    let target = max_w - ew;
    let mut w = 0.0;
    let mut end = 0;
    for (i, ch) in text.char_indices() {
        let cw = fonts.measure_text(&text[i..i + ch.len_utf8()], size, bold);
        if w + cw > target { break; }
        w += cw;
        end = i + ch.len_utf8();
    }
    format!("{}{}", &text[..end], ellipsis)
}
