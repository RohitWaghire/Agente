//! PROTOTYPE — tiny terminal emulator used by the scripted session and
//! tests. Applies the subset of ANSI the flow renderer emits so the
//! spike can prove claims (no duplicate transcript after resize, idle
//! paints emit nothing) without a real terminal.
//!
//! Supported: CUP/ED/EL/CUU/CUD/CUF/CUB/CHA, SGR (ignored), DECSTBM
//! (ignored), private modes 1049 (alt screen), others ignored, OSC/APC
//! strings skipped, CR/LF/BS, deferred autowrap, scroll into
//! scrollback on main screen.

pub struct Screen {
    cols: usize,
    rows: usize,
    buf: Vec<Vec<char>>,
    cursor: (usize, usize),
    scrollback: Vec<String>,
    alt: bool,
    saved: Option<(Vec<Vec<char>>, (usize, usize), Vec<String>)>,
    saved_cursor: Option<(usize, usize)>,
    pending_wrap: bool,
    pub purges: usize,
    pub full_clears: usize,
}

impl Screen {
    pub fn new(cols: usize, rows: usize) -> Self {
        Screen {
            cols,
            rows,
            buf: vec![vec![' '; cols]; rows],
            cursor: (0, 0),
            scrollback: Vec::new(),
            alt: false,
            saved: None,
            saved_cursor: None,
            pending_wrap: false,
            purges: 0,
            full_clears: 0,
        }
    }

    pub fn feed(&mut self, bytes: &[u8]) {
        let s = String::from_utf8_lossy(bytes);
        let mut it = s.chars().peekable();
        while let Some(c) = it.next() {
            match c {
                '\x1b' => self.escape(&mut it),
                '\r' => {
                    self.cursor.1 = 0;
                    self.pending_wrap = false;
                }
                '\n' => self.line_feed(),
                '\u{8}' => {
                    self.cursor.1 = self.cursor.1.saturating_sub(1);
                    self.pending_wrap = false;
                }
                c if (c as u32) >= 0x20 && c != '\u{7f}' => self.put(c),
                _ => {}
            }
        }
    }

    fn escape(&mut self, it: &mut std::iter::Peekable<std::str::Chars<'_>>) {
        let Some(c) = it.next() else { return };
        match c {
            '[' => {
                let mut params = String::new();
                while let Some(&p) = it.peek() {
                    if ('\u{40}'..='\u{7e}').contains(&p) {
                        it.next();
                        self.csi(&params, p);
                        return;
                    }
                    params.push(p);
                    it.next();
                }
            }
            ']' | '_' | 'P' | '^' => {
                while let Some(c) = it.next() {
                    if c == '\u{7}' {
                        break;
                    }
                    if c == '\x1b' {
                        if it.peek() == Some(&'\\') {
                            it.next();
                        }
                        break;
                    }
                }
            }
            '7' => self.saved_cursor = Some(self.cursor),
            '8' => {
                if let Some(c) = self.saved_cursor {
                    self.cursor = c;
                    self.pending_wrap = false;
                }
            }
            _ => {}
        }
    }

    fn csi(&mut self, params: &str, final_char: char) {
        let (private, body) = match params.strip_prefix('?') {
            Some(rest) => (true, rest),
            None => (false, params),
        };
        let nums: Vec<isize> = if body.is_empty() {
            Vec::new()
        } else {
            body.split(';')
                .map(|p| p.parse::<isize>().unwrap_or(-1))
                .collect()
        };
        let n1 = nums.first().copied().filter(|&v| v > 0).unwrap_or(1);
        let n0 = nums.first().copied().unwrap_or(0);

        match final_char {
            'H' | 'f' => {
                let row = nums.first().copied().unwrap_or(1).max(1) as usize;
                let col = nums.get(1).copied().unwrap_or(1).max(1) as usize;
                self.cursor.0 = (row - 1).min(self.rows.saturating_sub(1));
                self.cursor.1 = (col - 1).min(self.cols.saturating_sub(1));
                self.pending_wrap = false;
            }
            'A' => {
                self.cursor.0 = self.cursor.0.saturating_sub(n1 as usize);
                self.pending_wrap = false;
            }
            'B' => {
                self.cursor.0 = (self.cursor.0 + n1 as usize).min(self.rows.saturating_sub(1));
                self.pending_wrap = false;
            }
            'C' => {
                self.cursor.1 = (self.cursor.1 + n1 as usize).min(self.cols.saturating_sub(1));
                self.pending_wrap = false;
            }
            'D' => {
                self.cursor.1 = self.cursor.1.saturating_sub(n1 as usize);
                self.pending_wrap = false;
            }
            'G' => {
                self.cursor.1 = ((n1 as usize).saturating_sub(1)).min(self.cols.saturating_sub(1));
                self.pending_wrap = false;
            }
            'J' => match n0 {
                2 => {
                    for row in self.buf.iter_mut() {
                        row.iter_mut().for_each(|c| *c = ' ');
                    }
                    self.full_clears += 1;
                }
                3 => {
                    self.scrollback.clear();
                    self.purges += 1;
                }
                _ => {
                    let (r, c) = self.cursor;
                    for col in c..self.cols {
                        self.buf[r][col] = ' ';
                    }
                    for row in (r + 1)..self.rows {
                        self.buf[row].iter_mut().for_each(|c| *c = ' ');
                    }
                }
            },
            'K' => {
                let (r, c) = self.cursor;
                match n0 {
                    2 => self.buf[r].iter_mut().for_each(|ch| *ch = ' '),
                    1 => {
                        for col in 0..=c {
                            self.buf[r][col] = ' ';
                        }
                    }
                    _ => {
                        for col in c..self.cols {
                            self.buf[r][col] = ' ';
                        }
                    }
                }
            }
            'S' => {
                for _ in 0..n1 {
                    self.scroll_up();
                }
            }
            'h' | 'l' if private => {
                for n in &nums {
                    if *n == 1049 {
                        if final_char == 'h' && !self.alt {
                            self.saved = Some((
                                std::mem::take(&mut self.buf),
                                self.cursor,
                                std::mem::take(&mut self.scrollback),
                            ));
                            self.buf = vec![vec![' '; self.cols]; self.rows];
                            self.cursor = (0, 0);
                            self.alt = true;
                        } else if final_char == 'l' && self.alt {
                            if let Some((buf, cursor, sb)) = self.saved.take() {
                                self.buf = buf;
                                self.cursor = cursor;
                                self.scrollback = sb;
                            }
                            self.alt = false;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn line_feed(&mut self) {
        if self.cursor.0 + 1 < self.rows {
            self.cursor.0 += 1;
        } else {
            self.scroll_up();
        }
        self.pending_wrap = false;
    }

    fn scroll_up(&mut self) {
        let line = self.buf.remove(0);
        if !self.alt {
            let text: String = line.iter().collect();
            self.scrollback.push(text.trim_end().to_string());
        }
        self.buf.push(vec![' '; self.cols]);
    }

    fn put(&mut self, c: char) {
        if self.pending_wrap {
            self.pending_wrap = false;
            self.cursor.1 = 0;
            self.line_feed();
        }
        let (r, col) = self.cursor;
        if r < self.rows && col < self.cols {
            self.buf[r][col] = c;
        }
        if col + 1 < self.cols {
            self.cursor.1 = col + 1;
        } else {
            self.pending_wrap = true;
        }
    }

    pub fn text(&self) -> String {
        self.buf
            .iter()
            .map(|row| row.iter().collect::<String>().trim_end().to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn all(&self) -> String {
        let mut s = self.scrollback.join("\n");
        if !s.is_empty() {
            s.push('\n');
        }
        s.push_str(&self.text());
        s
    }

    pub fn scrollback_len(&self) -> usize {
        self.scrollback.len()
    }

    /// Resize the emulated terminal. A real terminal preserves cells it
    /// can; the renderer under test is expected to clear and repaint,
    /// so padding/truncating the buffer is faithful enough.
    pub fn set_size(&mut self, cols: usize, rows: usize) {
        let mut buf = vec![vec![' '; cols]; rows];
        for (r, row) in self.buf.iter().take(rows).enumerate() {
            for (c, ch) in row.iter().take(cols).enumerate() {
                buf[r][c] = *ch;
            }
        }
        self.buf = buf;
        self.cols = cols;
        self.rows = rows;
        self.cursor.0 = self.cursor.0.min(rows.saturating_sub(1));
        self.cursor.1 = self.cursor.1.min(cols.saturating_sub(1));
        self.pending_wrap = false;
    }
}
