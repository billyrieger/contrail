#[test]
fn html_root_url() {
    version_sync::assert_html_root_url_updated!("src/lib.rs");
}