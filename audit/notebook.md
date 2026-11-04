# lazyui-ratatui Security Audit

## Environment / HPC Node Information

- Ubuntu 24.04, Linux 6.8.0, 31 GB RAM, 99 GB swap
- Rust 1.99.1 (stable), nightly-2026-10-28
- Kani 0.68.0 (CBMC 6.11.0), cargo-fuzz 0.13.2, cargo-deny 0.20.2, cargo-vet 0.10.3, cargo-geiger 0.14.0,
  cargo-llvm-cov 0.9.1, tmux 3.4
- Ratatui 0.30.2, ratatui-core 0.1.2; 84 packages in `Cargo.lock`, 52 in `fuzz/Cargo.lock`
- Every heavy task ran in a `systemd-run --user --scope` with `MemoryMax`, one at a time

All commands below were run from the repository root, on branch `security-audit`. The latest logs of each task are
in `audit/<task>/`.

## Task 1: Threat Model and Review

###### Motivation
- **Why**: The crate is published on crates.io with no security evidence. Most of what matters for a widget library
  is not memory safety but what untrusted text and odd sizes can do.
- **Source**: The user's `/rust-security-audit` request, 2026-11-02.

#### Findings Fixed

| Finding | Input | Found by | Fix |
|---|---|---|---|
| Cursor column past the field | `Field::view("⌚\u{fe0e}", 1, 2)` | fuzz, probe | `cells_before` sums whole graphemes |
| Text over a corner | 65,532+ cells in a window 65,535 wide | Kani | `corner_place` uses `checked_add(4)` |
| Cursor on the `<` marker | scrolled field | fuzz | as the first |
| Cursor moving left as it moves right | cursor inside a grapheme | review | as the first |
| Arrow on the corner of a clipped window | window cut off by the buffer | security review | `arrow_ends` clips to the screen |

The threat model, what the crate leaves to programs and the remaining risks are in `SECURITY.md`.

#### Verification
Passed: 2 of 2 mutants of the window oracle caught (arrow without clipping, corner off by one), and the unfixed
`view` bug caught by the strengthened `view` target. The old and new code were compared on about 1.1 billion
`view` and `render` calls, plus windows and arrows: 0 panics, and every difference was one of the fixes above
(cursor inside a grapheme, texts of 65,532 to 65,535 cells, arrows at clipped edges).

###### Note
The terminal width disagreement was reproduced on tmux 3.4: 92 bytes of text in a field 28 cells wide painted into
the next window, borders intact. It is not fixed: any Ratatui widget does the same.

> Revised in Task 6: fixed for this crate's fields and window corners on tmux 3.4.

## Task 2: Dependencies

###### Motivation
- **Why**: `forbid(unsafe_code)` does not reach dependencies, which are where this crate's `unsafe` code runs.
- **Source**: Step 3 of the audit.

```sh
cargo xtask deny
cargo xtask vet
cargo xtask geiger
```

Output (`audit/deny/run.log`, `audit/vet/run.log`):
```plaintext
advisories ok, bans ok, licenses ok, sources ok
advisories ok, sources ok
Vetting Succeeded (27 fully audited, 55 exempted)
```

###### Note
The one warning from `cargo deny` is two versions of `syn`, both only used at build time. Exemptions were not
certified in the user's name. `SUPPLY_CHAIN.md` has the `unsafe` counts and the code run at build time.

## Task 3: Fuzzing

###### Motivation
- **Why**: All external input of this crate is text, and Kani cannot reach it (Task 4.1).
- **Source**: Step 5 of the audit.

### 3.1 First Run

Run from a clean tree at `e9e8c25`:

```sh
cargo xtask all
```

| Target | Runs | Seconds |
|---|---|---|
| text | 34,709,580 | 3601 |
| view | 36,794,785 | 3601 |
| field | 17,373,328 | 3601 |
| window | 8,186,074 | 3601 |

No failure, but the commit's Kani run failed (Task 4.3), so this run was replaced by the next.

### 3.2 Second Run

Run at `8ed5335`:

```sh
cargo xtask all --if-needed
```

| Target | Runs | Seconds |
|---|---|---|
| text | 34,783,550 | 3601 |
| view | 35,431,682 | 3601 |
| field | 15,303,966 | 3601 |
| window | 7,741,869 | 3601 |

Total 93,261,067 runs, no failure, no artifact.

## Task 4: Kani

###### Motivation
- **Why**: Layout arithmetic near `u16::MAX` is where sampling is weakest, and it depends only on integers.
- **Source**: Step 6 of the audit.

### 4.1 [Failed] Proofs on Text

`Field::view` on real text goes through the tables of unicode-segmentation and unicode-width, which Kani cannot
get through.

<details>
<summary>Archived</summary>

A proof of `Field::view` on up to 3 ASCII characters ran out its 15 minutes at a peak of 7 GB. The exact command
was not kept. Text is left to fuzzing and property tests; the arithmetic was moved into functions of integers
(`scroll`, `scrolled_cursor`, `cursor_cell`, `corner_place`, `arrow_ends`).

</details>

### 4.2 Proofs of the Arithmetic

Seven proofs in `src/proofs.rs`, for every value of their integers, with no bound.

```sh
cargo xtask all --if-needed
```

Output (`audit/kani/harnesses.txt`, commit `8ed5335`, 9 s in all):
```plaintext
proofs::a_scrolled_cursor_keeps_off_the_markers: succeeded, 2 s, peak memory 21 MiB
proofs::a_scrolled_field_uses_its_cells_exactly: succeeded, 0 s, peak memory 22 MiB
proofs::a_text_on_the_border_leaves_the_corners_alone: succeeded, 0 s, peak memory 22 MiB
proofs::an_arrow_is_drawn_when_nothing_stops_it: succeeded, 1 s, peak memory 49 MiB
proofs::an_arrow_joins_the_borders_of_both_windows: succeeded, 1 s, peak memory 49 MiB
proofs::an_arrow_stays_on_the_screen: succeeded, 0 s, peak memory 27 MiB
proofs::the_cursor_goes_on_the_field: succeeded, 0 s, peak memory 21 MiB
```

#### Verification
Passed: both `kani::cover!` conditions are satisfied, so the arrow proofs are not vacuous, and none of the checks
reported `UNREACHABLE` is in this crate's code (2 are in `core::fmt`).

### 4.3 A Proof That Failed

`any_scroll` was widened so that the proofs would not depend on when `Field::view` scrolls, and the full audit was
started without running Kani on the change first. At `e9e8c25`:

```plaintext
Failed Checks: assertion failed: left <= column && column < w - right
 File: "src/proofs.rs", line 114, in proofs::a_scrolled_cursor_keeps_off_the_markers
VERIFICATION:- FAILED
```

`scrolled_cursor` added the marker's cell and then took off the skipped cells, so with `before == usize::MAX` the
addition saturated and the column came out as 0, on the marker. No text has that many cells, as a text's cells do
not exceed its bytes. The operations were swapped in `8ed5335`, which agrees with the old code for every `before`
that is reachable and needs no assumption in the proof, and a unit test was added.

## Task 5: Tests, Miri and Coverage

###### Motivation
- **Why**: Miri runs the `unsafe` code of the dependencies that the tests reach, and coverage shows which conditions
  no test tells apart.
- **Source**: Step 4 of the audit.

Miri at `8ed5335` (`audit/miri/run.log`):
```plaintext
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 56.83s
```

### 5.1 Property Tests That Drew Nothing

Branch coverage fell from 93.55% to 90.32% between two runs, when only a test had been added. The text report
showed why:

```sh
cargo +nightly-2026-10-28 llvm-cov report --text
```

Output, for `if area.is_empty()` in `Field::render`, in the binary of `tests/no_panic.rs`:
```plaintext
|  Branch (51:12): [True: 257, False: 0]
```

All 257 cases of `fields_stay_in_their_first_row` returned early: the buffer and the area were picked apart, and
rarely meet. Arrows were never drawn at all (`window.rs:102`). `bb7fab1` starts areas near their buffer and windows
in it:

```plaintext
|  Branch (51:12): [True: 204, False: 53]    field drawn in 53 of 257 cases
|  Branch (102:9): [True: 11, False: 246]    arrow drawn in 11 of 257 cases
```

###### Note
The final figure is in `audit/coverage/summary.txt`. The 4 branches left unreached are in `field.rs:226` and
`field.rs:231`, a test's own assertions failing. `Field::style` and `Window::border_style` are the 2 functions no
test here calls; the second is run by a doctest, which coverage does not count.

###### Analysis from AI
The audit gives real evidence for the crate's own arithmetic and for what it does with text, within what was
sampled; it says little about Ratatui, which nobody has audited for this project.

- Firm: no `unsafe` code in the crate; the integer layout holds for all values; 93 million fuzz runs found no failure
  with oracles that were shown to catch real bugs.
- Not settled: 55 exempted crates; other terminals than tmux 3.4; the width disagreement, which remains open.
- Change first: decide whether the crate should offer a way to clean text before drawing, since every program that
  shows untrusted text has to solve this on its own now.

> Revised in Task 6: the maintainer decided the disagreement must be fixed, and it was.

Two of this audit's mistakes were found only because a number moved: a proof changed and not rerun, and a property
test passing without drawing. A run that exercises nothing passes as quietly as one that holds.

## Task 6: Terminal Widths and the Hidden Character

###### Motivation
- **Why**: Task 1 left two things open: text could leave its window on a terminal that measures it differently, and
  a field could hide the character under the cursor. This task fixes both and re-runs the audit.
- **Source**: The maintainer's answers to the audit report, 2026-11-02: "3. 要修", "5. 得修", and "审计工具自身的小问题要修复并同步到
  skill".

#### Environment
- tmux 3.4, glibc 2.39 (Unicode 15)
- Python 3 scripts in the session's scratchpad, not kept in the repository

### 6.1 Measuring the Terminal

Inside a tmux pane, each scalar value was printed after an `X` and the cursor read back with a status report:

```plaintext
\r\x1b[KX<char>\x1b[6n
```

| Measure | Result |
|---|---|
| Scalars measured | 1,112,064 |
| Disagree with Ratatui | 64,768, in 130 ranges |

Most are unassigned code points; regional indicators and U+FFF9 to U+FFFB are among the rest. glibc's `wcwidth` does
not stand in for tmux: a lone regional indicator is 2 cells in tmux, 1 in Ratatui. Random strings then showed that
single characters are not enough, as U+FE0F widens the character before it, U+200D swallows the one after it, and
U+1F3FB to U+1F3FF join the one before them. Those made it 132 ranges, in `src/unsteady.rs`.

### 6.2 Sanitizing What Is Drawn

`text::sanitize` replaces each grapheme a terminal may measure differently with one of the same cells, and `Field`
and `Window` draw only its output; there is no way to turn it off. It keeps the width of the text, is idempotent,
drops zero-width graphemes and checks that neighbours do not join once something between them has gone. It takes
about 60 ms per MB.

The tmux measurement was then run on random text, on all planes, of length up to 16:

| Corpus | Texts | Disagree, raw | Disagree, sanitized |
|---|---|---|---|
| Without skin tones and U+200D handled | | | 23,111, then 7,377 |
| Final, second seed | 400,000 | 140,589 | 0 |

### 6.3 The Character Under the Cursor

`cells_before` became `cells_around`, which also gives the cells of the grapheme under the cursor, and `scroll` keeps
those cells in view. The left and right markers now need the cursor's cells plus 2. A grapheme wider than the field
counts as 1, so that no part of it is drawn.

#### Verification
Passed: each fix undone by hand was caught by the matching target within 45 s.

| Undone | Caught by |
|---|---|
| Under-cursor fix | view |
| Check that neighbours do not join | text |
| `Field` drawing raw text | field |
| `Window` drawing raw text | window |

### 6.4 The Audit's Tools

- A run fails if `HEAD` or the tree changes while it runs.
- `geiger` checks its table is complete, and `outdated` prints only what is printable from what it fetched.
- Recording errors fail the run, where they were ignored before.
- `verify.yml` is manual only, and its check job and the advisories jobs keep no credentials after checkout.

All of it is in the skill's `reference/`; `xtask/` of this repository differs from it only where marked per project.

### 6.5 Results

Run from a clean tree at `3f07829`, with Miri and Kani run again later for changes to their own tasks:

```sh
cargo xtask all --if-needed
```

| Target | Runs | Seconds |
|---|---|---|
| text | 7,123,895 | 3601 |
| view | 43,056,180 | 3601 |
| field | 18,853,225 | 3601 |
| window | 8,853,523 | 3601 |

Total 77,886,823 runs, no failure, no artifact. `text` runs about a fifth as often as before; its target now also
sanitizes each input and checks the result, which is the likely cause. Branch coverage 95.56%, 86 of 90.

Output (`audit/kani/harnesses.txt`, final run):
```plaintext
proofs::a_scrolled_cursor_keeps_off_the_markers: succeeded, 18 s, peak memory 22 MiB
proofs::a_scrolled_field_uses_its_cells_exactly: succeeded, 0 s, peak memory 22 MiB
proofs::a_text_on_the_border_leaves_the_corners_alone: succeeded, 0 s, peak memory 22 MiB
proofs::an_arrow_is_drawn_when_nothing_stops_it: succeeded, 1 s, peak memory 43 MiB
proofs::an_arrow_joins_the_borders_of_both_windows: succeeded, 0 s, peak memory 46 MiB
proofs::an_arrow_stays_on_the_screen: succeeded, 0 s, peak memory 26 MiB
proofs::the_cursor_goes_on_the_field: succeeded, 0 s, peak memory 22 MiB
```

###### Note
Miri took 12,313 s at `3f07829`, against 57 s before, because of `sanitize_holds_for_graphemes_of_any_length`, whose
two texts have 32,769 and 65,537 characters. It passed there, 25 of 27 tests. It is now in `SKIP`, and `unsteady::`,
missing from that run, was added to `MODULES`: 26 of 27 tests in 66 s. The 4 branches
left unreached are now at `field.rs:232` and `field.rs:237`, again a test's own assertions failing.

###### Analysis from AI
Text can no longer leave its window on tmux 3.4 through this crate's widgets, as far as 400,000 random texts can tell.

- Firm: 0 of 400,000 sanitized texts disagree on tmux 3.4, against 140,589 raw; the arithmetic proofs still hold with
  the cursor's cells added; 78 million fuzz runs found no failure.
- Not settled: every terminal but tmux 3.4 is unmeasured; `src/unsteady.rs` was measured against Ratatui 0.30.2 and
  has to be measured again after an upgrade; joined emoji, flags and U+FE0F sequences now show plainer.
- Change first: measure the terminals users actually run, such as kitty, WezTerm, Alacritty and iTerm2.

---

*Timestamp: 2026-11-02*

---

## Task 7: Running the Audit on GitHub Actions

###### Motivation
- **Why**: The audit ran only by hand, and nothing showed when a result had lapsed. The maintainer wants GitHub
  Actions used for every task whose length is known before it starts, the rest left to a person, and a badge per
  task.
- **Source**: The maintainer, 2026-11-02 and 2026-11-03: "能用actions的都用, 用不了的再手动, 这是为了充分利用github
  actions", fuzz "拉满4小时", Kani and Miri by hand, "审计过期不影响发版", one badge per task plus one for the whole
  audit, one "pins" badge for tools and actions, and badges that can be refreshed by hand.

#### Design
| Workflow | When | What |
|---|---|---|
| `verify.yml` | a release is published; by hand | deny, vet, geiger, coverage, fuzz for 14,400 s a target; Kani and Miri only by hand |
| `advisories.yml` | daily; after each `verify.yml` run; by hand | deny, known vulnerabilities in pinned tools and actions, the badges; weekly all updates |

Kani and Miri stay off every event because their time depends on the proofs and tests (Miri went from 57 s to
12,313 s with one test in Task 6), and a job left to time out at each release wastes runners. Each workflow has one
job that may write, which runs no project code: `record` commits `audit/`, `badges` pushes the `badges` branch.
Actions are pinned to commits; `cargo xtask outdated` checks them against their releases.

#### Changes to xtask
- A fuzz run at least 3,600 s long is recorded and stands for the task; `needed <task> [args]` asks about a given
  length, so a release runs fuzz again unless a 4-hour result stands.
- `badges <dir> --deny <outcome>` writes shields.io endpoint files.
- `outdated` reads each `uses:` of the workflows. OSV does not order the versions of GitHub Actions, so a query by
  version finds nothing:

```sh
curl -s https://api.osv.dev/v1/query \
  -d '{"package":{"name":"tj-actions/changed-files","ecosystem":"GitHub Actions"},"version":"45.0.7"}' | jq -c '[.vulns[]?.id]'
```

Output:
```plaintext
[]
```

  The advisory of that version, `GHSA-mrrh-fwg8-r2c3`, is listed by package name, with `fixed: 46.0.1`; `outdated`
  reads every advisory of an action and compares the ranges itself.

#### Verification
Passed: xtask's 26 tests and Clippy; the reference copy, with ureq and serde_json, has 24 tests passing and no Clippy
warning.

<details>
<summary>Commands and output</summary>

Before pinning, `outdated` found every action unpinned:

```sh
GITHUB_TOKEN=$(gh auth token) cargo xtask outdated </dev/null
```

Output:
```plaintext
actions/checkout: v4 -> 7.0.1 (not pinned to a commit)
dtolnay/rust-toolchain: stable -> nothing to update to yet (not pinned to a commit)
actions/upload-artifact: v4 -> 7.0.2 (not pinned to a commit)
actions/download-artifact: v4 -> 8.0.2 (not pinned to a commit)
```

After pinning those and dropping `dtolnay/rust-toolchain` (the runner's rustup installs the toolchain of
`rust-toolchain.toml`), `cargo xtask badges target/badges --deny success` gave `pins: up to date`. The `badges`
job's check, run on that output, let it through, and refused a file whose message was `<img>`.

The `record` job's script was run against a local repository: a release at a commit, then local vet results and
more code on `main`:

```plaintext
vet: results committed since 52042f9 stand
ba3f1ce audit: results for v0.2.0 (52042f9)
```

`audit/fuzz` took the run's results, `audit/vet` kept the local ones and no code changed. A run for a commit not on
`main` printed `634e1be is not on main; not recording.`

As `main.rs` changed, every task's result lapsed; all seven were run again from the clean tree at e090ba6:

```sh
cargo xtask all --if-needed
```

Output (the last line of each task, and the test summaries):
```plaintext
xtask: deny: succeeded
xtask: vet: succeeded
xtask: geiger: succeeded
test result: ok. 26 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 67.31s
xtask: miri: succeeded
xtask: coverage: succeeded
proofs::an_arrow_is_drawn_when_nothing_stops_it: succeeded, 1 s, peak memory 48 MiB
... (six other proofs, all succeeded, 0 to 18 s, 21 to 46 MiB)
xtask: kani: succeeded
xtask: fuzz: succeeded
```

Fuzzing an hour each ran 12,708,232 (`text`), 37,216,615 (`view`), 15,486,960 (`field`) and 7,722,036 (`window`)
inputs, with no crash; coverage stayed at 86 of 90 branches. `cargo xtask badges target/badges --deny success` then
read `up to date` for every badge, `passing` for deny.

</details>

###### Note
Neither workflow had run on GitHub when this was written. See 7.1 for the first runs.

---

*Timestamp: 2026-11-03*

---

### 7.1 First Runs on GitHub
Actions was turned on, `main` pushed at 24ed65f, and both workflows started by hand.

| Run | Result |
|---|---|
| Advisories 41723342774 | `outdated` refused `GITHUB_TOKEN`: the token a job gets has characters besides letters, digits and `_`. Fixed in 38849c5 by checking it against RFC 6750's b64token grammar. |
| Advisories 41724007566 | passed; the `badges` branch was created with all nine badges, which shields.io renders |
| Verify 41723338689 | deny, vet, geiger and coverage stood and were skipped; fuzz ran 4 h 4 min with no corpus to start from, passed, and uploaded `fuzz-corpus` (1.6 MB); `record` committed nothing |

`record` should have kept the newer local fuzz results anyway, but it never looked at the run's: with one artifact
matching `audit-*`, download-artifact v8.0.2 put its files straight into `results/`, not `results/audit-fuzz/`, so
the loop over `results/audit-*` found nothing. Had no newer result been committed, a run's results would have been
dropped silently. Fixed by uploading the task's directory itself and downloading with `merge-multiple: true`, so each
task's results land in `results/<task>/` whatever the count; the new loop was checked against that layout locally.
It has not run on GitHub yet, as every task's result stands.

The READMEs' examples became doctests (`#[cfg(doctest)]` with `include_str!`), with the license URL in a plaintext
block instead of an indented one, which rustdoc compiled as Rust; `tests/readme.rs` and `READ_BY_TESTS` went. As
`src/lib.rs` and `xtask/src/main.rs` changed, every task ran again from the clean tree at d8f67c9: all succeeded;
fuzzing an hour each ran 14,114,280 (`text`), 44,366,313 (`view`), 17,550,546 (`field`) and 8,950,947 (`window`)
inputs; coverage stayed at 86 of 90 branches.

The release of v0.2.0 started Verify 41763432089: deny, vet, coverage and fuzz passed and `record` committed them
(f941b35), but geiger failed with `cargo geiger wrote no whole table`. verify.yml sets `CARGO_TERM_COLOR: always`,
and geiger colours its table as cargo's output is coloured, so the row of totals started with an escape code rather
than a digit. Every earlier geiger success was a local run, where the output is a file and gets no colour. Fixed by
running geiger with `CARGO_TERM_COLOR=never`; with `CARGO_TERM_COLOR=always`, a `--no-record` run failed the same
way before the fix and succeeded after it, with no escape codes and the same table apart from the crate's version.

Aligned with the skill's reference again: geiger scans a copy of the work tree in `target/geiger-src/`, geiger and
coverage empty their build directories when the workspace's packages are not those they were built for
(`own_target`, which reads `cargo metadata` with `xtask/src/json.rs`, so that module now counts for geiger and
coverage as well as kani), and Verify's last job starts Advisories in place of its `workflow_run` trigger. As
`xtask/src/main.rs` and `xtask/src/tasks/mod.rs` changed, every result lapsed, and Verify ran all seven tasks.

---

*Timestamp: 2026-11-04*

---
