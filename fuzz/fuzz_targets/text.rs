//! `text::width`, `text::take_width` and `text::grapheme_width` against what ratatui draws,
//! and against the definition of each; `text::sanitize` keeps widths and has nothing left to do
//! on what it returns.
#![no_main]

use lazyui_ratatui::text::{grapheme_width, sanitize, take_width, width};
use lazyui_ratatui_fuzz::{assert_clean, reaches_no_terminal, text};
use libfuzzer_sys::fuzz_target;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use unicode_segmentation::UnicodeSegmentation;

fuzz_target!(|input: (Vec<u16>, u16)| {
    let (units, limit) = input;
    let s = text(&units);
    let graphemes: Vec<&str> = s.graphemes(true).collect();
    let total = width(&s);

    // Ratatui's own count of the cells it took, on a row wide enough for all of it.
    if let Ok(cells) = u16::try_from(total)
        && cells < 4096
    {
        let mut buf = Buffer::empty(Rect::new(0, 0, cells + 1, 1));
        let (end, _) = buf.set_stringn(0, 0, &s, usize::MAX, Style::new());
        assert_eq!(usize::from(end), total, "{s:?}");
        assert_clean(&buf);
    }

    // Cell for cell, with nothing left that takes no cell or that terminals measure otherwise.
    let clean = sanitize(&s);
    assert_eq!(width(&clean), total, "{s:?}: {clean:?}");
    assert_eq!(sanitize(&clean), clean, "{s:?}");
    assert!(clean.graphemes(true).all(|g| grapheme_width(g) > 0), "{s:?}: {clean:?}");
    assert!(!clean.chars().any(reaches_no_terminal), "{s:?}: {clean:?}");

    // The longest prefix that fits, in whole graphemes.
    let limit = usize::from(limit % 128);
    let taken = take_width(&s, limit);
    assert!(s.starts_with(taken));
    let used = width(taken);
    assert!(used <= limit, "{s:?}, {limit}: {taken:?}");
    let count = taken.graphemes(true).count();
    assert_eq!(graphemes[..count].concat(), taken, "{s:?}, {limit}");
    if let Some(next) = graphemes.get(count) {
        assert!(used + grapheme_width(next) > limit, "{s:?}, {limit}: {taken:?}");
    }
});
