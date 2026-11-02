//! `Window` and `arrow_down`: nothing panics, nothing is drawn off the area (or the arrow's
//! column), texts on the border leave the corners alone, and no control character reaches
//! the buffer.
#![no_main]

use lazyui_ratatui::{Window, arrow_down};
use lazyui_ratatui_fuzz::{AnyBuffer, AnyRect, assert_clean, assert_kept, text};
use libfuzzer_sys::fuzz_target;
use ratatui::layout::{Position, Rect};
use ratatui::style::Style;
use ratatui::widgets::Widget;

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
    // The corners of what was drawn are the border's, whatever the texts are.
    let drawn = area.intersection(buf.area);
    if drawn.width >= 2 && drawn.height >= 2 {
        let (right, bottom) = (drawn.right() - 1, drawn.bottom() - 1);
        for (cx, cy, corner) in [
            (drawn.x, drawn.y, "╭"),
            (right, drawn.y, "╮"),
            (drawn.x, bottom, "╰"),
            (right, bottom, "╯"),
        ] {
            assert_eq!(buf[(cx, cy)].symbol(), corner, "{drawn:?}");
        }
    }

    let (x, upper, lower): (u16, Rect, Rect) = (x % 64, upper.near(far), lower.near(far));
    let before = buf.clone();
    arrow_down(&mut buf, x, upper, lower, Style::new());
    let column =
        |p: Position| p.x == x && p.y >= upper.bottom().saturating_sub(1) && p.y <= lower.y;
    assert_kept(&before, &buf, column);
});
