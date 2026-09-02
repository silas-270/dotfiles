//! Code shared by the `rustbar` status bar and the `control-center` panel.
//!
//! Both are Wayland layer-shell surfaces that read the same generated palette
//! and render with the same font, so theme loading, drawing primitives, text
//! rendering, buffer presentation and the audio API live here. Anything that
//! genuinely differs between the two surfaces — event handling, layout, the
//! remaining API modules — stays in the binaries.

pub mod api;
pub mod font;
pub mod paint;
pub mod theme;
pub mod wayland;
