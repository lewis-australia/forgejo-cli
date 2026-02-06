use forgejo_cli::ftl_format;

#[test]
fn localization_fallback() {
    std::env::set_var("LANG", "tok");
    let s = ftl_format!(test_fallback_only_english);
    assert_eq!(s, "This message is only in english");
}
