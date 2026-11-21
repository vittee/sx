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
    e.move_rows(1);
    assert_eq!((e.row, e.col), (1, 1));
    e.move_rows(5);
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
