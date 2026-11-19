//! Kani proofs, compiled only under `cargo kani`. What these cover holds for every value of
//! the integers in it: the layout arithmetic, with no text in it. The text itself goes
//! through tables in unicode-segmentation and unicode-width, which the fuzz targets cover: a
//! proof of `Field::view` on up to 3 chars of ASCII ran out its 15 minutes at 7 GB (Kani 0.68.0).

// A proof may do plain arithmetic: Kani checks that for overflow itself.
#![allow(clippy::arithmetic_side_effects)]

use ratatui::layout::{Position, Rect};

use crate::choice::{between, forward};
use crate::dropdown::{offset, place, place_joined};
use crate::field::{Scroll, cursor_cell, scroll, scrolled_cursor};
use crate::message::{centered, first_row, popup_width};
use crate::prompt::boxes;
use crate::window::{Side, arrow_ends, corner_place};

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
    let side = if kani::any() { Side::Left } else { Side::Right };
    if let Some((x, w)) = corner_place(area, width, side) {
        assert_eq!(usize::from(w), width);
        assert!(w > 0);
        // A `─` and the corner on each side, the text at its end of the border.
        assert!(x >= area.x + 2);
        assert!(x + w + 2 <= area.right());
        match side {
            Side::Left => assert_eq!(x, area.x + 2),
            Side::Right => assert_eq!(x + w + 2, area.right()),
        }
    } else {
        assert!(width == 0 || width.saturating_add(4) > usize::from(area.width));
    }
}

#[kani::proof]
fn an_arrow_stays_on_the_screen() {
    let (x, upper, lower, screen) = (kani::any(), any_rect(), any_rect(), any_rect());
    let ends = arrow_ends(x, upper, lower, screen);
    kani::cover!(ends.is_some());
    if let Some((top, bottom)) = ends {
        assert!(top < bottom);
        // Every row it draws on, not only its ends.
        let y: u16 = kani::any();
        kani::assume(top <= y && y <= bottom);
        assert!(screen.contains(Position::new(x, y)));
    }
}

#[kani::proof]
fn an_arrow_joins_the_borders_of_both_windows() {
    let (x, upper, lower, screen) = (kani::any(), any_rect(), any_rect(), any_rect());
    let ends = arrow_ends(x, upper, lower, screen);
    kani::cover!(ends.is_some());
    if let Some((top, bottom)) = ends {
        assert_eq!(top + 1, upper.bottom());
        assert!(upper.height > 0 && lower.height > 0);
        assert_eq!(bottom, lower.y);
        for window in [upper, lower] {
            // On the border, off its corners, which the screen's edge may have moved.
            let window = window.intersection(screen);
            assert!(window.x < x && x + 1 < window.right());
        }
    }
}

#[kani::proof]
fn an_arrow_is_drawn_when_nothing_stops_it() {
    let (x, upper, lower, screen) = (kani::any(), any_rect(), any_rect(), any_rect());
    kani::assume(upper.height > 0 && lower.height > 0 && upper.bottom() <= lower.y);
    for window in [upper, lower] {
        let window = window.intersection(screen);
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

/// A cursor within a text of any length, in a field of any width but 0: more than
/// `Field::view` scrolls, so that these proofs do not depend on when it does.
fn any_scroll() -> (usize, usize, usize, Scroll) {
    let (before, under, total, w): (usize, usize, usize, usize) =
        (kani::any(), kani::any(), kani::any(), kani::any());
    kani::assume(w > 0 && under > 0 && before <= total);
    (before, under, w, scroll(before, under, total, w))
}

#[kani::proof]
fn a_scrolled_field_uses_its_cells_exactly() {
    let (_, _, w, s) = any_scroll();
    let (left, right) = (usize::from(s.left), usize::from(s.right_cell));
    assert_eq!(left + s.fits + right, w);
}

#[kani::proof]
fn a_scrolled_cursor_keeps_off_the_markers() {
    let (before, under, w, s) = any_scroll();
    let (left, right) = (usize::from(s.left), usize::from(s.right_cell));
    // Whole graphemes are left out, so any number of cells from `skip` to the cursor.
    let skipped: usize = kani::any();
    kani::assume(s.skip <= skipped && skipped <= before);
    let column = scrolled_cursor(before, s.left, skipped);
    assert!(left <= column && column < w - right);
    if under <= w {
        // What is under the cursor fits, before any `>`.
        assert!(under <= w - right - column);
    }
}

#[kani::proof]
fn a_list_stays_on_the_screen_and_off_its_control() {
    let (anchor, screen, w, h) = (any_rect(), any_rect(), kani::any(), kani::any());
    let list = place(anchor, screen, w, h);
    assert!(list.width <= w && list.height <= h);
    kani::cover!(!list.is_empty());
    if !list.is_empty() {
        assert_eq!(list.intersection(screen), list);
        // Under the control or above it, never on one of its rows.
        assert!(list.y >= anchor.bottom() || list.bottom() <= anchor.y);
    }
}

#[kani::proof]
fn a_list_is_whole_where_the_screen_has_room() {
    // A screen as a buffer has one: it ends by `u16::MAX`.
    let (anchor, screen, w, h) = (any_rect(), any_clipped(), kani::any(), kani::any());
    let anchor = anchor.intersection(screen);
    kani::assume(!anchor.is_empty() && w <= screen.width);
    let (below, above) = (screen.bottom() - anchor.bottom(), anchor.y - screen.y);
    kani::assume(h <= below || h <= above);
    let list = place(anchor, screen, w, h);
    assert_eq!((list.width, list.height), (w, h));
    // Under the control whenever it fits there, and at its left edge when it fits there too.
    assert_eq!(list.y == anchor.bottom(), h <= below || h == 0);
    if u32::from(anchor.x) + u32::from(w) <= u32::from(screen.right()) {
        assert_eq!(list.x, anchor.x);
    }
}

#[kani::proof]
fn a_joined_list_stays_on_the_screen_at_its_controls_row() {
    let (x, row, screen): (u16, u16, Rect) = (kani::any(), kani::any(), any_rect());
    let (w, h): (u16, u16) = (kani::any(), kani::any());
    let list = place_joined(x, row, screen, w, h);
    assert!(list.width <= w && list.height <= h);
    kani::cover!(!list.is_empty());
    if !list.is_empty() {
        assert_eq!(list.intersection(screen), list);
        // From the control's row down, or up to it.
        assert!(list.y == row || list.bottom() - 1 == row);
    }
}

#[kani::proof]
fn a_joined_list_is_whole_where_the_screen_has_room() {
    let (x, row, screen): (u16, u16, Rect) = (kani::any(), kani::any(), any_clipped());
    let (w, h): (u16, u16) = (kani::any(), kani::any());
    kani::assume(row >= screen.y && row < screen.bottom() && w <= screen.width);
    let (below, above) = (screen.bottom() - row, row - screen.y + 1);
    kani::assume(h <= below || h <= above);
    let list = place_joined(x, row, screen, w, h);
    assert_eq!((list.width, list.height), (w, h));
    // Down from the control's row whenever it fits there.
    if h <= below {
        assert_eq!(list.y, row);
    }
}

#[kani::proof]
fn the_arrows_of_a_choice_stay_in_its_area() {
    let (area, wanted): (Rect, usize) = (any_clipped(), kani::any());
    if let Some(cells) = between(area.width, wanted) {
        // No more cells than wanted, and all that are wanted where the area has them.
        assert!(usize::from(cells) <= wanted);
        assert!(usize::from(cells) == wanted || cells + 4 == area.width);
        // Two cells for each arrow, the value's between them, all in the area.
        let after = forward(area.x, cells);
        assert_eq!(after, area.x + 2 + cells);
        assert!(after + 2 <= area.right());
    } else {
        assert!(area.width < 4);
    }
}

#[kani::proof]
fn a_scrolled_list_shows_its_cursor() {
    let (cursor, len, rows): (usize, usize, usize) = (kani::any(), kani::any(), kani::any());
    kani::assume(len > 0 && rows > 0);
    let first = offset(cursor, len, rows);
    let cursor = if cursor < len { cursor } else { len - 1 };
    assert!(first <= cursor && cursor - first < rows);
    // No row is left empty while an item is cut off above.
    assert!(first == 0 || first + rows <= len);
}

#[kani::proof]
fn a_popup_is_no_wider_than_it_asks_or_the_screen_has() {
    let (asked, screen): (u16, u16) = (kani::any(), kani::any());
    let width = popup_width(asked, screen);
    assert!(width <= asked && width <= screen);
    // A cell stays free on each side, and the popup is as wide as it asks once 3 do.
    assert!(screen < 2 || width <= screen - 2);
    if u32::from(asked) + 6 <= u32::from(screen) {
        assert_eq!(width, asked);
    }
    // Narrower than it asks only where the screen leaves no more than that cell.
    assert!(width == asked || u32::from(width) + 6 >= u32::from(screen));
}

#[kani::proof]
fn a_popup_stays_on_the_screen_in_its_middle() {
    let (screen, w, h) = (any_rect(), kani::any(), kani::any());
    let popup = centered(screen, w, h);
    assert_eq!(popup.intersection(screen), popup);
    // A screen as a buffer has one: it ends by `u16::MAX`.
    let screen = screen.intersection(Rect::new(0, 0, u16::MAX, u16::MAX));
    assert_eq!(popup.width, if w < screen.width { w } else { screen.width });
    assert_eq!(popup.height, if h < screen.height { h } else { screen.height });
    // As much of the screen on each side, but for the odd cell on the right, and under it
    // the odd row of the screen and that of the popup.
    let (left, right) = (popup.x - screen.x, screen.right() - popup.right());
    assert!(left <= right && right - left <= 1);
    let (above, below) = (popup.y - screen.y, screen.bottom() - popup.bottom());
    assert!(above <= below && below - above <= 2);
}

#[kani::proof]
fn the_windows_of_a_prompt_share_their_edges_on_the_screen() {
    let (screen, w, rows, gap) = (any_rect(), kani::any(), kani::any(), kani::any());
    let (input, list) = boxes(screen, w, rows, gap);
    assert_eq!((input.x, input.width), (list.x, list.width));
    assert!(input.width <= w && input.height <= 3);
    for window in [input, list] {
        assert_eq!(window.intersection(screen), window);
    }
    // The suggestions under the text, never on one of its rows.
    assert!(list.y >= input.bottom());
    if rows.is_none() {
        assert!(list.height == 0);
    }
}

#[kani::proof]
fn a_prompt_is_whole_where_the_screen_has_room() {
    // A screen as a buffer has one: it ends by `u16::MAX`.
    let (screen, w, rows, gap): (_, _, u16, u16) =
        (any_clipped(), kani::any(), kani::any(), kani::any());
    let pair = 3 + u32::from(gap) + u32::from(rows) + 2;
    kani::assume(pair <= u32::from(screen.height));
    let (input, list) = boxes(screen, w, Some(rows), gap);
    assert_eq!(input.height, 3);
    assert_eq!((list.y, list.height), (input.bottom() + gap, rows + 2));
    // A text's window alone has its three rows on any screen that has them.
    assert_eq!(boxes(screen, w, None, gap).0.height, 3);
}

#[kani::proof]
fn a_scrolled_message_fills_its_rows() {
    let (scroll, lines, rows): (usize, usize, usize) = (kani::any(), kani::any(), kani::any());
    let first = first_row(scroll, lines, rows);
    assert!(first <= scroll && first <= lines);
    // No row is left empty while a row of text is cut off above.
    assert!(first == 0 || first + rows <= lines);
    // As far as asked, where the text goes on that far.
    if scroll.checked_add(rows).is_some_and(|end| end <= lines) {
        assert_eq!(first, scroll);
    }
}
