//! Minimal multi-line text editor state for the snippet pane.

#[derive(Default, Debug)]
pub struct Editor {
    pub lines: Vec<String>,
    /// Cursor position: line index and char index within the line.
    pub row: usize,
    pub col: usize,
    /// First visible line and first visible column.
    pub top: usize,
    pub left: usize,
    pub dirty: bool,
}

fn byte_idx(s: &str, col: usize) -> usize {
    s.char_indices().nth(col).map_or(s.len(), |(i, _)| i)
}

fn char_len(s: &str) -> usize {
    s.chars().count()
}

impl Editor {
    pub fn from_text(text: &str) -> Editor {
        let mut lines: Vec<String> = text.lines().map(String::from).collect();
        if lines.is_empty() {
            lines.push(String::new());
        }
        Editor { lines, ..Editor::default() }
    }

    pub fn text(&self) -> String {
        let mut t = self.lines.join("\n");
        t.push('\n');
        t
    }

    fn line_len(&self) -> usize {
        char_len(&self.lines[self.row])
    }

    pub fn insert_char(&mut self, c: char) {
        let at = byte_idx(&self.lines[self.row], self.col);
        self.lines[self.row].insert(at, c);
        self.col += 1;
        self.dirty = true;
    }

    pub fn insert_str(&mut self, s: &str) {
        for c in s.chars() {
            self.insert_char(c);
        }
    }

    pub fn newline(&mut self) {
        let at = byte_idx(&self.lines[self.row], self.col);
        let rest = self.lines[self.row].split_off(at);
        self.lines.insert(self.row + 1, rest);
        self.row += 1;
        self.col = 0;
        self.dirty = true;
    }

    pub fn backspace(&mut self) {
        if self.col > 0 {
            let at = byte_idx(&self.lines[self.row], self.col - 1);
            self.lines[self.row].remove(at);
            self.col -= 1;
        } else if self.row > 0 {
            let cur = self.lines.remove(self.row);
            self.row -= 1;
            self.col = self.line_len();
            self.lines[self.row].push_str(&cur);
        } else {
            return;
        }
        self.dirty = true;
    }

    pub fn delete(&mut self) {
        if self.col < self.line_len() {
            let at = byte_idx(&self.lines[self.row], self.col);
            self.lines[self.row].remove(at);
        } else if self.row + 1 < self.lines.len() {
            let next = self.lines.remove(self.row + 1);
            self.lines[self.row].push_str(&next);
        } else {
            return;
        }
        self.dirty = true;
    }

    pub fn left(&mut self) {
        if self.col > 0 {
            self.col -= 1;
        } else if self.row > 0 {
            self.row -= 1;
            self.col = self.line_len();
        }
    }

    pub fn right(&mut self) {
        if self.col < self.line_len() {
            self.col += 1;
        } else if self.row + 1 < self.lines.len() {
            self.row += 1;
            self.col = 0;
        }
    }

    pub fn move_rows(&mut self, delta: isize) {
        let last = self.lines.len() - 1;
        self.row = self.row.saturating_add_signed(delta).min(last);
        self.col = self.col.min(self.line_len());
    }

    pub fn home(&mut self) {
        self.col = 0;
    }

    pub fn end(&mut self) {
        self.col = self.line_len();
    }

    /// Scrolls the view without moving the cursor.
    pub fn scroll(&mut self, delta: isize, view_h: usize) {
        let max_top = self.lines.len().saturating_sub(view_h.max(1));
        self.top = self.top.saturating_add_signed(delta).min(max_top);
    }

    /// Puts the cursor at a view position (relative to the text area's top-left).
    pub fn click(&mut self, x: usize, y: usize) {
        self.row = (self.top + y).min(self.lines.len() - 1);
        self.col = (self.left + x).min(self.line_len());
    }

    /// Adjusts `top` and `left` so the cursor is inside a `w` x `h` view.
    pub fn keep_cursor_visible(&mut self, w: usize, h: usize) {
        let (w, h) = (w.max(1), h.max(1));
        if self.row < self.top {
            self.top = self.row;
        } else if self.row >= self.top + h {
            self.top = self.row + 1 - h;
        }
        if self.col < self.left {
            self.left = self.col;
        } else if self.col >= self.left + w {
            self.left = self.col + 1 - w;
        }
    }
}

#[cfg(test)]
mod tests;
