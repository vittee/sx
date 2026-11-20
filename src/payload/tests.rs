use super::*;
use flate2::read::GzDecoder;
use std::io::Read;

fn decode(payload: &str) -> String {
    let bytes = STANDARD.decode(payload).unwrap();
    let mut out = String::new();
    GzDecoder::new(&bytes[..]).read_to_string(&mut out).unwrap();
    out
}

#[test]
fn roundtrip_prepends_server_bashrc() {
    let out = decode(&encode("alias ll='ls -l'\n").unwrap());
    assert!(out.starts_with(SERVER_RC_PRELUDE));
    assert!(out.ends_with("alias ll='ls -l'\n"));
}

#[test]
fn crlf_is_normalised() {
    let out = decode(&encode("a\r\nb\r\n").unwrap());
    assert!(!out.contains('\r'));
}

#[test]
fn payload_has_no_whitespace() {
    let p = encode(&"export X=1\n".repeat(200)).unwrap();
    assert!(!p.contains(char::is_whitespace));
}
