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
    // Off the markers, on the text shown or just after it; only a grapheme wider than two
    // cells that does not fit can leave a gap between that text and the cursor.
    assert!(v.cursor < w - right, "{what}");
    assert!(left <= v.cursor, "{what}");
    let narrow = s.graphemes(true).all(|g| width(g) <= 2);
    assert!(!narrow || v.cursor <= left + shown, "{what}");

    // A piece of the text, cut between graphemes.
    let start = v.shown.as_ptr() as usize - s.as_ptr() as usize;
    let end = start + v.shown.len();
    let (before, after) = (&s[..start], &s[end..]);
    assert_eq!(s, format!("{before}{}{after}", v.shown), "{what}");
    let bounds: Vec<usize> = s.grapheme_indices(true).map(|(i, _)| i).collect();
    for cut in [start, end] {
        assert!(cut == s.len() || bounds.contains(&cut), "{what}");
    }

    // A marker means there is something to see on its side; from 3 cells on, whatever is
    // cut off there gets one.
    assert!(!v.left || width(before) > 0, "{what}");
    assert!(!v.right || width(after) > 0, "{what}");
    if w >= 3 {
        assert!(v.left || width(before) == 0, "{what}");
        assert!(v.right || width(after) == 0, "{what}");
    }

    // With the cursor on a grapheme boundary inside what is shown, its column is that of
    // the text before it.
    let at = s.char_indices().nth(cursor).map_or(s.len(), |(i, _)| i);
    if (start..=end).contains(&at) && (at == s.len() || bounds.contains(&at)) {
        assert_eq!(v.cursor, left + width(&s[start..at]), "{what}");
    }
});
