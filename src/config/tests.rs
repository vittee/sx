use super::*;

fn write(dir: &Path, file: &str, text: &str) {
    fs::create_dir_all(snippets_dir(dir)).unwrap();
    fs::write(snippets_dir(dir).join(file), text).unwrap();
}

#[test]
fn snippets_are_listed_sorted_and_only_sh_files() {
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), "20-b.sh", "");
    write(tmp.path(), "10-a.sh", "");
    write(tmp.path(), "notes.txt", "");
    assert_eq!(list_snippets(tmp.path()), ["10-a", "20-b"]);
}

#[test]
fn build_rc_joins_snippets_in_name_order_with_headers() {
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), "20-b.sh", "echo b");
    write(tmp.path(), "10-a.sh", "echo a\n");
    assert_eq!(build_rc(tmp.path()), "# --- 10-a ---\necho a\n# --- 20-b ---\necho b\n");
}

#[test]
fn no_snippets_gives_an_empty_rc() {
    let tmp = tempfile::tempdir().unwrap();
    assert_eq!(build_rc(tmp.path()), "");
}

#[test]
fn init_creates_the_sample_once_and_keeps_existing_snippets() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(ensure_initialized(tmp.path()).unwrap());
    assert_eq!(list_snippets(tmp.path()), ["20-aliases"]);
    write(tmp.path(), "20-aliases.sh", "alias x=y\n");
    assert!(!ensure_initialized(tmp.path()).unwrap());
    assert_eq!(build_rc(tmp.path()), "# --- 20-aliases ---\nalias x=y\n");
}
