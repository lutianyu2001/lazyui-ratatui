//! `Field::view`: what it shows fits, is a piece of the text, and the markers and the cursor
//! tell the truth about it.
#![no_main]

use lazyui_ratatui::Field;
use lazyui_ratatui::text::width;
use lazyui_ratatui_fuzz::text;
use libfuzzer_sys::fuzz_target;
use unicode_segmentation::UnicodeSegmentation;

fuzz_target!(|input: (Vec<u16>, u16, u8, bool)| {
    let (units, cursor, w, far) = input;
    let s = text(&units);
    // On the text or just past it, where a cursor is: anywhere, it would mostly be past it.
    let cursor = if far { usize::MAX } else { usize::from(cursor) % (s.chars().count() + 2) };
    let w = usize::from(w);
    let v = Field::view(&s, cursor, w);
    let what = format!("{s:?}, cursor {cursor}, {w} cells: {v:?}");
    let (left, right, shown) = (usize::from(v.left), usize::from(v.right), width(v.shown));

    assert!(left + shown + right <= w, "{what}");
    if w == 0 {
        assert_eq!((v.left, v.shown, v.right, v.cursor), (false, "", false, 0), "{what}");
        return;
    }
    // Off the markers, on the text shown or just after it.
    assert!(v.cursor < w - right, "{what}");
    assert!(left <= v.cursor && v.cursor <= left + shown, "{what}");

    // A piece of the text, cut between graphemes.
    let start = v.shown.as_ptr() as usize - s.as_ptr() as usize;
    let end = start + v.shown.len();
    let (before, after) = (&s[..start], &s[end..]);
    let bounds: Vec<usize> = s.grapheme_indices(true).map(|(i, _)| i).collect();
    for cut in [start, end] {
        assert!(cut == s.len() || bounds.contains(&cut), "{what}");
    }

    // What is drawn under the cursor, and the cells it takes.
    let at = s.char_indices().nth(cursor).map_or(s.len(), |(i, _)| i);
    let at =
        if at == s.len() { at } else { bounds.iter().copied().rfind(|&b| b <= at).unwrap_or(0) };
    let under = s[at..].graphemes(true).map(width).find(|&cells| cells > 0);

    // A marker means there is something to see on its side; with room for what is under the
    // cursor and both markers, whatever is cut off there gets one.
    assert!(!v.left || width(before) > 0, "{what}");
    assert!(!v.right || width(after) > 0, "{what}");
    if w >= under.unwrap_or(1).max(1) + 2 {
        assert!(v.left || width(before) == 0, "{what}");
        assert!(v.right || width(after) == 0, "{what}");
    }

    // The cursor is at the start of its grapheme, among the text shown, in the column of
    // the text before it.
    assert!((start..=end).contains(&at), "{what}");
    assert_eq!(v.cursor, left + width(&s[start..at]), "{what}");

    // What is under the cursor is shown, unless the field is too narrow for it.
    if let Some(cells) = under.filter(|&cells| cells <= w) {
        assert!(v.cursor + cells <= left + shown, "{what}");
    }
});
