//! Kani proofs, compiled only under `cargo kani`. What these cover holds for every value of
//! the integers in it: the layout arithmetic, with no text in it. The text itself goes
//! through tables in unicode-segmentation and unicode-width, which the fuzz targets cover: a
//! proof of `Field::view` on up to 3 chars of ASCII ran out its 15 minutes at 7 GB (Kani 0.68.0).

// A proof may do plain arithmetic: Kani checks that for overflow itself.
#![allow(clippy::arithmetic_side_effects)]

use ratatui::layout::{Position, Rect};

use crate::field::{Scroll, cursor_cell, scroll, scrolled_cursor};
use crate::window::{arrow_ends, corner_place};

/// Any rectangle at all, its right or bottom edge past `u16::MAX` included.
fn any_rect() -> Rect {
    Rect { x: kani::any(), y: kani::any(), width: kani::any(), height: kani::any() }
}

/// A rectangle as `render` works with one, its area clipped to the buffer's.
fn any_clipped() -> Rect {
    any_rect().intersection(any_rect())
}

#[kani::proof]
fn a_text_on_the_border_leaves_the_corners_alone() {
    let area = any_clipped();
    let width: usize = kani::any();
    if let Some((x, w)) = corner_place(area, width) {
        assert_eq!(usize::from(w), width);
        assert!(w > 0);
        // A `─` and the corner on each side.
        assert!(x >= area.x + 2);
        assert_eq!(x + w + 2, area.right());
    } else {
        assert!(width == 0 || width.saturating_add(4) > usize::from(area.width));
    }
}

#[kani::proof]
fn an_arrow_stays_on_the_screen() {
    let (x, upper, lower, screen) = (kani::any(), any_rect(), any_rect(), any_rect());
    if let Some((top, bottom)) = arrow_ends(x, upper, lower, screen) {
        // Every row it draws on, not only its ends.
        let y: u16 = kani::any();
        kani::assume(top <= y && y <= bottom);
        assert!(screen.contains(Position::new(x, y)));
        assert!(top < bottom);
    }
}

#[kani::proof]
fn an_arrow_joins_the_borders_of_both_windows() {
    let (x, upper, lower, screen) = (kani::any(), any_rect(), any_rect(), any_rect());
    if let Some((top, bottom)) = arrow_ends(x, upper, lower, screen) {
        assert_eq!(top + 1, upper.bottom());
        assert!(upper.height > 0 && lower.height > 0);
        assert_eq!(bottom, lower.y);
        for window in [upper, lower] {
            // On the border, off its corners.
            assert!(window.x < x && x + 1 < window.right());
        }
    }
}

#[kani::proof]
fn an_arrow_is_drawn_when_nothing_stops_it() {
    let (x, upper, lower, screen) = (kani::any(), any_rect(), any_rect(), any_rect());
    kani::assume(upper.height > 0 && lower.height > 0 && upper.bottom() <= lower.y);
    for window in [upper, lower] {
        kani::assume(window.x < x && u32::from(x) + 1 < u32::from(window.right()));
    }
    kani::assume(screen.contains(Position::new(x, upper.bottom() - 1)));
    kani::assume(screen.contains(Position::new(x, lower.y)));
    assert_eq!(arrow_ends(x, upper, lower, screen), Some((upper.bottom() - 1, lower.y)));
}

#[kani::proof]
fn the_cursor_goes_on_the_field() {
    let area = any_clipped();
    kani::assume(!area.is_empty());
    let at = cursor_cell(area, kani::any());
    assert!(area.contains(at));
}

/// The inputs `Field::view` calls `scroll` with: a cursor within the text, in a field the
/// text and the cursor's cell do not fit in, or with the cursor past where it scrolls.
fn any_scroll() -> (usize, usize, usize, Scroll) {
    let (before, total, w): (usize, usize, usize) = (kani::any(), kani::any(), kani::any());
    kani::assume(w > 0 && before <= total && total >= w);
    kani::assume(before.saturating_add(2) > w);
    (before, total, w, scroll(before, total, w))
}

#[kani::proof]
fn a_scrolled_field_uses_its_cells_exactly() {
    let (_, _, w, s) = any_scroll();
    let (left, right) = (usize::from(s.left), usize::from(s.right_cell));
    assert_eq!(left + s.fits + right, w);
}

#[kani::proof]
fn a_scrolled_cursor_keeps_off_the_markers() {
    let (before, _, w, s) = any_scroll();
    let (left, right) = (usize::from(s.left), usize::from(s.right_cell));
    // Whole graphemes are left out, so any number of cells from `skip` on, past the cursor
    // too.
    let skipped: usize = kani::any();
    kani::assume(skipped >= s.skip);
    let column = scrolled_cursor(before, s.left, skipped);
    assert!(left <= column && column < w - right);
}

#[kani::proof]
fn only_the_end_of_the_text_keeps_the_last_cell() {
    let (before, total, w, s) = any_scroll();
    assert_eq!(s.right_cell, before != total && w >= 2);
}
