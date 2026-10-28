# lazyui-ratatui

<p align="right">
    <strong>English</strong> | <a href="./README/README_CN.md">中文</a>
</p>

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

A small widget library that brings lazygit-style panels to [Ratatui](https://ratatui.rs):
rounded windows named with their focus key, corner notes, arrows between windows and
scrolling one-line text fields.

## Table of Contents

- [Overview](#overview)
- [Features](#features)
- [Prerequisites](#prerequisites)
- [Installation](#installation)
- [Usage](#usage)
- [API](#api)
- [Testing](#testing)
- [Compatibility](#compatibility)
- [Disclaimer](#disclaimer)
- [License](#license)
- [Author](#author)

## Overview

The widgets cover the parts of a lazygit-style screen that Ratatui's own `Block` and
`Paragraph` leave to the program: window names tied to focus keys, notes on the
bottom border, arrows from one window to another and a text field that scrolls with
its cursor. They draw into a Ratatui `Buffer` like any other widget and depend only on
Ratatui and unicode-segmentation.

```text
╭────────[1] Files─╮   Window::new().name("1", "Files").note("[2 marked]")
│                  │
╰───────[2 marked]─╯

    ╰─┬────╯           arrow_down(buf, x, upper, lower, style)
╭─────▼────╮

<56789a>               Field::new(text, cursor).render(area, buf)
```

Note: "Lazy" in the name refers to lazygit, not lazy evaluation.

## Features

- Rounded windows with the focus key and name in the top right corner (`[1] Files`)
  and an optional note in the bottom right corner, both in the border's style
- Arrows that join a window to one below it (`┬`, `│`, `▼`)
- One-line text fields that scroll to keep the cursor visible and mark cut-off text
  with a highlighted `<` / `>`, as readline's horizontal-scroll-mode does
- Widths measured per grapheme, the way Ratatui draws text, so CJK text and emoji
  line up
- Safe at any size: areas that are empty or reach outside the buffer are clipped
  instead of causing a panic

## Prerequisites

- Rust ≥1.88
- Ratatui 0.30

## Installation

```bash
cargo add lazyui-ratatui
```

Or add it to `Cargo.toml` by hand:

```toml
[dependencies]
lazyui-ratatui = "0.1"
```

## Usage

A focused window with a text field in it:

```rust
use lazyui_ratatui::{Field, Window};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};

/// A focused `[1] Search` window with a text field in it.
fn draw_search(frame: &mut Frame, area: Rect, text: &str, cursor: usize) {
    let window = Window::new().name("1", "Search").border_style(Style::new().fg(Color::Green));
    let inner = window.inner(area);
    frame.render_widget(window, area);
    let at = Field::new(text, cursor).render(inner, frame.buffer_mut());
    frame.set_cursor_position(at);
}
```

Two windows joined by an arrow:

```rust
use lazyui_ratatui::{Window, arrow_down};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Widget;

let mut buf = Buffer::empty(Rect::new(0, 0, 20, 6));
let (upper, lower) = (Rect::new(0, 0, 20, 3), Rect::new(0, 3, 20, 3));
Window::new().name("1", "Address").render(upper, &mut buf);
Window::new().name("2", "Files").note("[2 marked]").render(lower, &mut buf);
arrow_down(&mut buf, 4, upper, lower, Style::new());
```

`Field::view` gives a field's layout without drawing it, for programs that draw text
their own way:

```rust
use lazyui_ratatui::Field;

// "0123456789" with the cursor at the end, in a field 6 cells wide.
let view = Field::view("0123456789", 10, 6);
assert!(view.left && !view.right);
assert_eq!((view.shown, view.cursor), ("6789", 5));
```

## API

| Item | Description |
|------|-------------|
| `Window` | Rounded window; `name(key, name)`, `note(text)`, `border_style(style)`, `inner(area)` |
| `arrow_down` | Draws `┬` / `│` / `▼` at column `x` from one window down to the next; does nothing if `x` is not on both borders |
| `Field` | One-line text field; `new(text, cursor)` takes the cursor as a char index, `render(area, buf)` returns where the terminal cursor goes |
| `FieldView` | What `Field::view(text, cursor, width)` returns: the visible text, both markers and the cursor's column |
| `text::width` | Cells a string takes on screen |
| `text::take_width` | The longest prefix of a string that fits in a number of cells |
| `text::grapheme_width` | Cells one grapheme takes; 0 for control characters, which Ratatui skips |

Full documentation is on [docs.rs](https://docs.rs/lazyui-ratatui).

## Testing

```bash
cargo test
cargo clippy --all-targets
```

The examples in this README are also in the crate documentation, where they are
compiled and run as doctests; `tests/readme.rs` checks that both places show the same
code.

The crate forbids `unsafe` code, and Clippy rejects arithmetic that doesn't say how it
handles overflow. `tests/no_panic.rs` uses [proptest](https://crates.io/crates/proptest)
to draw random text into random areas and buffers, some next to `u16::MAX`, and checks
that nothing panics and nothing is drawn outside the area. Set `PROPTEST_CASES` to run
more cases than the default 256.

## Compatibility

Tested on:

- Ubuntu 24.04.4 LTS
  - Rust 1.99.1
  - [Ratatui](https://ratatui.rs) 0.30.2

## Disclaimer

This project is not affiliated with, endorsed by, or sponsored by lazygit or its maintainers. "lazygit" refers to the software developed by Jesse Duffield (https://github.com/jesseduffield/lazygit) and is referenced here solely for descriptive purposes.

This project is not affiliated with, endorsed by, or sponsored by Ratatui or its maintainers. "Ratatui" refers to the software developed by The Ratatui Developers (https://github.com/ratatui/ratatui) and is referenced here solely for descriptive purposes.

## License

Copyright 2026 Tianyu (Sky) Lu

The entire repository is licensed under the Apache License, Version 2.0 
(the "License"); you may not use any file in this repository except in 
compliance with the License. You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.

## Author

Tianyu (Sky) Lu
