use forgejo_cli::ftl_format;

#[test]
fn placeables() {
    std::env::set_var("LANG", "en-US");

    let s = ftl_format!(test_placeables, name = "Tester");
    assert_eq!(s, "Hello, Tester! You're Tester.");
}

#[test]
fn switch() {
    std::env::set_var("LANG", "en-US");

    assert_eq!(ftl_format!(test_switch, n = 0), "0 things");
    assert_eq!(ftl_format!(test_switch, n = 1), "A thing");
    assert_eq!(ftl_format!(test_switch, n = 2), "2 things");
}
