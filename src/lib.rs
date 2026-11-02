//! Lazygit-style windows for Ratatui.
//!
//! - [`Window`]: rounded borders, the key that focuses the window and its name in the
//!   top right corner (`[1] Files`), a note in the bottom right corner.
//! - [`arrow_down`]: joins a window to one below it.
//! - [`Field`]: a one-line text field that scrolls to keep its cursor visible and marks
//!   text it cuts off with `<` and `>`, as readline's horizontal-scroll-mode does.
//! - [`text`]: the widths these measure text in, the way Ratatui draws it, and
//!   [`text::sanitize`], which they draw text through.
//!
//! # Examples
//!
//! The READMEs show these too; `tests/readme.rs` checks that they show them unchanged.
//!
//! ```rust
//! use lazyui_ratatui::{Field, Window};
//! use ratatui::Frame;
//! use ratatui::layout::Rect;
//! use ratatui::style::{Color, Style};
//!
//! /// A focused `[1] Search` window with a text field in it.
//! fn draw_search(frame: &mut Frame, area: Rect, text: &str, cursor: usize) {
//!     let window = Window::new().name("1", "Search").border_style(Style::new().fg(Color::Green));
//!     let inner = window.inner(area);
//!     frame.render_widget(window, area);
//!     let at = Field::new(text, cursor).render(inner, frame.buffer_mut());
//!     frame.set_cursor_position(at);
//! }
//! ```
//!
//! ```rust
//! use lazyui_ratatui::{Window, arrow_down};
//! use ratatui::buffer::Buffer;
//! use ratatui::layout::Rect;
//! use ratatui::style::Style;
//! use ratatui::widgets::Widget;
//!
//! let mut buf = Buffer::empty(Rect::new(0, 0, 20, 6));
//! let (upper, lower) = (Rect::new(0, 0, 20, 3), Rect::new(0, 3, 20, 3));
//! Window::new().name("1", "Address").render(upper, &mut buf);
//! Window::new().name("2", "Files").note("[2 marked]").render(lower, &mut buf);
//! arrow_down(&mut buf, 4, upper, lower, Style::new());
//! ```
//!
//! ```rust
//! use lazyui_ratatui::Field;
//!
//! // "0123456789" with the cursor at the end, in a field 6 cells wide.
//! let view = Field::view("0123456789", 10, 6);
//! assert!(view.left && !view.right);
//! assert_eq!((view.shown, view.cursor), ("6789", 5));
//! ```

#![forbid(unsafe_code)]

mod field;
#[cfg(kani)]
mod proofs;
pub mod text;
mod unsteady;
mod window;

pub use field::{Field, FieldView};
pub use window::{Window, arrow_down};
