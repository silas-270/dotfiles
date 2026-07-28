// ---------------------------------------------------------------------------
// Grid constants – 60px base cell size, 5px cell padding, 5px outer margin
// ---------------------------------------------------------------------------

/// Base cell size in pixels (both width and height of 1x1 block).
pub const CELL_SIZE: i32 = 60;

/// Padding inset in pixels inside each cell boundary.
pub const PADDING: i32 = 5;

/// Extra outer margin around the whole grid in pixels so that the outer window
/// padding (OUTER_MARGIN + PADDING = 10px) matches the gap between cells (5px + 5px = 10px).
pub const OUTER_MARGIN: i32 = 5;

/// Number of columns in the grid.
pub const GRID_COLS: i32 = 6;

/// Base number of rows in the grid for standard controls (rows 0..=2).
pub const BASE_GRID_ROWS: i32 = 3;

/// Maximum number of rows in the grid including media player (rows 0..=4).
pub const MAX_GRID_ROWS: i32 = 5;

/// Derived grid container dimensions in pixels.
pub const GRID_WIDTH: i32 = GRID_COLS * CELL_SIZE + 2 * OUTER_MARGIN; // 370px
pub const GRID_HEIGHT: i32 = BASE_GRID_ROWS * CELL_SIZE + 2 * OUTER_MARGIN; // 190px
pub const GRID_HEIGHT_WITH_MEDIA: i32 = MAX_GRID_ROWS * CELL_SIZE + 2 * OUTER_MARGIN; // 310px

/// Corner radius for every card / widget, in pixels.
pub const WIDGET_RADIUS: f32 = 12.0;

/// Corner radius of the outer window, matching Hyprland's rounding = 16.
#[allow(dead_code)]
pub const WINDOW_RADIUS: f32 = 16.0;

/// Calculate pixel rectangle `(x, y, width, height)` for a grid coordinate range.
///
/// # Parameters
/// * `top_left`     – `(col, row)` 0-indexed inclusive.
/// * `bottom_right` – `(col, row)` 0-indexed inclusive.
pub fn calc_rect(top_left: (i32, i32), bottom_right: (i32, i32)) -> (i32, i32, i32, i32) {
    let (col, row) = top_left;
    let (end_col, end_row) = bottom_right;

    let x1 = OUTER_MARGIN + col * CELL_SIZE + PADDING;
    let y1 = OUTER_MARGIN + row * CELL_SIZE + PADDING;

    let x2 = OUTER_MARGIN + (end_col + 1) * CELL_SIZE - PADDING;
    let y2 = OUTER_MARGIN + (end_row + 1) * CELL_SIZE - PADDING;

    let width = x2 - x1;
    let height = y2 - y1;

    (x1, y1, width, height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_pixel_calc() {
        assert_eq!(GRID_WIDTH, 370);
        assert_eq!(GRID_HEIGHT, 190);
        assert_eq!(GRID_HEIGHT_WITH_MEDIA, 310);

        // 1x1 cell at (0,0) -> (0,0)
        let (x, y, w, h) = calc_rect((0, 0), (0, 0));
        assert_eq!((x, y, w, h), (10, 10, 50, 50));

        // 3x1 span at (0,0) -> (2,0)
        let (x, y, w, h) = calc_rect((0, 0), (2, 0));
        assert_eq!((x, y, w, h), (10, 10, 170, 50));

        // Media Player span at (0,3) -> (5,4)
        let (mx, my, mw, mh) = calc_rect((0, 3), (5, 4));
        assert_eq!((mx, my, mw, mh), (10, 190, 350, 110));

        // Gap between Volume row (row 2) and Media Player (row 3)
        let row2_end_y = 10 + 50 + 10 + 50 + 10 + 50; // 180px
        assert_eq!(my - row2_end_y, 10); // Exactly 10px gap!

        // Outer bottom margin
        assert_eq!(GRID_HEIGHT_WITH_MEDIA - (my + mh), 10); // Exactly 10px outer margin!
    }
}
