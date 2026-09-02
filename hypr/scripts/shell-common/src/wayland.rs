//! The buffer→surface presentation sequence shared by both surfaces.
//!
//! Allocating an SHM buffer plus a matching transparent `Pixmap`, and later
//! converting to BGRA and running attach/damage/commit, is identical at every
//! draw site in both binaries — only the painting in between differs. These are
//! the prologue and epilogue around that painting.

use smithay_client_toolkit::shm::slot::{Buffer, SlotPool};
use tiny_skia::Pixmap;
use wayland_client::protocol::{wl_shm, wl_surface::WlSurface};

#[derive(Debug)]
pub enum SurfaceError {
    ZeroSize,
    Buffer(String),
    Pixmap,
}

impl std::fmt::Display for SurfaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SurfaceError::ZeroSize => write!(f, "surface has zero width or height"),
            SurfaceError::Buffer(e) => write!(f, "failed to create SHM buffer: {e}"),
            SurfaceError::Pixmap => write!(f, "failed to allocate pixmap"),
        }
    }
}

/// Allocates an ARGB8888 SHM buffer and a matching fully-transparent `Pixmap`.
///
/// Paint into the pixmap, then hand both back to [`present`].
pub fn acquire<'a>(
    pool: &'a mut SlotPool,
    width: u32,
    height: u32,
) -> Result<(Buffer, &'a mut [u8], Pixmap), SurfaceError> {
    if width == 0 || height == 0 {
        return Err(SurfaceError::ZeroSize);
    }

    let stride = width as i32 * 4;
    let (buffer, canvas) = pool
        .create_buffer(width as i32, height as i32, stride, wl_shm::Format::Argb8888)
        .map_err(|e| SurfaceError::Buffer(format!("{e:?}")))?;

    let mut pixmap = Pixmap::new(width, height).ok_or(SurfaceError::Pixmap)?;
    pixmap.fill(tiny_skia::Color::TRANSPARENT);

    Ok((buffer, canvas, pixmap))
}

/// Converts the painted pixmap into the SHM canvas and commits it to `surface`.
///
/// Damage covers the whole surface, which is what every call site already did.
pub fn present(
    buffer: &Buffer,
    canvas: &mut [u8],
    surface: &WlSurface,
    pixmap: &Pixmap,
    width: u32,
    height: u32,
) {
    crate::paint::rgba_to_bgra(pixmap.data(), canvas);
    surface.attach(Some(buffer.wl_buffer()), 0, 0);
    surface.damage_buffer(0, 0, width as i32, height as i32);
    surface.commit();
}
