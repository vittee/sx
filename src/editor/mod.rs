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
    /// The other end of the selection. The cursor is the end that moves.
    pub anchor: Option<(usize, usize)>,
}

/// A cursor move. `Rows` moves by lines, `To` goes to a line and column.
#[derive(Clone, Copy, Debug)]
pub enum Move {
    Left,
    Right,
    Rows(isize),
    Home,
    End,
    To(usize, usize),
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

    /// The selection as two ordered `(row, col)` positions, or `None` when nothing is selected.
    pub fn selection(&self) -> Option<((usize, usize), (usize, usize))> {
        let a = self.anchor?;
        let c = (self.row, self.col);
        match a.cmp(&c) {
            std::cmp::Ordering::Less => Some((a, c)),
            std::cmp::Ordering::Greater => Some((c, a)),
            std::cmp::Ordering::Equal => None,
        }
    }

    /// The selected columns of line `row`: from the first one to one past the last one.
    pub fn selected_cols(&self, row: usize) -> Option<(usize, usize)> {
        let ((r1, c1), (r2, c2)) = self.selection()?;
        if row < r1 || row > r2 {
            return None;
        }
        let from = if row == r1 { c1 } else { 0 };
        let to = if row == r2 { c2 } else { char_len(self.lines.get(row)?) };
        Some((from, to))
    }

    pub fn select_all(&mut self) {
        self.anchor = Some((0, 0));
        self.row = self.lines.len() - 1;
        self.col = self.line_len();
    }

    /// Removes the selected text and puts the cursor where it was. False when nothing is selected.
    pub fn delete_selection(&mut self) -> bool {
        let Some(((r1, c1), (r2, c2))) = self.selection() else {
            self.anchor = None;
            return false;
        };
        let tail = self.lines[r2][byte_idx(&self.lines[r2], c2)..].to_string();
        let head_end = byte_idx(&self.lines[r1], c1);
        self.lines[r1].truncate(head_end);
        self.lines[r1].push_str(&tail);
        self.lines.drain(r1 + 1..=r2);
        (self.row, self.col) = (r1, c1);
        self.anchor = None;
        self.dirty = true;
        true
    }

    pub fn insert_char(&mut self, c: char) {
        self.delete_selection();
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
        self.delete_selection();
        let at = byte_idx(&self.lines[self.row], self.col);
        let rest = self.lines[self.row].split_off(at);
        self.lines.insert(self.row + 1, rest);
        self.row += 1;
        self.col = 0;
        self.dirty = true;
    }

    pub fn backspace(&mut self) {
        if self.delete_selection() {
            return;
        }
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
        if self.delete_selection() {
            return;
        }
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

    fn step(&mut self, mv: Move) {
        match mv {
            Move::Left => {
                if self.col > 0 {
                    self.col -= 1;
                } else if self.row > 0 {
                    self.row -= 1;
                    self.col = self.line_len();
                }
            }
            Move::Right => {
                if self.col < self.line_len() {
                    self.col += 1;
                } else if self.row + 1 < self.lines.len() {
                    self.row += 1;
                    self.col = 0;
                }
            }
            Move::Rows(delta) => {
                let last = self.lines.len() - 1;
                self.row = self.row.saturating_add_signed(delta).min(last);
                self.col = self.col.min(self.line_len());
            }
            Move::Home => self.col = 0,
            Move::End => self.col = self.line_len(),
            Move::To(row, col) => {
                self.row = row.min(self.lines.len() - 1);
                self.col = col.min(self.line_len());
            }
        }
    }

    /// Moves the cursor and drops the selection. `Left` and `Right` on a selection go to its start or end.
    pub fn go(&mut self, mv: Move) {
        match (self.selection(), mv) {
            (Some((start, _)), Move::Left) => (self.row, self.col) = start,
            (Some((_, end)), Move::Right) => (self.row, self.col) = end,
            _ => self.step(mv),
        }
        self.anchor = None;
    }

    /// Moves the cursor and keeps the selection anchored where it started.
    pub fn extend(&mut self, mv: Move) {
        let anchor = self.anchor.unwrap_or((self.row, self.col));
        self.step(mv);
        self.anchor = Some(anchor);
    }

    pub fn left(&mut self) {
        self.go(Move::Left);
    }

    pub fn right(&mut self) {
        self.go(Move::Right);
    }

    pub fn home(&mut self) {
        self.go(Move::Home);
    }

    pub fn end(&mut self) {
        self.go(Move::End);
    }

    /// Scrolls the view without moving the cursor.
    pub fn scroll(&mut self, delta: isize, view_h: usize) {
        let max_top = self.lines.len().saturating_sub(view_h.max(1));
        self.top = self.top.saturating_add_signed(delta).min(max_top);
    }

    /// Puts the cursor at a view position (relative to the text area's top-left).
    pub fn click(&mut self, x: usize, y: usize) {
        self.go(Move::To(self.top + y, self.left + x));
    }

    /// Selects from the anchor (the cursor, when there is none) to a view position. It can lie
    /// outside the view, so a drag past an edge selects up to that edge and `keep_cursor_visible` scrolls.
    pub fn drag(&mut self, x: isize, y: isize) {
        let row = self.top.saturating_add_signed(y);
        let col = self.left.saturating_add_signed(x);
        self.extend(Move::To(row, col));
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
