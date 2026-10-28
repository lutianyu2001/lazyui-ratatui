# lazyui-ratatui

<p align="right">
    <a href="../README.md">English</a> | <strong>中文</strong>
</p>

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

一个小型组件库，为 [Ratatui](https://ratatui.rs) 提供 lazygit 风格的面板：标有焦点键和名称的圆角窗口、角落注释、窗口之间的箭头，以及可滚动的单行文本框。

## 目录

- [概述](#概述)
- [功能特性](#功能特性)
- [前置要求](#前置要求)
- [安装](#安装)
- [用法](#用法)
- [API](#api)
- [测试](#测试)
- [兼容性](#兼容性)
- [免责声明](#免责声明)
- [许可证](#许可证)
- [作者](#作者)

## 概述

这些组件补上了 lazygit 风格界面里 Ratatui 自带的 `Block` 和 `Paragraph` 留给程序自己处理的部分：与焦点键对应的窗口名称、底部边框上的注释、窗口之间的箭头，以及随光标滚动的文本框。它们和其他 Ratatui 组件一样绘制到 `Buffer` 上，只依赖 Ratatui 和 unicode-segmentation。

```text
╭────────[1] Files─╮   Window::new().name("1", "Files").note("[2 marked]")
│                  │
╰───────[2 marked]─╯

    ╰─┬────╯           arrow_down(buf, x, upper, lower, style)
╭─────▼────╮

<56789a>               Field::new(text, cursor).render(area, buf)
```

注：名字里的 "Lazy" 指 lazygit，与惰性求值无关。

## 功能特性

- 圆角窗口：右上角显示焦点键和名称（`[1] Files`），右下角可加一条注释，两者都使用边框的样式
- 箭头：把一个窗口连到它下方的窗口（`┬`、`│`、`▼`）
- 单行文本框：文本过长时滚动以保持光标可见，并用高亮的 `<` / `>` 标出被截断的一侧，与 readline 的 horizontal-scroll-mode 相同
- 按字素簇计算宽度，与 Ratatui 绘制文本的方式一致，中日韩文字和 emoji 都能对齐
- 任意尺寸下都安全：区域为空或超出 buffer 时会被裁剪，不会 panic

## 前置要求

- Rust ≥1.88
- Ratatui 0.30

## 安装

```bash
cargo add lazyui-ratatui
```

或手动加到 `Cargo.toml`：

```toml
[dependencies]
lazyui-ratatui = "0.1"
```

## 用法

带文本框的焦点窗口：

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

用箭头连接的两个窗口：

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

`Field::view` 只计算文本框的布局而不绘制，适合自行绘制文本的程序：

```rust
use lazyui_ratatui::Field;

// "0123456789" with the cursor at the end, in a field 6 cells wide.
let view = Field::view("0123456789", 10, 6);
assert!(view.left && !view.right);
assert_eq!((view.shown, view.cursor), ("6789", 5));
```

## API

| 项 | 说明 |
|------|-------------|
| `Window` | 圆角窗口；`name(key, name)`、`note(text)`、`border_style(style)`、`inner(area)` |
| `arrow_down` | 在第 `x` 列从一个窗口向下画 `┬` / `│` / `▼` 到下一个窗口；`x` 不在两个边框上时什么都不画 |
| `Field` | 单行文本框；`new(text, cursor)` 的光标是字符索引，`render(area, buf)` 返回终端光标应在的位置 |
| `FieldView` | `Field::view(text, cursor, width)` 的返回值：可见文本、两侧标记和光标所在列 |
| `text::width` | 字符串在屏幕上占的格数 |
| `text::take_width` | 字符串能放进指定格数的最长前缀 |
| `text::grapheme_width` | 单个字素簇占的格数；控制字符为 0，因为 Ratatui 会跳过它们 |

完整文档见 [docs.rs](https://docs.rs/lazyui-ratatui)。

## 测试

```bash
cargo test
cargo clippy --all-targets
```

README 里的示例同样写在 crate 文档中，在那里作为 doctest 编译并运行；`tests/readme.rs` 检查两处的代码一致。

crate 禁止 `unsafe` 代码，Clippy 拒绝没有写明如何处理溢出的算术。`tests/no_panic.rs` 用
[proptest](https://crates.io/crates/proptest) 把随机文本画进随机的区域和缓冲区（其中一些
紧挨 `u16::MAX`），检查没有 panic、也没有画到区域之外。设置 `PROPTEST_CASES` 可以跑比默认
256 个更多的用例。

## 兼容性

测试环境：

- Ubuntu 24.04.4 LTS
  - Rust 1.99.1
  - [Ratatui](https://ratatui.rs) 0.30.2

## 免责声明

This project is not affiliated with, endorsed by, or sponsored by lazygit or its maintainers. "lazygit" refers to the software developed by Jesse Duffield (https://github.com/jesseduffield/lazygit) and is referenced here solely for descriptive purposes.

This project is not affiliated with, endorsed by, or sponsored by Ratatui or its maintainers. "Ratatui" refers to the software developed by The Ratatui Developers (https://github.com/ratatui/ratatui) and is referenced here solely for descriptive purposes.

## 许可证

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

## 作者

Tianyu (Sky) Lu
