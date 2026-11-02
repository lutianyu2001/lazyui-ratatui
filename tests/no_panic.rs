//! Random areas, buffers and text: nothing panics, and nothing is drawn outside the area
//! it was asked for. Coordinates include ones next to `u16::MAX`, where overflow would be.

// In a test an overflow panics, and so fails it, as it should.
#![allow(clippy::arithmetic_side_effects)]

use lazyui_ratatui::text::{take_width, width};
use lazyui_ratatui::{Field, Window, arrow_down};
use proptest::prelude::*;
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::Style;
use ratatui::widgets::Widget;

fn coord() -> impl Strategy<Value = u16> {
    prop_oneof![0..24u16, (u16::MAX - 24)..=u16::MAX]
}

/// Rects as `Rect::new` makes them, so ones near `u16::MAX` are clipped to fit.
fn rect() -> impl Strategy<Value = Rect> {
    (coord(), coord(), 0..24u16, 0..8u16).prop_map(|(x, y, w, h)| Rect::new(x, y, w, h))
}

/// A buffer small enough to compare cell by cell, somewhere on the screen.
fn buffer() -> impl Strategy<Value = Buffer> {
    (coord(), coord(), 0..24u16, 0..8u16)
        .prop_map(|(x, y, w, h)| Buffer::empty(Rect::new(x, y, w, h)))
}

/// A buffer and an area, mostly a few cells from it: picked apart, the two rarely meet, and
/// then nothing is drawn at all.
fn buffer_and_area() -> impl Strategy<Value = (Buffer, Rect)> {
    let near = (buffer(), 0..8u16, 0..4u16, 0..24u16, 0..24u16, 0..4u16, 0..4u16).prop_map(
        |(buf, x, y, w, h, left, up)| {
            let (at_x, at_y) = (
                buf.area.x.saturating_sub(left).saturating_add(x),
                buf.area.y.saturating_sub(up).saturating_add(y),
            );
            (buf, Rect::new(at_x, at_y, w, h))
        },
    );
    prop_oneof![1 => (buffer(), rect()), 3 => near]
}

/// A buffer, a column and two windows, mostly from the buffer's top left corner with one
/// over the other, so that an arrow gets drawn.
fn arrow() -> impl Strategy<Value = (Buffer, u16, Rect, Rect)> {
    let joined = (buffer(), 0..24u16, 0..24u16, 0..4u16, 0..3u16, 0..4u16).prop_map(
        |(buf, along, w, upper_height, gap, lower_height)| {
            let Position { x, y } = buf.area.as_position();
            let upper = Rect::new(x, y, w, upper_height);
            let lower = Rect::new(x, upper.bottom().saturating_add(gap), w, lower_height);
            (buf, x.saturating_add(along), upper, lower)
        },
    );
    prop_oneof![1 => (buffer(), coord(), rect(), rect()), 3 => joined]
}

/// Text mixing ASCII, wide characters, emoji, combining accents and control characters.
fn text() -> impl Strategy<Value = String> {
    let special = prop::sample::select(vec![
        '文', '件', '❤', '\u{fe0f}', '\u{301}', 'ｶ', 'ﾞ', '\t', '\u{1b}', '\0', '\u{231a}',
        '\u{fe0e}', '\u{1100}', '\u{202e}', '\u{2066}', '\u{200f}', '\u{9b}',
    ]);
    let c = prop_oneof![3 => prop::char::range('a', 'z'), 2 => special, 1 => any::<char>()];
    prop::collection::vec(c, 0..24).prop_map(String::from_iter)
}

/// Cells holding what must not reach a terminal: C0 and C1 controls, and the controls that
/// reorder text.
fn unclean(buf: &Buffer) -> Vec<Position> {
    let bad = |c: char| {
        c.is_control()
            || matches!(c, '\u{61c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}')
            || matches!(c, '\u{2066}'..='\u{2069}')
    };
    buf.area.positions().filter(|&p| buf[p].symbol().chars().any(bad)).collect()
}

/// Cells of `after` outside `kept` that differ from `before`.
fn changed_outside(
    before: &Buffer,
    after: &Buffer,
    kept: impl Fn(Position) -> bool,
) -> Vec<Position> {
    before.area.positions().filter(|&p| !kept(p) && before[p] != after[p]).collect()
}

proptest! {
    #[test]
    fn windows_stay_in_their_area(
        (mut buf, area) in buffer_and_area(),
        (key, name, note) in (text(), text(), text()),
    ) {
        let blank = buf.clone();
        Window::new().name(&key, &name).note(&note).render(area, &mut buf);
        prop_assert_eq!(changed_outside(&blank, &buf, |p| area.contains(p)), []);
        prop_assert_eq!(unclean(&buf), []);
    }

    #[test]
    fn arrows_stay_in_their_column((mut buf, x, upper, lower) in arrow()) {
        let blank = buf.clone();
        arrow_down(&mut buf, x, upper, lower, Style::new());
        let between = |p: Position| p.x == x && p.y >= upper.y && p.y <= lower.y;
        prop_assert_eq!(changed_outside(&blank, &buf, between), []);
    }

    #[test]
    fn fields_stay_in_their_first_row((mut buf, area) in buffer_and_area(), text in text(), cursor in 0..32usize) {
        let blank = buf.clone();
        let at = Field::new(&text, cursor).render(area, &mut buf);
        let row = Rect { height: area.height.min(1), ..area };
        prop_assert_eq!(changed_outside(&blank, &buf, |p| row.contains(p)), []);
        let drawn = row.intersection(buf.area);
        if drawn.is_empty() {
            prop_assert_eq!(at, area.as_position());
        } else {
            prop_assert!(drawn.contains(at), "cursor {at:?} outside {drawn:?}");
        }
        prop_assert_eq!(unclean(&buf), []);
    }

    #[test]
    fn views_fit_their_width(text in text(), cursor in 0..32usize, w in 0..32usize) {
        let v = Field::view(&text, cursor, w);
        let (left, right) = (usize::from(v.left), usize::from(v.right));
        prop_assert!(left + width(v.shown) + right <= w, "{v:?}");
        prop_assert!(text.contains(v.shown));
        if w > 0 {
            prop_assert!(v.cursor < w - right, "{v:?}");
        }
    }

    #[test]
    fn take_width_takes_a_prefix_that_fits(text in text(), w in 0..32usize) {
        let taken = take_width(&text, w);
        prop_assert!(text.starts_with(taken));
        prop_assert!(width(taken) <= w);
    }
}
