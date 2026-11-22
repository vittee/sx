use super::*;

#[test]
fn plain_when_color_is_off() {
    assert_eq!(render(Level::Error, "boom", false), "sx: boom");
}

#[test]
fn colored_by_level() {
    assert_eq!(render(Level::Error, "boom", true), "\x1b[1;31msx:\x1b[0m \x1b[31mboom\x1b[0m");
    assert!(render(Level::Warn, "w", true).contains("\x1b[33mw"));
    assert!(render(Level::Ok, "k", true).contains("\x1b[32mk"));
    assert!(render(Level::Info, "i", true).contains("\x1b[36mi"));
}

#[test]
fn never_colors_a_non_tty() {
    assert!(!color_enabled(false));
}

#[test]
fn erase_goes_up_deletes_the_line_and_comes_back_to_the_same_cell() {
    assert_eq!(erase_sequence(1, 0), "\x1b[1A\r\x1b[M\r");
    assert_eq!(erase_sequence(3, 0), "\x1b[3A\r\x1b[M\x1b[2B\r");
    assert_eq!(erase_sequence(2, 5), "\x1b[2A\r\x1b[M\x1b[1B\r\x1b[5C");
}

#[test]
fn the_title_sequence_is_osc_0_and_drops_control_characters() {
    assert_eq!(title_sequence("me@pc: ~"), "\x1b]0;me@pc: ~\x07");
    assert_eq!(title_sequence("a\x07b\x1b]c"), "\x1b]0;ab]c\x07");
}
