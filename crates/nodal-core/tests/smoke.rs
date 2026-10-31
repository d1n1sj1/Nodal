#[test]
fn version_is_cargo_pkg_version() {
    assert_eq!(nodal_core::version(), env!("CARGO_PKG_VERSION"));
}
