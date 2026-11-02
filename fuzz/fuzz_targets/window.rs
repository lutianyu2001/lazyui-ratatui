//! `Window` and `arrow_down`: nothing panics, nothing is drawn off the area (or the arrow's
//! column), a `─` and the corner stay at each end of the border, a text that fits is there in
//! its place, an arrow is made of its three symbols and leaves the corners of both windows
//! alone, and no control character reaches the buffer.
#![no_main]

use lazyui_ratatui::text::{sanitize, width};
use lazyui_ratatui::{Window, arrow_down};
use lazyui_ratatui_fuzz::{
    AnyBuffer, AnyRect, assert_clean, assert_drawn, assert_kept, corners, text,
};
use libfuzzer_sys::fuzz_target;
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::Style;
use ratatui::widgets::Widget;

/// A text on row `y` of the window drawn on `drawn`: there if it fits beside a `─` and the
/// corner on each side, and those stay.
fn check_border(buf: &Buffer, drawn: Rect, y: u16, label: &str) {
    if drawn.width < 3 {
        return;
    }
    assert_eq!(buf[(drawn.x + 1, y)].symbol(), "─", "{drawn:?}: {label:?}");
    assert_eq!(buf[(drawn.right() - 2, y)].symbol(), "─", "{drawn:?}: {label:?}");
    let cells = width(label);
    if cells > 0 && cells + 4 <= usize::from(drawn.width) {
        let x = drawn.right() - 2 - u16::try_from(cells).expect("fits in a window");
        assert_drawn(buf, x, y, &sanitize(label));
    }
}

fuzz_target!(|input: (AnyBuffer, AnyRect, bool, [Vec<u16>; 3], u16, AnyRect, AnyRect)| {
    let (buffer, area, far, [key, name, note], x, upper, lower) = input;
    let mut buf = buffer.buffer();
    let area = area.near(far);
    let (key, name, note) = (text(&key), text(&name), text(&note));

    let window = Window::new().name(&key, &name).note(&note);
    let inner = window.inner(area);
    assert!(inner.x >= area.x && inner.y >= area.y, "{area:?}: {inner:?}");
    assert!(
        inner.right() <= area.right() && inner.bottom() <= area.bottom(),
        "{area:?}: {inner:?}"
    );

    let before = buf.clone();
    window.render(area, &mut buf);
    assert_kept(&before, &buf, |p| area.contains(p));
    assert_clean(&buf);
    let drawn = area.intersection(buf.area);
    if drawn.width >= 2 && drawn.height >= 2 {
        for (at, corner) in corners(drawn) {
            assert_eq!(buf[at].symbol(), corner, "{drawn:?}");
        }
        let label = match (key.as_str(), name.as_str()) {
            (_, "") => String::new(),
            ("", name) => name.to_owned(),
            (key, name) => format!("[{key}] {name}"),
        };
        check_border(&buf, drawn, drawn.y, &label);
        check_border(&buf, drawn, drawn.bottom() - 1, &note);
    }

    // On a buffer of its own, with the two windows the arrow joins.
    let mut buf = buffer.buffer();
    let (upper, lower) = (upper.near(far), lower.near(far));
    let x = if far { u16::MAX - x % 128 } else { x % 64 };
    Window::new().render(upper, &mut buf);
    Window::new().render(lower, &mut buf);
    let before = buf.clone();
    arrow_down(&mut buf, x, upper, lower, Style::new());
    let top = upper.bottom().wrapping_sub(1);
    assert_kept(&before, &buf, |p: Position| p.x == x && p.y >= top && p.y <= lower.y);
    if buf != before {
        assert_eq!(buf[(x, top)].symbol(), "┬");
        assert_eq!(buf[(x, lower.y)].symbol(), "▼");
        for y in top + 1..lower.y {
            assert_eq!(buf[(x, y)].symbol(), "│");
        }
        for window in [upper, lower].map(|w| w.intersection(buf.area)) {
            if window.width >= 2 && window.height >= 2 {
                for (at, _) in corners(window) {
                    assert_eq!(buf[at], before[at], "{window:?}");
                }
            }
        }
    }
});
