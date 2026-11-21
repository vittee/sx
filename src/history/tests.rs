use super::*;

fn entry(when: u64, host: &str, outcome: &str) -> Entry {
    Entry { when, host: host.into(), secs: 5, outcome: outcome.into() }
}

#[test]
fn append_and_read_back_newest_first_per_host() {
    let tmp = tempfile::tempdir().unwrap();
    append(tmp.path(), &entry(1, "a", "ok")).unwrap();
    append(tmp.path(), &entry(2, "b", "ok")).unwrap();
    append(tmp.path(), &entry(3, "a", "exit 2")).unwrap();
    let got = for_host(tmp.path(), "a", 10);
    assert_eq!(got, [entry(3, "a", "exit 2"), entry(1, "a", "ok")]);
    assert_eq!(for_host(tmp.path(), "a", 1).len(), 1);
    assert_eq!(count_for_host(tmp.path(), "a"), 2);
    assert!(for_host(tmp.path(), "nobody", 5).is_empty());
}

#[test]
fn missing_or_damaged_file_is_not_an_error() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(for_host(tmp.path(), "a", 5).is_empty());
    fs::write(tmp.path().join(FILE), "garbage\n1\ta\t5\tok\n\t\t\n").unwrap();
    assert_eq!(for_host(tmp.path(), "a", 5), [entry(1, "a", "ok")]);
}

#[test]
fn tabs_and_newlines_cannot_break_the_format() {
    let tmp = tempfile::tempdir().unwrap();
    append(tmp.path(), &entry(1, "we\tird\nhost", "ok")).unwrap();
    assert_eq!(for_host(tmp.path(), "we ird host", 5).len(), 1);
}

#[test]
fn a_big_file_is_trimmed_to_the_newest_lines() {
    let tmp = tempfile::tempdir().unwrap();
    let line = format!("1\t{}\t5\tok\n", "h".repeat(200));
    fs::write(tmp.path().join(FILE), line.repeat(2000)).unwrap(); // ~420 KB
    append(tmp.path(), &entry(9, "last", "ok")).unwrap();
    let text = fs::read_to_string(tmp.path().join(FILE)).unwrap();
    assert_eq!(text.lines().count(), KEEP_LINES);
    assert!(text.lines().last().unwrap().starts_with("9\tlast"));
}

#[test]
fn outcomes_and_durations_read_well() {
    assert_eq!(outcome_text(false, 0), "ok");
    assert_eq!(outcome_text(false, 255), "failed");
    assert_eq!(outcome_text(false, 3), "exit 3");
    assert_eq!(outcome_text(true, 130), "aborted");
    assert_eq!(format_secs(7), "7s");
    assert_eq!(format_secs(183), "3m 03s");
    assert_eq!(format_secs(7500), "2h 05m");
}
