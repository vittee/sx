mod payload;

use std::io::Read;

/// Reads an rc from stdin and prints the value `sx` will send in `LC_MONETARY`.
fn main() {
    let mut rc = String::new();
    std::io::stdin().read_to_string(&mut rc).expect("cannot read stdin");
    let encoded = payload::encode(&rc).expect("cannot encode rc");
    if encoded.len() > payload::WARN_ENCODED_LEN {
        eprintln!("sx: payload is {} bytes, the server may reject it", encoded.len());
    }
    println!("{encoded}");
}
