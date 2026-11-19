//! `Message`: nothing panics, nothing is drawn off its window but in the column before it,
//! where only a wide character goes, the window is on the screen, no wider than asked, and
//! keeps its corners and borders, a text that needs no wrapping is there line by line, a
//! window too small for its text is full of it, scrolling stops at the last screenful, and
//! no control character reaches the buffer.
#![no_main]

use lazyui_ratatui::text::{sanitize, width};
use lazyui_ratatui::{Message, Window};
use lazyui_ratatui_fuzz::{
    AnyBuffer, AnyRect, assert_clean, assert_drawn, assert_kept, corners, text,
};
use libfuzzer_sys::fuzz_target;
use ratatui::style::Style;
use ratatui::widgets::Widget;

type Input = (AnyBuffer, AnyRect, bool, Vec<u16>, Vec<Vec<u16>>, (u16, u8), Vec<u16>);

fuzz_target!(|input: Input| {
    let (buffer, screen, far, title, lines, (wide, scroll), under) = input;
    let mut buf = buffer.buffer();
    let screen = screen.near(far);
    let title = text(&title);
    // A line break in a line makes two of it.
    let body = lines.iter().take(24).map(|line| text(line)).collect::<Vec<_>>().join("\n");
    let wide = if far { wide } else { wide % 96 };
    let scroll = usize::from(scroll);
    let message = Message::new(&body).title(&title).width(wide).scroll(scroll);

    // Over text already there, wide characters among it.
    for y in buf.area.top()..buf.area.bottom() {
        buf.set_stringn(buf.area.x, y, text(&under), usize::from(buf.area.width), Style::new());
    }
    let before = buf.clone();
    message.clone().render(screen, &mut buf);
    let drawn = screen.intersection(buf.area);
    let area = message.area(drawn);
    assert_eq!(area.intersection(drawn), area, "{area:?} on {drawn:?}");
    assert!(area.width <= wide, "{area:?} wider than {wide}");
    assert_kept(&before, &buf, |p| {
        let cut = p.x.checked_add(1) == Some(area.x) && p.y >= area.y && p.y < area.bottom();
        area.contains(p) || (cut && width(before[p].symbol()) > 1 && buf[p].symbol() == " ")
    });
    assert_clean(&buf);
    let inner = Window::new().inner(area);
    if area.width >= 2 && area.height >= 2 {
        for (at, corner) in corners(area) {
            assert_eq!(buf[at].symbol(), corner, "{area:?}");
        }
        for y in inner.y..inner.bottom() {
            assert_eq!(buf[(area.x, y)].symbol(), "│", "{area:?}");
            assert_eq!(buf[(area.right() - 1, y)].symbol(), "│", "{area:?}");
        }
    }

    // The window is no larger than the text needs, and where it holds less, it is full.
    let overflow = message.overflow(drawn);
    let fits = body.lines().all(|line| width(line) <= usize::from(inner.width));
    if inner.width > 0 && fits {
        // Nothing to wrap: the lines of the text are the rows, from the one scrolled to.
        let rows = body.lines().count();
        assert_eq!(overflow, rows.saturating_sub(usize::from(inner.height)), "{area:?}");
        assert!(usize::from(inner.height) <= rows, "{} rows for {rows}", inner.height);
        let first = scroll.min(overflow);
        for (line, y) in body.lines().skip(first).zip(inner.y..inner.bottom()) {
            assert_drawn(&buf, inner.x, y, &sanitize(line));
        }
    }
    if overflow > 0 {
        assert_eq!(area.height, drawn.height, "{overflow} rows left out of {area:?}");
    }
    // Asked to scroll past the last screenful, it shows that one.
    if scroll >= overflow {
        let mut last = before.clone();
        message.scroll(overflow).render(screen, &mut last);
        assert_eq!(buf, last);
    }
});
