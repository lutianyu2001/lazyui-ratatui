# lazyui-ratatui

<p align="right">
    <a href="../README.md">English</a> | <strong>中文</strong>
</p>

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![audit](https://img.shields.io/endpoint?url=https%3A%2F%2Fraw.githubusercontent.com%2Flutianyu2001%2Flazyui-ratatui%2Fbadges%2Faudit.json)](../SECURITY.md#ci)
[![pins](https://img.shields.io/endpoint?url=https%3A%2F%2Fraw.githubusercontent.com%2Flutianyu2001%2Flazyui-ratatui%2Fbadges%2Fpins.json)](../xtask/src/pins.rs)

一个小型组件库，为 [Ratatui](https://ratatui.rs) 提供 lazygit 风格的面板：标有焦点键和名称的圆角窗口、角落注释、窗口之间的箭头，以及可滚动的单行文本框。

## 目录

- [概述](#概述)
- [功能特性](#功能特性)
- [安全审计](#安全审计)
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
- 按字素簇计算宽度，与 Ratatui 绘制文本的方式一致，中日韩文字和单个 emoji 能够对齐；终端算法不同的文本在绘制时会被替换，所以组合 emoji 和国旗会显示得朴素一些（见 [SECURITY.md](../SECURITY.md)）
- 任意尺寸下都安全：区域为空或超出 buffer 时会被裁剪，不会 panic

## 安全审计

[![deny](https://img.shields.io/endpoint?url=https%3A%2F%2Fraw.githubusercontent.com%2Flutianyu2001%2Flazyui-ratatui%2Fbadges%2Fdeny.json)](https://github.com/lutianyu2001/lazyui-ratatui/actions/workflows/advisories.yml)
[![vet](https://img.shields.io/endpoint?url=https%3A%2F%2Fraw.githubusercontent.com%2Flutianyu2001%2Flazyui-ratatui%2Fbadges%2Fvet.json)](../audit/vet)
[![geiger](https://img.shields.io/endpoint?url=https%3A%2F%2Fraw.githubusercontent.com%2Flutianyu2001%2Flazyui-ratatui%2Fbadges%2Fgeiger.json)](../audit/geiger)
[![miri](https://img.shields.io/endpoint?url=https%3A%2F%2Fraw.githubusercontent.com%2Flutianyu2001%2Flazyui-ratatui%2Fbadges%2Fmiri.json)](../audit/miri)
[![coverage](https://img.shields.io/endpoint?url=https%3A%2F%2Fraw.githubusercontent.com%2Flutianyu2001%2Flazyui-ratatui%2Fbadges%2Fcoverage.json)](../audit/coverage)
[![kani](https://img.shields.io/endpoint?url=https%3A%2F%2Fraw.githubusercontent.com%2Flutianyu2001%2Flazyui-ratatui%2Fbadges%2Fkani.json)](../audit/kani)
[![fuzz](https://img.shields.io/endpoint?url=https%3A%2F%2Fraw.githubusercontent.com%2Flutianyu2001%2Flazyui-ratatui%2Fbadges%2Ffuzz.json)](../audit/fuzz)

这是项目维护者自己做的审计，不是第三方审计，也不保证这个 crate 没有 bug。它针对的是下面写明的提交里 `Cargo.lock` 记录的版本。

这个 crate 只画文本：不起进程，不开文件和 socket。所以审计要回答两个问题：能控制程序所画文本的人能用它做什么，以及尺寸到了终端不该有的值时会发生什么。

| 审计 | 做了什么 | 结果 |
|---|---|---|
| 威胁模型 | 来自他人的文本：终端转义序列、文字重排、超出窗口、超长文本；超出范围的尺寸和光标 | 发现并修复 7 个问题；遗留问题见下 |
| `unsafe` 代码 | 本 crate 禁止使用 | 没有 |
| 依赖 | 运行和构建时的 40 个 crate：用 cargo-deny 查来源、许可证和已知漏洞，用 cargo-vet 管审计记录 | 没有安全公告；全部 82 个里 27 个有他人的审计，55 个由维护者豁免 |
| Miri | 在全部四个模块的 27 个单元测试里跑了 26 个，查未定义行为；剩下那个测超长文本，要跑几小时，跑通过一次 | 未发现 |
| 模糊测试 | 4 个 target，本机每个一小时，每次发版时在 GitHub Actions 上每个四小时：文本宽度、输入框布局、画进 buffer 的输入框、画进 buffer 的窗口和箭头，并检查各自应画出的内容 | 每个一小时共约 7800 万次运行（2026-11-02），没有失败 |
| Kani 证明 | 7 个布局算术的证明，对整数的每个取值成立，没有界限 | 7 个全部通过 |
| 分支覆盖率 | 测试，全部四个模块 | 90 个分支走到 86 个；其余 4 个是测试自身断言失败的一侧 |
| 评审 | 代码、威胁模型和审计工具本身都与编写分开评审，并在相同输入上对比了新旧代码 | 发现的问题已修复，或写入 SECURITY.md |

下面的证明都不限制整数的取值，取自 2026-11-02 在提交 `9470670` 上用 Kani 0.68.0 的运行：

| 证明 | 成立的性质 | 耗时（秒） | 峰值内存（MiB） |
|---|---|---|---|
| `a_text_on_the_border_leaves_the_corners_alone` | 名称或注释正好占自身宽度，两侧各留一个 `─` 和边角 | <1 | 22 |
| `an_arrow_stays_on_the_screen` | 箭头的每一行都在屏幕内 | <1 | 26 |
| `an_arrow_joins_the_borders_of_both_windows` | 箭头从上窗口的下边框连到下窗口的上边框，不碰边角 | <1 | 46 |
| `an_arrow_is_drawn_when_nothing_stops_it` | 两条边框都在屏幕内且不在边角时，箭头一定画出 | 1 | 43 |
| `the_cursor_goes_on_the_field` | 无论列号是多少，光标都在输入框上 | <1 | 22 |
| `a_scrolled_field_uses_its_cells_exactly` | 标记和文本的宽度之和正好等于输入框宽度 | <1 | 22 |
| `a_scrolled_cursor_keeps_off_the_markers` | 滚动后的光标不会落在标记上，光标下的内容也放得下，不被右侧标记挡住 | 18 | 22 |

这些结果能说明什么、不能说明什么：

- Rust 排除了本 crate 里的内存错误，但不排除逻辑错误，后者要靠测试、模糊测试和证明。
- 依赖里的 `unsafe` 代码不受本 crate 控制：数量写在 [SUPPLY_CHAIN.md](../SUPPLY_CHAIN.md) 里，安全性靠上游作者和已有的审计记录。Ratatui 本身在 55 个豁免之列，所以这里没有它的审计记录。
- 模糊测试只是抽样，证明不了什么。Kani 证明只覆盖整数：文本要经过 Unicode 查找表，Kani 处理不了，所以文本方面没有任何证明。
- `Field` 和 `Window` 会替换终端与 Ratatui 算法不同的文本，这样它就跑不出窗口。这一点只在 tmux 3.4 上测过：40 万条随机文本里，替换前有 140,589 条不一致，替换后没有。其他终端，以及程序用别的组件绘制、没经过 `text::sanitize` 的文本，不在覆盖范围内，见 [SECURITY.md](../SECURITY.md#known-remaining-risks)。
- `FieldView::shown` 和 `take_width` 原样返回文本，包括控制字符；形近字符也按原样绘制。留给程序处理的完整清单见[SECURITY.md](../SECURITY.md#what-the-crate-leaves-to-the-program)。

每项审计的结果和代码一起提交在 `audit/<task>/` 里，运行方法见 SECURITY.md。本节开头的 badge 显示每项结果对 `main` 上
现在的代码是否仍然有效（绿色）、已经过期（黄色）还是失败（红色）；标题下的 audit badge 是它们的汇总，旁边的 pins badge 显示固定的工具和 action 是否该升级了。耗时可预期的
检查在每次发版时由 GitHub Actions 自动运行；Kani 和 Miri 手动运行，因为一处改动就可能让它们跑上几小时。审计过期不影响发版。

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
lazyui-ratatui = "0.2"
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
| `text::width` | 字符串在 `Buffer` 里占的格数 |
| `text::take_width` | 字符串能放进指定格数的最长前缀 |
| `text::grapheme_width` | 单个字素簇占的格数；控制字符为 0，因为 Ratatui 会跳过它们 |
| `text::sanitize` | 终端能按预期宽度绘制的文本，占的格数不变；用于其他组件要画的文本 |

完整文档见 [docs.rs](https://docs.rs/lazyui-ratatui)。

## 测试

```bash
cargo test
cargo clippy --all-targets
```

README 里的示例和 crate 文档里的一样，作为 doctest 编译并运行。

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

```plaintext
http://www.apache.org/licenses/LICENSE-2.0
```

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.

## 作者

Tianyu (Sky) Lu
