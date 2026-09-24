//! PROTOTYPE — throwaway flow renderer for wayfinder ticket 02.
//!
//! A deliberately small port of the `DrawLog` path in
//! `packages/tui/render.go` (logical buffer diff, viewport anchor,
//! scroll-on-append, full repaint on resize, idle no-op). It exists so
//! the rendering-stack decision has a concrete artifact to react to.
//! It is NOT production code: no ANSI-aware width math, no markdown,
//! no highlighting, no images, no reflow-rebase branch.
//!
//! Escape bytes are written directly (not through crossterm commands)
//! so the renderer is testable against a `Vec<u8>` with no terminal.

use std::io::Write;

pub const SYNCH_ON: &str = "\x1b[?2026h";
pub const SYNCH_OFF: &str = "\x1b[?2026l";
pub const HIDE_CURSOR: &str = "\x1b[?25l";
pub const SHOW_CURSOR: &str = "\x1b[?25h";
pub const RESET: &str = "\x1b[0m";
pub const CLEAR_LINE: &str = "\x1b[2K";
pub const CLEAR_SCREEN_NO_HOME: &str = "\x1b[2J";
pub const CURSOR_HOME: &str = "\x1b[H";
pub const CLEAR_TO_END: &str = "\x1b[0J";
pub const CLEAR_SCROLLBACK: &str = "\x1b[3J";
pub const DELETE_KITTY_IMAGES: &str = "\x1b_Ga=d\x1b\\";
pub const ALT_ON: &str = "\x1b[?1049h";
pub const ALT_OFF: &str = "\x1b[?1049l";

pub fn move_to(row: isize, col: isize) -> String {
    format!("\x1b[{};{}H", row.max(1), col.max(1))
}

fn cursor_down(n: isize) -> String {
    format!("\x1b[{}B", n.max(1))
}

fn cursor_up(n: isize) -> String {
    format!("\x1b[{}A", n.max(1))
}

/// Rough cell-count truncation. Counts escape bytes as cells (a known
/// simplification vs. the Go renderer's ANSI-aware truncateToWidth).
pub fn truncate_plain(s: &str, cols: usize) -> String {
    if cols == 0 {
        return String::new();
    }
    if s.chars().count() <= cols {
        return s.to_string();
    }
    s.chars().take(cols).collect()
}

/// Word-aware wrap for plain text. Not width/ANSI aware; long words
/// hard-break. Empty input always yields one empty row.
pub fn wrap_plain(s: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut out = Vec::new();
    for para in s.split('\n') {
        let words: Vec<&str> = para.split_whitespace().collect();
        if words.is_empty() {
            out.push(String::new());
            continue;
        }
        let mut line = String::new();
        for word in words {
            let wlen = word.chars().count();
            if !line.is_empty() && line.chars().count() + 1 + wlen <= width {
                line.push(' ');
                line.push_str(word);
                continue;
            }
            if !line.is_empty() {
                out.push(std::mem::take(&mut line));
            }
            if wlen > width {
                let mut rest: Vec<char> = word.chars().collect();
                while rest.len() > width {
                    out.push(rest.drain(..width).collect());
                }
                line = rest.into_iter().collect();
            } else {
                line = word.to_string();
            }
        }
        if !line.is_empty() {
            out.push(line);
        }
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}

struct MainState {
    chat: Vec<String>,
    bottom: Vec<String>,
    lines: Vec<String>,
    viewport_top: isize,
    hardware_row: isize,
    cursor: (isize, isize),
    init: bool,
}

pub struct FlowRenderer<W: Write> {
    out: W,
    cols: usize,
    rows: usize,
    chat: Vec<String>,
    bottom: Vec<String>,
    lines: Vec<String>,
    /// Logical row currently at the top of the visible viewport.
    viewport_top: isize,
    /// Logical row the hardware cursor sits on after the last paint.
    hardware_row: isize,
    /// Cursor position passed to the last draw: (row within bottom
    /// block, col). Row < 0 hides the cursor.
    cursor: (isize, isize),
    init: bool,
    alt: bool,
    alt_saved: Option<MainState>,
    keep_scrollback: bool,
    pub paints: usize,
    pub full_paints: usize,
}

impl<W: Write> FlowRenderer<W> {
    pub fn new(out: W, keep_scrollback: bool) -> Self {
        FlowRenderer {
            out,
            cols: 0,
            rows: 0,
            chat: Vec::new(),
            bottom: Vec::new(),
            lines: Vec::new(),
            viewport_top: 0,
            hardware_row: 0,
            cursor: (-1, 0),
            init: false,
            alt: false,
            alt_saved: None,
            keep_scrollback,
            paints: 0,
            full_paints: 0,
        }
    }

    /// Mirror of Renderer.Resize: on a real size change, clear screen
    /// and scrollback so stale old-width content cannot linger, then
    /// restart from a blank state. Go purges scrollback on resize even
    /// under VS Code (render.go:137-156).
    pub fn resize(&mut self, cols: usize, rows: usize) {
        if cols == self.cols && rows == self.rows {
            return;
        }
        self.cols = cols;
        self.rows = rows;
        self.chat.clear();
        self.bottom.clear();
        self.lines.clear();
        self.viewport_top = 0;
        self.hardware_row = 0;
        self.cursor = (-1, 0);
        self.init = false;
        if cols > 0 && rows > 0 {
            let mut s = String::from(DELETE_KITTY_IMAGES);
            if self.keep_scrollback {
                s.push_str(CURSOR_HOME);
            } else {
                s.push_str(CLEAR_SCREEN_NO_HOME);
            }
            s.push_str(CLEAR_SCROLLBACK);
            s.push_str(&move_to(1, 1));
            let _ = self.out.write_all(s.as_bytes());
        }
    }

    pub fn draw_log(
        &mut self,
        chat: &[String],
        bottom: &[String],
        cursor_row: isize,
        cursor_col: isize,
    ) {
        self.close_history();
        if self.cols == 0 || self.rows == 0 {
            return;
        }
        let bottom: Vec<String> = if bottom.is_empty() {
            vec![String::new()]
        } else {
            bottom.to_vec()
        };
        let mut lines: Vec<String> = Vec::with_capacity(chat.len() + bottom.len() + 1);
        lines.extend(chat.iter().cloned());
        lines.extend(bottom.iter().cloned());
        lines.push(String::new()); // renderer-owned bottom margin row
        if lines.is_empty() {
            lines.push(String::new());
        }

        let cursor_target_row: isize = if cursor_row >= 0 && (cursor_row as usize) < bottom.len() {
            chat.len() as isize + cursor_row
        } else {
            -1
        };

        // Idle no-op fast path: unchanged buffer and cursor emit nothing,
        // so the terminal's cursor blink timer is never reset.
        if self.init
            && cursor_row == self.cursor.0
            && cursor_col == self.cursor.1
            && same_lines(&lines, &self.lines)
        {
            return;
        }

        let was_initialized = self.init;
        let bottom_shrunk_above_viewport = if bottom.len() < self.bottom.len() {
            let new_viewport_top = (lines.len() as isize - self.rows as isize).max(0);
            new_viewport_top < self.viewport_top
        } else {
            false
        };
        let full = !was_initialized || self.lines.is_empty() || bottom_shrunk_above_viewport;

        let mut w = String::new();
        w.push_str(SYNCH_ON);
        w.push_str(HIDE_CURSOR);

        if full {
            self.write_full(&mut w, &lines, true, !was_initialized);
            self.init = true;
        } else {
            self.diff_paint(&mut w, &lines);
        }

        self.position_cursor(&mut w, lines.len(), cursor_target_row, cursor_col, false);
        w.push_str(SYNCH_OFF);

        let _ = self.out.write_all(w.as_bytes());

        self.paints += 1;
        if full {
            self.full_paints += 1;
        }
        self.chat = chat.to_vec();
        self.bottom = bottom;
        self.lines = lines;
        self.cursor = (cursor_row, cursor_col);
    }

    /// Crude alt-screen history view: full frame, no diffing, no
    /// scrollback. Mirrors Renderer.DrawHistory's separate screen.
    pub fn draw_history(&mut self, frame: &[String], cursor_row: isize, cursor_col: isize) {
        if self.cols == 0 || self.rows == 0 {
            return;
        }
        if !self.alt {
            self.alt_saved = Some(MainState {
                chat: std::mem::take(&mut self.chat),
                bottom: std::mem::take(&mut self.bottom),
                lines: std::mem::take(&mut self.lines),
                viewport_top: self.viewport_top,
                hardware_row: self.hardware_row,
                cursor: self.cursor,
                init: self.init,
            });
            let _ = self.out.write_all(ALT_ON.as_bytes());
            self.alt = true;
        }
        let start = frame.len().saturating_sub(self.rows);
        let visible = &frame[start..];
        let mut lines: Vec<String> = Vec::with_capacity(self.rows);
        for _ in 0..self.rows.saturating_sub(visible.len()) {
            lines.push(String::new());
        }
        lines.extend(visible.iter().cloned());

        let mut w = String::new();
        w.push_str(SYNCH_ON);
        w.push_str(HIDE_CURSOR);
        w.push_str(DELETE_KITTY_IMAGES);
        w.push_str(CLEAR_SCREEN_NO_HOME);
        w.push_str(&move_to(1, 1));
        for (i, line) in lines.iter().enumerate() {
            if i > 0 {
                w.push_str("\r\n");
            }
            w.push_str(RESET);
            w.push_str(CLEAR_LINE);
            w.push_str(line);
        }
        self.viewport_top = 0;
        self.hardware_row = lines.len() as isize - 1;
        if cursor_row >= 0 {
            let target = (chat_history_len(frame, self.rows) as isize + cursor_row)
                .clamp(0, self.rows as isize - 1);
            self.move_to_logical_row(&mut w, self.rows, target);
            w.push('\r');
            if cursor_col > 0 {
                w.push_str(&format!("\x1b[{}C", cursor_col));
            }
            w.push_str(SHOW_CURSOR);
        }
        w.push_str(SYNCH_OFF);
        let _ = self.out.write_all(w.as_bytes());
        self.paints += 1;
        self.full_paints += 1;
    }

    pub fn close_history(&mut self) {
        if !self.alt {
            return;
        }
        let _ = self.out.write_all(ALT_OFF.as_bytes());
        self.alt = false;
        if let Some(saved) = self.alt_saved.take() {
            self.chat = saved.chat;
            self.bottom = saved.bottom;
            self.lines = saved.lines;
            self.viewport_top = saved.viewport_top;
            self.hardware_row = saved.hardware_row;
            self.cursor = saved.cursor;
            self.init = saved.init;
        }
    }

    fn write_full(&mut self, w: &mut String, lines: &[String], clear: bool, purge: bool) {
        if clear {
            w.push_str(DELETE_KITTY_IMAGES);
            if self.keep_scrollback {
                // \x1b[2J under xterm.js scrolls the visible frame into
                // scrollback; home + erase-to-end clears in place.
                w.push_str(CURSOR_HOME);
                w.push_str(CLEAR_TO_END);
            } else {
                w.push_str(CLEAR_SCREEN_NO_HOME);
                if purge {
                    w.push_str(CLEAR_SCROLLBACK);
                }
                w.push_str(&move_to(1, 1));
            }
        }
        for (i, line) in lines.iter().enumerate() {
            if i > 0 {
                w.push_str("\r\n");
            }
            w.push_str(RESET);
            w.push_str(CLEAR_LINE);
            w.push_str(line);
        }
        self.hardware_row = lines.len() as isize - 1;
        self.viewport_top = (lines.len() as isize - self.rows as isize).max(0);
    }

    fn diff_paint(&mut self, w: &mut String, lines: &[String]) {
        let max_lines = lines.len().max(self.lines.len());
        let mut first: isize = -1;
        let mut last: isize = -1;
        for idx in 0..max_lines {
            let old_line = self.lines.get(idx).map(String::as_str).unwrap_or("");
            let new_line = lines.get(idx).map(String::as_str).unwrap_or("");
            if old_line != new_line {
                if first == -1 {
                    first = idx as isize;
                }
                last = idx as isize;
            }
        }
        if lines.len() > self.lines.len() {
            if first == -1 {
                first = self.lines.len() as isize;
            }
            if last < lines.len() as isize - 1 {
                last = lines.len() as isize - 1;
            }
        }

        // Rows above the viewport are terminal history we cannot rewrite.
        // Ignore inaccessible changes and patch only from the viewport
        // top. (Go's reflow-rebase branch, render.go:702-753, is
        // deliberately omitted from this spike.)
        if first >= 0 && first < self.viewport_top {
            first = -1;
            last = -1;
            for idx in self.viewport_top.max(0) as usize..max_lines {
                let old_line = self.lines.get(idx).map(String::as_str).unwrap_or("");
                let new_line = lines.get(idx).map(String::as_str).unwrap_or("");
                if old_line != new_line {
                    if first == -1 {
                        first = idx as isize;
                    }
                    last = idx as isize;
                }
            }
            if first == -1
                && lines.len() > self.lines.len()
                && self.lines.len() >= self.viewport_top.max(0) as usize
            {
                first = self.lines.len() as isize;
                last = lines.len() as isize - 1;
            }
        }

        if first >= 0 {
            let prev_viewport_top = self.viewport_top;
            let mut viewport_top = self.viewport_top;
            let mut hardware_row = self.hardware_row;
            let prev_viewport_bottom = prev_viewport_top + self.rows as isize - 1;
            let append_start =
                lines.len() > self.lines.len() && first == self.lines.len() as isize && first > 0;
            let move_target = if append_start { first - 1 } else { first };

            if move_target > prev_viewport_bottom {
                let current_screen_row =
                    (hardware_row - prev_viewport_top).clamp(0, self.rows as isize - 1);
                let move_to_bottom = self.rows as isize - 1 - current_screen_row;
                if move_to_bottom > 0 {
                    w.push_str(&cursor_down(move_to_bottom));
                }
                let scroll = move_target - prev_viewport_bottom;
                for _ in 0..scroll {
                    w.push_str("\r\n");
                }
                viewport_top += scroll;
                hardware_row = move_target;
            }

            let current_screen_row = hardware_row - prev_viewport_top;
            let target_screen_row = move_target - viewport_top;
            let line_diff = target_screen_row - current_screen_row;
            if line_diff > 0 {
                w.push_str(&cursor_down(line_diff));
            } else if line_diff < 0 {
                w.push_str(&cursor_up(-line_diff));
            }
            if append_start {
                w.push_str("\r\n");
            } else {
                w.push('\r');
            }

            let render_end = last.min(lines.len() as isize - 1);
            let mut idx = first;
            while idx <= render_end {
                if idx > first {
                    w.push_str("\r\n");
                }
                w.push_str(RESET);
                w.push_str(CLEAR_LINE);
                w.push_str(&lines[idx as usize]);
                idx += 1;
            }

            let mut final_row = render_end;
            let mut full_repaint = false;
            if self.lines.len() > lines.len() {
                let extra = self.lines.len() - lines.len();
                if extra >= self.rows {
                    self.write_full(w, lines, true, false);
                    full_repaint = true;
                } else {
                    for _ in 0..extra {
                        w.push_str("\x1b[1B\r");
                        w.push_str(RESET);
                        w.push_str(CLEAR_LINE);
                        final_row += 1;
                    }
                    w.push_str(&cursor_up(extra as isize));
                    final_row -= extra as isize;
                }
            }
            if !full_repaint {
                self.hardware_row = final_row;
                self.viewport_top = viewport_top;
                let min_top = self.hardware_row - self.rows as isize + 1;
                if min_top > self.viewport_top {
                    self.viewport_top = min_top;
                }
                if self.viewport_top < 0 {
                    self.viewport_top = 0;
                }
            }
        }
    }

    fn move_to_logical_row(&mut self, w: &mut String, lines_len: usize, target_row: isize) {
        let target = target_row.clamp(0, lines_len.saturating_sub(1) as isize);
        let viewport_bottom = self.viewport_top + self.rows as isize - 1;
        if target > viewport_bottom {
            let current_screen_row =
                (self.hardware_row - self.viewport_top).clamp(0, self.rows as isize - 1);
            let move_to_bottom = self.rows as isize - 1 - current_screen_row;
            if move_to_bottom > 0 {
                w.push_str(&cursor_down(move_to_bottom));
            }
            let scroll = target - viewport_bottom;
            for _ in 0..scroll {
                w.push_str("\r\n");
            }
            self.viewport_top += scroll;
            self.hardware_row = target;
            return;
        }
        let current_screen_row = self.hardware_row - self.viewport_top;
        let target_screen_row = target - self.viewport_top;
        let line_diff = target_screen_row - current_screen_row;
        if line_diff > 0 {
            w.push_str(&cursor_down(line_diff));
        } else if line_diff < 0 {
            w.push_str(&cursor_up(-line_diff));
        }
        self.hardware_row = target;
    }

    fn position_cursor(
        &mut self,
        w: &mut String,
        lines_len: usize,
        target_row: isize,
        cursor_col: isize,
        force_show: bool,
    ) {
        if target_row < 0 || target_row >= lines_len as isize {
            if force_show {
                w.push_str(SHOW_CURSOR);
            }
            return;
        }
        self.move_to_logical_row(w, lines_len, target_row);
        w.push('\r');
        if cursor_col > 0 {
            w.push_str(&format!("\x1b[{}C", cursor_col));
        }
        w.push_str(SHOW_CURSOR);
    }
}

impl FlowRenderer<Vec<u8>> {
    pub fn take_output(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.out)
    }
}

fn same_lines(a: &[String], b: &[String]) -> bool {
    a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x == y)
}

fn chat_history_len(frame: &[String], rows: usize) -> usize {
    frame.len().saturating_sub(rows)
}
