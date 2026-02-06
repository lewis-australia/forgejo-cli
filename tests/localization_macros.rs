use forgejo_cli::{ftl_eprintln, ftl_format, ftl_println, ftl_write};

use std::fmt::Write;

#[test]
fn localization_macros() {
    std::env::set_var("LANG", "en-US");

    let s = ftl_format!(test_hello);
    assert_eq!(s, "Hello");

    let mut s = String::new();
    ftl_write!(&mut s, test_hello).unwrap();
    assert_eq!(s, "Hello");

    ftl_println!(test_hello);
    ftl_eprintln!(test_hello);
}
