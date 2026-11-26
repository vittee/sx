use super::*;

#[test]
fn typing_and_newline() {
    let mut e = Editor::from_text("");
    e.insert_str("ab");
    e.newline();
    e.insert_char('c');
    assert_eq!(e.text(), "ab\nc\n");
    assert!(e.dirty);
}

#[test]
fn backspace_joins_lines_and_handles_unicode() {
    let mut e = Editor::from_text("é1\n2");
    e.row = 1;
    e.col = 0;
    e.backspace();
    assert_eq!(e.lines, ["é12"]);
    assert_eq!((e.row, e.col), (0, 2));
    e.backspace();
    assert_eq!(e.lines, ["é2"]);
}

#[test]
fn delete_joins_next_line() {
    let mut e = Editor::from_text("a\nb");
    e.end();
    e.delete();
    assert_eq!(e.lines, ["ab"]);
}

#[test]
fn cursor_moves_wrap_and_clamp() {
    let mut e = Editor::from_text("abc\nd");
    e.end();
    e.right();
    assert_eq!((e.row, e.col), (1, 0));
    e.left();
    assert_eq!((e.row, e.col), (0, 3));
    e.go(Move::Rows(1));
    assert_eq!((e.row, e.col), (1, 1));
    e.go(Move::Rows(5));
    assert_eq!(e.row, 1);
}

#[test]
fn click_and_scroll() {
    let mut e = Editor::from_text(&"line\n".repeat(30));
    e.scroll(10, 5);
    e.click(2, 1);
    assert_eq!((e.row, e.col), (11, 2));
    e.click(50, 99);
    assert_eq!((e.row, e.col), (29, 4));
    e.scroll(-100, 5);
    assert_eq!(e.top, 0);
}

#[test]
fn keeps_cursor_in_view() {
    let mut e = Editor::from_text(&"x\n".repeat(50));
    e.row = 40;
    e.keep_cursor_visible(10, 10);
    assert_eq!(e.top, 31);
    e.row = 3;
    e.keep_cursor_visible(10, 10);
    assert_eq!(e.top, 3);
}

#[test]
fn empty_text_has_one_line() {
    assert_eq!(Editor::from_text("").lines, [""]);
}

fn at(e: &Editor) -> (usize, usize) {
    (e.row, e.col)
}

#[test]
fn extend_selects_and_a_plain_move_drops_the_selection() {
    let mut e = Editor::from_text("abc
def");
    e.extend(Move::Right);
    e.extend(Move::Right);
    assert_eq!(e.selection(), Some(((0, 0), (0, 2))));
    assert_eq!(e.selected_cols(0), Some((0, 2)));
    assert_eq!(e.selected_cols(1), None);
    e.extend(Move::Left);
    assert_eq!(e.selection(), Some(((0, 0), (0, 1))));
    e.go(Move::Home);
    assert_eq!(e.selection(), None);
    assert_eq!(e.anchor, None);
}

#[test]
fn a_selection_has_no_size_when_the_ends_meet() {
    let mut e = Editor::from_text("abc");
    e.extend(Move::Right);
    e.extend(Move::Left);
    assert_eq!(e.selection(), None);
}

#[test]
fn a_selection_can_run_backwards_and_over_lines() {
    let mut e = Editor::from_text("abc
def
ghi");
    e.go(Move::To(2, 2));
    e.extend(Move::To(0, 1));
    assert_eq!(e.selection(), Some(((0, 1), (2, 2))));
    assert_eq!(e.selected_cols(0), Some((1, 3)));
    assert_eq!(e.selected_cols(1), Some((0, 3)));
    assert_eq!(e.selected_cols(2), Some((0, 2)));
}

#[test]
fn left_and_right_go_to_the_ends_of_a_selection() {
    let mut e = Editor::from_text("abcd");
    e.go(Move::To(0, 1));
    e.extend(Move::Right);
    e.extend(Move::Right);
    e.left();
    assert_eq!(at(&e), (0, 1));
    assert_eq!(e.selection(), None);
    e.extend(Move::Right);
    e.extend(Move::Right);
    e.right();
    assert_eq!(at(&e), (0, 3));
}

#[test]
fn typing_replaces_the_selection() {
    let mut e = Editor::from_text("hello world");
    e.go(Move::To(0, 6));
    e.extend(Move::End);
    e.insert_str("sx");
    assert_eq!(e.lines, ["hello sx"]);
    assert_eq!(at(&e), (0, 8));
    assert_eq!(e.anchor, None);
}

#[test]
fn backspace_and_delete_remove_only_the_selection() {
    let mut e = Editor::from_text("abcdef");
    e.go(Move::To(0, 2));
    e.extend(Move::To(0, 4));
    e.backspace();
    assert_eq!(e.lines, ["abef"]);
    assert_eq!(at(&e), (0, 2));
    e.extend(Move::Right);
    e.delete();
    assert_eq!(e.lines, ["abf"]);
}

#[test]
fn deleting_a_selection_over_lines_joins_the_ends() {
    let mut e = Editor::from_text("abc
def
ghi");
    e.go(Move::To(0, 1));
    e.extend(Move::To(2, 2));
    assert!(e.delete_selection());
    assert_eq!(e.lines, ["ai"]);
    assert_eq!(at(&e), (0, 1));
    assert!(e.dirty);
    assert!(!e.delete_selection());
}

#[test]
fn newline_replaces_the_selection_and_unicode_columns_count_chars() {
    let mut e = Editor::from_text("é1é2");
    e.go(Move::To(0, 1));
    e.extend(Move::To(0, 3));
    e.newline();
    assert_eq!(e.lines, ["é", "2"]);
}

#[test]
fn select_all_takes_every_line() {
    let mut e = Editor::from_text("a
bc");
    e.select_all();
    assert_eq!(e.selection(), Some(((0, 0), (1, 2))));
    e.insert_char('x');
    assert_eq!(e.lines, ["x"]);
}

#[test]
fn drag_selects_from_the_click_and_clamps_outside_the_text() {
    let mut e = Editor::from_text("abc
def
ghi");
    e.click(1, 0);
    e.drag(2, 1);
    assert_eq!(e.selection(), Some(((0, 1), (1, 2))));
    e.drag(50, 50);
    assert_eq!(e.selection(), Some(((0, 1), (2, 3))));
    e.drag(-1, -1);
    assert_eq!(e.selection(), Some(((0, 0), (0, 1))));
}
